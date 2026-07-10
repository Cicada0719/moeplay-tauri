//! 萌游下载管理器 - 流式下载、暂停续传、断点续传、限速、队列管理

use crate::task_queue::{AppTask, CancellationHandle, TaskQueue, TaskStatus};
use reqwest::header::{CONTENT_RANGE, RANGE};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use sysinfo::Disks;
use tokio::sync::{Mutex, Notify, Semaphore};
use tokio::time::{sleep, Duration};

use futures_util::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadPreflightEvidence {
    pub checked_at: String,
    pub available_bytes: Option<u64>,
    pub required_bytes: Option<u64>,
    pub minimum_free_bytes: u64,
    pub quota_bytes: Option<u64>,
    pub quota_used_bytes: u64,
    pub quota_remaining_bytes: Option<u64>,
    pub quota_source: String,
    pub accepted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub save_path: PathBuf,
    pub total_size: u64,
    pub downloaded_size: u64,
    pub progress: f32,
    pub speed: f64, // bytes/sec
    pub status: DownloadStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error: Option<String>,
    pub auto_extract: bool,
    pub auto_import: bool,
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub recovered: bool,
    #[serde(default)]
    pub resumable: bool,
    #[serde(default)]
    pub retryable: bool,
    #[serde(default)]
    pub quota_bytes: Option<u64>,
    #[serde(default)]
    pub preflight: Option<DownloadPreflightEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Paused,
    Completed,
    Failed,
    Extracting,
    Importing,
    Cancelled,
}

// ---- 内部类型 ----

#[derive(Debug)]
struct TaskControl {
    pause_notify: Notify,
    execution_stopped: Notify,
    write_barrier: Mutex<()>,
    paused: AtomicBool,
    cancelled: AtomicBool,
    started: AtomicBool,
    archive_active: AtomicBool,
}

impl TaskControl {
    fn new() -> Self {
        Self {
            pause_notify: Notify::new(),
            execution_stopped: Notify::new(),
            write_barrier: Mutex::new(()),
            paused: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            started: AtomicBool::new(false),
            archive_active: AtomicBool::new(false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadStreamOutcome {
    Completed,
    Paused,
}

/// ZIP extraction can only observe cancellation between entries; external 7z/rar
/// extraction cannot be interrupted safely by the current blocking process API.
pub const ARCHIVE_CANCELLATION_LIMITATION: &str = crate::archive::ARCHIVE_CANCELLATION_LIMITATION;

#[derive(Debug)]
struct TokenBucketInner {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Debug)]
struct TokenBucket {
    rate: AtomicU64,
    inner: Mutex<TokenBucketInner>,
}

impl TokenBucket {
    fn new(rate: u64) -> Self {
        Self {
            rate: AtomicU64::new(rate),
            inner: Mutex::new(TokenBucketInner {
                tokens: rate as f64,
                last_refill: Instant::now(),
            }),
        }
    }
    fn set_rate(&self, r: u64) {
        self.rate.store(r, Ordering::Relaxed);
    }
    async fn consume(&self, bytes: u64) {
        let rate = self.rate.load(Ordering::Relaxed);
        if rate == 0 || bytes == 0 {
            return;
        }
        let rf = rate as f64;
        let need = bytes as f64;
        loop {
            let wait = {
                let mut inner = self.inner.lock().await;
                let now = Instant::now();
                inner.tokens = (inner.tokens
                    + now.duration_since(inner.last_refill).as_secs_f64() * rf)
                    .min(rf);
                inner.last_refill = now;
                if inner.tokens >= need {
                    inner.tokens -= need;
                    return;
                }
                let d = need - inner.tokens;
                inner.tokens = 0.0;
                d / rf
            };
            sleep(Duration::from_secs_f64(wait)).await;
        }
    }
}

// ---- 下载器 ----

#[derive(Clone)]
struct PersistentOperation {
    queue: TaskQueue,
    cancellation: CancellationHandle,
}

#[derive(Debug, Clone)]
pub struct Downloader {
    tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
    controls: Arc<Mutex<HashMap<String, Arc<TaskControl>>>>,
    semaphore: Arc<Semaphore>,
    client: Client,
    download_dir: PathBuf,
    speed_limit: Arc<AtomicU64>,
    token_bucket: Arc<TokenBucket>,
    quota_bytes: Arc<AtomicU64>,
    minimum_free_bytes: u64,
}

impl Downloader {
    pub fn new(download_dir: PathBuf, max_concurrent: u32) -> Self {
        let max = if max_concurrent > 0 {
            max_concurrent as usize
        } else {
            3
        };
        let client = crate::http_client::build_reqwest_client(
            30 * 60,
            crate::http_client::browser_user_agent(),
        );

        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            controls: Arc::new(Mutex::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max)),
            client,
            download_dir,
            speed_limit: Arc::new(AtomicU64::new(0)),
            token_bucket: Arc::new(TokenBucket::new(0)),
            quota_bytes: Arc::new(AtomicU64::new(env_u64("MOEPLAY_DOWNLOAD_QUOTA_BYTES"))),
            minimum_free_bytes: match env_u64("MOEPLAY_DOWNLOAD_MIN_FREE_BYTES") {
                0 => 256 * 1024 * 1024,
                configured => configured,
            },
        }
    }

    // ---- 公共 API ----

    pub async fn enqueue(
        &self,
        url: String,
        filename: String,
        auto_extract: bool,
        auto_import: bool,
    ) -> DownloadTask {
        let id = uuid::Uuid::new_v4().to_string();
        // 按文件名建子目录
        let safe_name = sanitize_filename(&filename);
        let save_dir = self.download_dir.join(&safe_name);
        std::fs::create_dir_all(&save_dir).ok();
        let save_path = get_unique_file_path(&save_dir.join(&filename));
        let actual_filename = save_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let task = DownloadTask {
            id: id.clone(),
            url,
            filename: actual_filename,
            save_path,
            total_size: 0,
            downloaded_size: 0,
            progress: 0.0,
            speed: 0.0,
            status: DownloadStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            error: None,
            auto_extract,
            auto_import,
            headers: HashMap::new(),
            recovered: false,
            resumable: true,
            retryable: false,
            quota_bytes: configured_quota(self.quota_bytes.load(Ordering::Relaxed), None),
            preflight: None,
        };

        let control = Arc::new(TaskControl::new());
        self.tasks.lock().await.insert(id.clone(), task.clone());
        self.controls.lock().await.insert(id.clone(), control);
        self.spawn_download(&id, None);
        task
    }

    pub async fn enqueue_persistent(
        &self,
        queue: TaskQueue,
        job: AppTask,
        url: String,
        filename: String,
        auto_extract: bool,
        auto_import: bool,
        quota_override: Option<u64>,
    ) -> Result<DownloadTask, String> {
        if let Some(existing) = self.tasks.lock().await.get(&job.id).cloned() {
            return Ok(existing);
        }

        let metadata = queue.metadata(&job.id)?;
        let saved_path = metadata
            .get("savePath")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        let save_path = match saved_path {
            Some(path) => path,
            None => {
                let safe_name = sanitize_filename(&filename);
                let save_dir = self.download_dir.join(&safe_name);
                std::fs::create_dir_all(&save_dir)
                    .map_err(|error| format!("无法创建下载目录: {error}"))?;
                get_unique_file_path(&save_dir.join(&filename))
            }
        };
        let actual_filename = save_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let existing_bytes = std::fs::metadata(&save_path).map(|m| m.len()).unwrap_or(0);
        let quota_bytes = configured_quota(
            self.quota_bytes.load(Ordering::Relaxed),
            quota_override.or_else(|| metadata.get("quotaBytes").and_then(Value::as_u64)),
        );
        let control = TaskControl::new();
        control
            .paused
            .store(job.status == TaskStatus::Paused, Ordering::Relaxed);
        control
            .cancelled
            .store(job.status == TaskStatus::Cancelled, Ordering::Relaxed);
        let task = DownloadTask {
            id: job.id.clone(),
            url: metadata
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or(&url)
                .to_string(),
            filename: actual_filename,
            save_path: save_path.clone(),
            total_size: metadata
                .get("totalSize")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            downloaded_size: metadata
                .get("downloadedSize")
                .and_then(Value::as_u64)
                .unwrap_or(existing_bytes),
            progress: job.progress as f32,
            speed: 0.0,
            status: download_status(job.status),
            retry_count: metadata
                .get("retryCount")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            max_retries: metadata
                .get("maxRetries")
                .and_then(Value::as_u64)
                .unwrap_or(3) as u32,
            error: if job.status == TaskStatus::Failed {
                job.message.clone()
            } else {
                None
            },
            auto_extract: metadata
                .get("autoExtract")
                .and_then(Value::as_bool)
                .unwrap_or(auto_extract),
            auto_import: metadata
                .get("autoImport")
                .and_then(Value::as_bool)
                .unwrap_or(auto_import),
            headers: HashMap::new(),
            recovered: job.recovered,
            resumable: job.resumable,
            retryable: job.retryable,
            quota_bytes,
            preflight: metadata
                .get("preflight")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok()),
        };

        if matches!(
            job.status,
            TaskStatus::Queued | TaskStatus::Running | TaskStatus::Paused
        ) {
            queue.patch_metadata(
                &job.id,
                json!({
                    "url": task.url,
                    "filename": task.filename,
                    "savePath": task.save_path,
                    "autoExtract": task.auto_extract,
                    "autoImport": task.auto_import,
                    "quotaBytes": task.quota_bytes,
                    "downloadedSize": task.downloaded_size,
                    "totalSize": task.total_size,
                    "maxRetries": task.max_retries,
                    "resumable": true
                }),
            )?;
        }
        let control = Arc::new(control);
        self.tasks.lock().await.insert(job.id.clone(), task.clone());
        self.controls.lock().await.insert(job.id.clone(), control);

        if job.status == TaskStatus::Queued {
            let cancellation = queue.register_operation(&job.id)?;
            self.spawn_download(
                &job.id,
                Some(PersistentOperation {
                    queue,
                    cancellation,
                }),
            );
        }
        Ok(task)
    }

