use crate::ai::{
    AiError, AiErrorKind, AiExecutionResult, AiOrchestrator, AiProviderSpec, AiProviderStatus,
    AiTaskStartSpec, BudgetSnapshot, CancellationProbe, SecretStoreCredentialSource,
};
use crate::db_sqlite::repositories::{AiTaskResultRecord, AiTaskResultRepository};
use crate::db_sqlite::SqliteDb;
use crate::secret_store::SecretStore;
use crate::task_queue::{AppTask, CancellationHandle, TaskQueue, TaskStatus};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};

pub struct AiV2State {
    orchestrator: AiOrchestrator,
    database: Arc<SqliteDb>,
    outcomes: Mutex<HashMap<String, StoredOutcome>>,
}

impl AiV2State {
    pub fn try_new(database: Arc<SqliteDb>) -> Result<Self, AiV2ErrorDto> {
        AiTaskResultRepository::new(&database)
            .prune_expired(Utc::now())
            .map_err(|_| persistence_error())?;
        Ok(Self {
            orchestrator: AiOrchestrator::production().map_err(AiV2ErrorDto::from)?,
            database,
            outcomes: Mutex::new(HashMap::new()),
        })
    }

    fn store(&self, task_id: String, outcome: StoredOutcome) -> Result<(), AiV2ErrorDto> {
        let now = Utc::now();
        let outcome_json = serde_json::to_string(&outcome).map_err(|_| persistence_error())?;
        AiTaskResultRepository::new(&self.database)
            .upsert(&AiTaskResultRecord {
                task_id: task_id.clone(),
                outcome_kind: outcome.kind().to_string(),
                outcome_json,
                created_at: now.to_rfc3339(),
                expires_at: (now + Duration::days(7)).to_rfc3339(),
            })
            .map_err(|_| persistence_error())?;
        self.outcomes
            .lock()
            .map_err(|_| persistence_error())?
            .insert(task_id, outcome);
        Ok(())
    }

    fn get(&self, task_id: &str) -> Result<Option<StoredOutcome>, AiV2ErrorDto> {
        if let Some(outcome) = self
            .outcomes
            .lock()
            .map_err(|_| persistence_error())?
            .get(task_id)
            .cloned()
        {
            return Ok(Some(outcome));
        }
        let Some(record) = AiTaskResultRepository::new(&self.database)
            .get(task_id, Utc::now())
            .map_err(|_| persistence_error())?
        else {
            return Ok(None);
        };
        let outcome: StoredOutcome =
            serde_json::from_str(&record.outcome_json).map_err(|_| persistence_error())?;
        if outcome.kind() != record.outcome_kind {
            return Err(persistence_error());
        }
        self.outcomes
            .lock()
            .map_err(|_| persistence_error())?
            .insert(task_id.to_string(), outcome.clone());
        Ok(Some(outcome))
    }

