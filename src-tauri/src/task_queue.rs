//! Persistent background-job control plane.
//!
//! The public command surface keeps the old `TaskQueue` names, while the storage
//! and state machine are backed by `BackgroundJobRepository` and `BackgroundJob`.
//! Progress is always persisted as a fraction in the inclusive range `0..=1`.

pub use crate::db_sqlite::repositories::jobs::{
    BackgroundJobEvent as TaskEvent, BackgroundJobEventLevel as TaskEventLevel,
};
use crate::db_sqlite::{
    repositories::{jobs::redact_event_message, BackgroundJobRepository},
    SqliteDb,
};
use crate::domain::{BackgroundJob, BackgroundJobStatus, ProviderError, ProviderErrorKind};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[serde(alias = "pending")]
    Queued,
    Running,
    Paused,
    #[serde(alias = "completed")]
    Succeeded,
    Failed,
    Cancelled,
}

impl From<BackgroundJobStatus> for TaskStatus {
    fn from(status: BackgroundJobStatus) -> Self {
        match status {
            BackgroundJobStatus::Queued => Self::Queued,
            BackgroundJobStatus::Running => Self::Running,
            BackgroundJobStatus::Paused => Self::Paused,
            BackgroundJobStatus::Succeeded => Self::Succeeded,
            BackgroundJobStatus::Failed => Self::Failed,
            BackgroundJobStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<TaskStatus> for BackgroundJobStatus {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Queued => Self::Queued,
            TaskStatus::Running => Self::Running,
            TaskStatus::Paused => Self::Paused,
            TaskStatus::Succeeded => Self::Succeeded,
            TaskStatus::Failed => Self::Failed,
            TaskStatus::Cancelled => Self::Cancelled,
        }
    }
}

/// The current on-disk envelope version for backend-owned job operations.
/// New operation variants must use a new envelope version rather than changing
/// the meaning of an existing serialized payload.
pub const JOB_OPERATION_VERSION: u16 = 1;

/// Backend-owned, fixed-dispatch operations that may be queued by Stage 3
/// producers. There is intentionally no "custom command" or arbitrary payload
/// variant: frontend callers cannot smuggle request bodies, headers, URLs with
/// credentials, or command-line arguments into durable job metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobOperation {
    Import {
        source: String,
        reference_id: String,
    },
    Scrape {
        game_id: String,
        provider_id: Option<String>,
    },
    ProviderVerify {
        media_type: String,
        provider_id: String,
    },
    Backup {
        scope: String,
    },
    Restore {
        snapshot_id: String,
    },
    DiagnosticsExport,
    UpdateCheck,
}

/// The durable representation stored under `background_jobs.metadata_json`.
/// The flattening keeps the payload easy to inspect while making the version an
/// explicit compatibility boundary for future workers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionedJobOperation {
    pub version: u16,
    #[serde(flatten)]
    pub operation: JobOperation,
}

impl JobOperation {
    pub const VERSION: u16 = JOB_OPERATION_VERSION;

    pub fn version(&self) -> u16 {
        Self::VERSION
    }

    pub fn storage_kind(&self) -> &'static str {
        match self {
            Self::Import { .. } => "import",
            Self::Scrape { .. } => "scrape",
            Self::ProviderVerify { .. } => "provider_verify",
            Self::Backup { .. } => "backup",
            Self::Restore { .. } => "restore",
            Self::DiagnosticsExport => "diagnostics.export",
            Self::UpdateCheck => "update.check",
        }
    }

    pub fn versioned(self) -> Result<VersionedJobOperation, String> {
        VersionedJobOperation::new(self)
    }

    fn source_metadata(&self) -> Value {
        match self {
            Self::Import {
                source,
                reference_id,
            } => json!({
                "area": "import",
                "entityId": reference_id,
                "label": source,
            }),
            Self::Scrape {
                game_id,
                provider_id,
            } => json!({
                "area": "game",
                "entityId": game_id,
                "label": provider_id,
            }),
            Self::ProviderVerify {
                media_type,
                provider_id,
            } => json!({
                "area": "provider",
                "entityId": provider_id,
                "label": media_type,
            }),
            Self::Backup { scope } => json!({
                "area": "backup",
                "entityId": scope,
                "label": scope,
            }),
            Self::Restore { snapshot_id } => json!({
                "area": "restore",
                "entityId": snapshot_id,
                "label": "snapshot",
            }),
            Self::DiagnosticsExport => json!({
                "area": "diagnostics",
                "entityId": Value::Null,
                "label": "export",
            }),
            Self::UpdateCheck => json!({
                "area": "update",
                "entityId": Value::Null,
                "label": "check",
            }),
        }
    }

    fn sanitize_for_persistence(self) -> Result<Self, String> {
        Ok(match self {
            Self::Import {
                source,
                reference_id,
            } => Self::Import {
                source: sanitize_operation_text("source", source, 512)?,
                reference_id: sanitize_operation_token("reference_id", reference_id)?,
            },
            Self::Scrape {
                game_id,
                provider_id,
            } => Self::Scrape {
                game_id: sanitize_operation_token("game_id", game_id)?,
                provider_id: provider_id
                    .map(|value| sanitize_operation_token("provider_id", value))
                    .transpose()?,
            },
            Self::ProviderVerify {
                media_type,
                provider_id,
            } => Self::ProviderVerify {
                media_type: sanitize_operation_token("media_type", media_type)?,
                provider_id: sanitize_operation_token("provider_id", provider_id)?,
            },
            Self::Backup { scope } => Self::Backup {
                scope: sanitize_operation_token("scope", scope)?,
            },
            Self::Restore { snapshot_id } => Self::Restore {
                snapshot_id: sanitize_operation_token("snapshot_id", snapshot_id)?,
            },
            Self::DiagnosticsExport => Self::DiagnosticsExport,
            Self::UpdateCheck => Self::UpdateCheck,
        })
    }
}

impl VersionedJobOperation {
    pub fn new(operation: JobOperation) -> Result<Self, String> {
        Ok(Self {
            version: JOB_OPERATION_VERSION,
            operation: operation.sanitize_for_persistence()?,
        })
    }

    pub fn is_supported(&self) -> bool {
        self.version == JOB_OPERATION_VERSION
    }
}

/// Details safe for a Task Center drawer. Unlike raw metadata, this only
/// exposes a recognized, versioned operation envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskCenterJobDetail {
    pub job: TaskCenterJob,
    pub operation: Option<VersionedJobOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTask {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: TaskStatus,
    /// Persisted and returned as a fraction, never as a legacy percentage.
    pub progress: f64,
    pub created_at: String,
    pub updated_at: String,
    pub message: Option<String>,
    #[serde(default)]
    pub recovered: bool,
    #[serde(default)]
    pub resumable: bool,
    #[serde(default)]
    pub retryable: bool,
}

/// Stable application-level task categories. Persisted jobs may use a more
/// specific implementation kind (for example `ai_v2.recommendation`); the Task
/// Center always maps those values into this forward-compatible projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Download,
    Import,
    Scrape,
    ProviderVerify,
    Ai,
    Backup,
    Restore,
    Diagnostics,
    Update,
    Generic,
}