    pub async fn hydrate_persistent_jobs(&self, queue: &TaskQueue) -> Result<(), String> {
        let jobs = queue.list_result()?;
        for job in jobs.into_iter().filter(|job| job.kind == "download") {
            if self.tasks.lock().await.contains_key(&job.id) {
                continue;
            }
            let metadata = queue.metadata(&job.id)?;
            let Some(url) = metadata.get("url").and_then(Value::as_str) else {
                continue;
            };
            let filename = metadata
                .get("filename")
                .and_then(Value::as_str)
                .unwrap_or("download.bin");
            self.enqueue_persistent(
                queue.clone(),
                job,
                url.to_string(),
                filename.to_string(),
                metadata
                    .get("autoExtract")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                metadata
                    .get("autoImport")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                metadata.get("quotaBytes").and_then(Value::as_u64),
            )
            .await?;
        }
        Ok(())
    }

    pub async fn resume_persistent(&self, queue: TaskQueue, task_id: &str) -> Result<(), String> {
        self.hydrate_persistent_jobs(&queue).await?;
        self.prepare_resume(task_id).await?;
        let cancellation = queue.register_operation(task_id)?;
        self.spawn_download(
            task_id,
            Some(PersistentOperation {
                queue,
                cancellation,
            }),
        );
        Ok(())
    }

    pub async fn retry_persistent(&self, queue: TaskQueue, task_id: &str) -> Result<(), String> {
        self.hydrate_persistent_jobs(&queue).await?;
        {
            let mut tasks = self.tasks.lock().await;
            let task = tasks.get_mut(task_id).ok_or("任务不存在")?;
            task.status = DownloadStatus::Pending;
            task.error = None;
            task.retryable = false;
            task.recovered = false;
        }
        if let Some(ctrl) = self.controls.lock().await.get(task_id) {
            ctrl.cancelled.store(false, Ordering::Relaxed);
            ctrl.paused.store(false, Ordering::Relaxed);
            ctrl.pause_notify.notify_one();
        }
        let cancellation = queue.register_operation(task_id)?;
        self.spawn_download(
            task_id,
            Some(PersistentOperation {
                queue,
                cancellation,
            }),
        );
        Ok(())
    }

    pub async fn pause(&self, task_id: &str) -> Result<(), String> {
        let ctrl = self
            .controls
            .lock()
            .await
            .get(task_id)
            .cloned()
            .ok_or("任务不存在")?;
        let status = self
            .tasks
            .lock()
            .await
            .get(task_id)
            .map(|task| task.status.clone())
            .ok_or("任务不存在")?;
        if ctrl.archive_active.load(Ordering::Acquire)
            || matches!(
                status,
                DownloadStatus::Extracting | DownloadStatus::Importing
            )
        {
            return Err(ARCHIVE_CANCELLATION_LIMITATION.to_string());
        }
        if matches!(
            status,
            DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
        ) {
            return Err("终态下载不可暂停".to_string());
        }

        ctrl.paused.store(true, Ordering::Release);
        ctrl.pause_notify.notify_waiters();
        // A writer checks the pause flag while holding this barrier. Once we
        // acquire it, no later chunk can be committed after pause() returns.
        let barrier = ctrl.write_barrier.lock().await;
        drop(barrier);
        wait_for_execution_stop(&ctrl).await;

        let mut tasks = self.tasks.lock().await;
        let task = tasks.get_mut(task_id).ok_or("任务不存在")?;
        if task.status != DownloadStatus::Cancelled {
            task.status = DownloadStatus::Paused;
            task.speed = 0.0;
            task.resumable = true;
        }
        Ok(())
    }

    pub async fn resume(&self, task_id: &str) -> Result<(), String> {
        self.prepare_resume(task_id).await?;
        self.spawn_download(task_id, None);
        Ok(())
    }

    async fn prepare_resume(&self, task_id: &str) -> Result<(), String> {
        let ctrl = self
            .controls
            .lock()
            .await
            .get(task_id)
            .cloned()
            .ok_or("任务不存在")?;
        if ctrl.archive_active.load(Ordering::Acquire) {
            return Err(ARCHIVE_CANCELLATION_LIMITATION.to_string());
        }
        let mut tasks = self.tasks.lock().await;
        let task = tasks.get_mut(task_id).ok_or("任务不存在")?;
        if !matches!(task.status, DownloadStatus::Paused | DownloadStatus::Failed) {
            return Err("只有暂停或失败的下载可以恢复".to_string());
        }
        ctrl.cancelled.store(false, Ordering::Release);
        ctrl.paused.store(false, Ordering::Release);
        ctrl.pause_notify.notify_waiters();
        task.status = DownloadStatus::Downloading;
        task.error = None;
        task.retryable = false;
        task.resumable = true;
        Ok(())
    }

