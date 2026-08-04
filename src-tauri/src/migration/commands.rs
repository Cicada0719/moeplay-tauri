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

/// 查询当前迁移状态（前端启动时首先调用）。
#[tauri::command]
pub async fn migration_status(state: State<'_, AppState>) -> Result<MigrationStatus, String> {
    match &state.migrator {
        Some(migrator) => {
            let migrator = migrator.clone();
            let gate = Arc::clone(&state.migration_status);
            tauri::async_runtime::spawn_blocking(move || {
                let status = migrator.check().map_err(|error| error.to_string())?;
                set_gate(&gate, status.clone());
                Ok(status)
            })
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
    let gate_status = state
        .migration_status
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    ensure_history_available(&gate_status)?;

    let history = history(&state)?;
    let content_type = match content_type {
        Some(raw) if !raw.trim().is_empty() => {
            Some(ContentType::from_str(raw.trim()).map_err(|error| error)?)
        }
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
    let gate_status = state
        .migration_status
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    ensure_history_available(&gate_status)?;

    let history = history(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        history.tombstone(&id).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
