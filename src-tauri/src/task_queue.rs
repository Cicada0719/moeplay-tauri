//! 简单任务队列

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTask {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub created_at: String,
    pub updated_at: String,
    pub message: Option<String>,
}

pub struct TaskQueue {
    tasks: Mutex<Vec<AppTask>>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
        }
    }

    pub fn enqueue(&self, title: String, kind: String) -> AppTask {
        let now = timestamp();
        let task = AppTask {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            kind,
            status: TaskStatus::Pending,
            progress: 0.0,
            created_at: now.clone(),
            updated_at: now,
            message: None,
        };
        self.tasks.lock().unwrap().push(task.clone());
        task
    }

    pub fn list(&self) -> Vec<AppTask> {
        let mut tasks = self.tasks.lock().unwrap().clone();
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        tasks
    }

    pub fn update(
        &self,
        id: &str,
        status: Option<TaskStatus>,
        progress: Option<f64>,
        message: Option<String>,
    ) -> Result<AppTask, String> {
        let mut tasks = self.tasks.lock().unwrap();
        let task = tasks
            .iter_mut()
            .find(|task| task.id == id)
            .ok_or("任务不存在")?;
        if let Some(status) = status {
            task.status = status;
        }
        if let Some(progress) = progress {
            task.progress = progress.clamp(0.0, 100.0);
        }
        if message.is_some() {
            task.message = message;
        }
        task.updated_at = timestamp();
        Ok(task.clone())
    }

    pub fn cancel(&self, id: &str) -> Result<AppTask, String> {
        self.update(
            id,
            Some(TaskStatus::Cancelled),
            None,
            Some("已取消".to_string()),
        )
    }

    pub fn clear_finished(&self) {
        self.tasks.lock().unwrap().retain(|task| {
            !matches!(
                task.status,
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
            )
        });
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn timestamp() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