    pub async fn archive_cancellation_is_deferred(&self, task_id: &str) -> Result<bool, String> {
        let ctrl = self
            .controls
            .lock()
            .await
            .get(task_id)
            .cloned()
            .ok_or("任务不存在")?;
        let status = self
            .tasks
            .lock()
            .await
            .get(task_id)
            .map(|task| task.status.clone())
            .ok_or("任务不存在")?;
        Ok(ctrl.archive_active.load(Ordering::Acquire)
            || matches!(
                status,
                DownloadStatus::Extracting | DownloadStatus::Importing
            ))
    }

    pub async fn cancel(&self, task_id: &str) -> Result<(), String> {
        let ctrl = self
            .controls
            .lock()
            .await
            .get(task_id)
            .cloned()
            .ok_or("任务不存在")?;
        ctrl.cancelled.store(true, Ordering::Release);
        ctrl.pause_notify.notify_waiters();
        let barrier = ctrl.write_barrier.lock().await;
        drop(barrier);

        let mut tasks = self.tasks.lock().await;
        let task = tasks.get_mut(task_id).ok_or("任务不存在")?;
        task.status = DownloadStatus::Cancelled;
        task.speed = 0.0;
        task.resumable = false;
        task.retryable = false;
        if ctrl.archive_active.load(Ordering::Acquire) {
            task.error = Some(ARCHIVE_CANCELLATION_LIMITATION.to_string());
        }
        Ok(())
    }

    pub async fn cancel_all(&self) -> Result<(), String> {
        let controls = self
            .controls
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for ctrl in &controls {
            ctrl.cancelled.store(true, Ordering::Release);
            ctrl.pause_notify.notify_waiters();
        }
        for ctrl in &controls {
            let barrier = ctrl.write_barrier.lock().await;
            drop(barrier);
        }
        let mut tasks = self.tasks.lock().await;
        for task in tasks.values_mut() {
            if matches!(
                task.status,
                DownloadStatus::Downloading
                    | DownloadStatus::Pending
                    | DownloadStatus::Paused
                    | DownloadStatus::Extracting
                    | DownloadStatus::Importing
            ) {
                task.status = DownloadStatus::Cancelled;
                task.speed = 0.0;
                task.resumable = false;
                task.retryable = false;
            }
        }
        Ok(())
    }

    pub async fn retry(&self, task_id: &str) -> Result<(), String> {
        {
            let mut t = self.tasks.lock().await;
            let task = t.get_mut(task_id).ok_or("任务不存在")?;
            if task.status != DownloadStatus::Failed || task.retry_count >= task.max_retries {
                return Err("无法重试".into());
            }
            task.status = DownloadStatus::Pending;
            task.retry_count += 1;
            task.error = None;
        }
        {
            let c = self.controls.lock().await;
            if let Some(ctrl) = c.get(task_id) {
                ctrl.cancelled.store(false, Ordering::Relaxed);
            }
        }
        self.spawn_download(task_id, None);
        Ok(())
    }

    pub async fn remove(&self, task_id: &str) -> Result<(), String> {
        let c = self.controls.lock().await;
        if let Some(ctrl) = c.get(task_id) {
            ctrl.cancelled.store(true, Ordering::Relaxed);
            ctrl.pause_notify.notify_one();
        }
        drop(c);
        self.tasks
            .lock()
            .await
            .remove(task_id)
            .map(|_| ())
            .ok_or("任务不存在".into())
    }

    pub async fn get_all(&self) -> Vec<DownloadTask> {
        self.tasks.lock().await.values().cloned().collect()
    }

    pub async fn get_active(&self) -> Vec<DownloadTask> {
        self.tasks
            .lock()
            .await
            .values()
            .filter(|t| {
                matches!(
                    t.status,
                    DownloadStatus::Downloading | DownloadStatus::Pending
                )
            })
            .cloned()
            .collect()
    }

    pub async fn clear_finished(&self) -> Result<(), String> {
        let mut t = self.tasks.lock().await;
        t.retain(|_, t| {
            !matches!(
                t.status,
                DownloadStatus::Completed | DownloadStatus::Cancelled | DownloadStatus::Failed
            )
        });
        Ok(())
    }

    pub async fn set_speed_limit(&self, limit: Option<u64>) {
        let r = limit.unwrap_or(0);
        self.speed_limit.store(r, Ordering::Relaxed);
        self.token_bucket.set_rate(r);
    }

    pub async fn get_speed_limit(&self) -> u64 {
        self.speed_limit.load(Ordering::Relaxed)
    }

    pub async fn set_max_concurrent(&self, _max: u32) {
        log::warn!("[萌游] set_max_concurrent 需重启生效");
    }

    pub async fn get_max_concurrent(&self) -> u32 {
        self.semaphore.available_permits() as u32
    }

    // ---- 内部方法 ----

