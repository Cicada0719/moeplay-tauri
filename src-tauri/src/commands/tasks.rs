use crate::task_queue::{AppTask, TaskQueue, TaskStatus};
use tauri::State;

#[tauri::command]
pub fn enqueue_task(queue: State<'_, TaskQueue>, title: String, kind: String) -> AppTask {
    queue.enqueue(title, kind)
}

#[tauri::command]
pub fn get_tasks(queue: State<'_, TaskQueue>) -> Vec<AppTask> {
    queue.list()
}

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
pub fn clear_finished_tasks(queue: State<'_, TaskQueue>) {
    queue.clear_finished();
}
