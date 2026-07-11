use crate::downloader::Downloader;
use crate::task_queue::{
    AppTask, TaskAction, TaskCenterJob, TaskControlError, TaskKind, TaskQueue, TaskStatus,
};
use tauri::State;

/// Legacy command name backed by the persistent BackgroundJob control plane.
#[tauri::command]
pub fn enqueue_task(
    queue: State<'_, TaskQueue>,
    title: String,
    kind: String,
    idempotency_key: Option<String>,
) -> Result<AppTask, String> {
    queue.enqueue_with_key(title, kind, idempotency_key)
}

/// Backward-compatible command name with additive optional Task Center filters.
/// Calls that pass no arguments retain the previous all-jobs behavior.
#[tauri::command]
pub fn get_tasks(
    queue: State<'_, TaskQueue>,
    status: Option<TaskStatus>,
    kind: Option<TaskKind>,
    limit: Option<usize>,
) -> Result<Vec<TaskCenterJob>, String> {
    queue.list_task_center(status, kind, limit)
}

/// `progress` accepts both the new fraction (`0..=1`) and the legacy
/// percentage (`0..=100`) at the command boundary. It is persisted as a
/// fraction by TaskQueue.
#[tauri::command]
pub fn update_task(
    queue: State<'_, TaskQueue>,
    id: String,
    status: Option<TaskStatus>,
    progress: Option<f64>,
    message: Option<String>,
) -> Result<AppTask, String> {
    queue.update(&id, status, progress, message)
}

/// Cancels through the queue's atomic operation handle. Keeping this command
/// independent from producer-specific state preserves the legacy boundary and
/// lets every registered worker observe cancellation immediately.
#[tauri::command]
pub fn cancel_task(
    queue: State<'_, TaskQueue>,
    id: String,
) -> Result<TaskCenterJob, TaskControlError> {
    let task = task_for_action(&queue, &id, TaskAction::Cancel)?;
    if !task.cancellable {
        return Err(TaskControlError::invalid_state(TaskAction::Cancel, &task));
    }
    queue
        .cancel(&id)
        .map_err(|error| control_error(error, TaskAction::Cancel, &task))?;
    queue
        .get_task_center(&id)
        .map_err(TaskControlError::internal)
}

/// Pausing is currently implemented only for downloads because it requires a
/// real producer-specific stop boundary. Other kinds return a stable,
/// structured `action_not_supported` error instead of changing only the row.
#[tauri::command]
pub async fn pause_task(
    downloader: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    id: String,
) -> Result<TaskCenterJob, TaskControlError> {
    let task = task_for_action(&queue, &id, TaskAction::Pause)?;
    require_download_action(&task, TaskAction::Pause)?;
    if !task.pausable {
        return Err(TaskControlError::invalid_state(TaskAction::Pause, &task));
    }
    downloader
        .hydrate_persistent_jobs(queue.inner())
        .await
        .map_err(|error| control_error(error, TaskAction::Pause, &task))?;
    downloader
        .pause(&id)
        .await
        .map_err(|error| control_error(error, TaskAction::Pause, &task))?;
    if let Err(error) = queue.pause(
        &id,
        Some("已暂停；HTTP 连接已关闭，恢复时将使用 Range 续传".to_string()),
    ) {
        let _ = downloader
            .resume_persistent(queue.inner().clone(), &id)
            .await;
        return Err(control_error(error, TaskAction::Pause, &task));
    }
    queue
        .get_task_center(&id)
        .map_err(TaskControlError::internal)
}

#[tauri::command]
pub async fn resume_task(
    downloader: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    id: String,
) -> Result<TaskCenterJob, TaskControlError> {
    let task = task_for_action(&queue, &id, TaskAction::Resume)?;
    require_download_action(&task, TaskAction::Resume)?;
    if !task.resumable {
        return Err(TaskControlError::invalid_state(TaskAction::Resume, &task));
    }
    downloader
        .hydrate_persistent_jobs(queue.inner())
        .await
        .map_err(|error| control_error(error, TaskAction::Resume, &task))?;
    queue
        .resume(&id, Some("正在继续下载".to_string()))
        .map_err(|error| control_error(error, TaskAction::Resume, &task))?;
    if let Err(error) = downloader
        .resume_persistent(queue.inner().clone(), &id)
        .await
    {
        let _ = queue.pause(&id, Some(format!("恢复失败: {error}")));
        return Err(control_error(error, TaskAction::Resume, &task));
    }
    queue
        .get_task_center(&id)
        .map_err(TaskControlError::internal)
}

#[tauri::command]
pub async fn retry_task(
    downloader: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    id: String,
) -> Result<TaskCenterJob, TaskControlError> {
    let task = task_for_action(&queue, &id, TaskAction::Retry)?;
    require_download_action(&task, TaskAction::Retry)?;
    if !task.retryable {
        return Err(TaskControlError::invalid_state(TaskAction::Retry, &task));
    }
    downloader
        .hydrate_persistent_jobs(queue.inner())
        .await
        .map_err(|error| control_error(error, TaskAction::Retry, &task))?;
    queue
        .retry(&id, Some("已排队重试".to_string()))
        .map_err(|error| control_error(error, TaskAction::Retry, &task))?;
    downloader
        .retry_persistent(queue.inner().clone(), &id)
        .await
        .map_err(|error| control_error(error, TaskAction::Retry, &task))?;
    queue
        .get_task_center(&id)
        .map_err(TaskControlError::internal)
}

#[tauri::command]
pub fn clear_finished_tasks(queue: State<'_, TaskQueue>) -> Result<(), String> {
    queue.clear_finished()
}

fn task_for_action(
    queue: &TaskQueue,
    id: &str,
    action: TaskAction,
) -> Result<TaskCenterJob, TaskControlError> {
    queue
        .get_task_center(id)
        .map_err(|message| TaskControlError {
            code: if message.contains("不存在") {
                "task_not_found".to_string()
            } else {
                "task_control_failed".to_string()
            },
            message,
            action: Some(action),
            task_id: Some(id.to_string()),
            kind: None,
            status: None,
        })
}

fn require_download_action(
    task: &TaskCenterJob,
    action: TaskAction,
) -> Result<(), TaskControlError> {
    if task.kind == TaskKind::Download {
        Ok(())
    } else {
        Err(TaskControlError::unsupported(action, task))
    }
}

fn control_error(message: String, action: TaskAction, task: &TaskCenterJob) -> TaskControlError {
    TaskControlError::internal(message).with_context(action, task)
}
