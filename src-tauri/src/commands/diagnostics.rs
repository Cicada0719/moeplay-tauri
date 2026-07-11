use crate::db::Database;
use crate::domain::{ProviderError, ProviderErrorKind};
use crate::task_queue::{JobOperation, TaskCenterJob, TaskEventLevel, TaskQueue, TaskStatus};
use crate::{diagnostics, performance};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub fn get_performance_snapshot(db: State<'_, Database>) -> performance::PerformanceSnapshot {
    performance::snapshot(&db)
}

#[tauri::command]
pub fn run_diagnostics(
    db: State<'_, Database>,
    queue: State<'_, TaskQueue>,
) -> diagnostics::DiagnosticsReport {
    let task_id = begin_task(&queue, ObservedDiagnosticsOperation::Run);
    mark_running(
        &queue,
        task_id.as_deref(),
        ObservedDiagnosticsOperation::Run,
    );
    let (data_dir, count, db_size) = diagnostics_context(&db);
    let report = diagnostics::run_diagnostics(&data_dir, count, db_size);
    finish_task(
        &queue,
        task_id.as_deref(),
        ObservedDiagnosticsOperation::Run,
        true,
    );
    report
}

#[tauri::command]
pub fn export_diagnostics_zip(
    db: State<'_, Database>,
    queue: State<'_, TaskQueue>,
) -> Result<String, String> {
    observe_result_task(&queue, ObservedDiagnosticsOperation::Export, || {
        let (data_dir, count, db_size) = diagnostics_context(&db);
        let export_path = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(format!(
                "moeplay_diagnostics_{}_{}.zip",
                chrono::Utc::now().format("%Y%m%d_%H%M%S%3f"),
                uuid::Uuid::new_v4()
            ));

        diagnostics::export_diagnostics_zip(&export_path, &data_dir, count, db_size)
    })
}

/// A redacted result of a backend-owned updater probe. The signed updater
/// resource remains inside Tauri; the frontend receives only whether a newer
/// release was found and the Task Center projection for observability.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub task: TaskCenterJob,
    pub available: bool,
}

/// Performs the actual signed-updater check in the Tauri backend and records
/// its honest lifecycle. This command intentionally does not return download
/// URLs, signatures, headers, or the updater resource to the webview.
#[tauri::command]
pub async fn start_update_check_task(
    app: AppHandle,
    queue: State<'_, TaskQueue>,
) -> Result<UpdateCheckResult, String> {
    let task = queue.enqueue_operation(
        "Check for updates".to_string(),
        JobOperation::UpdateCheck,
        None,
    )?;
    execute_update_check(&app, queue.inner(), &task.id).await
}

/// Fixed backend dispatcher for persisted Stage 3 operations encountered at
/// startup. Only `UpdateCheck` has a complete, payload-free worker today; all
/// other queued operations are explicitly failed rather than silently left
/// queued or replayed without their required runtime-only input.
pub async fn dispatch_queued_operations(app: AppHandle, queue: TaskQueue) {
    let Ok(jobs) = queue.list_task_center(Some(TaskStatus::Queued), None, Some(500)) else {
        return;
    };
    for job in jobs {
        let operation = queue
            .get_task_detail(&job.id)
            .ok()
            .and_then(|detail| detail.operation)
            .map(|operation| operation.operation);
        match operation {
            Some(JobOperation::UpdateCheck) => {
                let _ = execute_update_check(&app, &queue, &job.id).await;
            }
            Some(_) => {
                let _ = queue.mark_failed(
                    &job.id,
                    ProviderError {
                        kind: ProviderErrorKind::Unknown,
                        message: "Queued task could not be safely replayed after a restart"
                            .to_string(),
                        retryable: true,
                        retry_after_ms: None,
                        provider_id: None,
                        operation: Some("process_restart".to_string()),
                    },
                );
                let _ = queue.append_event(
                    &job.id,
                    TaskEventLevel::Warn,
                    "job_dispatch.restart_not_replayable".to_string(),
                    "Queued task requires runtime-only input and was not replayed".to_string(),
                    None,
                );
            }
            None => {}
        }
    }
}

async fn execute_update_check(
    app: &AppHandle,
    queue: &TaskQueue,
    task_id: &str,
) -> Result<UpdateCheckResult, String> {
    queue.append_event(
        task_id,
        TaskEventLevel::Info,
        "update_check.queued".to_string(),
        "Update check queued".to_string(),
        Some(0.0),
    )?;
    queue.mark_running(
        task_id,
        Some("Checking signed update feed".to_string()),
        Some(0.1),
    )?;
    queue.append_event(
        task_id,
        TaskEventLevel::Info,
        "update_check.running".to_string(),
        "Checking signed update feed".to_string(),
        Some(0.1),
    )?;

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(error) => return fail_update_check(queue, task_id, &error.to_string()),
    };
    let available = match updater.check().await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(error) => return fail_update_check(queue, task_id, &error.to_string()),
    };

    let message = if available {
        "Update available"
    } else {
        "No update available"
    };
    let completed = queue.mark_succeeded(task_id, Some(message.to_string()))?;
    queue.append_event(
        task_id,
        TaskEventLevel::Info,
        if available {
            "update_check.available".to_string()
        } else {
            "update_check.not_available".to_string()
        },
        message.to_string(),
        Some(1.0),
    )?;
    Ok(UpdateCheckResult {
        task: completed,
        available,
    })
}