impl TaskKind {
    pub fn from_storage_kind(kind: &str) -> Self {
        let normalized = kind.trim().to_ascii_lowercase();
        if normalized == "download" || normalized.starts_with("download.") {
            Self::Download
        } else if normalized == "import"
            || normalized.starts_with("import.")
            || normalized.starts_with("library_import")
        {
            Self::Import
        } else if normalized == "scrape"
            || normalized.starts_with("scrape.")
            || normalized.contains("metadata_refresh")
        {
            Self::Scrape
        } else if normalized == "provider_verify"
            || normalized.starts_with("provider_verify.")
            || (normalized.contains("provider") && normalized.contains("verify"))
        {
            Self::ProviderVerify
        } else if normalized == "ai"
            || normalized.starts_with("ai_")
            || normalized.starts_with("ai.")
        {
            Self::Ai
        } else if normalized == "backup" || normalized.starts_with("backup.") {
            Self::Backup
        } else if normalized == "restore" || normalized.starts_with("restore.") {
            Self::Restore
        } else if normalized == "diagnostics" || normalized.starts_with("diagnostics.") {
            Self::Diagnostics
        } else if normalized == "update" || normalized.starts_with("update.") {
            Self::Update
        } else {
            Self::Generic
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSource {
    pub area: String,
    pub entity_id: Option<String>,
    pub label: Option<String>,
}

/// Canonical Task Center DTO. Field names intentionally remain snake_case to
/// preserve the established `AppTask` wire shape; the frontend mapper accepts
/// both this shape and camelCase aliases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskCenterJob {
    pub id: String,
    pub kind: TaskKind,
    pub title: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub message: Option<String>,
    pub error_kind: Option<String>,
    pub retryable: bool,
    pub resumable: bool,
    pub cancellable: bool,
    /// Compatibility extension used by the current JobsPanel.
    pub pausable: bool,
    pub recovered: bool,
    pub created_at: String,
    pub updated_at: String,
    pub idempotency_key: Option<String>,
    pub source: Option<TaskSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAction {
    Pause,
    Resume,
    Retry,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskControlError {
    pub code: String,
    pub message: String,
    pub action: Option<TaskAction>,
    pub task_id: Option<String>,
    pub kind: Option<TaskKind>,
    pub status: Option<TaskStatus>,
}

impl TaskControlError {
    pub fn unsupported(action: TaskAction, task: &TaskCenterJob) -> Self {
        Self {
            code: "action_not_supported".to_string(),
            message: format!("任务类型 {:?} 不支持 {:?} 操作", task.kind, action),
            action: Some(action),
            task_id: Some(task.id.clone()),
            kind: Some(task.kind),
            status: Some(task.status),
        }
    }

    pub fn invalid_state(action: TaskAction, task: &TaskCenterJob) -> Self {
        Self {
            code: "invalid_task_state".to_string(),
            message: format!("任务当前状态不允许 {:?} 操作", action),
            action: Some(action),
            task_id: Some(task.id.clone()),
            kind: Some(task.kind),
            status: Some(task.status),
        }
    }

    pub fn internal(message: String) -> Self {
        Self {
            code: "task_control_failed".to_string(),
            message,
            action: None,
            task_id: None,
            kind: None,
            status: None,
        }
    }

    pub fn with_context(mut self, action: TaskAction, task: &TaskCenterJob) -> Self {
        self.action = Some(action);
        self.task_id = Some(task.id.clone());
        self.kind = Some(task.kind);
        self.status = Some(task.status);
        self
    }
}

/// A cancellation handle handed to the operation that performs a job.
///
/// It deliberately uses an atomic flag so it is observable from synchronous,
/// async, and worker-thread code without requiring the operation to share the
/// queue lock. The queue also checks the persisted status before every write,
/// which prevents a late success response from overwriting `cancelled`.
#[derive(Clone)]
pub struct CancellationHandle {
    job_id: String,
    cancelled: Arc<AtomicBool>,
}

impl CancellationHandle {
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn check_cancelled(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err("任务已取消".to_string())
        } else {
            Ok(())
        }
    }
}

struct QueueInner {
    db: Arc<SqliteDb>,
    cancellations: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Serializes read/validate/write transitions in this control-plane
    /// instance. The repository remains the source of truth for persistence.
    transition_lock: Mutex<()>,
}

#[derive(Clone)]
pub struct TaskQueue {
    inner: Arc<QueueInner>,
}

impl TaskQueue {
    /// Production constructor retained for the existing `lib.rs` builder.
    /// Batch integration should replace this with `from_database` so the
    /// already-open `Database`/`SqliteDb` handle is shared instead of opening a
    /// second connection to the same file.
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|error| panic!("TaskQueue initialization failed: {error}"))
    }

    pub fn try_new() -> Result<Self, String> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("moeplay");
        std::fs::create_dir_all(&data_dir).map_err(|error| {
            format!(
                "failed to create task database directory {}: {error}",
                data_dir.display()
            )
        })?;
        let db = Arc::new(SqliteDb::open(data_dir.join("moegame.db"))?);
        Ok(Self::from_database(db))
    }

    /// Builder/test seam for sharing the application's SQLite handle.
    pub fn from_database(db: Arc<SqliteDb>) -> Self {
        let queue = Self {
            inner: Arc::new(QueueInner {
                db,
                cancellations: Mutex::new(HashMap::new()),
                transition_lock: Mutex::new(()),
            }),
        };
        queue.recover_running_jobs();
        queue
    }

    /// Registers the operation's cancellation handle. Calling this more than
    /// once for a job is idempotent and returns a handle observing the same flag.
    pub fn register_cancellation_handle(&self, id: &str) -> Result<CancellationHandle, String> {
        let job = self.get_job(id)?;
        let mut handles = self.inner.cancellations.lock().map_err(|e| e.to_string())?;
        let cancelled = handles
            .entry(id.to_string())
            .or_insert_with(|| {
                Arc::new(AtomicBool::new(
                    job.status == BackgroundJobStatus::Cancelled,
                ))
            })
            .clone();
        Ok(CancellationHandle {
            job_id: id.to_string(),
            cancelled,
        })
    }

    /// Alias used by operation runners that register their cancellation handle
    /// immediately before doing network/file work.
    pub fn register_operation(&self, id: &str) -> Result<CancellationHandle, String> {
        self.register_cancellation_handle(id)
    }

    pub fn enqueue(&self, title: String, kind: String) -> AppTask {
        self.enqueue_with_key(title, kind, None)
            .unwrap_or_else(|error| panic!("enqueue task failed: {error}"))
    }

    /// Compatibility entry point for legacy callers. Inputs are sanitized at
    /// the persistence boundary before they can reach SQLite or a frontend DTO.
    pub fn enqueue_legacy(
        &self,
        title: String,
        kind: String,
        idempotency_key: Option<String>,
    ) -> Result<AppTask, String> {
        self.enqueue_with_key(title, kind, idempotency_key)
    }

    pub fn enqueue_with_key(
        &self,
        title: String,
        kind: String,
        idempotency_key: Option<String>,
    ) -> Result<AppTask, String> {
        self.enqueue_with_metadata(title, kind, idempotency_key, json!({}))
    }

    /// Enqueues a fixed, versioned backend operation and returns the Task
    /// Center projection. The persisted envelope is sanitized before it is
    /// written, so callers cannot persist credentials or prompt/response data.
    pub fn enqueue_operation(
        &self,
        title: String,
        operation: JobOperation,
        idempotency_key: Option<String>,
    ) -> Result<TaskCenterJob, String> {
        let operation = operation.versioned()?;
        let kind = operation.operation.storage_kind().to_string();
        let source = operation.operation.source_metadata();
        let metadata = json!({
            "operation": operation,
            "source": source,
        });
        let task = self.enqueue_with_metadata(title, kind, idempotency_key, metadata)?;
        self.get_task_center(&task.id)
    }