    fn spawn_download(&self, task_id: &str, persistent: Option<PersistentOperation>) {
        let tid = task_id.to_string();
        let tasks = self.tasks.clone();
        let controls = self.controls.clone();
        let sem = self.semaphore.clone();
        let client = self.client.clone();
        let tb = self.token_bucket.clone();
        let download_dir = self.download_dir.clone();
        let minimum_free_bytes = self.minimum_free_bytes;
        tokio::spawn(async move {
            let Some(control) = controls.lock().await.get(&tid).cloned() else {
                return;
            };
            if control
                .started
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return;
            }
            Self::execute_download(
                tid,
                tasks,
                controls,
                sem,
                client,
                tb,
                download_dir,
                minimum_free_bytes,
                persistent,
            )
            .await;
            control.started.store(false, Ordering::Release);
            control.execution_stopped.notify_waiters();
        });
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_download(
        task_id: String,
        tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
        controls: Arc<Mutex<HashMap<String, Arc<TaskControl>>>>,
        semaphore: Arc<Semaphore>,
        client: Client,
        token_bucket: Arc<TokenBucket>,
        download_dir: PathBuf,
        minimum_free_bytes: u64,
        persistent: Option<PersistentOperation>,
    ) {
        let ctrl = match controls.lock().await.get(&task_id).cloned() {
            Some(control) => control,
            None => return,
        };
        let max_retries = tasks
            .lock()
            .await
            .get(&task_id)
            .map(|task| task.max_retries)
            .unwrap_or(3);

        loop {
            if operation_cancelled(&ctrl, persistent.as_ref()) {
                mark_local_cancelled(&tasks, &task_id).await;
                return;
            }

            if ctrl.paused.load(Ordering::Acquire) {
                mark_local_paused(&tasks, &task_id).await;
                return;
            }

            let acquire = semaphore.clone().acquire_owned();
            tokio::pin!(acquire);
            let permit = loop {
                tokio::select! {
                    result = &mut acquire => match result {
                        Ok(permit) => break permit,
                        Err(_) => return,
                    },
                    _ = ctrl.pause_notify.notified() => {
                        if operation_cancelled(&ctrl, persistent.as_ref()) {
                            mark_local_cancelled(&tasks, &task_id).await;
                            return;
                        }
                        if ctrl.paused.load(Ordering::Acquire) {
                            mark_local_paused(&tasks, &task_id).await;
                            return;
                        }
                    }
                    _ = sleep(Duration::from_millis(100)) => {
                        if operation_cancelled(&ctrl, persistent.as_ref()) {
                            mark_local_cancelled(&tasks, &task_id).await;
                            return;
                        }
                        if ctrl.paused.load(Ordering::Acquire) {
                            mark_local_paused(&tasks, &task_id).await;
                            return;
                        }
                    }
                }
            };
            if operation_cancelled(&ctrl, persistent.as_ref()) {
                drop(permit);
                mark_local_cancelled(&tasks, &task_id).await;
                return;
            }
            if ctrl.paused.load(Ordering::Acquire) {
                drop(permit);
                mark_local_paused(&tasks, &task_id).await;
                return;
            }

            if let Some(operation) = persistent.as_ref() {
                if operation
                    .queue
                    .update(
                        &task_id,
                        Some(TaskStatus::Running),
                        None,
                        Some("下载中".to_string()),
                    )
                    .is_err()
                {
                    drop(permit);
                    mark_local_cancelled(&tasks, &task_id).await;
                    return;
                }
            }
            {
                let mut tasks = tasks.lock().await;
                if let Some(task) = tasks.get_mut(&task_id) {
                    task.status = DownloadStatus::Downloading;
                    task.recovered = false;
                    task.retryable = false;
                    task.resumable = true;
                }
            }

            let (url, existing_bytes, known_total) = {
                let tasks = tasks.lock().await;
                match tasks.get(&task_id) {
                    Some(task) => (
                        task.url.clone(),
                        std::fs::metadata(&task.save_path)
                            .map(|metadata| metadata.len())
                            .unwrap_or(0),
                        task.total_size,
                    ),
                    None => {
                        drop(permit);
                        return;
                    }
                }
            };

            let result = if known_total > 0 && existing_bytes == known_total {
                // A pause can race with the final network chunk. Keep the
                // completed payload as a checkpoint and continue extraction on
                // resume without issuing an invalid `Range: bytes=len-` request.
                persist_download_checkpoint(
                    &task_id,
                    existing_bytes,
                    Some(known_total),
                    &tasks,
                    persistent.as_ref(),
                    "下载数据已完整写入，等待后续处理",
                )
                .await
                .map(|_| DownloadStreamOutcome::Completed)
            } else {
                let mut request = client.get(&url);
                if existing_bytes > 0 {
                    request = request.header(RANGE, format!("bytes={existing_bytes}-"));
                }

                Self::download_stream(
                    &task_id,
                    request,
                    existing_bytes,
                    &tasks,
                    &ctrl,
                    &token_bucket,
                    &download_dir,
                    minimum_free_bytes,
                    persistent.as_ref(),
                )
                .await
            };
            drop(permit);

            match result {
                Ok(DownloadStreamOutcome::Paused) => {
                    mark_local_paused(&tasks, &task_id).await;
                    return;
                }
                Ok(DownloadStreamOutcome::Completed) => {
                    if operation_cancelled(&ctrl, persistent.as_ref()) {
                        mark_local_cancelled(&tasks, &task_id).await;
                        return;
                    }
                    if ctrl.paused.load(Ordering::Acquire) {
                        mark_local_paused(&tasks, &task_id).await;
                        return;
                    }
                    let (auto_extract, save_path) = {
                        let tasks = tasks.lock().await;
                        match tasks.get(&task_id) {
                            Some(task) => (task.auto_extract, task.save_path.clone()),
                            None => return,
                        }
                    };
                    if auto_extract {
                        if let Some(operation) = persistent.as_ref() {
                            let _ = operation.queue.update(
                                &task_id,
                                None,
                                None,
                                Some("下载完成，正在解压".to_string()),
                            );
                        }
                        Self::auto_extract(
                            &task_id,
                            &save_path,
                            &tasks,
                            &ctrl,
                            persistent.as_ref(),
                        )
                        .await;
                    }
                    if operation_cancelled(&ctrl, persistent.as_ref()) {
                        mark_local_cancelled(&tasks, &task_id).await;
                        return;
                    }
                    if let Some(operation) = persistent.as_ref() {
                        if operation
                            .queue
                            .update_with_metadata(
                                &task_id,
                                Some(TaskStatus::Succeeded),
                                Some(1.0),
                                Some("下载完成".to_string()),
                                Some(json!({ "downloadedSize": file_size(&save_path) })),
                            )
                            .is_err()
                        {
                            mark_local_cancelled(&tasks, &task_id).await;
                            return;
                        }
                    }
                    let mut tasks = tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.status = DownloadStatus::Completed;
                        task.progress = 1.0;
                        task.speed = 0.0;
                        task.resumable = false;
                        task.retryable = false;
                    }
                    return;
                }
                Err(error) => {
                    if operation_cancelled(&ctrl, persistent.as_ref()) {
                        mark_local_cancelled(&tasks, &task_id).await;
                        return;
                    }
                    let retry_count = tasks
                        .lock()
                        .await
                        .get(&task_id)
                        .map(|task| task.retry_count)
                        .unwrap_or(0);
                    let preflight_failed = error.starts_with("下载预检失败");
                    if retry_count < max_retries && !preflight_failed {
                        let delay = 2u64.pow(retry_count + 1);
                        let next_retry = retry_count + 1;
                        {
                            let mut tasks = tasks.lock().await;
                            if let Some(task) = tasks.get_mut(&task_id) {
                                task.retry_count = next_retry;
                                task.error = Some(format!("重试中 ({next_retry}/{max_retries})"));
                            }
                        }
                        if let Some(operation) = persistent.as_ref() {
                            let _ = operation.queue.update_with_metadata(
                                &task_id,
                                None,
                                None,
                                Some(format!(
                                    "下载失败，{delay} 秒后重试 ({next_retry}/{max_retries})"
                                )),
                                Some(json!({ "retryCount": next_retry })),
                            );
                        }
                        for _ in 0..delay * 10 {
                            tokio::select! {
                                _ = ctrl.pause_notify.notified() => {}
                                _ = sleep(Duration::from_millis(100)) => {}
                            }
                            if operation_cancelled(&ctrl, persistent.as_ref()) {
                                mark_local_cancelled(&tasks, &task_id).await;
                                return;
                            }
                            if ctrl.paused.load(Ordering::Acquire) {
                                mark_local_paused(&tasks, &task_id).await;
                                return;
                            }
                        }
                        continue;
                    }

                    if let Some(operation) = persistent.as_ref() {
                        let _ = operation.queue.update_with_metadata(
                            &task_id,
                            Some(TaskStatus::Failed),
                            None,
                            Some(error.clone()),
                            Some(json!({
                                "retryable": true,
                                "resumable": true,
                                "retryCount": retry_count
                            })),
                        );
                    }
                    let mut tasks = tasks.lock().await;
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.status = DownloadStatus::Failed;
                        task.error = Some(error);
                        task.speed = 0.0;
                        task.retryable = true;
                        task.resumable = true;
                    }
                    return;
                }
            }
        }
    }

    /// 流式下载核心：写文件 + 暂停/取消 + 限速 + 磁盘/配额预检
    #[allow(clippy::too_many_arguments)]
    async fn download_stream(
        task_id: &str,
        request: reqwest::RequestBuilder,
        existing_bytes: u64,
        tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>,
        ctrl: &TaskControl,
        token_bucket: &TokenBucket,
        download_dir: &Path,
        minimum_free_bytes: u64,
        persistent: Option<&PersistentOperation>,
    ) -> Result<DownloadStreamOutcome, String> {
        if operation_cancelled(ctrl, persistent) {
            return Err("已取消".to_string());
        }
        if ctrl.paused.load(Ordering::Acquire) {
            return Ok(DownloadStreamOutcome::Paused);
        }

        let send = request.send();
        tokio::pin!(send);
        let response = loop {
            tokio::select! {
                result = &mut send => {
                    break result.map_err(|error| {
                        if error.is_timeout() {
                            "下载超时".to_string()
                        } else {
                            format!("请求失败: {error}")
                        }
                    })?;
                }
                _ = ctrl.pause_notify.notified() => {
                    if operation_cancelled(ctrl, persistent) {
                        return Err("已取消".to_string());
                    }
                    if ctrl.paused.load(Ordering::Acquire) {
                        return Ok(DownloadStreamOutcome::Paused);
                    }
                }
                _ = sleep(Duration::from_millis(100)) => {
                    if operation_cancelled(ctrl, persistent) {
                        return Err("已取消".to_string());
                    }
                    if ctrl.paused.load(Ordering::Acquire) {
                        return Ok(DownloadStreamOutcome::Paused);
                    }
                }
            }
        };
        if operation_cancelled(ctrl, persistent) {
            return Err("已取消".to_string());
        }
        if ctrl.paused.load(Ordering::Acquire) {
            return Ok(DownloadStreamOutcome::Paused);
        }
        let status = response.status();
        if status != StatusCode::OK && status != StatusCode::PARTIAL_CONTENT {
            return Err(format!("HTTP {}", status.as_u16()));
        }
        if status == StatusCode::PARTIAL_CONTENT {
            let range_start = response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(content_range_start)
                .ok_or_else(|| "服务器返回 206 但缺少有效 Content-Range".to_string())?;
            if range_start != existing_bytes {
                return Err(format!(
                    "服务器断点响应起点不匹配: 请求 {existing_bytes}，返回 {range_start}"
                ));
            }
        }

        let content_length = response.content_length();
        let total = content_length.map(|length| {
            if status == StatusCode::PARTIAL_CONTENT {
                existing_bytes.saturating_add(length)
            } else {
                length
            }
        });
        let starting_bytes = if status == StatusCode::PARTIAL_CONTENT {
            existing_bytes
        } else {
            0
        };
        let required_bytes = total.map(|total| total.saturating_sub(starting_bytes));
        let (save_path, quota_bytes) = {
            let tasks = tasks.lock().await;
            let task = tasks.get(task_id).ok_or("任务不存在")?;
            (task.save_path.clone(), task.quota_bytes)
        };
        let evidence = preflight_download(
            download_dir,
            &save_path,
            required_bytes,
            quota_bytes,
            minimum_free_bytes,
        );
        {
            let mut tasks = tasks.lock().await;
            if let Some(task) = tasks.get_mut(task_id) {
                task.total_size = total.unwrap_or(0);
                task.downloaded_size = starting_bytes;
                task.preflight = Some(evidence.clone());
            }
        }
        if let Some(operation) = persistent {
            operation.queue.patch_metadata(
                task_id,
                json!({
                    "preflight": evidence,
                    "totalSize": total.unwrap_or(0),
                    "downloadedSize": starting_bytes,
                    "quotaBytes": quota_bytes
                }),
            )?;
        }
        if !evidence.accepted {
            return Err(format!(
                "下载预检失败: {}",
                evidence.reason.as_deref().unwrap_or("磁盘空间或配额不足")
            ));
        }

        if status != StatusCode::PARTIAL_CONTENT {
            if let Some(parent) = save_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("无法创建下载目录: {error}"))?;
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(status == StatusCode::PARTIAL_CONTENT)
            .truncate(status != StatusCode::PARTIAL_CONTENT)
            .write(true)
            .open(&save_path)
            .map_err(|error| format!("无法创建文件: {error}"))?;

        let mut stream = response.bytes_stream();
        let mut downloaded = starting_bytes;
        let mut bytes_since_update = 0u64;
        let mut last_update = Instant::now();

        loop {
            if operation_cancelled(ctrl, persistent) {
                return Err("已取消".to_string());
            }
            if ctrl.paused.load(Ordering::Acquire) {
                file.flush().map_err(|error| format!("刷新失败: {error}"))?;
                persist_download_checkpoint(
                    task_id,
                    downloaded,
                    total,
                    tasks,
                    persistent,
                    "已暂停，连接已关闭，可使用 Range 继续",
                )
                .await?;
                return Ok(DownloadStreamOutcome::Paused);
            }

            let next_chunk = stream.next();
            tokio::pin!(next_chunk);
            let chunk = loop {
                tokio::select! {
                    item = &mut next_chunk => match item {
                        Some(Ok(data)) => break Some(data),
                        Some(Err(error)) => return Err(format!("下载流错误: {error}")),
                        None => break None,
                    },
                    _ = ctrl.pause_notify.notified() => {
                        if operation_cancelled(ctrl, persistent) {
                            return Err("已取消".to_string());
                        }
                        if ctrl.paused.load(Ordering::Acquire) {
                            break None;
                        }
                    }
                    _ = sleep(Duration::from_millis(100)) => {
                        if operation_cancelled(ctrl, persistent) {
                            return Err("已取消".to_string());
                        }
                        if ctrl.paused.load(Ordering::Acquire) {
                            break None;
                        }
                    }
                }
            };
            let Some(chunk) = chunk else {
                if ctrl.paused.load(Ordering::Acquire) {
                    file.flush().map_err(|error| format!("刷新失败: {error}"))?;
                    persist_download_checkpoint(
                        task_id,
                        downloaded,
                        total,
                        tasks,
                        persistent,
                        "已暂停，连接已关闭，可使用 Range 继续",
                    )
                    .await?;
                    return Ok(DownloadStreamOutcome::Paused);
                }
                break;
            };

            let chunk_len = chunk.len() as u64;
            let consume = token_bucket.consume(chunk_len);
            tokio::pin!(consume);
            loop {
                tokio::select! {
                    _ = &mut consume => break,
                    _ = ctrl.pause_notify.notified() => {
                        if operation_cancelled(ctrl, persistent) {
                            return Err("已取消".to_string());
                        }
                        if ctrl.paused.load(Ordering::Acquire) {
                            file.flush().map_err(|error| format!("刷新失败: {error}"))?;
                            persist_download_checkpoint(
                                task_id,
                                downloaded,
                                total,
                                tasks,
                                persistent,
                                "已暂停，连接已关闭，可使用 Range 继续",
                            )
                            .await?;
                            return Ok(DownloadStreamOutcome::Paused);
                        }
                    }
                    _ = sleep(Duration::from_millis(100)) => {
                        if operation_cancelled(ctrl, persistent) {
                            return Err("已取消".to_string());
                        }
                        if ctrl.paused.load(Ordering::Acquire) {
                            file.flush().map_err(|error| format!("刷新失败: {error}"))?;
                            persist_download_checkpoint(
                                task_id,
                                downloaded,
                                total,
                                tasks,
                                persistent,
                                "已暂停，连接已关闭，可使用 Range 继续",
                            )
                            .await?;
                            return Ok(DownloadStreamOutcome::Paused);
                        }
                    }
                }
            }

            let write_guard = ctrl.write_barrier.lock().await;
            if operation_cancelled(ctrl, persistent) {
                drop(write_guard);
                return Err("已取消".to_string());
            }
            if ctrl.paused.load(Ordering::Acquire) {
                drop(write_guard);
                file.flush().map_err(|error| format!("刷新失败: {error}"))?;
                persist_download_checkpoint(
                    task_id,
                    downloaded,
                    total,
                    tasks,
                    persistent,
                    "已暂停，连接已关闭，可使用 Range 继续",
                )
                .await?;
                return Ok(DownloadStreamOutcome::Paused);
            }
            file.write_all(&chunk)
                .map_err(|error| format!("写入失败: {error}"))?;
            drop(write_guard);
            downloaded = downloaded.saturating_add(chunk_len);
            bytes_since_update = bytes_since_update.saturating_add(chunk_len);

            if last_update.elapsed().as_millis() >= 200 {
                let elapsed = last_update.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    bytes_since_update as f64 / elapsed
                } else {
                    0.0
                };
                let progress = total
                    .filter(|total| *total > 0)
                    .map(|total| (downloaded as f64 / total as f64).clamp(0.0, 1.0))
                    .unwrap_or(0.0);
                {
                    let mut tasks = tasks.lock().await;
                    if let Some(task) = tasks.get_mut(task_id) {
                        task.downloaded_size = downloaded;
                        task.progress = progress as f32;
                        task.speed = speed;
                    }
                }
                if let Some(operation) = persistent {
                    operation.queue.update_with_metadata(
                        task_id,
                        None,
                        Some(progress),
                        Some(format!("下载中 {}%", (progress * 100.0).round() as u32)),
                        Some(json!({
                            "downloadedSize": downloaded,
                            "totalSize": total.unwrap_or(0)
                        })),
                    )?;
                }
                bytes_since_update = 0;
                last_update = Instant::now();
            }
        }
        file.flush().map_err(|error| format!("刷新失败: {error}"))?;
        if operation_cancelled(ctrl, persistent) {
            return Err("已取消".to_string());
        }

        let progress = total
            .filter(|total| *total > 0)
            .map(|total| (downloaded as f64 / total as f64).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        {
            let mut tasks = tasks.lock().await;
            if let Some(task) = tasks.get_mut(task_id) {
                task.downloaded_size = downloaded;
                task.progress = progress as f32;
                task.speed = 0.0;
            }
        }
        if let Some(operation) = persistent {
            operation.queue.update_with_metadata(
                task_id,
                None,
                Some(progress),
                Some("下载数据已写入磁盘".to_string()),
                Some(json!({
                    "downloadedSize": downloaded,
                    "totalSize": total.unwrap_or(downloaded)
                })),
            )?;
        }
        Ok(DownloadStreamOutcome::Completed)
    }

    async fn auto_extract(
        task_id: &str,
        save_path: &Path,
        tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>,
        ctrl: &TaskControl,
        persistent: Option<&PersistentOperation>,
    ) {
        let ext = save_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !matches!(ext.as_str(), "zip" | "rar" | "7z") {
            return;
        }
        {
            let mut t = tasks.lock().await;
            if let Some(task) = t.get_mut(task_id) {
                task.status = DownloadStatus::Extracting;
            }
        }
        let ed = save_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                "{}_extracted",
                save_path.file_stem().unwrap_or_default().to_string_lossy()
            ));
        ctrl.archive_active.store(true, Ordering::Release);
        let result = Self::do_extract(save_path, &ed, ctrl, persistent).await;
        ctrl.archive_active.store(false, Ordering::Release);
        match result {
            Ok(exes) => {
                let mut t = tasks.lock().await;
                if let Some(task) = t.get_mut(task_id) {
                    task.status = DownloadStatus::Importing;
                }
                log::info!("[萌游] 解压完成: {} 个可执行文件", exes.len());
            }
            Err(e) => {
                let mut t = tasks.lock().await;
                if let Some(task) = t.get_mut(task_id) {
                    task.error = Some(format!("解压失败: {}", e));
                }
                log::warn!("[萌游] 解压失败: {}", e);
            }
        }
    }

    async fn do_extract(
        archive: &Path,
        dir: &Path,
        ctrl: &TaskControl,
        persistent: Option<&PersistentOperation>,
    ) -> Result<Vec<PathBuf>, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {}", e))?;
        match archive.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "zip" => {
                let f = std::fs::File::open(archive).map_err(|e| format!("打开失败: {}", e))?;
                let mut za = zip::ZipArchive::new(f).map_err(|e| format!("ZIP错误: {}", e))?;
                let mut scope = crate::security::SecurityScope::new();
                scope.allow(dir);
                for i in 0..za.len() {
                    if operation_cancelled(ctrl, persistent) {
                        return Err(ARCHIVE_CANCELLATION_LIMITATION.to_string());
                    }
                    let mut e = za.by_index(i).map_err(|e| format!("条目错误: {}", e))?;
                    let safe_name = match e.enclosed_name() {
                        Some(p) => p.to_path_buf(),
                        None => {
                            tracing::warn!("跳过不安全的下载 zip 条目: {}", e.name());
                            continue;
                        }
                    };
                    let op = dir.join(&safe_name);
                    if e.is_dir() {
                        std::fs::create_dir_all(&op).ok();
                        let _ = scope.resolve(&op)?;
                    } else {
                        if let Some(p) = op.parent() {
                            std::fs::create_dir_all(p).ok();
                        }
                        let op = scope.resolve(&op)?;
                        let mut of =
                            std::fs::File::create(&op).map_err(|e| format!("创建失败: {}", e))?;
                        std::io::copy(&mut e, &mut of).map_err(|e| format!("写入失败: {}", e))?;
                    }
                }
            }
            "rar" | "7z" => return Self::extract_7z(archive, dir, ctrl, persistent),
            _ => return Err("不支持的格式".into()),
        }
        Ok(find_executables(dir))
    }

    fn extract_7z(
        archive: &Path,
        dir: &Path,
        ctrl: &TaskControl,
        persistent: Option<&PersistentOperation>,
    ) -> Result<Vec<PathBuf>, String> {
        if operation_cancelled(ctrl, persistent) {
            return Err(ARCHIVE_CANCELLATION_LIMITATION.to_string());
        }
        let s7z = find_7z()?;
        let out = std::process::Command::new(&s7z)
            .args([
                "x",
                archive.to_str().unwrap_or(""),
                &format!("-o{}", dir.to_string_lossy()),
                "-y",
            ])
            .output()
            .map_err(|e| format!("7z错误: {}", e))?;
        if !out.status.success() {
            return Err(format!("7z失败: {}", String::from_utf8_lossy(&out.stderr)));
        }
        if operation_cancelled(ctrl, persistent) {
            return Err(ARCHIVE_CANCELLATION_LIMITATION.to_string());
        }
        Ok(find_executables(dir))
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

fn content_range_start(value: &str) -> Option<u64> {
    let value = value.trim();
    let range = value.strip_prefix("bytes ")?.split('/').next()?;
    range.split('-').next()?.parse().ok()
}

fn env_u64(name: &str) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn configured_quota(environment_quota: u64, override_quota: Option<u64>) -> Option<u64> {
    match override_quota {
        Some(0) => None,
        Some(value) => Some(value),
        None if environment_quota > 0 => Some(environment_quota),
        None => None,
    }
}

async fn wait_for_execution_stop(control: &TaskControl) {
    while control.started.load(Ordering::Acquire) {
        tokio::select! {
            _ = control.execution_stopped.notified() => {}
            _ = sleep(Duration::from_millis(25)) => {}
        }
    }
}

fn download_status(status: TaskStatus) -> DownloadStatus {
    match status {
        TaskStatus::Queued => DownloadStatus::Pending,
        TaskStatus::Running => DownloadStatus::Paused,
        TaskStatus::Paused => DownloadStatus::Paused,
        TaskStatus::Succeeded => DownloadStatus::Completed,
        TaskStatus::Failed => DownloadStatus::Failed,
        TaskStatus::Cancelled => DownloadStatus::Cancelled,
    }
}

fn operation_cancelled(control: &TaskControl, persistent: Option<&PersistentOperation>) -> bool {
    control.cancelled.load(Ordering::Acquire)
        || persistent
            .map(|operation| operation.cancellation.is_cancelled())
            .unwrap_or(false)
}

async fn persist_download_checkpoint(
    task_id: &str,
    downloaded: u64,
    total: Option<u64>,
    tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>,
    persistent: Option<&PersistentOperation>,
    message: &str,
) -> Result<(), String> {
    let progress = total
        .filter(|total| *total > 0)
        .map(|total| (downloaded as f64 / total as f64).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    {
        let mut tasks = tasks.lock().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.downloaded_size = downloaded;
            task.progress = progress as f32;
            task.speed = 0.0;
            task.resumable = true;
        }
    }
    if let Some(operation) = persistent {
        operation.queue.update_with_metadata(
            task_id,
            None,
            Some(progress),
            Some(message.to_string()),
            Some(json!({
                "downloadedSize": downloaded,
                "totalSize": total.unwrap_or(0),
                "resumable": true,
                "httpConnectionOpen": false
            })),
        )?;
    }
    Ok(())
}

async fn mark_local_paused(tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>, task_id: &str) {
    let mut tasks = tasks.lock().await;
    if let Some(task) = tasks.get_mut(task_id) {
        if task.status != DownloadStatus::Cancelled {
            task.status = DownloadStatus::Paused;
            task.speed = 0.0;
            task.resumable = true;
            task.retryable = false;
        }
    }
}

async fn mark_local_cancelled(tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>, task_id: &str) {
    let mut tasks = tasks.lock().await;
    if let Some(task) = tasks.get_mut(task_id) {
        task.status = DownloadStatus::Cancelled;
        task.speed = 0.0;
        task.resumable = false;
        task.retryable = false;
    }
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

fn directory_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut pending = vec![path.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    total
}

fn available_space(path: &Path) -> Option<u64> {
    let mut candidate = path;
    while !candidate.exists() {
        candidate = candidate.parent()?;
    }
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .filter(|disk| candidate.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .or_else(|| disks.iter().max_by_key(|disk| disk.available_space()))
        .map(|disk| disk.available_space())
}

fn preflight_download(
    download_dir: &Path,
    save_path: &Path,
    required_bytes: Option<u64>,
    quota_bytes: Option<u64>,
    minimum_free_bytes: u64,
) -> DownloadPreflightEvidence {
    let available_bytes = available_space(save_path);
    let quota_used_bytes = directory_size(download_dir);
    let quota_remaining_bytes = quota_bytes.map(|quota| quota.saturating_sub(quota_used_bytes));
    let disk_ok = available_bytes
        .map(|available| {
            required_bytes
                .map(|required| available >= required.saturating_add(minimum_free_bytes))
                .unwrap_or(available >= minimum_free_bytes)
        })
        .unwrap_or(false);
    let quota_ok = match (quota_bytes, required_bytes) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(_), Some(required)) => quota_remaining_bytes
            .map(|remaining| remaining >= required)
            .unwrap_or(false),
    };
    let accepted = disk_ok && quota_ok;
    let reason = if available_bytes.is_none() {
        Some("无法识别目标磁盘的可用空间".to_string())
    } else if !disk_ok {
        Some(format!(
            "磁盘空间不足，需要下载空间并保留至少 {} 字节",
            minimum_free_bytes
        ))
    } else if quota_bytes.is_some() && required_bytes.is_none() {
        Some("服务器未提供文件大小，启用配额时无法安全预检".to_string())
    } else if !quota_ok {
        Some("下载目录配额不足".to_string())
    } else {
        None
    };

    DownloadPreflightEvidence {
        checked_at: chrono::Utc::now().to_rfc3339(),
        available_bytes,
        required_bytes,
        minimum_free_bytes,
        quota_bytes,
        quota_used_bytes,
        quota_remaining_bytes,
        quota_source: if quota_bytes.is_some() {
            "configured".to_string()
        } else {
            "unlimited".to_string()
        },
        accepted,
        reason,
    }
}

fn sanitize_filename(name: &str) -> String {
    let invalid = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let s: String = name
        .chars()
        .map(|c| if invalid.contains(&c) { '_' } else { c })
        .collect();
    let s = s.trim().trim_end_matches('.').to_string();
    if s.is_empty() {
        "Unknown".into()
    } else {
        s
    }
}

fn get_unique_file_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    let mut i = 1u32;
    loop {
        let c = if ext.is_empty() {
            dir.join(format!("{} ({})", stem, i))
        } else {
            dir.join(format!("{} ({}).{}", stem, i, ext))
        };
        if !c.exists() {
            return c;
        }
        i += 1;
    }
}