    fn remove(&self, task_id: &str) {
        if let Ok(mut outcomes) = self.outcomes.lock() {
            outcomes.remove(task_id);
        }
        if let Err(error) = AiTaskResultRepository::new(&self.database).delete(task_id) {
            tracing::warn!(task_id, error = %error, "failed to delete persisted AI task result");
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "payload", rename_all = "snake_case")]
enum StoredOutcome {
    Succeeded(AiExecutionResult),
    Failed(AiError),
}

impl StoredOutcome {
    fn kind(&self) -> &'static str {
        match self {
            Self::Succeeded(_) => "succeeded",
            Self::Failed(_) => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiV2ErrorDto {
    pub kind: AiErrorKind,
    pub message: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
}

impl From<AiError> for AiV2ErrorDto {
    fn from(error: AiError) -> Self {
        Self {
            kind: error.kind,
            message: error.message,
            retryable: error.retryable,
            retry_after_ms: error.retry_after_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiV2TaskStatusDto {
    pub id: String,
    pub kind: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub created_at: String,
    pub updated_at: String,
    pub message: Option<String>,
    pub result_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiV2TaskResultDto {
    pub task: AiV2TaskStatusDto,
    pub result: Option<AiExecutionResult>,
    pub error: Option<AiV2ErrorDto>,
}

#[tauri::command]
pub fn ai_v2_provider_status(
    state: State<'_, AiV2State>,
    secrets: State<'_, SecretStore>,
    provider: AiProviderSpec,
) -> Result<AiProviderStatus, AiV2ErrorDto> {
    let credentials = SecretStoreCredentialSource::new(&secrets);
    state
        .orchestrator
        .provider_status(&provider, &credentials)
        .map_err(AiV2ErrorDto::from)
}

#[tauri::command]
pub fn ai_v2_budget_status(state: State<'_, AiV2State>) -> BudgetSnapshot {
    state.orchestrator.budget_snapshot()
}

#[tauri::command]
pub fn ai_v2_start_structured_task(
    app: AppHandle,
    state: State<'_, AiV2State>,
    queue: State<'_, TaskQueue>,
    secrets: State<'_, SecretStore>,
    request: AiTaskStartSpec,
) -> Result<AiV2TaskStatusDto, AiV2ErrorDto> {
    let credentials = SecretStoreCredentialSource::new(&secrets);
    state
        .orchestrator
        .validate_start_spec(&request, &credentials)
        .map_err(AiV2ErrorDto::from)?;

    let task = queue
        .enqueue_with_key(
            request.task.task_title().to_string(),
            request.task.task_kind().to_string(),
            None,
        )
        .map_err(queue_error)?;
    let task_id = task.id.clone();
    tauri::async_runtime::spawn(run_task(app, task_id, request));
    Ok(to_status(task, false))
}
#[tauri::command]
pub fn ai_v2_task_status(
    state: State<'_, AiV2State>,
    queue: State<'_, TaskQueue>,
    task_id: String,
) -> Result<AiV2TaskStatusDto, AiV2ErrorDto> {
    let task = find_ai_task(&queue, &task_id)?;
    Ok(to_status(task, state.get(&task_id)?.is_some()))
}

#[tauri::command]
pub fn ai_v2_task_result(
    state: State<'_, AiV2State>,
    queue: State<'_, TaskQueue>,
    task_id: String,
) -> Result<AiV2TaskResultDto, AiV2ErrorDto> {
    let task = find_ai_task(&queue, &task_id)?;
    let outcome = state.get(&task_id)?;
    let status = to_status(task, outcome.is_some());
    let (result, error) = match outcome {
        Some(StoredOutcome::Succeeded(result)) => (Some(result), None),
        Some(StoredOutcome::Failed(error)) => (None, Some(error.into())),
        None if status.status == TaskStatus::Succeeded => (
            None,
            Some(AiV2ErrorDto::from(AiError::new(
                AiErrorKind::NotConfigured,
                "AI task result is no longer retained in memory",
                false,
            ))),
        ),
        None => (None, None),
    };
    Ok(AiV2TaskResultDto {
        task: status,
        result,
        error,
    })
}

#[tauri::command]
pub fn ai_v2_cancel_task(
    state: State<'_, AiV2State>,
    queue: State<'_, TaskQueue>,
    task_id: String,
) -> Result<AiV2TaskStatusDto, AiV2ErrorDto> {
    let _ = find_ai_task(&queue, &task_id)?;
    let task = queue.cancel(&task_id).map_err(queue_error)?;
    state.remove(&task_id);
    Ok(to_status(task, false))
}

async fn run_task(app: AppHandle, task_id: String, request: AiTaskStartSpec) {
    let queue = app.state::<TaskQueue>();
    let cancellation = match queue.register_operation(&task_id) {
        Ok(handle) => QueueCancellation(handle),
        Err(_) => return,
    };
    if cancellation.is_cancelled()
        || queue
            .update(
                &task_id,
                Some(TaskStatus::Running),
                Some(0.05),
                Some("AI task running".to_string()),
            )
            .is_err()
    {
        return;
    }

    let result = {
        let state = app.state::<AiV2State>();
        let secrets = app.state::<SecretStore>();
        let credentials = SecretStoreCredentialSource::new(&secrets);
        state
            .orchestrator
            .execute(&task_id, &request, &credentials, &cancellation)
            .await
    };

    match result {
        Ok(result) => {
            if cancellation.is_cancelled() {
                return;
            }
            let state = app.state::<AiV2State>();
            if state
                .store(task_id.clone(), StoredOutcome::Succeeded(result))
                .is_err()
            {
                let _ = queue.update(
                    &task_id,
                    Some(TaskStatus::Failed),
                    None,
                    Some("AI task result persistence failed".to_string()),
                );
                return;
            }
            if queue
                .update(
                    &task_id,
                    Some(TaskStatus::Succeeded),
                    Some(1.0),
                    Some("AI task completed".to_string()),
                )
                .is_err()
            {
                state.remove(&task_id);
            }
        }
        Err(error) => {
            if cancellation.is_cancelled() || error.kind == AiErrorKind::Cancelled {
                return;
            }
            let message = format!("AI task failed: {:?}", error.kind).to_ascii_lowercase();
            let state = app.state::<AiV2State>();
            if state
                .store(task_id.clone(), StoredOutcome::Failed(error))
                .is_err()
            {
                let _ = queue.update(
                    &task_id,
                    Some(TaskStatus::Failed),
                    None,
                    Some("AI task result persistence failed".to_string()),
                );
                return;
            }
            if queue
                .update(&task_id, Some(TaskStatus::Failed), None, Some(message))
                .is_err()
            {
                state.remove(&task_id);
            }
        }
    }
}

struct QueueCancellation(CancellationHandle);

impl CancellationProbe for QueueCancellation {
    fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

fn find_ai_task(queue: &TaskQueue, task_id: &str) -> Result<AppTask, AiV2ErrorDto> {
    let task = queue
        .list_result()
        .map_err(queue_error)?
        .into_iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| {
            AiV2ErrorDto::from(AiError::new(
                AiErrorKind::NotConfigured,
                "AI task was not found",
                false,
            ))
        })?;
    if !task.kind.starts_with("ai_v2.") {
        return Err(AiV2ErrorDto::from(AiError::new(
            AiErrorKind::PolicyRejected,
            "task is not owned by AI v2",
            false,
        )));
    }
    Ok(task)
}

fn to_status(task: AppTask, result_available: bool) -> AiV2TaskStatusDto {
    AiV2TaskStatusDto {
        id: task.id,
        kind: task.kind,
        status: task.status,
        progress: task.progress,
        created_at: task.created_at,
        updated_at: task.updated_at,
        message: task.message,
        result_available,
    }
}

fn persistence_error() -> AiV2ErrorDto {
    AiV2ErrorDto::from(AiError::new(
        AiErrorKind::ProviderUnavailable,
        "AI task result persistence failed",
        true,
    ))
}

fn queue_error(_: String) -> AiV2ErrorDto {
    AiV2ErrorDto::from(AiError::new(
        AiErrorKind::ProviderUnavailable,
        "AI task queue operation failed",
        true,
    ))
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn completed_outcome_survives_state_recreation() {
        let path = std::env::temp_dir().join(format!(
            "moeplay_ai_result_restart_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let database = Arc::new(SqliteDb::open(&path).unwrap());
            let first = AiV2State::try_new(Arc::clone(&database)).unwrap();
            first
                .store(
                    "task-persisted".to_string(),
                    StoredOutcome::Failed(AiError::new(
                        AiErrorKind::InvalidOutput,
                        "validated output was rejected",
                        false,
                    )),
                )
                .unwrap();
        }

        {
            let reopened = Arc::new(SqliteDb::open(&path).unwrap());
            let restored = AiV2State::try_new(reopened).unwrap();
            match restored.get("task-persisted").unwrap() {
                Some(StoredOutcome::Failed(error)) => {
                    assert_eq!(error.kind, AiErrorKind::InvalidOutput);
                    assert_eq!(error.message, "validated output was rejected");
                }
                _ => panic!("persisted AI outcome was not restored"),
            }
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    #[test]
    fn cancellation_removes_persisted_outcome() {
        let database = Arc::new(SqliteDb::open_in_memory().unwrap());
        let state = AiV2State::try_new(Arc::clone(&database)).unwrap();
        state
            .store(
                "task-cancelled".to_string(),
                StoredOutcome::Failed(AiError::new(AiErrorKind::Cancelled, "cancelled", false)),
            )
            .unwrap();
        state.remove("task-cancelled");
        drop(state);

        let restored = AiV2State::try_new(database).unwrap();
        assert!(restored.get("task-cancelled").unwrap().is_none());
    }
}