    pub fn enqueue_with_metadata(
        &self,
        title: String,
        kind: String,
        idempotency_key: Option<String>,
        mut metadata: Value,
    ) -> Result<AppTask, String> {
        // This method is intentionally the shared persistence boundary for
        // typed producers, legacy compatibility commands, and durable downloads.
        // Do not move these checks into only the Tauri command: internal callers
        // can also create jobs and their values must receive the same treatment.
        let title = sanitize_legacy_text("title", title, 256)?;
        let kind = sanitize_storage_kind(kind)?;
        let idempotency_key = sanitize_idempotency_key(idempotency_key)?;
        let _guard = self
            .inner
            .transition_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let repository = self.repository();
        let normalized_key = idempotency_key.as_deref();
        let stable_id = normalized_key.map(idempotent_job_id);
        if let Some(key) = normalized_key {
            if let Some(id) = stable_id.as_deref() {
                if let Some(existing) = repository.get(id)? {
                    return Ok(to_app_task(&existing));
                }
            }
            // Compatibility for jobs persisted before stable IDs were introduced.
            if let Some(existing) = repository.list(&[], 500)?.into_iter().find(|job| {
                metadata_idempotency_key(&job.metadata).as_deref()
                    == Some(idempotency_fingerprint(key).as_str())
            }) {
                return Ok(to_app_task(&existing));
            }
        }

        let now = timestamp();
        if !metadata.is_object() {
            metadata = json!({});
        }
        if let Some(key) = idempotency_key.as_deref() {
            // The raw key is used only in-memory to derive the deterministic job
            // identifier. SQLite and all public DTOs receive an opaque digest.
            metadata["idempotencyKey"] = Value::String(idempotency_fingerprint(key));
        }
        let job = BackgroundJob {
            id: stable_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            kind,
            title,
            status: BackgroundJobStatus::Queued,
            progress: 0.0,
            created_at: now.clone(),
            updated_at: now,
            error: None,
            metadata,
        };
        repository.insert(&job)?;
        self.register_cancellation_handle(&job.id)?;
        Ok(to_app_task(&job))
    }

    pub fn list(&self) -> Vec<AppTask> {
        self.list_result().unwrap_or_else(|error| {
            tracing::error!(error = %error, "failed to list background jobs");
            Vec::new()
        })
    }

    pub fn list_result(&self) -> Result<Vec<AppTask>, String> {
        self.repository()
            .list(&[], 500)
            .map(|jobs| jobs.iter().map(to_app_task).collect())
    }

    pub fn list_task_center(
        &self,
        status: Option<TaskStatus>,
        kind: Option<TaskKind>,
        limit: Option<usize>,
    ) -> Result<Vec<TaskCenterJob>, String> {
        let statuses = status
            .map(|status| vec![BackgroundJobStatus::from(status)])
            .unwrap_or_default();
        let requested_limit = limit.unwrap_or(500).clamp(1, 500);
        let jobs = self.repository().list(&statuses, 500)?;
        Ok(jobs
            .iter()
            .map(to_task_center_job)
            .filter(|job| kind.is_none_or(|kind| job.kind == kind))
            .take(requested_limit)
            .collect())
    }

    pub fn get(&self, id: &str) -> Result<AppTask, String> {
        self.get_job(id).map(|job| to_app_task(&job))
    }

    pub fn get_task_center(&self, id: &str) -> Result<TaskCenterJob, String> {
        self.get_job(id).map(|job| to_task_center_job(&job))
    }

    pub fn get_task_detail(&self, id: &str) -> Result<TaskCenterJobDetail, String> {
        let job = self.get_job(id)?;
        let operation = job
            .metadata
            .get("operation")
            .cloned()
            .and_then(|value| serde_json::from_value::<VersionedJobOperation>(value).ok())
            .filter(VersionedJobOperation::is_supported);
        Ok(TaskCenterJobDetail {
            job: to_task_center_job(&job),
            operation,
        })
    }

    /// Appends a redacted event to the persistent job timeline. Progress is a
    /// fraction and is intentionally independent from the job's lifecycle
    /// progress update, allowing workers to emit informational milestones.
    pub fn append_event(
        &self,
        job_id: &str,
        level: TaskEventLevel,
        code: String,
        message: String,
        progress: Option<f64>,
    ) -> Result<TaskEvent, String> {
        let code = sanitize_event_code(code)?;
        let message = sanitize_legacy_text("event_message", message, 2048)?;
        let progress = progress.map(normalize_progress).transpose()?.map(f64::from);
        self.repository()
            .append_event(job_id, level, &code, &message, progress)
    }

    /// Lists events after an exclusive sequence cursor in ascending order.
    pub fn list_events(
        &self,
        job_id: &str,
        after_sequence: Option<i64>,
        limit: usize,
    ) -> Result<Vec<TaskEvent>, String> {
        self.repository().list_events(job_id, after_sequence, limit)
    }

    /// Command-facing alias with a name matching the Task Center details API.
    pub fn get_task_events(
        &self,
        job_id: &str,
        after_sequence: Option<i64>,
        limit: usize,
    ) -> Result<Vec<TaskEvent>, String> {
        self.list_events(job_id, after_sequence, limit)
    }

    pub fn metadata(&self, id: &str) -> Result<Value, String> {
        self.get_job(id).map(|job| job.metadata)
    }

    /// Marks a typed operation running and records a safe lifecycle event.
    pub fn mark_running(
        &self,
        id: &str,
        message: Option<String>,
        progress: Option<f64>,
    ) -> Result<TaskCenterJob, String> {
        let message = message.map(|value| redact_event_message(&value));
        let task = self.update(id, Some(TaskStatus::Running), progress, message.clone())?;
        self.append_event(
            id,
            TaskEventLevel::Info,
            "job_running".to_string(),
            message.unwrap_or_else(|| "任务开始运行".to_string()),
            Some(task.progress),
        )?;
        self.get_task_center(id)
    }

    /// Marks a typed operation successfully complete, always persisting full
    /// progress and a terminal timeline event. A cancelled job rejects this
    /// late completion through the existing monotonic transition guard.
    pub fn mark_succeeded(
        &self,
        id: &str,
        message: Option<String>,
    ) -> Result<TaskCenterJob, String> {
        let message = message.map(|value| redact_event_message(&value));
        let task = self.update(id, Some(TaskStatus::Succeeded), Some(1.0), message.clone())?;
        self.append_event(
            id,
            TaskEventLevel::Info,
            "job_succeeded".to_string(),
            message.unwrap_or_else(|| "任务已完成".to_string()),
            Some(task.progress),
        )?;
        self.get_task_center(id)
    }

    /// Marks a typed operation failed, persists a redacted provider error, and
    /// appends a stable error-code event. Error details cannot overwrite a
    /// cancellation because the status transition executes first.
    pub fn mark_failed(&self, id: &str, mut error: ProviderError) -> Result<TaskCenterJob, String> {
        error.message = redact_event_message(&error.message);
        error.provider_id = error
            .provider_id
            .take()
            .map(|value| sanitize_operation_token("provider_id", value))
            .transpose()?;
        error.operation = error
            .operation
            .take()
            .map(|value| sanitize_operation_token("operation", value))
            .transpose()?;
        let message = error.message.clone();
        self.update(id, Some(TaskStatus::Failed), None, Some(message.clone()))?;
        self.persist_failed_error(id, error.clone())?;
        let kind = serialized_enum_name(&error.kind).unwrap_or_else(|| "unknown".to_string());
        self.append_event(
            id,
            TaskEventLevel::Error,
            format!("job_failed.{kind}"),
            message,
            None,
        )?;
        self.get_task_center(id)
    }

