use crate::db::Database;
use crate::domain::{ProviderError, ProviderErrorKind};
use crate::models::{Game, SaveBackup, SaveData, SaveInfo};
use crate::sync;
use crate::task_queue::{JobOperation, TaskEventLevel, TaskQueue};
use std::{fs, path::PathBuf};
use tauri::State;

// ===== Save data =====

#[tauri::command]
pub fn update_save_data(
    db: State<'_, Database>,
    id: String,
    save_data: SaveData,
) -> Result<Game, String> {
    db.update_save_data(&id, save_data)
}

#[tauri::command]
pub fn set_save_dir(
    db: State<'_, Database>,
    id: String,
    save_dir: Option<String>,
) -> Result<Game, String> {
    db.set_save_dir(&id, save_dir)
}

#[tauri::command]
pub fn configure_auto_backup(
    db: State<'_, Database>,
    id: String,
    auto_backup: bool,
    interval_minutes: u32,
    max_backups: u32,
) -> Result<Game, String> {
    db.configure_auto_backup(&id, auto_backup, interval_minutes, max_backups)
}

#[tauri::command]
pub fn add_game_backup(
    db: State<'_, Database>,
    id: String,
    backup: SaveBackup,
) -> Result<Game, String> {
    db.add_backup(&id, backup)
}

#[tauri::command]
pub fn remove_game_backup(
    db: State<'_, Database>,
    id: String,
    backup_id: String,
) -> Result<Game, String> {
    db.remove_backup(&id, &backup_id)
}

#[tauri::command]
pub fn update_backup_note(
    db: State<'_, Database>,
    id: String,
    backup_id: String,
    note: Option<String>,
) -> Result<Game, String> {
    db.update_backup_note(&id, &backup_id, note)
}

#[tauri::command]
pub fn configure_cloud_sync(
    db: State<'_, Database>,
    id: String,
    cloud_sync: bool,
    cloud_provider: Option<String>,
) -> Result<Game, String> {
    db.configure_cloud_sync(&id, cloud_sync, cloud_provider)
}

// ===== Save files and snapshots =====

#[tauri::command]
pub fn get_game_saves(game_id: String, db: State<'_, Database>) -> Result<Vec<SaveInfo>, String> {
    let games = db.get_games();
    let game = games.iter().find(|g| g.id == game_id).ok_or("游戏不存在")?;

    let exe_path = PathBuf::from(&game.exe_path);
    let game_dir = exe_path.parent().ok_or("无法获取游戏目录")?;

    let save_dirs = vec![
        game_dir.join("Save"),
        game_dir.join("Saves"),
        game_dir.join("save"),
        game_dir.join("saves"),
        game_dir.join("UserData"),
        dirs::document_dir()
            .unwrap_or_default()
            .join("My Games")
            .join(&game.name),
        dirs::data_local_dir().unwrap_or_default().join(&game.name),
    ];

    let mut saves = vec![];
    for save_dir in save_dirs {
        if save_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&save_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let metadata = fs::metadata(&path).ok();
                        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                        let created = metadata
                            .and_then(|m| m.modified().ok())
                            .map(|t| {
                                chrono::DateTime::<chrono::Utc>::from(t)
                                    .format("%Y-%m-%d %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_default();
                        saves.push(SaveInfo {
                            name: path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_string(),
                            path: path.to_string_lossy().to_string(),
                            size,
                            created,
                        });
                    }
                }
            }
        }
    }

    Ok(saves)
}

#[tauri::command]
pub fn backup_save(queue: State<'_, TaskQueue>, save_path: String) -> Result<String, String> {
    observe_result_task(&queue, ObservedSaveOperation::BackupFile, || {
        backup_save_impl(save_path)
    })
}