fn fail_update_check(
    queue: &TaskQueue,
    task_id: &str,
    raw_error: &str,
) -> Result<UpdateCheckResult, String> {
    // Do not persist endpoint, response, or transport details. They may carry
    // an updater URL or credentials supplied by a corporate proxy.
    let kind = if raw_error.to_ascii_lowercase().contains("timeout") {
        ProviderErrorKind::Timeout
    } else {
        ProviderErrorKind::Network
    };
    let _ = queue.mark_failed(
        task_id,
        ProviderError {
            kind,
            message: "Update check failed".to_string(),
            retryable: true,
            retry_after_ms: None,
            provider_id: None,
            operation: Some("update_check".to_string()),
        },
    );
    let _ = queue.append_event(
        task_id,
        TaskEventLevel::Error,
        "update_check.failed".to_string(),
        "Update check failed".to_string(),
        None,
    );
    Err("update_check_failed".to_string())
}

fn diagnostics_context(db: &Database) -> (PathBuf, u32, u64) {
    let count = db.game_count() as u32;
    let data_dir = dirs::data_dir().unwrap_or_default().join("moeplay");
    let db_size = std::fs::metadata(data_dir.join("moegame.db"))
        .map(|m| m.len())
        .unwrap_or(0);
    (data_dir, count, db_size)
}

/// The operation enum currently has `DiagnosticsExport` but no diagnostics-run
/// variant, so the non-export run is deliberately projected to that fixed
/// diagnostics type while its timeline retains the `diagnostics_run.*` phases.
#[derive(Clone, Copy)]
enum ObservedDiagnosticsOperation {
    Run,
    Export,
}

impl ObservedDiagnosticsOperation {
    fn operation(self) -> JobOperation {
        match self {
            Self::Run | Self::Export => JobOperation::DiagnosticsExport,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::Run => "diagnostics_run",
            Self::Export => "diagnostics_export",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Run => "Run diagnostics",
            Self::Export => "Export diagnostics archive",
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

fn begin_task(queue: &TaskQueue, operation: ObservedDiagnosticsOperation) -> Option<String> {
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
            tracing::warn!(operation = operation.code(), error = %error, "failed to create diagnostics task");
            None
        }
    }
}

fn mark_running(queue: &TaskQueue, task_id: Option<&str>, operation: ObservedDiagnosticsOperation) {
    record_phase(queue, task_id, operation, TaskPhase::Running);
}

fn finish_task(
    queue: &TaskQueue,
    task_id: Option<&str>,
    operation: ObservedDiagnosticsOperation,
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
    operation: ObservedDiagnosticsOperation,
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
        // Never persist the raw diagnostics/export failure: it can contain a
        // user path or an archive/provider detail.
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
        // The queue rejects late updates after cancellation; that race is
        // expected and must not alter a legacy command result.
        tracing::debug!(task_id, operation = operation.code(), error = %error, "diagnostics task lifecycle update skipped");
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
    operation: ObservedDiagnosticsOperation,
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
        tracing::debug!(task_id, operation = operation.code(), error = %error, "diagnostics task phase event skipped");
    }
}

fn observe_result_task<T>(
    queue: &TaskQueue,
    operation: ObservedDiagnosticsOperation,
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let task_id = begin_task(queue, operation);
    mark_running(queue, task_id.as_deref(), operation);
    let result = work();
    finish_task(queue, task_id.as_deref(), operation, result.is_ok());
    result
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
    fn diagnostics_export_failure_is_redacted_in_the_task_lifecycle() {
        let queue = queue();
        let error = "archive failed at C:\\Users\\alice\\secret.zip?token=private";
        assert_eq!(
            observe_result_task(
                &queue,
                ObservedDiagnosticsOperation::Export,
                || Err::<(), _>(error.to_string())
            ),
            Err(error.to_string())
        );
        let task = queue
            .list_task_center(None, Some(TaskKind::Diagnostics), Some(1))
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(
            queue
                .get_task_detail(&task.id)
                .unwrap()
                .operation
                .unwrap()
                .operation,
            JobOperation::DiagnosticsExport
        );
        let persisted_events = queue
            .list_events(&task.id, None, 20)
            .unwrap()
            .into_iter()
            .map(|event| format!("{}:{}", event.code, event.message))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(persisted_events.contains("diagnostics_export.failed"));
        assert!(!persisted_events.contains("alice"));
        assert!(!persisted_events.contains("token=private"));
    }

    #[test]
    fn update_check_failure_is_recorded_without_persisting_transport_details() {
        let queue = queue();
        let task = queue
            .enqueue_operation(
                "Check for updates".to_string(),
                JobOperation::UpdateCheck,
                None,
            )
            .unwrap();
        queue.mark_running(&task.id, None, None).unwrap();
        assert_eq!(
            fail_update_check(
                &queue,
                &task.id,
                "timeout at https://alice:secret@example.test/latest.json?token=private",
            )
            .unwrap_err(),
            "update_check_failed"
        );
        let detail = queue.get_task_detail(&task.id).unwrap();
        assert_eq!(detail.job.status, TaskStatus::Failed);
        assert_eq!(detail.job.message.as_deref(), Some("Update check failed"));
        let persisted = queue
            .list_events(&task.id, None, 20)
            .unwrap()
            .into_iter()
            .map(|event| format!("{}:{}", event.code, event.message))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(persisted.contains("update_check.failed"));
        assert!(!persisted.contains("alice"));
        assert!(!persisted.contains("secret"));
        assert!(!persisted.contains("token=private"));
    }
}
