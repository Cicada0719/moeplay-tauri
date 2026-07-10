use crate::task_queue::{AppTask, TaskQueue, TaskStatus};
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

#[tauri::command]
pub fn get_tasks(queue: State<'_, TaskQueue>) -> Result<Vec<AppTask>, String> {
    queue.list_result()
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

#[tauri::command]
pub fn cancel_task(queue: State<'_, TaskQueue>, id: String) -> Result<AppTask, String> {
    queue.cancel(&id)
}

#[tauri::command]
pub fn clear_finished_tasks(queue: State<'_, TaskQueue>) -> Result<(), String> {
    queue.clear_finished()
}
