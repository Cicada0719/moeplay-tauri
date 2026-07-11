//! Stage 4 extension-directory command surface.
//!
//! Coordinator integration only (this sidecar intentionally does not modify
//! shared registration files): declare/re-export this module, manage one
//! ExtensionIndexService::default() state, and register both commands and
//! capability/build allow-list entries.

use crate::{
    domain::{ProviderError, ProviderErrorKind},
    extension_index::{
        ExtensionIndexError, ExtensionIndexErrorKind, ExtensionIndexRefresh, ExtensionIndexService,
        ExtensionIndexSnapshot,
    },
    task_queue::{JobOperation, TaskCenterJob, TaskEventLevel, TaskQueue},
};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionIndexCommandError {
    pub kind: ProviderErrorKind,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionIndexRefreshResponse {
    pub task: TaskCenterJob,
    pub refresh: ExtensionIndexRefresh,
}

/// Refreshes metadata only. The TaskQueue stores a one-way endpoint fingerprint,
/// never the endpoint itself, package URLs, credentials, or response body.
#[tauri::command]
pub async fn refresh_extension_index(
    service: State<'_, ExtensionIndexService>,
    queue: State<'_, TaskQueue>,
    endpoint: String,
    force: Option<bool>,
) -> Result<ExtensionIndexRefreshResponse, ExtensionIndexCommandError> {
    let fingerprint = service
        .endpoint_fingerprint(&endpoint)
        .map_err(command_error)?;
    let task = queue
        .enqueue_operation(
            "Refresh extension directory".to_string(),
            JobOperation::ProviderVerify {
                media_type: "extension_index".to_string(),
                provider_id: format!("extension-index:{fingerprint}"),
            },
            None,
        )
        .map_err(queue_error)?;
    let cancellation = queue.register_operation(&task.id).map_err(queue_error)?;
    queue
        .mark_running(
            &task.id,
            Some("Refreshing extension directory metadata".to_string()),
            Some(0.1),
        )
        .map_err(queue_error)?;
    queue
        .append_event(
            &task.id,
            TaskEventLevel::Info,
            "extension_index.refresh_started".to_string(),
            "Refreshing extension directory metadata".to_string(),
            Some(0.1),
        )
        .map_err(queue_error)?;
    if cancellation.check_cancelled().is_err() {
        return Err(cancelled(&queue, &task.id));
    }

    match service.refresh(&endpoint, force.unwrap_or(false)).await {
        Ok(refresh) => {
            if cancellation.check_cancelled().is_err() {
                return Err(cancelled(&queue, &task.id));
            }
            let (code, message) = match refresh.state {
                crate::extension_index::ExtensionIndexRefreshState::FreshCache => (
                    "extension_index.fresh_cache",
                    "Using fresh extension directory snapshot",
                ),
                crate::extension_index::ExtensionIndexRefreshState::Refreshed => (
                    "extension_index.refreshed",
                    "Extension directory metadata refreshed",
                ),
                crate::extension_index::ExtensionIndexRefreshState::NotModified => (
                    "extension_index.not_modified",
                    "Extension directory metadata is unchanged",
                ),
                crate::extension_index::ExtensionIndexRefreshState::OfflineSnapshot => (
                    "extension_index.offline_snapshot",
                    "Using cached extension directory snapshot while offline",
                ),
            };
            let completed = queue
                .mark_succeeded(&task.id, Some(message.to_string()))
                .map_err(queue_error)?;
            queue
                .append_event(
                    &task.id,
                    TaskEventLevel::Info,
                    code.to_string(),
                    message.to_string(),
                    Some(1.0),
                )
                .map_err(queue_error)?;
            Ok(ExtensionIndexRefreshResponse {
                task: completed,
                refresh,
            })
        }
        Err(error) => {
            let command = command_error(error);
            let _ = queue.mark_failed(
                &task.id,
                ProviderError {
                    kind: command.kind,
                    message: command.message.clone(),
                    retryable: command.retryable,
                    retry_after_ms: None,
                    provider_id: Some(format!("extension-index:{fingerprint}")),
                    operation: Some("refresh_extension_index".to_string()),
                },
            );
            let _ = queue.append_event(
                &task.id,
                TaskEventLevel::Error,
                format!("extension_index.failed.{}", command.code),
                command.message.clone(),
                None,
            );
            Err(command)
        }
    }
}

#[tauri::command]
pub fn get_extension_index_snapshot(
    service: State<'_, ExtensionIndexService>,
    endpoint: String,
) -> Result<Option<ExtensionIndexSnapshot>, ExtensionIndexCommandError> {
    service.get_snapshot(&endpoint).map_err(command_error)
}

fn command_error(error: ExtensionIndexError) -> ExtensionIndexCommandError {
    let (kind, message) = match error.kind {
        ExtensionIndexErrorKind::PolicyBlocked => (
            ProviderErrorKind::PolicyBlocked,
            "Extension directory endpoint is blocked by policy",
        ),
        ExtensionIndexErrorKind::Timeout => (
            ProviderErrorKind::Timeout,
            "Extension directory refresh timed out",
        ),
        ExtensionIndexErrorKind::Network => (
            ProviderErrorKind::Network,
            "Extension directory is unavailable",
        ),
        ExtensionIndexErrorKind::InvalidMetadata => (
            ProviderErrorKind::ParseChanged,
            "Extension directory returned invalid metadata",
        ),
        ExtensionIndexErrorKind::Storage => (
            ProviderErrorKind::Unknown,
            "Extension directory cache is unavailable",
        ),
    };
    ExtensionIndexCommandError {
        kind,
        code: error.code.to_string(),
        message: message.to_string(),
        retryable: error.retryable,
    }
}
fn queue_error(_: String) -> ExtensionIndexCommandError {
    ExtensionIndexCommandError {
        kind: ProviderErrorKind::Unknown,
        code: "extension_index_task_failed".to_string(),
        message: "Extension directory task could not be recorded".to_string(),
        retryable: true,
    }
}
fn cancelled(queue: &TaskQueue, task_id: &str) -> ExtensionIndexCommandError {
    // Cancellation owns the terminal state; do not overwrite it with a late result.
    let _ = queue.append_event(
        task_id,
        TaskEventLevel::Info,
        "extension_index.cancelled".to_string(),
        "Extension directory refresh was cancelled".to_string(),
        None,
    );
    ExtensionIndexCommandError {
        kind: ProviderErrorKind::Cancelled,
        code: "extension_index_cancelled".to_string(),
        message: "Extension directory refresh was cancelled".to_string(),
        retryable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn errors_are_stable_and_do_not_expose_transport_details() {
        let error = command_error(ExtensionIndexError {
            kind: ExtensionIndexErrorKind::Network,
            code: "extension_index_network_failed",
            retryable: true,
        });
        assert_eq!(error.kind, ProviderErrorKind::Network);
        assert_eq!(error.message, "Extension directory is unavailable");
        assert!(!serde_json::to_string(&error).unwrap().contains("http"));
    }
}