    /// Compatibility update entry point. Message sanitization remains enforced
    /// by `update_with_metadata`, which is also used by backend workers.
    pub fn update_legacy(
        &self,
        id: &str,
        status: Option<TaskStatus>,
        progress: Option<f64>,
        message: Option<String>,
    ) -> Result<AppTask, String> {
        self.update(id, status, progress, message)
    }

    pub fn update(
        &self,
        id: &str,
        status: Option<TaskStatus>,
        progress: Option<f64>,
        message: Option<String>,
    ) -> Result<AppTask, String> {
        self.update_with_metadata(id, status, progress, message, None)
    }

    pub fn update_with_metadata(
        &self,
        id: &str,
        status: Option<TaskStatus>,
        progress: Option<f64>,
        message: Option<String>,
        metadata_patch: Option<Value>,
    ) -> Result<AppTask, String> {
        let message = message
            .map(|value| sanitize_legacy_text("message", value, 1024))
            .transpose()?;
        let _guard = self
            .inner
            .transition_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let repository = self.repository();
        let mut job = self.get_job_locked(&repository, id)?;
        let requested_status = status.map(BackgroundJobStatus::from);
        let cancellation = self.cancellation_flag(id)?;

        if cancellation.load(Ordering::Acquire)
            && requested_status.is_some_and(|next| next != BackgroundJobStatus::Cancelled)
        {
            return Err("任务已取消，拒绝迟到更新".to_string());
        }

        if is_terminal(job.status)
            && (requested_status != Some(job.status)
                || progress.is_some()
                || message.is_some()
                || metadata_patch.is_some())
        {
            return Err("终态任务不可继续更新".to_string());
        }

        if let Some(next) = requested_status {
            if !is_legal_transition(job.status, next) {
                return Err(format!(
                    "非法任务状态转换: {} -> {}",
                    status_name(job.status),
                    status_name(next)
                ));
            }
            if next == BackgroundJobStatus::Cancelled {
                cancellation.store(true, Ordering::Release);
            }
            job.status = next;
            apply_status_metadata(&mut job.metadata, next);
        }

        if let Some(progress) = progress {
            if is_terminal(job.status) && requested_status.is_none() {
                return Err("终态任务不可更新进度".to_string());
            }
            job.progress = normalize_progress(progress)?;
        }
        if let Some(message) = message {
            set_message(&mut job.metadata, message);
        }
        if let Some(metadata_patch) = metadata_patch {
            merge_metadata(&mut job.metadata, metadata_patch);
        }
        job.updated_at = timestamp();
        repository.upsert(&job)?;
        Ok(to_app_task(&job))
    }

    pub fn pause(&self, id: &str, message: Option<String>) -> Result<AppTask, String> {
        let task = self.get_task_center(id)?;
        if task.kind != TaskKind::Download {
            return Err("该任务类型不支持暂停".to_string());
        }
        if !task.pausable {
            return Err("任务当前状态不可暂停".to_string());
        }
        self.update(id, Some(TaskStatus::Paused), None, message)
    }

    pub fn resume(&self, id: &str, message: Option<String>) -> Result<AppTask, String> {
        let current = self.get_task_center(id)?;
        if current.kind != TaskKind::Download {
            return Err("该任务类型不支持恢复".to_string());
        }
        if !current.resumable {
            return Err("任务当前状态不可恢复".to_string());
        }
        let task = self.update_with_metadata(
            id,
            Some(TaskStatus::Running),
            None,
            message,
            Some(json!({
                "recoverable": false,
                "recovered": false,
                "resumable": true,
                "retryable": false
            })),
        )?;
        self.cancellation_flag(id)?.store(false, Ordering::Release);
        Ok(task)
    }

    pub fn retry(&self, id: &str, message: Option<String>) -> Result<AppTask, String> {
        let _guard = self
            .inner
            .transition_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let repository = self.repository();
        let mut job = self.get_job_locked(&repository, id)?;
        if TaskKind::from_storage_kind(&job.kind) != TaskKind::Download {
            return Err("该任务类型不支持重试".to_string());
        }
        if !matches!(
            job.status,
            BackgroundJobStatus::Failed | BackgroundJobStatus::Paused
        ) {
            return Err("只有失败或暂停任务可以重试".to_string());
        }
        let cancellation = self.cancellation_flag(id)?;
        let was_cancelled = cancellation.load(Ordering::Acquire);
        cancellation.store(false, Ordering::Release);
        job.status = BackgroundJobStatus::Queued;
        job.error = None;
        apply_status_metadata(&mut job.metadata, BackgroundJobStatus::Queued);
        merge_metadata(
            &mut job.metadata,
            json!({
                "recoverable": false,
                "recovered": false,
                "resumable": true,
                "retryable": false
            }),
        );
        if let Some(message) = message {
            set_message(&mut job.metadata, message);
        }
        job.updated_at = timestamp();
        if let Err(error) = repository.upsert(&job) {
            cancellation.store(was_cancelled, Ordering::Release);
            return Err(error);
        }
        Ok(to_app_task(&job))
    }

    pub fn patch_metadata(&self, id: &str, patch: Value) -> Result<AppTask, String> {
        self.update_with_metadata(id, None, None, None, Some(patch))
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        let _guard = self
            .inner
            .transition_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let repository = self.repository();
        let job = self.get_job_locked(&repository, id)?;
        if !is_terminal(job.status) && job.status != BackgroundJobStatus::Paused {
            return Err("只能移除暂停或终态任务".to_string());
        }
        if job.status == BackgroundJobStatus::Paused {
            self.cancellation_flag(id)?.store(true, Ordering::Release);
        }
        repository.delete(id)?;
        self.inner
            .cancellations
            .lock()
            .map_err(|e| e.to_string())?
            .remove(id);
        Ok(())
    }

    pub fn cancel(&self, id: &str) -> Result<AppTask, String> {
        let _guard = self
            .inner
            .transition_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let repository = self.repository();
        let mut job = self.get_job_locked(&repository, id)?;
        if job.status == BackgroundJobStatus::Cancelled {
            let cancellation = self.cancellation_flag(id)?;
            cancellation.store(true, Ordering::Release);
            return Ok(to_app_task(&job));
        }
        if is_terminal(job.status) {
            return Err("终态任务不可取消".to_string());
        }

        // Publish the atomic cancellation before the durable write, while the
        // transition lock prevents any competing success/failure update from
        // passing validation. Roll it back if persistence fails.
        let cancellation = self.cancellation_flag(id)?;
        cancellation.store(true, Ordering::Release);
        job.status = BackgroundJobStatus::Cancelled;
        set_message(&mut job.metadata, "已取消".to_string());
        job.updated_at = timestamp();
        if let Err(error) = repository.upsert(&job) {
            cancellation.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(to_app_task(&job))
    }

    pub fn clear_finished(&self) -> Result<(), String> {
        self.clear_finished_kind(None)
    }

    pub fn clear_finished_for_kind(&self, kind: &str) -> Result<(), String> {
        self.clear_finished_kind(Some(kind))
    }

    fn clear_finished_kind(&self, kind: Option<&str>) -> Result<(), String> {
        let _guard = self
            .inner
            .transition_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let repository = self.repository();
        loop {
            let jobs = repository.list_filtered(
                &[
                    BackgroundJobStatus::Succeeded,
                    BackgroundJobStatus::Failed,
                    BackgroundJobStatus::Cancelled,
                ],
                kind,
                500,
            )?;
            if jobs.is_empty() {
                break;
            }
            for job in &jobs {
                repository.delete_if_terminal(&job.id)?;
                self.inner
                    .cancellations
                    .lock()
                    .map_err(|e| e.to_string())?
                    .remove(&job.id);
            }
            if jobs.len() < 500 {
                break;
            }
        }
        Ok(())
    }

    pub fn database(&self) -> Arc<SqliteDb> {
        Arc::clone(&self.inner.db)
    }

    fn repository(&self) -> BackgroundJobRepository<'_> {
        BackgroundJobRepository::new(&self.inner.db)
    }

