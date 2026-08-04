//! Tauri commands：历史迁移 + v2 历史列表/删除（FR-08 / FR-10 下游接口）。
//!
//! 迁移门控约定：`history_*` 系列命令在迁移状态非 `Completed | NotNeeded`
//! 时返回 `"MIGRATION_PENDING"`，前端据此展示迁移进度页而非历史列表。

use crate::db_sqlite::{HistoryDb, HistoryRepo};
use crate::domain::history::{ContentType, HistoryRecord};
use crate::migration::{MigrationReport, MigrationStatus, Migrator};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tauri::State;

/// 历史/迁移相关的全局应用状态（由 `lib.rs` setup 注入）。
pub struct AppState {
    pub migrator: Option<Migrator>,
    pub history: Option<HistoryDb>,
    pub migration_status: Arc<RwLock<MigrationStatus>>,
}

/// 迁移门控：非 `Completed | NotNeeded` 时拒绝历史读写。
pub(super) fn ensure_history_available(status: &MigrationStatus) -> Result<(), String> {
    if matches!(status, MigrationStatus::Completed | MigrationStatus::NotNeeded) {
        Ok(())
    } else {
        Err("MIGRATION_PENDING".to_string())
    }
}

/// 更新内存门控（供命令与启动迁移任务使用）。
pub(crate) fn set_gate(gate: &Arc<RwLock<MigrationStatus>>, status: MigrationStatus) {
    if let Ok(mut guard) = gate.write() {
        *guard = status;
    }
}

fn migrator(state: &AppState) -> Result<Migrator, String> {
    state
        .migrator
        .clone()
        .ok_or_else(|| "history database unavailable".to_string())
}

fn history(state: &AppState) -> Result<HistoryDb, String> {
    state
        .history
        .clone()
        .ok_or_else(|| "history database unavailable".to_string())
}

/// 以 `migrator.check()`（读 `migration_state` 落盘状态）刷新内存门控并返回刷新后的状态。
fn check_and_update_gate(
    migrator: &Migrator,
    gate: &Arc<RwLock<MigrationStatus>>,
) -> Result<MigrationStatus, String> {
    let status = migrator.check().map_err(|error| error.to_string())?;
    set_gate(gate, status.clone());
    Ok(status)
}

/// 刷新内存门控：以落盘状态为准（同步；调用方在 async 上下文需自行 `spawn_blocking`）。
///
/// 启动迁移在 `spawn_blocking` 中运行时，内存门控可能仍停留在 `Pending/InProgress`，
/// 即使后端已经完成。`history_*` 命令入口先调用本函数，避免迁移完成后前端仍收到
/// `MIGRATION_PENDING`。
pub(crate) fn refresh_gate(state: &AppState) -> Result<MigrationStatus, String> {
    match &state.migrator {
        Some(migrator) => check_and_update_gate(migrator, &state.migration_status),
        None => Ok(state
            .migration_status
            .read()
            .map_err(|error| error.to_string())?
            .clone()),
    }
}

/// 查询当前迁移状态（前端启动时首先调用）。
///
/// 在 `spawn_blocking` 中执行 `check()`：迁移批量写入持有数据库锁时，避免阻塞 async 执行器。
#[tauri::command]
pub async fn migration_status(state: State<'_, AppState>) -> Result<MigrationStatus, String> {
    match &state.migrator {
        Some(migrator) => {
            let migrator = migrator.clone();
            let gate = Arc::clone(&state.migration_status);
            tauri::async_runtime::spawn_blocking(move || check_and_update_gate(&migrator, &gate))
                .await
                .map_err(|error| error.to_string())?
        }
        None => Ok(state
            .migration_status
            .read()
            .map_err(|error| error.to_string())?
            .clone()),
    }
}

/// 立即执行迁移（同步阻塞，在 spawn_blocking 中运行）。
#[tauri::command]
pub async fn migration_run(state: State<'_, AppState>) -> Result<MigrationReport, String> {
    let migrator = migrator(&state)?;
    let gate = Arc::clone(&state.migration_status);
    tauri::async_runtime::spawn_blocking(move || {
        let report = migrator.run().map_err(|error| error.to_string())?;
        set_gate(&gate, report.status.clone());
        Ok(report)
    })
    .await
    .map_err(|error| error.to_string())?
}

/// “从备份恢复”入口（R4 应对）。
#[tauri::command]
pub async fn migration_restore_backup(
    path: String,
    state: State<'_, AppState>,
) -> Result<MigrationReport, String> {
    let migrator = migrator(&state)?;
    let gate = Arc::clone(&state.migration_status);
    tauri::async_runtime::spawn_blocking(move || {
        let report = migrator
            .restore_from_backup(Path::new(&path))
            .map_err(|error| error.to_string())?;
        set_gate(&gate, report.status.clone());
        Ok(report)
    })
    .await
    .map_err(|error| error.to_string())?
}

/// 分页历史列表（类型筛选 + 关键词模糊匹配 + `updated_at DESC`）。
#[tauri::command]
pub async fn history_list(
    content_type: Option<String>,
    keyword: Option<String>,
    limit: u32,
    offset: u32,
    state: State<'_, AppState>,
) -> Result<Vec<HistoryRecord>, String> {
    // 先刷新内存门控：spawn_blocking 迁移可能刚完成但内存状态未更新。
    let gate_status = refresh_gate(&state)?;
    ensure_history_available(&gate_status)?;

    let history = history(&state)?;
    // `ContentType::from_str` 的 Err 就是 `String`，与命令签名一致，直接 `?` 传播。
    let content_type = match content_type {
        Some(raw) if !raw.trim().is_empty() => Some(ContentType::from_str(raw.trim())?),
        _ => None,
    };
    let keyword = keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    tauri::async_runtime::spawn_blocking(move || {
        history
            .list(content_type, keyword.as_deref(), limit, offset)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// 墓碑删除单条历史。
#[tauri::command]
pub async fn history_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    // 先刷新内存门控：spawn_blocking 迁移可能刚完成但内存状态未更新。
    let gate_status = refresh_gate(&state)?;
    ensure_history_available(&gate_status)?;

    let history = history(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        history.tombstone(&id).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
