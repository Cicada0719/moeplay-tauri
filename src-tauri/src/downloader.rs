//! 萌游下载管理器 - 流式下载、暂停续传、断点续传、限速、队列管理

use reqwest::header::RANGE;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Notify, Semaphore};
use tokio::time::{sleep, Duration};

use futures_util::StreamExt;

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
    paused: AtomicBool,
    cancelled: AtomicBool,
}

impl TaskControl {
    fn new() -> Self {
        Self {
            pause_notify: Notify::new(),
            paused: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct Downloader {
    tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
    controls: Arc<Mutex<HashMap<String, Arc<TaskControl>>>>,
    semaphore: Arc<Semaphore>,
    client: Client,
    download_dir: PathBuf,
    speed_limit: Arc<AtomicU64>,
    token_bucket: Arc<TokenBucket>,
}

impl Downloader {
    pub fn new(download_dir: PathBuf, max_concurrent: u32) -> Self {
        let max = if max_concurrent > 0 {
            max_concurrent as usize
        } else {
            3
        };
        let client = Client::builder()
            .timeout(Duration::from_secs(30 * 60))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 MoeGame/1.0")
            .danger_accept_invalid_certs(true)
            .build().expect("HTTP client");

        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            controls: Arc::new(Mutex::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max)),
            client,
            download_dir,
            speed_limit: Arc::new(AtomicU64::new(0)),
            token_bucket: Arc::new(TokenBucket::new(0)),
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
        };

        let control = Arc::new(TaskControl::new());
        self.tasks.lock().await.insert(id.clone(), task.clone());
        self.controls.lock().await.insert(id.clone(), control);
        self.spawn_download(&id);
        task
    }

    pub async fn pause(&self, task_id: &str) -> Result<(), String> {
        let c = self.controls.lock().await;
        let ctrl = c.get(task_id).ok_or("任务不存在")?;
        ctrl.paused.store(true, Ordering::Relaxed);
        let mut t = self.tasks.lock().await;
        t.get_mut(task_id).ok_or("任务不存在")?.status = DownloadStatus::Paused;
        Ok(())
    }

    pub async fn resume(&self, task_id: &str) -> Result<(), String> {
        let c = self.controls.lock().await;
        let ctrl = c.get(task_id).ok_or("任务不存在")?;
        ctrl.paused.store(false, Ordering::Relaxed);
        ctrl.pause_notify.notify_one();
        let mut t = self.tasks.lock().await;
        t.get_mut(task_id).ok_or("任务不存在")?.status = DownloadStatus::Downloading;
        Ok(())
    }

    pub async fn cancel(&self, task_id: &str) -> Result<(), String> {
        let c = self.controls.lock().await;
        if let Some(ctrl) = c.get(task_id) {
            ctrl.cancelled.store(true, Ordering::Relaxed);
            ctrl.pause_notify.notify_one();
        }
        let mut t = self.tasks.lock().await;
        t.get_mut(task_id).ok_or("任务不存在")?.status = DownloadStatus::Cancelled;
        Ok(())
    }

    pub async fn cancel_all(&self) -> Result<(), String> {
        let c = self.controls.lock().await;
        for ctrl in c.values() {
            ctrl.cancelled.store(true, Ordering::Relaxed);
            ctrl.pause_notify.notify_one();
        }
        drop(c);
        let mut t = self.tasks.lock().await;
        for task in t.values_mut() {
            if matches!(
                task.status,
                DownloadStatus::Downloading | DownloadStatus::Pending
            ) {
                task.status = DownloadStatus::Cancelled;
            }
        }
        Ok(())
    }

    pub async fn retry(&self, task_id: &str) -> Result<(), String> {
        let can = {
            let t = self.tasks.lock().await;
            let task = t.get(task_id).ok_or("任务不存在")?;
            task.status == DownloadStatus::Failed && task.retry_count < task.max_retries
        };
        if !can {
            return Err("无法重试".into());
        }
        {
            let mut t = self.tasks.lock().await;
            let task = t.get_mut(task_id).unwrap();
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
        self.spawn_download(task_id);
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

    fn spawn_download(&self, task_id: &str) {
        let tid = task_id.to_string();
        let tasks = self.tasks.clone();
        let controls = self.controls.clone();
        let sem = self.semaphore.clone();
        let client = self.client.clone();
        let tb = self.token_bucket.clone();
        tokio::spawn(async move {
            Self::execute_download(tid, tasks, controls, sem, client, tb).await;
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
    ) {
        let max_retries = {
            tasks
                .lock()
                .await
                .get(&task_id)
                .map(|t| t.max_retries)
                .unwrap_or(3)
        };

        loop {
            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => return,
            };
            {
                let mut t = tasks.lock().await;
                if let Some(task) = t.get_mut(&task_id) {
                    task.status = DownloadStatus::Downloading;
                }
            }

            let ctrl = match controls.lock().await.get(&task_id).cloned() {
                Some(c) => c,
                None => {
                    drop(permit);
                    return;
                }
            };

            let (url, _save_path, existing_bytes) = {
                let t = tasks.lock().await;
                match t.get(&task_id) {
                    Some(task) => {
                        let eb = std::fs::metadata(&task.save_path)
                            .map(|m| m.len())
                            .unwrap_or(0);
                        (task.url.clone(), task.save_path.clone(), eb)
                    }
                    None => {
                        drop(permit);
                        return;
                    }
                }
            };

            let mut req = client.get(&url);
            if existing_bytes > 0 {
                req = req.header(RANGE, format!("bytes={}-", existing_bytes));
            }

            let result =
                Self::download_stream(&task_id, req, existing_bytes, &tasks, &ctrl, &token_bucket)
                    .await;
            drop(permit);

            match result {
                Ok(()) => {
                    let (ae, sp) = {
                        let mut t = tasks.lock().await;
                        if let Some(task) = t.get_mut(&task_id) {
                            task.status = DownloadStatus::Completed;
                            task.progress = 1.0;
                            task.speed = 0.0;
                            (task.auto_extract, task.save_path.clone())
                        } else {
                            return;
                        }
                    };
                    if ae {
                        Self::auto_extract(&task_id, &sp, &tasks).await;
                    }
                    return;
                }
                Err(e) => {
                    if ctrl.cancelled.load(Ordering::Relaxed) {
                        let mut t = tasks.lock().await;
                        if let Some(task) = t.get_mut(&task_id) {
                            task.status = DownloadStatus::Cancelled;
                        }
                        return;
                    }
                    let rc = {
                        tasks
                            .lock()
                            .await
                            .get(&task_id)
                            .map(|t| t.retry_count)
                            .unwrap_or(0)
                    };
                    if rc < max_retries {
                        let delay = 2u64.pow(rc + 1);
                        {
                            let mut t = tasks.lock().await;
                            if let Some(task) = t.get_mut(&task_id) {
                                task.retry_count += 1;
                                task.error =
                                    Some(format!("重试中 ({}/{})", task.retry_count, max_retries));
                            }
                        }
                        sleep(Duration::from_secs(delay)).await;
                        if ctrl.cancelled.load(Ordering::Relaxed) {
                            let mut t = tasks.lock().await;
                            if let Some(task) = t.get_mut(&task_id) {
                                task.status = DownloadStatus::Cancelled;
                            }
                            return;
                        }
                        continue;
                    } else {
                        let mut t = tasks.lock().await;
                        if let Some(task) = t.get_mut(&task_id) {
                            task.status = DownloadStatus::Failed;
                            task.error = Some(e);
                        }
                        return;
                    }
                }
            }
        }
    }

    /// 流式下载核心：写文件 + 暂停/取消 + 限速
    async fn download_stream(
        task_id: &str,
        request: reqwest::RequestBuilder,
        existing_bytes: u64,
        tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>,
        ctrl: &TaskControl,
        tb: &TokenBucket,
    ) -> Result<(), String> {
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                "下载超时".to_string()
            } else {
                format!("请求失败: {}", e)
            }
        })?;
        let status = response.status();
        if status != StatusCode::OK && status != StatusCode::PARTIAL_CONTENT {
            return Err(format!("HTTP {}", status.as_u16()));
        }

        let cl = response.content_length().unwrap_or(0);
        let total = if status == StatusCode::PARTIAL_CONTENT {
            existing_bytes + cl
        } else {
            cl
        };

        let save_path = {
            let mut t = tasks.lock().await;
            if let Some(task) = t.get_mut(task_id) {
                task.total_size = total;
                task.downloaded_size = if status == StatusCode::PARTIAL_CONTENT {
                    existing_bytes
                } else {
                    0
                };
                task.save_path.clone()
            } else {
                return Err("任务不存在".into());
            }
        };

        if status != StatusCode::PARTIAL_CONTENT {
            if let Some(p) = Path::new(&save_path).parent() {
                std::fs::create_dir_all(p).ok();
            }
        }

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(status == StatusCode::PARTIAL_CONTENT)
            .write(true)
            .open(&save_path)
            .map_err(|e| format!("无法创建文件: {}", e))?;

        let mut stream = response.bytes_stream();
        let mut downloaded = if status == StatusCode::PARTIAL_CONTENT {
            existing_bytes
        } else {
            0
        };
        let mut last_update = Instant::now();

        loop {
            while ctrl.paused.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = ctrl.pause_notify.notified() => {}
                    _ = sleep(Duration::from_millis(100)) => { if ctrl.cancelled.load(Ordering::Relaxed) { return Err("已取消".into()); } }
                }
            }
            if ctrl.cancelled.load(Ordering::Relaxed) {
                return Err("已取消".into());
            }

            let chunk = match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
                Ok(Some(Ok(d))) => d,
                Ok(Some(Err(e))) => return Err(format!("下载流错误: {}", e)),
                Ok(None) => break,
                Err(_) => continue,
            };
            let clen = chunk.len() as u64;
            tb.consume(clen).await;
            file.write_all(&chunk)
                .map_err(|e| format!("写入失败: {}", e))?;
            downloaded += clen;