fn find_executables(dir: &Path) -> Vec<PathBuf> {
    let mut exes = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                exes.extend(find_executables(&p));
            } else if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "exe" | "bat" | "cmd" | "lnk") {
                    exes.push(p);
                }
            }
        }
    }
    exes.sort_by(|a, b| {
        b.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase() == "exe")
            .unwrap_or(false)
            .cmp(
                &a.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase() == "exe")
                    .unwrap_or(false),
            )
            .then_with(|| a.to_string_lossy().len().cmp(&b.to_string_lossy().len()))
    });
    exes
}

fn find_7z() -> Result<PathBuf, String> {
    for p in &[
        r"C:\Program Files\7-Zip\7z.exe",
        r"C:\Program Files (x86)\7-Zip\7z.exe",
    ] {
        if Path::new(p).exists() {
            return Ok(PathBuf::from(p));
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&path) {
            if d.join("7z.exe").exists() {
                return Ok(d.join("7z.exe"));
            }
            if d.join("7z").exists() {
                return Ok(d.join("7z"));
            }
        }
    }
    Err("未找到 7z.exe".into())
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test"), "test");
        assert_eq!(sanitize_filename("test:game"), "test_game");
        assert_eq!(sanitize_filename("  "), "Unknown");
    }

    #[test]
    fn test_unique_file_path() {
        let dir = std::env::temp_dir();
        let p = dir.join("test_dl_unique.txt");
        let _ = std::fs::remove_file(&p);
        assert_eq!(get_unique_file_path(&p), p);
        std::fs::write(&p, "x").expect("test fixture write");
        assert_ne!(get_unique_file_path(&p), p);
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn quota_preflight_records_rejection_evidence() {
        let root = std::env::temp_dir().join(format!("moeplay-preflight-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let evidence = preflight_download(&root, &root.join("file.bin"), Some(20), Some(10), 0);
        assert!(!evidence.accepted);
        assert_eq!(evidence.required_bytes, Some(20));
        assert_eq!(evidence.quota_bytes, Some(10));
        assert!(evidence.reason.unwrap().contains("配额"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn configured_quota_rejects_unknown_content_length() {
        let root = std::env::temp_dir().join(format!("moeplay-preflight-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let evidence = preflight_download(&root, &root.join("file.bin"), None, Some(1024), 0);
        assert!(!evidence.accepted);
        assert!(evidence.reason.unwrap().contains("文件大小"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn test_find_exes() {
        let d = std::env::temp_dir().join("test_find_exes");
        std::fs::create_dir_all(&d).ok();
        std::fs::write(d.join("game.exe"), "x").ok();
        std::fs::write(d.join("readme.txt"), "x").ok();
        let exes = find_executables(&d);
        assert_eq!(exes.len(), 1);
        std::fs::remove_dir_all(&d).ok();
    }

    struct LocalHttpServer {
        url: String,
        requests: Arc<Mutex<Vec<Option<u64>>>>,
        disconnects: Arc<std::sync::atomic::AtomicUsize>,
        task: tokio::task::JoinHandle<()>,
    }

    impl Drop for LocalHttpServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn start_local_http_server(data: Vec<u8>, chunk_delay: Duration) -> LocalHttpServer {
        use std::sync::atomic::AtomicUsize;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let data = Arc::new(data);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let disconnects = Arc::new(AtomicUsize::new(0));
        let server_requests = requests.clone();
        let server_disconnects = disconnects.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let data = data.clone();
                let requests = server_requests.clone();
                let disconnects = server_disconnects.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0u8; 2048];
                    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                        let Ok(read) = socket.read(&mut buffer).await else {
                            return;
                        };
                        if read == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..read]);
                        if request.len() > 32 * 1024 {
                            return;
                        }
                    }
                    let request_text = String::from_utf8_lossy(&request);
                    let range_start = request_text.lines().find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if !name.eq_ignore_ascii_case("range") {
                            return None;
                        }
                        let value = value.trim().strip_prefix("bytes=")?;
                        value.split('-').next()?.trim().parse::<u64>().ok()
                    });
                    requests.lock().await.push(range_start);
                    let start = range_start.unwrap_or(0) as usize;
                    if start >= data.len() {
                        let _ = socket
                            .write_all(
                                b"HTTP/1.1 416 Range Not Satisfiable\r\nConnection: close\r\n\r\n",
                            )
                            .await;
                        return;
                    }
                    let body = &data[start..];
                    let header = if range_start.is_some() {
                        format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                            body.len(),
                            start,
                            data.len() - 1,
                            data.len()
                        )
                    } else {
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                    };
                    if socket.write_all(header.as_bytes()).await.is_err() {
                        return;
                    }
                    for chunk in body.chunks(4096) {
                        if socket.write_all(chunk).await.is_err() {
                            disconnects.fetch_add(1, Ordering::SeqCst);
                            return;
                        }
                        sleep(chunk_delay).await;
                    }
                    let _ = socket.shutdown().await;
                });
            }
        });
        LocalHttpServer {
            url: format!("http://{address}/fixture.bin"),
            requests,
            disconnects,
            task,
        }
    }

    async fn wait_for_file_len(path: &Path, minimum: u64) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if std::fs::metadata(path)
                    .map(|metadata| metadata.len() >= minimum)
                    .unwrap_or(false)
                {
                    return;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("download did not write the expected checkpoint");
    }

    async fn wait_for_status(
        downloader: &Downloader,
        task_id: &str,
        expected: DownloadStatus,
    ) -> DownloadTask {
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if let Some(task) = downloader
                    .get_all()
                    .await
                    .into_iter()
                    .find(|task| task.id == task_id)
                {
                    if task.status == expected {
                        return task;
                    }
                    if task.status == DownloadStatus::Failed {
                        panic!("download failed unexpectedly: {:?}", task.error);
                    }
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("download did not reach the expected status")
    }

    #[tokio::test]
    async fn real_http_pause_closes_connection_and_resume_uses_range() {
        let root =
            std::env::temp_dir().join(format!("moeplay-http-pause-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let payload = (0..512 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let server = start_local_http_server(payload.clone(), Duration::from_millis(8)).await;
        let downloader = Downloader::new(root.clone(), 1);
        let task = downloader
            .enqueue(server.url.clone(), "fixture.bin".into(), false, false)
            .await;

        wait_for_file_len(&task.save_path, 32 * 1024).await;
        let pause_started = Instant::now();
        downloader.pause(&task.id).await.unwrap();
        assert!(
            pause_started.elapsed() < Duration::from_secs(2),
            "pause acknowledgement exceeded the two-second activity-stop budget"
        );
        let paused_len = std::fs::metadata(&task.save_path).unwrap().len();
        assert!(paused_len > 0 && paused_len < payload.len() as u64);
        sleep(Duration::from_millis(250)).await;
        assert_eq!(
            std::fs::metadata(&task.save_path).unwrap().len(),
            paused_len,
            "pause returned while file writes were still active"
        );
        assert_eq!(
            wait_for_status(&downloader, &task.id, DownloadStatus::Paused)
                .await
                .downloaded_size,
            paused_len
        );

        downloader.resume(&task.id).await.unwrap();
        let completed = wait_for_status(&downloader, &task.id, DownloadStatus::Completed).await;
        assert_eq!(completed.downloaded_size, payload.len() as u64);
        assert_eq!(std::fs::read(&task.save_path).unwrap(), payload);
        let requests = server.requests.lock().await.clone();
        assert_eq!(requests.first(), Some(&None));
        assert!(
            requests
                .iter()
                .skip(1)
                .any(|start| *start == Some(paused_len)),
            "resume did not issue Range from the durable file checkpoint: {requests:?}"
        );
        assert!(server.disconnects.load(Ordering::SeqCst) >= 1);
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn real_http_cancel_stops_file_writes_and_drops_connection() {
        let root =
            std::env::temp_dir().join(format!("moeplay-http-cancel-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let payload = vec![0x5a; 1024 * 1024];
        let server = start_local_http_server(payload, Duration::from_millis(8)).await;
        let downloader = Downloader::new(root.clone(), 1);
        let task = downloader
            .enqueue(server.url.clone(), "cancel.bin".into(), false, false)
            .await;

        wait_for_file_len(&task.save_path, 32 * 1024).await;
        let cancel_started = Instant::now();
        downloader.cancel(&task.id).await.unwrap();
        assert!(
            cancel_started.elapsed() < Duration::from_secs(2),
            "cancel acknowledgement exceeded the two-second activity-stop budget"
        );
        let cancelled_len = std::fs::metadata(&task.save_path).unwrap().len();
        sleep(Duration::from_millis(350)).await;
        assert_eq!(
            std::fs::metadata(&task.save_path).unwrap().len(),
            cancelled_len,
            "cancel returned while file writes were still active"
        );
        let cancelled = wait_for_status(&downloader, &task.id, DownloadStatus::Cancelled).await;
        assert!(!cancelled.resumable);
        tokio::time::timeout(Duration::from_secs(2), async {
            while server.disconnects.load(Ordering::SeqCst) == 0 {
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("server did not observe the cancelled HTTP connection closing");
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn archive_phase_reports_deferred_cancellation_and_rejects_pause() {
        let root =
            std::env::temp_dir().join(format!("moeplay-archive-limit-{}", uuid::Uuid::new_v4()));
        let downloader = Downloader::new(root.clone(), 1);
        let task_id = uuid::Uuid::new_v4().to_string();
        let control = Arc::new(TaskControl::new());
        control.archive_active.store(true, Ordering::Release);
        downloader
            .controls
            .lock()
            .await
            .insert(task_id.clone(), control);
        downloader.tasks.lock().await.insert(
            task_id.clone(),
            DownloadTask {
                id: task_id.clone(),
                url: "http://127.0.0.1/unused".into(),
                filename: "fixture.zip".into(),
                save_path: root.join("fixture.zip"),
                total_size: 1,
                downloaded_size: 1,
                progress: 1.0,
                speed: 0.0,
                status: DownloadStatus::Extracting,
                retry_count: 0,
                max_retries: 3,
                error: None,
                auto_extract: true,
                auto_import: false,
                headers: HashMap::new(),
                recovered: false,
                resumable: false,
                retryable: false,
                quota_bytes: None,
                preflight: None,
            },
        );

        let pause_error = downloader.pause(&task_id).await.unwrap_err();
        assert_eq!(pause_error, ARCHIVE_CANCELLATION_LIMITATION);
        assert!(downloader
            .archive_cancellation_is_deferred(&task_id)
            .await
            .unwrap());
        downloader.cancel(&task_id).await.unwrap();
        let cancelled = wait_for_status(&downloader, &task_id, DownloadStatus::Cancelled).await;
        assert_eq!(
            cancelled.error.as_deref(),
            Some(ARCHIVE_CANCELLATION_LIMITATION)
        );
        std::fs::remove_dir_all(root).ok();
    }
}