    fn get_job(&self, id: &str) -> Result<BackgroundJob, String> {
        self.repository()
            .get(id)?
            .ok_or_else(|| "任务不存在".to_string())
    }

    fn get_job_locked(
        &self,
        repository: &BackgroundJobRepository<'_>,
        id: &str,
    ) -> Result<BackgroundJob, String> {
        repository.get(id)?.ok_or_else(|| "任务不存在".to_string())
    }

    fn cancellation_flag(&self, id: &str) -> Result<Arc<AtomicBool>, String> {
        let _ = self.get_job(id)?;
        let mut handles = self.inner.cancellations.lock().map_err(|e| e.to_string())?;
        let cancelled_from_store = self
            .repository()
            .get(id)?
            .is_some_and(|job| job.status == BackgroundJobStatus::Cancelled);
        Ok(handles
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(AtomicBool::new(cancelled_from_store)))
            .clone())
    }

    fn persist_failed_error(&self, id: &str, error: ProviderError) -> Result<(), String> {
        let _guard = self
            .inner
            .transition_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let repository = self.repository();
        let mut job = self.get_job_locked(&repository, id)?;
        if job.status != BackgroundJobStatus::Failed {
            return Err("任务失败状态已被其他操作改变".to_string());
        }
        job.error = Some(error);
        job.updated_at = timestamp();
        repository.upsert(&job)
    }

    fn recover_running_jobs(&self) {
        let _guard = self
            .inner
            .transition_lock
            .lock()
            .expect("task queue lock poisoned");
        let repository = self.repository();
        let Ok(jobs) = repository.list(&[BackgroundJobStatus::Running], 500) else {
            return;
        };
        for mut job in jobs {
            let kind = TaskKind::from_storage_kind(&job.kind);
            let resumable = kind == TaskKind::Download;
            job.status = if resumable {
                BackgroundJobStatus::Paused
            } else {
                BackgroundJobStatus::Failed
            };
            job.updated_at = timestamp();
            if !job.metadata.is_object() {
                job.metadata = json!({});
            }
            merge_metadata(
                &mut job.metadata,
                json!({
                    "recoverable": resumable,
                    "recovered": true,
                    "resumable": resumable,
                    "retryable": resumable,
                    "recoveryReason": "process_restart",
                    "message": if resumable {
                        "已从上次运行恢复，可继续或重试"
                    } else {
                        "应用重启时任务仍在运行，已安全标记为失败"
                    }
                }),
            );
            if !resumable {
                job.error = Some(ProviderError {
                    kind: ProviderErrorKind::Unknown,
                    message: "task interrupted by process restart".to_string(),
                    retryable: false,
                    retry_after_ms: None,
                    provider_id: None,
                    operation: Some("process_restart_recovery".to_string()),
                });
            }
            let _ = repository.upsert(&job);
        }
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn to_app_task(job: &BackgroundJob) -> AppTask {
    AppTask {
        id: job.id.clone(),
        title: redacted_display_text(&job.title, "后台任务"),
        kind: public_storage_kind(&job.kind),
        status: job.status.into(),
        progress: f64::from(job.progress).clamp(0.0, 1.0),
        created_at: job.created_at.clone(),
        updated_at: job.updated_at.clone(),
        message: message_from_metadata(&job.metadata),
        recovered: metadata_bool(&job.metadata, "recovered")
            || metadata_bool(&job.metadata, "recoverable"),
        resumable: metadata_bool(&job.metadata, "resumable")
            || job.status == BackgroundJobStatus::Paused,
        retryable: metadata_bool(&job.metadata, "retryable")
            || job.status == BackgroundJobStatus::Failed,
    }
}

fn to_task_center_job(job: &BackgroundJob) -> TaskCenterJob {
    let kind = TaskKind::from_storage_kind(&job.kind);
    let status = TaskStatus::from(job.status);
    let is_active = matches!(
        job.status,
        BackgroundJobStatus::Queued | BackgroundJobStatus::Running | BackgroundJobStatus::Paused
    );
    let is_download = kind == TaskKind::Download;
    let pausable = is_download && job.status == BackgroundJobStatus::Running;
    let resumable = is_download && job.status == BackgroundJobStatus::Paused;
    let retryable = is_download
        && matches!(
            job.status,
            BackgroundJobStatus::Failed | BackgroundJobStatus::Paused
        );
    TaskCenterJob {
        id: job.id.clone(),
        kind,
        title: redacted_display_text(&job.title, "后台任务"),
        status,
        progress: f64::from(job.progress).clamp(0.0, 1.0),
        message: message_from_metadata(&job.metadata).or_else(|| {
            job.error
                .as_ref()
                .map(|error| redacted_display_text(&error.message, "任务失败"))
        }),
        error_kind: job
            .error
            .as_ref()
            .and_then(|error| serialized_enum_name(&error.kind)),
        retryable,
        resumable,
        cancellable: is_active,
        pausable,
        recovered: metadata_bool(&job.metadata, "recovered"),
        created_at: job.created_at.clone(),
        updated_at: job.updated_at.clone(),
        idempotency_key: metadata_idempotency_key(&job.metadata),
        source: source_from_metadata(&job.metadata),
    }
}

fn serialized_enum_name<T: Serialize>(value: &T) -> Option<String> {
    serde_json::to_value(value)
        .ok()?
        .as_str()
        .map(ToOwned::to_owned)
}

fn source_from_metadata(metadata: &Value) -> Option<TaskSource> {
    let source = metadata.get("source")?.as_object()?;
    let area = source.get("area")?.as_str()?.trim();
    if area.is_empty() {
        return None;
    }
    Some(TaskSource {
        area: area.to_string(),
        entity_id: source
            .get("entityId")
            .or_else(|| source.get("entity_id"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
        label: source
            .get("label")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
    })
}

fn idempotent_job_id(key: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"background-job:v1\0");
    digest.update(key.as_bytes());
    format!("job-{}", hex::encode(digest.finalize()))
}

/// Public, deterministic representation of an idempotency key. This is never
/// reversible and keeps the raw caller-provided key out of SQLite and DTOs.
fn idempotency_fingerprint(key: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"background-job:idempotency-public:v1\0");
    digest.update(key.as_bytes());
    format!("sha256:{}", hex::encode(digest.finalize()))
}

fn metadata_idempotency_key(metadata: &Value) -> Option<String> {
    metadata
        .get("idempotencyKey")
        .and_then(Value::as_str)
        .map(|value| {
            if is_idempotency_fingerprint(value) {
                value.to_string()
            } else {
                // Jobs written before Stage 3 stored the legacy key directly.
                // Continue to expose a stable correlation value without leaking it.
                idempotency_fingerprint(value)
            }
        })
}

fn message_from_metadata(metadata: &Value) -> Option<String> {
    metadata
        .get("message")
        .and_then(Value::as_str)
        .map(|value| redacted_display_text(value, "任务状态已更新"))
}

fn set_message(metadata: &mut Value, message: String) {
    if !metadata.is_object() {
        *metadata = json!({});
    }
    metadata["message"] = Value::String(message);
}

fn metadata_bool(metadata: &Value, key: &str) -> bool {
    metadata.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn merge_metadata(metadata: &mut Value, patch: Value) {
    if !metadata.is_object() {
        *metadata = json!({});
    }
    let Value::Object(patch) = patch else {
        return;
    };
    let Some(target) = metadata.as_object_mut() else {
        return;
    };
    for (key, value) in patch {
        target.insert(key, value);
    }
}

fn apply_status_metadata(metadata: &mut Value, status: BackgroundJobStatus) {
    let patch = match status {
        BackgroundJobStatus::Queued => json!({ "retryable": false }),
        BackgroundJobStatus::Running => json!({
            "recoverable": false,
            "recovered": false,
            "resumable": true,
            "retryable": false
        }),
        BackgroundJobStatus::Paused => json!({ "resumable": true }),
        BackgroundJobStatus::Succeeded => json!({ "resumable": false, "retryable": false }),
        BackgroundJobStatus::Failed => json!({ "resumable": true, "retryable": true }),
        BackgroundJobStatus::Cancelled => json!({ "resumable": false, "retryable": false }),
    };
    merge_metadata(metadata, patch);
}

const LEGACY_KIND_PREFIXES: &[&str] = &[
    "download",
    "import",
    "library_import",
    "scrape",
    "provider_verify",
    "ai",
    "backup",
    "restore",
    "diagnostics",
    "update",
    "generic",
];

fn sanitize_legacy_text(field: &str, value: String, max_chars: usize) -> Result<String, String> {
    let value = redact_event_message(&value);
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars || value.contains(['\r', '\n', '\0']) {
        return Err(format!("任务字段 {field} 无效"));
    }
    Ok(value.to_string())
}

fn sanitize_idempotency_key(value: Option<String>) -> Result<Option<String>, String> {
    value
        .map(|value| {
            let value = value.trim();
            if value.is_empty() {
                return Ok(None);
            }
            if value.chars().count() > 256 || value.contains(['\r', '\n', '\0']) {
                return Err("任务幂等键无效".to_string());
            }
            Ok(Some(value.to_string()))
        })
        .unwrap_or(Ok(None))
}

fn sanitize_event_code(value: String) -> Result<String, String> {
    let value = sanitize_legacy_text("event_code", value, 128)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("任务事件代码无效".to_string());
    }
    Ok(value)
}

fn sanitize_storage_kind(value: String) -> Result<String, String> {
    let value = sanitize_legacy_text("kind", value, 96)?;
    let normalized = value.to_ascii_lowercase();
    if !normalized
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Ok("generic".to_string());
    }
    let accepted = LEGACY_KIND_PREFIXES.iter().any(|prefix| {
        normalized == *prefix
            || normalized.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.starts_with('.') || suffix.starts_with('_') || suffix.starts_with('-')
            })
    });
    Ok(if accepted {
        normalized
    } else {
        "generic".to_string()
    })
}