            if last_update.elapsed().as_millis() >= 200 {
                let elapsed = last_update.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    clen as f64 / elapsed
                } else {
                    0.0
                };
                let mut t = tasks.lock().await;
                if let Some(task) = t.get_mut(task_id) {
                    task.downloaded_size = downloaded;
                    task.progress = if total > 0 {
                        downloaded as f32 / total as f32
                    } else {
                        0.0
                    };
                    task.speed = speed;
                }
                last_update = Instant::now();
            }
        }
        file.flush().map_err(|e| format!("刷新失败: {}", e))?;

        let mut t = tasks.lock().await;
        if let Some(task) = t.get_mut(task_id) {
            task.downloaded_size = downloaded;
            task.progress = if total > 0 {
                downloaded as f32 / total as f32
            } else {
                0.0
            };
        }
        Ok(())
    }

    async fn auto_extract(
        task_id: &str,
        save_path: &Path,
        tasks: &Arc<Mutex<HashMap<String, DownloadTask>>>,
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
        match Self::do_extract(save_path, &ed).await {
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

    async fn do_extract(archive: &Path, dir: &Path) -> Result<Vec<PathBuf>, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {}", e))?;
        match archive.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "zip" => {
                let f = std::fs::File::open(archive).map_err(|e| format!("打开失败: {}", e))?;
                let mut za = zip::ZipArchive::new(f).map_err(|e| format!("ZIP错误: {}", e))?;
                for i in 0..za.len() {
                    let mut e = za.by_index(i).map_err(|e| format!("条目错误: {}", e))?;
                    let op = dir.join(e.name());
                    if e.is_dir() {
                        std::fs::create_dir_all(&op).ok();
                    } else {
                        if let Some(p) = op.parent() {
                            std::fs::create_dir_all(p).ok();
                        }
                        let mut of =
                            std::fs::File::create(&op).map_err(|e| format!("创建失败: {}", e))?;
                        std::io::copy(&mut e, &mut of).map_err(|e| format!("写入失败: {}", e))?;
                    }
                }
            }
            "rar" | "7z" => return Self::extract_7z(archive, dir),
            _ => return Err("不支持的格式".into()),
        }
        Ok(find_executables(dir))
    }

    fn extract_7z(archive: &Path, dir: &Path) -> Result<Vec<PathBuf>, String> {
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
        Ok(find_executables(dir))
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

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
        std::fs::write(&p, "x").unwrap();
        assert_ne!(get_unique_file_path(&p), p);
        std::fs::remove_file(&p).ok();
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
}