fn backup_save_impl(save_path: String) -> Result<String, String> {
    let src = PathBuf::from(&save_path);
    if !src.exists() {
        return Err("存档文件不存在".to_string());
    }

    // 安全作用域：应用数据目录 + 该存档所在的目录
    let mut scope = crate::security::app_data_scope().unwrap_or_default();
    if let Some(parent) = src.parent() {
        scope.allow(parent);
    }
    let src = scope
        .resolve(&src)
        .map_err(|_| "路径不在允许范围内".to_string())?;

    let backup_dir = dirs::data_dir()
        .unwrap_or_default()
        .join("moeplay")
        .join("saves");

    fs::create_dir_all(&backup_dir).ok();

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let file_name = src.file_name().and_then(|s| s.to_str()).unwrap_or("save");
    let backup_name = format!("{}_{}", timestamp, file_name);
    let backup_path = backup_dir.join(&backup_name);

    fs::copy(&src, &backup_path).map_err(|e| format!("备份失败: {}", e))?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn restore_save(
    queue: State<'_, TaskQueue>,
    backup_path: String,
    target_path: String,
) -> Result<(), String> {
    observe_result_task(&queue, ObservedSaveOperation::RestoreFile, || {
        restore_save_impl(backup_path, target_path)
    })
}

fn restore_save_impl(backup_path: String, target_path: String) -> Result<(), String> {
    let src = PathBuf::from(&backup_path);
    let dst = PathBuf::from(&target_path);

    if !src.exists() {
        return Err("备份文件不存在".to_string());
    }

    // 安全作用域：应用数据目录 + 两端各自的父目录（即游戏存档目录）
    let mut scope = crate::security::app_data_scope().unwrap_or_default();
    if let Some(parent) = src.parent() {
        scope.allow(parent);
    }
    if let Some(parent) = dst.parent() {
        scope.allow(parent);
    }

    let src = scope
        .resolve(&src)
        .map_err(|_| "路径不在允许范围内".to_string())?;
    let dst = scope
        .resolve(&dst)
        .map_err(|_| "路径不在允许范围内".to_string())?;

    fs::copy(&src, &dst).map_err(|e| format!("恢复失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn detect_save_candidates(
    db: State<'_, Database>,
    game_id: String,
) -> Result<Vec<sync::SaveCandidateDir>, String> {
    let game = db.get_game(&game_id)?;
    let game_dir = resolve_game_dir(&game)?;
    Ok(sync::detect_save_candidates(
        &game_dir,
        &game.name,
        game.metadata.developer.as_deref(),
        game.metadata.engine.as_deref(),
    ))
}

#[tauri::command]
pub fn scan_save_dir(save_dir: String) -> Result<Vec<sync::SaveInfo>, String> {
    let path = PathBuf::from(save_dir);
    if !path.is_dir() {
        return Err("存档目录不存在".to_string());
    }
    Ok(sync::scan_saves(&path))
}

#[tauri::command]
pub fn create_save_snapshot(
    db: State<'_, Database>,
    queue: State<'_, TaskQueue>,
    game_id: String,
    save_dir: Option<String>,
    note: Option<String>,
) -> Result<sync::SaveSnapshot, String> {
    observe_result_task(&queue, ObservedSaveOperation::CreateSnapshot, || {
        create_save_snapshot_impl(&db, game_id, save_dir, note)
    })
}

fn create_save_snapshot_impl(
    db: &Database,
    game_id: String,
    save_dir: Option<String>,
    note: Option<String>,
) -> Result<sync::SaveSnapshot, String> {
    let game = db.get_game(&game_id)?;
    let save_dir = resolve_game_save_dir(&game, save_dir)?;
    sync::create_snapshot(&game_id, &save_dir, note.as_deref())
}

#[tauri::command]
pub fn list_save_snapshots(game_id: String) -> Result<Vec<sync::SaveSnapshot>, String> {
    Ok(sync::list_snapshots(&game_id))
}

#[tauri::command]
pub fn restore_save_snapshot(
    db: State<'_, Database>,
    queue: State<'_, TaskQueue>,
    game_id: String,
    snapshot_path: String,
    save_dir: Option<String>,
    create_safety: Option<bool>,
) -> Result<(), String> {
    observe_result_task(&queue, ObservedSaveOperation::RestoreSnapshot, || {
        restore_save_snapshot_impl(&db, game_id, snapshot_path, save_dir, create_safety)
    })
}

fn restore_save_snapshot_impl(
    db: &Database,
    game_id: String,
    snapshot_path: String,
    save_dir: Option<String>,
    create_safety: Option<bool>,
) -> Result<(), String> {
    let game = db.get_game(&game_id)?;
    let save_dir = resolve_game_save_dir(&game, save_dir)?;
    sync::restore_snapshot(
        &game_id,
        &PathBuf::from(snapshot_path),
        &save_dir,
        create_safety.unwrap_or(true),
    )
}

#[tauri::command]
pub fn delete_save_snapshot(snapshot_path: String) -> Result<(), String> {
    sync::delete_snapshot(&PathBuf::from(snapshot_path))
}

#[tauri::command]
pub fn compare_save_snapshot(
    snapshot_path: String,
    save_dir: String,
) -> Result<sync::SnapshotDiff, String> {
    sync::compare_snapshot(&PathBuf::from(snapshot_path), &PathBuf::from(save_dir))
}

#[tauri::command]
pub fn detect_save_conflicts(
    local_dir: String,
    remote_dir: String,
) -> Result<Vec<sync::SaveConflict>, String> {
    let local = PathBuf::from(local_dir);
    let remote = PathBuf::from(remote_dir);
    if !local.is_dir() {
        return Err("本地目录不存在".to_string());
    }
    if !remote.is_dir() {
        return Err("远端目录不存在".to_string());
    }
    Ok(sync::detect_conflicts(&local, &remote))
}

#[tauri::command]
pub async fn sync_save_snapshots_to_cloud(
    queue: State<'_, TaskQueue>,
    game_id: String,
    config: sync::CloudSyncConfig,
) -> Result<u32, String> {
    let task_id = begin_task(&queue, ObservedSaveOperation::CloudSnapshotSync);
    mark_running(
        &queue,
        task_id.as_deref(),
        ObservedSaveOperation::CloudSnapshotSync,
    );
    let result = sync::sync_snapshots_to_cloud(&game_id, &config).await;
    finish_task(
        &queue,
        task_id.as_deref(),
        ObservedSaveOperation::CloudSnapshotSync,
        result.is_ok(),
    );
    result
}

#[tauri::command]
pub fn restore_latest_save_snapshot_from_cloud(
    db: State<'_, Database>,
    queue: State<'_, TaskQueue>,
    game_id: String,
    cloud_dir: String,
    save_dir: Option<String>,
) -> Result<Option<sync::SaveSnapshot>, String> {
    observe_result_task(&queue, ObservedSaveOperation::RestoreCloudSnapshot, || {
        restore_latest_save_snapshot_from_cloud_impl(&db, game_id, cloud_dir, save_dir)
    })
}

fn restore_latest_save_snapshot_from_cloud_impl(
    db: &Database,
    game_id: String,
    cloud_dir: String,
    save_dir: Option<String>,
) -> Result<Option<sync::SaveSnapshot>, String> {
    let game = db.get_game(&game_id)?;
    let save_dir = resolve_game_save_dir(&game, save_dir)?;
    sync::restore_latest_snapshot_from_local_cloud(&game_id, &PathBuf::from(cloud_dir), &save_dir)
}

/// Fixed backend-owned save operations. Only stable scope/snapshot tokens are
/// serialized; filesystem paths, cloud configuration, snapshot notes, and
/// command arguments never enter the durable job envelope.
#[derive(Clone, Copy)]
enum ObservedSaveOperation {
    BackupFile,
    RestoreFile,
    CreateSnapshot,
    RestoreSnapshot,
    CloudSnapshotSync,
    RestoreCloudSnapshot,
}

impl ObservedSaveOperation {
    fn operation(self) -> JobOperation {
        match self {
            Self::BackupFile => JobOperation::Backup {
                scope: "save_file".to_string(),
            },
            Self::CreateSnapshot => JobOperation::Backup {
                scope: "snapshot".to_string(),
            },
            Self::CloudSnapshotSync => JobOperation::Backup {
                scope: "cloud_snapshot_sync".to_string(),
            },
            Self::RestoreFile => JobOperation::Restore {
                snapshot_id: "save_file".to_string(),
            },
            Self::RestoreSnapshot => JobOperation::Restore {
                snapshot_id: "snapshot".to_string(),
            },
            Self::RestoreCloudSnapshot => JobOperation::Restore {
                snapshot_id: "cloud_snapshot".to_string(),
            },
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::BackupFile => "backup_save",
            Self::RestoreFile => "restore_save",
            Self::CreateSnapshot => "create_save_snapshot",
            Self::RestoreSnapshot => "restore_save_snapshot",
            Self::CloudSnapshotSync => "sync_save_snapshots_to_cloud",
            Self::RestoreCloudSnapshot => "restore_latest_save_snapshot_from_cloud",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::BackupFile => "Back up save file",
            Self::RestoreFile => "Restore save file",
            Self::CreateSnapshot => "Create save snapshot",
            Self::RestoreSnapshot => "Restore save snapshot",
            Self::CloudSnapshotSync => "Sync save snapshots to cloud",
            Self::RestoreCloudSnapshot => "Restore latest cloud save snapshot",
        }
    }
}

#[derive(Clone, Copy)]
enum TaskPhase {
    Running,
    Succeeded,
    Failed,
}

impl TaskPhase {
    fn code(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
    fn message(self) -> &'static str {
        match self {
            Self::Running => "Task running",
            Self::Succeeded => "Task completed",
            Self::Failed => "Task failed",
        }
    }
    fn progress(self) -> Option<f64> {
        match self {
            Self::Running => Some(0.05),
            Self::Succeeded => Some(1.0),
            Self::Failed => None,
        }
    }
}

fn begin_task(queue: &TaskQueue, operation: ObservedSaveOperation) -> Option<String> {
    match queue.enqueue_operation(operation.title().to_string(), operation.operation(), None) {
        Ok(task) => {
            append_phase_event(
                queue,
                &task.id,
                operation,
                "queued",
                "Task queued",
                Some(0.0),
            );
            Some(task.id)
        }
        Err(error) => {
            tracing::warn!(operation = operation.code(), error = %error, "failed to create save task");
            None
        }
    }
}

fn mark_running(queue: &TaskQueue, task_id: Option<&str>, operation: ObservedSaveOperation) {
    record_phase(queue, task_id, operation, TaskPhase::Running);
}

fn finish_task(
    queue: &TaskQueue,
    task_id: Option<&str>,
    operation: ObservedSaveOperation,
    succeeded: bool,
) {
    record_phase(
        queue,
        task_id,
        operation,
        if succeeded {
            TaskPhase::Succeeded
        } else {
            TaskPhase::Failed
        },
    );
}

fn record_phase(
    queue: &TaskQueue,
    task_id: Option<&str>,
    operation: ObservedSaveOperation,
    phase: TaskPhase,
) {
    let Some(task_id) = task_id else {
        return;
    };
    let result = match phase {
        TaskPhase::Running => {
            queue.mark_running(task_id, Some(phase.message().to_string()), phase.progress())
        }
        TaskPhase::Succeeded => queue.mark_succeeded(task_id, Some(phase.message().to_string())),
        // Deliberately never copy the command error: it can contain a local
        // path, remote endpoint, or provider-supplied detail.
        TaskPhase::Failed => queue.mark_failed(
            task_id,
            ProviderError {
                kind: ProviderErrorKind::Unknown,
                message: phase.message().to_string(),
                retryable: false,
                retry_after_ms: None,
                provider_id: None,
                operation: Some(operation.code().to_string()),
            },
        ),
    };
    if let Err(error) = result {
        // A cancellation race is expected to reject a late completion. It must
        // not change the result returned by the legacy command.
        tracing::debug!(task_id, operation = operation.code(), error = %error, "save task lifecycle update skipped");
        return;
    }
    append_phase_event(
        queue,
        task_id,
        operation,
        phase.code(),
        phase.message(),
        phase.progress(),
    );
}

fn append_phase_event(
    queue: &TaskQueue,
    task_id: &str,
    operation: ObservedSaveOperation,
    phase: &'static str,
    message: &'static str,
    progress: Option<f64>,
) {
    if let Err(error) = queue.append_event(
        task_id,
        if phase == "failed" {
            TaskEventLevel::Error
        } else {
            TaskEventLevel::Info
        },
        format!("{}.{}", operation.code(), phase),
        message.to_string(),
        progress,
    ) {
        tracing::debug!(task_id, operation = operation.code(), error = %error, "save task phase event skipped");
    }
}

fn observe_result_task<T>(
    queue: &TaskQueue,
    operation: ObservedSaveOperation,
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let task_id = begin_task(queue, operation);
    mark_running(queue, task_id.as_deref(), operation);
    let result = work();
    finish_task(queue, task_id.as_deref(), operation, result.is_ok());
    result
}

pub(crate) fn resolve_game_dir(game: &Game) -> Result<PathBuf, String> {
    if let Some(ref install_dir) = game.install_dir {
        let path = PathBuf::from(install_dir);
        if path.is_dir() {
            return Ok(path);
        }
    }

    let exe_path = PathBuf::from(&game.exe_path);
    exe_path
        .parent()
        .map(|p| p.to_path_buf())
        .filter(|p| p.is_dir())
        .ok_or_else(|| "无法获取游戏目录".to_string())
}

fn resolve_game_save_dir(game: &Game, custom_save_dir: Option<String>) -> Result<PathBuf, String> {
    if let Some(path) = custom_save_dir {
        let path = PathBuf::from(path);
        if path.is_dir() {
            return Ok(path);
        }
        return Err("指定存档目录不存在".to_string());
    }

    if let Some(ref configured) = game.save_data.save_dir {
        let path = PathBuf::from(configured);
        if path.is_dir() {
            return Ok(path);
        }
    }

    let game_dir = resolve_game_dir(game)?;
    sync::detect_save_candidates(
        &game_dir,
        &game.name,
        game.metadata.developer.as_deref(),
        game.metadata.engine.as_deref(),
    )
    .into_iter()
    .next()
    .map(|candidate| PathBuf::from(candidate.path))
    .ok_or_else(|| "未检测到存档目录".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_sqlite::SqliteDb;
    use crate::task_queue::{TaskKind, TaskStatus};
    use std::sync::Arc;

    fn queue() -> TaskQueue {
        TaskQueue::from_database(Arc::new(SqliteDb::open_in_memory().unwrap()))
    }

    #[test]
    fn save_backup_task_records_a_typed_redacted_lifecycle() {
        let queue = queue();
        observe_result_task(&queue, ObservedSaveOperation::BackupFile, || {
            Ok::<_, String>(())
        })
        .unwrap();
        let task = queue
            .list_task_center(None, Some(TaskKind::Backup), Some(1))
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(task.kind, TaskKind::Backup);
        assert_eq!(task.status, TaskStatus::Succeeded);
        assert_eq!(task.message.as_deref(), Some("Task completed"));
        let detail = queue.get_task_detail(&task.id).unwrap();
        assert_eq!(
            detail.operation.unwrap().operation,
            JobOperation::Backup {
                scope: "save_file".to_string()
            }
        );
        let codes = queue
            .list_events(&task.id, None, 20)
            .unwrap()
            .into_iter()
            .map(|event| event.code)
            .collect::<Vec<_>>();
        assert!(codes.contains(&"backup_save.queued".to_string()));
        assert!(codes.contains(&"backup_save.running".to_string()));
        assert!(codes.contains(&"backup_save.succeeded".to_string()));
    }

    #[test]
    fn save_task_failure_does_not_persist_raw_command_error() {
        let queue = queue();
        let error = "sync failed at https://user:secret@example.test?token=private";
        assert_eq!(
            observe_result_task(&queue, ObservedSaveOperation::CloudSnapshotSync, || Err::<
                (),
                _,
            >(
                error.to_string()
            )),
            Err(error.to_string())
        );
        let task = queue
            .list_task_center(None, Some(TaskKind::Backup), Some(1))
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        let persisted_events = queue
            .list_events(&task.id, None, 20)
            .unwrap()
            .into_iter()
            .map(|event| format!("{}:{}", event.code, event.message))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(persisted_events.contains("sync_save_snapshots_to_cloud.failed"));
        assert!(!persisted_events.contains("user:secret"));
        assert!(!persisted_events.contains("token=private"));
    }
}