fn public_storage_kind(value: &str) -> String {
    sanitize_storage_kind(value.to_string()).unwrap_or_else(|_| "generic".to_string())
}

fn is_idempotency_fingerprint(value: &str) -> bool {
    value.len() == "sha256:".len() + 64
        && value.starts_with("sha256:")
        && value["sha256:".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
}

fn redacted_display_text(value: &str, fallback: &str) -> String {
    let value = redact_event_message(value);
    let value = value.trim();
    if value.is_empty() || value.contains(['\r', '\n', '\0']) {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn sanitize_operation_text(field: &str, value: String, max_chars: usize) -> Result<String, String> {
    let value = redact_event_message(&value);
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars || value.contains(['\r', '\n', '\0']) {
        return Err(format!("任务操作字段 {field} 无效"));
    }
    Ok(value.to_string())
}

fn sanitize_operation_token(field: &str, value: String) -> Result<String, String> {
    let value = sanitize_operation_text(field, value, 128)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
    {
        return Err(format!("任务操作字段 {field} 只能包含标识符字符"));
    }
    Ok(value)
}

fn normalize_progress(progress: f64) -> Result<f32, String> {
    if !progress.is_finite() {
        return Err("任务进度必须是有限数字".to_string());
    }
    // The old API accepted percentages. Keep accepting them at the boundary,
    // while every persisted/read value remains 0..=1.
    let fraction = if progress > 1.0 {
        progress / 100.0
    } else {
        progress
    };
    Ok(fraction.clamp(0.0, 1.0) as f32)
}

fn is_terminal(status: BackgroundJobStatus) -> bool {
    matches!(
        status,
        BackgroundJobStatus::Succeeded
            | BackgroundJobStatus::Failed
            | BackgroundJobStatus::Cancelled
    )
}

fn is_legal_transition(from: BackgroundJobStatus, to: BackgroundJobStatus) -> bool {
    if from == to {
        return true;
    }
    matches!(
        (from, to),
        (BackgroundJobStatus::Queued, BackgroundJobStatus::Running)
            | (BackgroundJobStatus::Queued, BackgroundJobStatus::Paused)
            | (BackgroundJobStatus::Queued, BackgroundJobStatus::Cancelled)
            | (BackgroundJobStatus::Running, BackgroundJobStatus::Paused)
            | (BackgroundJobStatus::Running, BackgroundJobStatus::Succeeded)
            | (BackgroundJobStatus::Running, BackgroundJobStatus::Failed)
            | (BackgroundJobStatus::Running, BackgroundJobStatus::Cancelled)
            | (BackgroundJobStatus::Paused, BackgroundJobStatus::Queued)
            | (BackgroundJobStatus::Paused, BackgroundJobStatus::Running)
            | (BackgroundJobStatus::Paused, BackgroundJobStatus::Cancelled)
            | (BackgroundJobStatus::Failed, BackgroundJobStatus::Queued)
    )
}

fn status_name(status: BackgroundJobStatus) -> &'static str {
    match status {
        BackgroundJobStatus::Queued => "queued",
        BackgroundJobStatus::Running => "running",
        BackgroundJobStatus::Paused => "paused",
        BackgroundJobStatus::Succeeded => "succeeded",
        BackgroundJobStatus::Failed => "failed",
        BackgroundJobStatus::Cancelled => "cancelled",
    }
}

fn timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn queue() -> TaskQueue {
        TaskQueue::from_database(Arc::new(SqliteDb::open_in_memory().unwrap()))
    }

    #[test]
    fn cancellation_is_observable_and_late_success_is_rejected() {
        let queue = queue();
        let task = queue.enqueue("download".into(), "download".into());
        queue
            .update(&task.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        let handle = queue.register_operation(&task.id).unwrap();
        queue.cancel(&task.id).unwrap();
        assert!(handle.is_cancelled());
        assert!(queue
            .update(&task.id, Some(TaskStatus::Succeeded), Some(1.0), None)
            .is_err());
        assert_eq!(queue.list()[0].status, TaskStatus::Cancelled);
    }

    #[test]
    fn only_legal_state_transitions_are_persisted() {
        let queue = queue();
        let task = queue.enqueue("job".into(), "test".into());
        assert!(queue
            .update(&task.id, Some(TaskStatus::Succeeded), None, None)
            .is_err());
        queue
            .update(&task.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        assert!(queue
            .update(&task.id, Some(TaskStatus::Queued), None, None)
            .is_err());
        queue
            .update(&task.id, Some(TaskStatus::Succeeded), Some(100.0), None)
            .unwrap();
        assert_eq!(queue.list()[0].progress, 1.0);
        assert!(queue
            .update(&task.id, Some(TaskStatus::Cancelled), None, None)
            .is_err());
    }

    #[test]
    fn legacy_persistence_boundary_redacts_and_hides_public_fields() {
        let queue = queue();
        let raw_key = "Authorization=Bearer compatibility-secret";
        let task = queue
            .enqueue_legacy(
                "token=title-secret".to_string(),
                "token=kind-secret".to_string(),
                Some(raw_key.to_string()),
            )
            .unwrap();
        let updated = queue
            .update_legacy(
                &task.id,
                None,
                Some(0.5),
                Some("request headers: Authorization: Bearer message-secret".to_string()),
            )
            .unwrap();

        let persisted = queue.get_job(&task.id).unwrap();
        let serialized = serde_json::to_string(&persisted).unwrap();
        assert!(!serialized.contains("title-secret"));
        assert!(!serialized.contains("kind-secret"));
        assert!(!serialized.contains("compatibility-secret"));
        assert!(!serialized.contains("message-secret"));
        assert_eq!(persisted.kind, "generic");
        assert_eq!(
            persisted.metadata["idempotencyKey"].as_str(),
            Some(idempotency_fingerprint(raw_key).as_str())
        );
        assert!(!updated
            .message
            .unwrap_or_default()
            .contains("message-secret"));

        let projected = queue.get_task_center(&task.id).unwrap();
        assert!(!projected.title.contains("title-secret"));
        assert_eq!(projected.kind, TaskKind::Generic);
        assert_eq!(
            projected.idempotency_key.as_deref(),
            Some(idempotency_fingerprint(raw_key).as_str())
        );
        assert!(queue
            .enqueue_legacy("bad\nname".to_string(), "download".to_string(), None)
            .is_err());
        assert!(queue
            .update_legacy(&task.id, None, None, Some("bad\nmessage".to_string()))
            .is_err());
    }

    #[test]
    fn idempotency_key_returns_the_original_job() {
        let queue = queue();
        let first = queue
            .enqueue_with_key("same".into(), "test".into(), Some("key-1".into()))
            .unwrap();
        let second = queue
            .enqueue_with_key("different".into(), "other".into(), Some("key-1".into()))
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(queue.list().len(), 1);
    }

    #[test]
    fn non_resumable_running_jobs_are_recovered_as_failed_after_restart() {
        let db = Arc::new(SqliteDb::open_in_memory().unwrap());
        let first = TaskQueue::from_database(Arc::clone(&db));
        let task = first.enqueue("recover".into(), "ai_v2.recommendation".into());
        first
            .update(&task.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        drop(first);
        let restarted = TaskQueue::from_database(db);
        let recovered = restarted.get_task_center(&task.id).unwrap();
        assert_eq!(recovered.status, TaskStatus::Failed);
        assert_eq!(recovered.kind, TaskKind::Ai);
        assert!(recovered.recovered);
        assert!(!recovered.resumable);
        assert!(!recovered.retryable);
        assert_eq!(recovered.error_kind.as_deref(), Some("unknown"));
    }

    #[test]
    fn failed_jobs_can_retry_but_cancelled_jobs_remain_terminal() {
        let queue = queue();
        let task = queue.enqueue("download".into(), "download".into());
        queue
            .update(&task.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        queue
            .update(
                &task.id,
                Some(TaskStatus::Failed),
                Some(0.5),
                Some("network error".into()),
            )
            .unwrap();
        let failed = queue.get(&task.id).unwrap();
        assert!(failed.retryable);
        assert!(failed.resumable);

        let queued = queue.retry(&task.id, Some("retrying".into())).unwrap();
        assert_eq!(queued.status, TaskStatus::Queued);
        assert!(!queued.retryable);
        queue.cancel(&task.id).unwrap();
        assert!(queue.retry(&task.id, None).is_err());
    }

    #[test]
    fn recovery_is_exposed_without_claiming_the_operation_is_running() {
        let db = Arc::new(SqliteDb::open_in_memory().unwrap());
        let first = TaskQueue::from_database(Arc::clone(&db));
        let task = first
            .enqueue_with_metadata(
                "archive.zip".into(),
                "download".into(),
                Some("download-key".into()),
                json!({ "url": "https://example.test/archive.zip" }),
            )
            .unwrap();
        first
            .update(&task.id, Some(TaskStatus::Running), Some(0.4), None)
            .unwrap();
        drop(first);

        let restarted = TaskQueue::from_database(db);
        let recovered = restarted.get(&task.id).unwrap();
        assert_eq!(recovered.status, TaskStatus::Paused);
        assert!(recovered.recovered);
        assert!(recovered.resumable);
        assert!(recovered.retryable);
        assert!(recovered.message.unwrap().contains("恢复"));
    }

    #[test]
    fn task_center_projection_maps_kinds_capabilities_and_metadata() {
        let queue = queue();
        let task = queue
            .enqueue_with_metadata(
                "AI recommendation".into(),
                "ai_v2.recommendation".into(),
                Some("ai-key".into()),
                json!({
                    "source": { "area": "library", "entityId": "game-1", "label": "Game" }
                }),
            )
            .unwrap();
        queue
            .update(&task.id, Some(TaskStatus::Running), Some(0.25), None)
            .unwrap();

        let projected = queue.get_task_center(&task.id).unwrap();
        assert_eq!(projected.kind, TaskKind::Ai);
        assert_eq!(projected.progress, 0.25);
        assert!(projected.cancellable);
        assert!(!projected.pausable);
        assert!(!projected.resumable);
        assert!(!projected.retryable);
        assert_eq!(
            projected.idempotency_key.as_deref(),
            Some(idempotency_fingerprint("ai-key").as_str())
        );
        assert_eq!(
            projected.source.unwrap().entity_id.as_deref(),
            Some("game-1")
        );
    }

    #[test]
    fn task_center_list_applies_status_kind_and_limit_filters() {
        let queue = queue();
        let download = queue.enqueue("download".into(), "download".into());
        queue
            .update(&download.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        queue.enqueue("ai".into(), "ai_v2.library_cleanup".into());
        queue.enqueue("other".into(), "future_kind".into());

        let running_downloads = queue
            .list_task_center(
                Some(TaskStatus::Running),
                Some(TaskKind::Download),
                Some(10),
            )
            .unwrap();
        assert_eq!(running_downloads.len(), 1);
        assert_eq!(running_downloads[0].id, download.id);
        assert_eq!(
            queue
                .list_task_center(None, Some(TaskKind::Ai), Some(1))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            queue.list_task_center(None, None, Some(2)).unwrap().len(),
            2
        );
    }

    #[test]
    fn producer_specific_actions_reject_unsupported_kinds() {
        let queue = queue();
        let ai = queue.enqueue("ai".into(), "ai_v2.recommendation".into());
        queue
            .update(&ai.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        assert!(queue
            .pause(&ai.id, None)
            .unwrap_err()
            .contains("不支持暂停"));
    }

    #[test]
    fn clear_finished_keeps_every_non_terminal_job() {
        let queue = queue();
        let queued = queue.enqueue("queued".into(), "generic".into());
        let paused = queue.enqueue("paused".into(), "download".into());
        queue
            .update(&paused.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        queue.pause(&paused.id, None).unwrap();
        let succeeded = queue.enqueue("done".into(), "generic".into());
        queue
            .update(&succeeded.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        queue
            .update(&succeeded.id, Some(TaskStatus::Succeeded), Some(1.0), None)
            .unwrap();
        let failed = queue.enqueue("failed".into(), "generic".into());
        queue
            .update(&failed.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        queue
            .update(&failed.id, Some(TaskStatus::Failed), None, None)
            .unwrap();

        queue.clear_finished().unwrap();
        let remaining = queue.list();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|job| job.id == queued.id));
        assert!(remaining.iter().any(|job| job.id == paused.id));
        assert!(queue.get(&succeeded.id).is_err());
        assert!(queue.get(&failed.id).is_err());
    }

    #[test]
    fn structured_unsupported_error_has_stable_wire_fields() {
        let queue = queue();
        let task = queue.enqueue("ai".into(), "ai_v2.recommendation".into());
        let projected = queue.get_task_center(&task.id).unwrap();
        let value =
            serde_json::to_value(TaskControlError::unsupported(TaskAction::Pause, &projected))
                .unwrap();
        assert_eq!(value["code"], "action_not_supported");
        assert_eq!(value["action"], "pause");
        assert_eq!(value["kind"], "ai");
        assert_eq!(value["status"], "queued");
    }

    #[test]
    fn versioned_operations_are_sanitized_and_exposed_through_typed_detail() {
        let queue = queue();
        let task = queue
            .enqueue_operation(
                "Import library".to_string(),
                JobOperation::Import {
                    source: "https://alice:super-secret@example.test/library".to_string(),
                    reference_id: "library-42".to_string(),
                },
                Some("typed-import-42".to_string()),
            )
            .unwrap();
        assert_eq!(task.kind, TaskKind::Import);
        assert!(!task.source.unwrap().label.unwrap().contains("super-secret"));

        let detail = queue.get_task_detail(&task.id).unwrap();
        let operation = detail.operation.unwrap();
        assert_eq!(operation.version, JOB_OPERATION_VERSION);
        assert!(operation.is_supported());
        let serialized = serde_json::to_value(&operation).unwrap();
        assert_eq!(serialized["version"], JOB_OPERATION_VERSION);
        assert_eq!(serialized["kind"], "import");
        assert!(!serialized.to_string().contains("super-secret"));

        let duplicate = queue
            .enqueue_operation(
                "A different title is ignored by idempotency".to_string(),
                JobOperation::Import {
                    source: "folder".to_string(),
                    reference_id: "library-42".to_string(),
                },
                Some("typed-import-42".to_string()),
            )
            .unwrap();
        assert_eq!(duplicate.id, task.id);
        assert!(queue
            .enqueue_operation(
                "Bad operation".to_string(),
                JobOperation::Backup {
                    scope: "all --unsafe-argument".to_string(),
                },
                None,
            )
            .is_err());
    }

    #[test]
    fn lifecycle_events_are_redacted_and_keyset_safe() {
        let queue = queue();
        let task = queue
            .enqueue_operation(
                "Verify provider".to_string(),
                JobOperation::ProviderVerify {
                    media_type: "game".to_string(),
                    provider_id: "provider-a".to_string(),
                },
                None,
            )
            .unwrap();
        queue
            .mark_running(
                &task.id,
                Some("Authorization: Bearer worker-secret".to_string()),
                Some(0.25),
            )
            .unwrap();
        queue
            .append_event(
                &task.id,
                TaskEventLevel::Warn,
                "remote_retry".to_string(),
                "https://alice:password@example.test/retry?token=query-secret".to_string(),
                Some(0.5),
            )
            .unwrap();
        queue
            .mark_failed(
                &task.id,
                ProviderError {
                    kind: ProviderErrorKind::Timeout,
                    message: "prompt={\"messages\":[\"do not persist\"]}".to_string(),
                    retryable: true,
                    retry_after_ms: None,
                    provider_id: Some("provider-a".to_string()),
                    operation: Some("verify".to_string()),
                },
            )
            .unwrap();

        let all = queue.get_task_events(&task.id, None, 20).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].code, "job_running");
        assert_eq!(all[1].code, "remote_retry");
        assert_eq!(all[2].code, "job_failed.timeout");
        assert!(all
            .windows(2)
            .all(|window| window[0].sequence < window[1].sequence));
        assert!(!all.iter().any(|event| event.message.contains("secret")));
        assert_eq!(all[2].message, "[REDACTED AI PAYLOAD]");
        assert_eq!(
            queue
                .get_task_events(&task.id, Some(all[0].sequence), 20)
                .unwrap()
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![all[1].sequence, all[2].sequence]
        );
        let failed = queue.get(&task.id).unwrap();
        assert_eq!(failed.status, TaskStatus::Failed);
        assert!(!failed.message.unwrap().contains("persist"));
    }

    #[test]
    fn late_worker_completion_cannot_overwrite_cancelled_lifecycle_state() {
        use std::sync::mpsc;

        let queue = Arc::new(queue());
        let task = queue
            .enqueue_operation("Update check".to_string(), JobOperation::UpdateCheck, None)
            .unwrap();
        queue.mark_running(&task.id, None, None).unwrap();

        let (worker_ready_tx, worker_ready_rx) = mpsc::channel();
        let (complete_tx, complete_rx) = mpsc::channel();
        let worker_queue = Arc::clone(&queue);
        let job_id = task.id.clone();
        let worker = thread::spawn(move || {
            worker_ready_tx.send(()).unwrap();
            complete_rx.recv().unwrap();
            worker_queue.mark_succeeded(&job_id, Some("late completion".to_string()))
        });

        worker_ready_rx.recv().unwrap();
        queue.cancel(&task.id).unwrap();
        complete_tx.send(()).unwrap();
        assert!(worker.join().unwrap().is_err());
        assert_eq!(queue.get(&task.id).unwrap().status, TaskStatus::Cancelled);
        assert!(queue
            .get_task_events(&task.id, None, 20)
            .unwrap()
            .iter()
            .all(|event| event.code != "job_succeeded"));
    }

    #[test]
    fn concurrent_updates_are_serialized_and_keep_progress_in_range() {
        let queue = Arc::new(queue());
        let task = queue.enqueue("parallel".into(), "test".into());
        queue
            .update(&task.id, Some(TaskStatus::Running), None, None)
            .unwrap();
        let mut workers = Vec::new();
        for i in 0..16 {
            let queue = Arc::clone(&queue);
            let id = task.id.clone();
            workers.push(thread::spawn(move || {
                queue
                    .update(&id, None, Some(i as f64 / 16.0), None)
                    .unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        let progress = queue.list()[0].progress;
        assert!((0.0..=1.0).contains(&progress));
    }
}
