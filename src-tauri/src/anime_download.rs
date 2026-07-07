//! 番剧下载管理器 — 支持 m3u8/HLS 分片下载 + 直链下载
//!
//! Kazumi 风格：解析 m3u8 → 下载所有分片 → 拼接为完整 mp4。
//! 复用现有 Downloader 的任务追踪 + 限速 + 并发控制。

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Notify};

// ============================================================================
// 数据模型
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimeDownloadTask {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub output_path: PathBuf,
    pub status: AnimeDownloadStatus,
    pub progress: f32,
    pub total_segments: u32,
    pub downloaded_segments: u32,
    pub total_size: u64,
    pub downloaded_size: u64,
    pub speed: f64,
    pub error: Option<String>,
    pub is_m3u8: bool,
    pub anime_name: Option<String>,
    pub episode_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum AnimeDownloadStatus {
    Pending,
    Parsing,
    Downloading,
    Merging,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

// ============================================================================
// m3u8 解析器
// ============================================================================

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct M3u8Segment {
    uri: String,
    duration: f64,
    key: Option<M3u8Key>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct M3u8Key {
    method: String,
    uri: String,
    #[allow(dead_code)]
    iv: Option<String>,
}

#[derive(Debug, Clone)]
struct M3u8Variant {
    bandwidth: u64,
    uri: String,
}

/// 检测 m3u8 内容类型
fn detect_m3u8_type(content: &str) -> M3u8Type {
    if content.contains("#EXT-X-STREAM-INF") {
        M3u8Type::Master
    } else {
        M3u8Type::Media
    }
}

enum M3u8Type {
    Master,
    Media,
}

/// 解析相对 URL 为绝对 URL
fn resolve_url(base_url: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    if let Ok(base) = url::Url::parse(base_url) {
        if relative.starts_with('/') {
            return format!(
                "{}://{}{}{}",
                base.scheme(),
                base.host_str().unwrap_or(""),
                if let Some(port) = base.port() {
                    format!(":{}", port)
                } else {
                    String::new()
                },
                relative
            );
        }
        if let Ok(resolved) = base.join(relative) {
            return resolved.to_string();
        }
    }
    // Fallback: simple concatenation
    if let Some(pos) = base_url.rfind('/') {
        format!("{}{}", &base_url[..pos + 1], relative)
    } else {
        relative.to_string()
    }
}

/// 解析 master playlist，返回最高带宽变体
fn parse_master_playlist(content: &str, base_url: &str) -> Option<M3u8Variant> {
    let lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();
    let mut variants = Vec::new();

    for i in 0..lines.len() {
        let line = lines[i];
        if let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let bandwidth = attrs
                .split(',')
                .find(|a| a.trim().starts_with("BANDWIDTH="))
                .and_then(|a| a.trim().strip_prefix("BANDWIDTH="))
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);

            if i + 1 < lines.len() && !lines[i + 1].starts_with('#') {
                let uri = resolve_url(base_url, lines[i + 1]);
                variants.push(M3u8Variant { bandwidth, uri });
            }
        }
    }

    variants.sort_by_key(|v| std::cmp::Reverse(v.bandwidth));
    variants.into_iter().next()
}

/// 解析 media playlist，返回分片列表
fn parse_media_playlist(content: &str, base_url: &str) -> Vec<M3u8Segment> {
    let lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();
    let mut segments = Vec::new();
    let mut current_duration = 0.0f64;
    let mut current_key: Option<M3u8Key> = None;

    for line in &lines {
        if line.starts_with("#EXT-X-TARGETDURATION:") {
            // parsed but not stored (not needed for download)
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-KEY:") {
            let method = attrs
                .split(',')
                .find(|a| a.trim().starts_with("METHOD="))
                .and_then(|a| a.trim().strip_prefix("METHOD="))
                .unwrap_or("NONE")
                .to_string();

            if method != "NONE" {
                let uri = attrs
                    .split(',')
                    .find(|a| a.trim().starts_with("URI=\""))
                    .and_then(|a| {
                        let s = a.trim();
                        let start = s.find("URI=\"")? + 5;
                        let end = s[start..].find('"')?;
                        Some(resolve_url(base_url, &s[start..start + end]))
                    })
                    .unwrap_or_default();

                let iv = attrs
                    .split(',')
                    .find(|a| a.trim().starts_with("IV="))
                    .and_then(|a| a.trim().strip_prefix("IV="))
                    .map(|s| s.to_string());

                current_key = Some(M3u8Key { method, uri, iv });
            } else {
                current_key = None;
            }
        } else if let Some(duration_str) = line.strip_prefix("#EXTINF:") {
            let duration_part = duration_str.split(',').next().unwrap_or("0");
            current_duration = duration_part.parse::<f64>().unwrap_or(0.0);
        } else if !line.is_empty() && !line.starts_with('#') {
            let uri = resolve_url(base_url, line);
            segments.push(M3u8Segment {
                uri,
                duration: current_duration,
                key: current_key.clone(),
            });
            current_duration = 0.0;
        }
    }

    segments
}

// ============================================================================
// 内部任务控制
// ============================================================================

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

// ============================================================================
// 番剧下载管理器
// ============================================================================

#[derive(Clone)]
pub struct AnimeDownloader {
    tasks: Arc<Mutex<HashMap<String, AnimeDownloadTask>>>,
    controls: Arc<Mutex<HashMap<String, Arc<TaskControl>>>>,
    client: reqwest::Client,
    download_dir: PathBuf,
    max_parallel_segments: usize,
}

impl AnimeDownloader {
    pub fn new(download_dir: PathBuf) -> Self {
        let client = crate::http_client::build_reqwest_client(
            30 * 60,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        );

        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            controls: Arc::new(Mutex::new(HashMap::new())),
            client,
            download_dir,
            max_parallel_segments: 4,
        }
    }

    /// 启动一个番剧下载任务
    pub async fn enqueue(
        &self,
        url: String,
        filename: String,
        output_dir: Option<String>,
        anime_name: Option<String>,
        episode_name: Option<String>,
        referer: Option<String>,
    ) -> AnimeDownloadTask {
        let id = uuid::Uuid::new_v4().to_string();

        let output_path = if let Some(dir) = output_dir {
            PathBuf::from(dir).join(&filename)
        } else {
            let safe_name = sanitize_filename(anime_name.as_deref().unwrap_or("anime"));
            let ep_dir = self.download_dir.join(&safe_name);
            std::fs::create_dir_all(&ep_dir).ok();
            ep_dir.join(&filename)
        };

        let task = AnimeDownloadTask {
            id: id.clone(),
            url: url.clone(),
            filename: filename.clone(),
            output_path: output_path.clone(),
            status: AnimeDownloadStatus::Pending,
            progress: 0.0,
            total_segments: 0,
            downloaded_segments: 0,
            total_size: 0,
            downloaded_size: 0,
            speed: 0.0,
            error: None,
            is_m3u8: false,
            anime_name,
            episode_name,
        };

        let control = Arc::new(TaskControl::new());
        self.tasks.lock().await.insert(id.clone(), task.clone());
        self.controls.lock().await.insert(id.clone(), control);

        self.spawn_download(id, url, output_path, referer);
        task
    }

    /// 暂停下载
    pub async fn pause(&self, task_id: &str) -> Result<(), String> {
        let c = self.controls.lock().await;
        let ctrl = c.get(task_id).ok_or("任务不存在")?;
        ctrl.paused.store(true, Ordering::Relaxed);
        let mut t = self.tasks.lock().await;
        if let Some(task) = t.get_mut(task_id) {
            task.status = AnimeDownloadStatus::Paused;
        }
        Ok(())
    }

    /// 恢复下载
    pub async fn resume(&self, task_id: &str) -> Result<(), String> {
        let c = self.controls.lock().await;
        let ctrl = c.get(task_id).ok_or("任务不存在")?;
        ctrl.paused.store(false, Ordering::Relaxed);
        ctrl.pause_notify.notify_one();
        let mut t = self.tasks.lock().await;
        if let Some(task) = t.get_mut(task_id) {
            task.status = AnimeDownloadStatus::Downloading;
        }
        Ok(())
    }

    /// 取消下载
    pub async fn cancel(&self, task_id: &str) -> Result<(), String> {
        let c = self.controls.lock().await;
        if let Some(ctrl) = c.get(task_id) {
            ctrl.cancelled.store(true, Ordering::Relaxed);
            ctrl.pause_notify.notify_one();
        }
        let mut t = self.tasks.lock().await;
        if let Some(task) = t.get_mut(task_id) {
            task.status = AnimeDownloadStatus::Cancelled;
        }
        Ok(())
    }

    /// 获取所有任务
    pub async fn get_all(&self) -> Vec<AnimeDownloadTask> {
        self.tasks.lock().await.values().cloned().collect()
    }

    /// 移除任务
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

    /// 清除已完成/取消/失败的任务
    pub async fn clear_finished(&self) {
        self.tasks.lock().await.retain(|_, t| {
            !matches!(
                t.status,
                AnimeDownloadStatus::Completed
                    | AnimeDownloadStatus::Cancelled
                    | AnimeDownloadStatus::Failed
            )
        });
    }

    /// 打开下载文件所在目录
    pub async fn open_download_folder(&self, task_id: &str) -> Result<(), String> {
        let t = self.tasks.lock().await;
        let task = t.get(task_id).ok_or("任务不存在")?;
        let dir = task
            .output_path
            .parent()
            .ok_or("无法获取目录")?
            .to_path_buf();
        open::that(&dir).map_err(|e| format!("打开目录失败: {}", e))
    }

    // ---- 内部方法 ----

    fn spawn_download(
        &self,
        task_id: String,
        url: String,
        output_path: PathBuf,
        referer: Option<String>,
    ) {
        let tasks = self.tasks.clone();
        let controls = self.controls.clone();
        let client = self.client.clone();
        let max_parallel = self.max_parallel_segments;

        tokio::spawn(async move {
            Self::execute_download(
                task_id,
                url,
                output_path,
                referer,
                tasks,
                controls,
                client,
                max_parallel,
            )
            .await;
        });
    }

    async fn execute_download(
        task_id: String,
        url: String,
        output_path: PathBuf,
        referer: Option<String>,
        tasks: Arc<Mutex<HashMap<String, AnimeDownloadTask>>>,
        controls: Arc<Mutex<HashMap<String, Arc<TaskControl>>>>,
        client: reqwest::Client,
        max_parallel: usize,
    ) {
        // 获取控制
        let ctrl = match controls.lock().await.get(&task_id).cloned() {
            Some(c) => c,
            None => return,
        };

        // Step 1: 尝试获取 URL 内容，判断是否 m3u8
        {
            let mut t = tasks.lock().await;
            if let Some(task) = t.get_mut(&task_id) {
                task.status = AnimeDownloadStatus::Parsing;
            }
        }

        let mut req = client.get(&url);
        if let Some(ref r) = referer {
            req = req.header("Referer", r.as_str());
        }

        let is_m3u8 = match req.send().await {
            Ok(resp) => {
                let content_type = resp
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_lowercase();
                let url_lower = url.to_lowercase();

                content_type.contains("mpegurl")
                    || content_type.contains("m3u8")
                    || url_lower.contains(".m3u8")
            }
            Err(_) => url.to_lowercase().contains(".m3u8"),
        };

        {
            let mut t = tasks.lock().await;
            if let Some(task) = t.get_mut(&task_id) {
                task.is_m3u8 = is_m3u8;
            }
        }

        if is_m3u8 {
            Self::download_m3u8(
                &task_id,
                &url,
                &output_path,
                referer.as_deref(),
                &tasks,
                &ctrl,
                &client,
                max_parallel,
            )
            .await;
        } else {
            Self::download_direct(
                &task_id,
                &url,
                &output_path,
                referer.as_deref(),
                &tasks,
                &ctrl,
                &client,
            )
            .await;
        }
    }

    /// m3u8/HLS 下载：解析 → 下载分片 → 拼接
    async fn download_m3u8(
        task_id: &str,
        url: &str,
        output_path: &Path,
        referer: Option<&str>,
        tasks: &Arc<Mutex<HashMap<String, AnimeDownloadTask>>>,
        ctrl: &Arc<TaskControl>,
        client: &reqwest::Client,
        max_parallel: usize,
    ) {
        // Fetch m3u8 content
        let m3u8_content = match Self::fetch_text(client, url, referer).await {
            Ok(c) => c,
            Err(e) => {
                Self::set_error(tasks, task_id, format!("获取 m3u8 失败: {}", e)).await;
                return;
            }
        };

        let trimmed = m3u8_content.trim_start();
        if !trimmed.starts_with("#EXTM3U") {
            Self::set_error(tasks, task_id, "不是有效的 m3u8 文件".into()).await;
            return;
        }

        // Detect type and resolve to media playlist
        let (segments, _media_url) = match detect_m3u8_type(&m3u8_content) {
            M3u8Type::Master => {
                let variant = match parse_master_playlist(&m3u8_content, url) {
                    Some(v) => v,
                    None => {
                        Self::set_error(tasks, task_id, "无法解析 master playlist".into()).await;
                        return;
                    }
                };
                let media_content = match Self::fetch_text(client, &variant.uri, referer).await {
                    Ok(c) => c,
                    Err(e) => {
                        Self::set_error(tasks, task_id, format!("获取 media playlist 失败: {}", e))
                            .await;
                        return;
                    }
                };
                let segs = parse_media_playlist(&media_content, &variant.uri);
                (segs, variant.uri)
            }
            M3u8Type::Media => {
                let segs = parse_media_playlist(&m3u8_content, url);
                (segs, url.to_string())
            }
        };

        if segments.is_empty() {
            Self::set_error(tasks, task_id, "m3u8 中未找到分片".into()).await;
            return;
        }

        let total_segments = segments.len() as u32;

        {
            let mut t = tasks.lock().await;
            if let Some(task) = t.get_mut(task_id) {
                task.total_segments = total_segments;
                task.status = AnimeDownloadStatus::Downloading;
            }
        }

        // Create temp dir for segments
        let seg_dir = output_path.with_extension("segments");
        std::fs::create_dir_all(&seg_dir).ok();

        // Download segments with concurrency control
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_parallel));
        let downloaded_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let failed_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let total_bytes = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let completer = Arc::new(tokio::sync::Notify::new());

        let start_time = Instant::now();

        for (i, seg) in segments.iter().enumerate() {
            if ctrl.cancelled.load(Ordering::Relaxed) {
                let mut t = tasks.lock().await;
                if let Some(task) = t.get_mut(task_id) {
                    task.status = AnimeDownloadStatus::Cancelled;
                }
                let _ = std::fs::remove_dir_all(&seg_dir);
                return;
            }

            // Wait if paused
            while ctrl.paused.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = ctrl.pause_notify.notified() => {}
                    _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                        if ctrl.cancelled.load(Ordering::Relaxed) {
                            let mut t = tasks.lock().await;
                            if let Some(task) = t.get_mut(task_id) {
                                task.status = AnimeDownloadStatus::Cancelled;
                            }
                            let _ = std::fs::remove_dir_all(&seg_dir);
                            return;
                        }
                    }
                }
            }

            let seg_path = seg_dir.join(format!("seg_{:05}.ts", i));
            let seg_url = seg.uri.clone();
            let seg_client = client.clone();
            let seg_referer = referer.map(|s| s.to_string());
            let sem = semaphore.clone();
            let dc = downloaded_count.clone();
            let fc = failed_count.clone();
            let tb = total_bytes.clone();
            let tasks_clone = tasks.clone();
            let tid = task_id.to_string();
            let comp = completer.clone();
            let total = total_segments;
            let elapsed = start_time;

            // Skip already downloaded segments
            if seg_path.exists() {
                if let Ok(meta) = std::fs::metadata(&seg_path) {
                    if meta.len() > 0 {
                        dc.fetch_add(1, Ordering::Relaxed);
                        tb.fetch_add(meta.len(), Ordering::Relaxed);
                        let count = dc.load(Ordering::Relaxed);
                        {
                            let mut t = tasks_clone.lock().await;
                            if let Some(task) = t.get_mut(&tid) {
                                task.downloaded_segments = count;
                                task.downloaded_size = tb.load(Ordering::Relaxed);
                                task.progress = count as f32 / total as f32;
                                let e = elapsed.elapsed().as_secs_f64();
                                if e > 0.0 {
                                    task.speed = tb.load(Ordering::Relaxed) as f64 / e;
                                }
                            }
                        }
                        continue;
                    }
                }
            }

            let permit = match sem.acquire_owned().await {
                Ok(p) => p,
                Err(_) => {
                    fc.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
            };

            tokio::spawn(async move {
                let result = Self::download_segment(
                    &seg_client,
                    &seg_url,
                    &seg_path,
                    seg_referer.as_deref(),
                )
                .await;

                match result {
                    Ok(bytes) => {
                        dc.fetch_add(1, Ordering::Relaxed);
                        tb.fetch_add(bytes, Ordering::Relaxed);
                    }
                    Err(_) => {
                        fc.fetch_add(1, Ordering::Relaxed);
                    }
                }

                let count = dc.load(Ordering::Relaxed);
                let failures = fc.load(Ordering::Relaxed);
                let bytes = tb.load(Ordering::Relaxed);
                let e = elapsed.elapsed().as_secs_f64();

                {
                    let mut t = tasks_clone.lock().await;
                    if let Some(task) = t.get_mut(&tid) {
                        task.downloaded_segments = count;
                        task.downloaded_size = bytes;
                        task.progress = count as f32 / total as f32;
                        if e > 0.0 {
                            task.speed = bytes as f64 / e;
                        }
                    }
                }

                drop(permit);

                if count + failures >= total {
                    comp.notify_one();
                }
            });
        }

        // Wait for all segments to complete
        let wait_start = Instant::now();
        loop {
            let count = downloaded_count.load(Ordering::Relaxed);
            let failures = failed_count.load(Ordering::Relaxed);
            if count + failures >= total_segments {
                break;
            }
            if ctrl.cancelled.load(Ordering::Relaxed) {
                break;
            }
            if wait_start.elapsed().as_secs() > 3600 {
                // Timeout after 1 hour
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        let final_failures = failed_count.load(Ordering::Relaxed);
        if final_failures > 0 {
            Self::set_error(tasks, task_id, format!("{} 个分片下载失败", final_failures)).await;
            return;
        }

        // Merge segments into single file
        {
            let mut t = tasks.lock().await;
            if let Some(task) = t.get_mut(task_id) {
                task.status = AnimeDownloadStatus::Merging;
                task.progress = 0.95;
            }
        }

        let merge_result = Self::merge_segments(&seg_dir, total_segments as usize, output_path);

        // Cleanup temp segments
        let _ = std::fs::remove_dir_all(&seg_dir);

        match merge_result {
            Ok(final_size) => {
                let mut t = tasks.lock().await;
                if let Some(task) = t.get_mut(task_id) {
                    task.status = AnimeDownloadStatus::Completed;
                    task.progress = 1.0;
                    task.speed = 0.0;
                    task.total_size = final_size;
                    task.downloaded_size = final_size;
                }
                tracing::info!(
                    "番剧下载完成: {} ({:.1} MB)",
                    output_path.display(),
                    final_size as f64 / 1024.0 / 1024.0
                );
            }
            Err(e) => {
                Self::set_error(tasks, task_id, format!("合并分片失败: {}", e)).await;
            }
        }
    }

    /// 直链下载
    async fn download_direct(
        task_id: &str,
        url: &str,
        output_path: &Path,
        referer: Option<&str>,
        tasks: &Arc<Mutex<HashMap<String, AnimeDownloadTask>>>,
        ctrl: &Arc<TaskControl>,
        client: &reqwest::Client,
    ) {
        // Ensure parent dir exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let tmp_path = output_path.with_extension("tmp");

        // Check for resume
        let existing_bytes = std::fs::metadata(&tmp_path).map(|m| m.len()).unwrap_or(0);

        {
            let mut t = tasks.lock().await;
            if let Some(task) = t.get_mut(task_id) {
                task.status = AnimeDownloadStatus::Downloading;
                task.downloaded_size = existing_bytes;
            }
        }

        let mut req = client.get(url);
        if existing_bytes > 0 {
            req = req.header("Range", format!("bytes={}-", existing_bytes));
        }
        if let Some(r) = referer {
            req = req.header("Referer", r);
        }

        let response = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                Self::set_error(tasks, task_id, format!("请求失败: {}", e)).await;
                return;
            }
        };

        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            Self::set_error(tasks, task_id, format!("HTTP {}", status.as_u16())).await;
            return;
        }

        let content_length = response.content_length().unwrap_or(0);
        let total_size = if status == reqwest::StatusCode::PARTIAL_CONTENT {
            existing_bytes + content_length
        } else {
            content_length
        };

        {
            let mut t = tasks.lock().await;
            if let Some(task) = t.get_mut(task_id) {
                task.total_size = total_size;
            }
        }

        let mut file = match std::fs::OpenOptions::new()
            .create(true)
            .append(status == reqwest::StatusCode::PARTIAL_CONTENT)
            .write(true)
            .open(&tmp_path)
        {
            Ok(f) => f,
            Err(e) => {
                Self::set_error(tasks, task_id, format!("无法创建文件: {}", e)).await;
                return;
            }
        };

        let mut stream = response.bytes_stream();
        let mut downloaded = existing_bytes;
        let mut last_update = Instant::now();
        let start_time = Instant::now();

        loop {
            // Check pause/cancel
            while ctrl.paused.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = ctrl.pause_notify.notified() => {}
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        if ctrl.cancelled.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            }
            if ctrl.cancelled.load(Ordering::Relaxed) {
                return;
            }

            let chunk =
                match tokio::time::timeout(std::time::Duration::from_secs(30), stream.next()).await
                {
                    Ok(Some(Ok(d))) => d,
                    Ok(Some(Err(e))) => {
                        Self::set_error(tasks, task_id, format!("下载流错误: {}", e)).await;
                        return;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        Self::set_error(tasks, task_id, "下载超时".into()).await;
                        return;
                    }
                };

            if file.write_all(&chunk).is_err() {
                Self::set_error(tasks, task_id, "写入失败".into()).await;
                return;
            }

            downloaded += chunk.len() as u64;

            if last_update.elapsed().as_millis() >= 200 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let mut t = tasks.lock().await;
                if let Some(task) = t.get_mut(task_id) {
                    task.downloaded_size = downloaded;
                    task.progress = if total_size > 0 {
                        downloaded as f32 / total_size as f32
                    } else {
                        0.0
                    };
                    if elapsed > 0.0 {
                        task.speed = downloaded as f64 / elapsed;
                    }
                }
                last_update = Instant::now();
            }
        }

        // Rename tmp to final
        if std::fs::rename(&tmp_path, output_path).is_err() {
            // If rename fails, try copy
            if std::fs::copy(&tmp_path, output_path).is_ok() {
                let _ = std::fs::remove_file(&tmp_path);
            } else {
                Self::set_error(tasks, task_id, "重命名文件失败".into()).await;
                return;
            }
        }

        let final_size = std::fs::metadata(output_path)
            .map(|m| m.len())
            .unwrap_or(downloaded);

        let mut t = tasks.lock().await;
        if let Some(task) = t.get_mut(task_id) {
            task.status = AnimeDownloadStatus::Completed;
            task.progress = 1.0;
            task.speed = 0.0;
            task.total_size = final_size;
            task.downloaded_size = final_size;
        }
    }

    /// 下载单个分片（带重试）
    async fn download_segment(
        client: &reqwest::Client,
        url: &str,
        path: &Path,
        referer: Option<&str>,
    ) -> Result<u64, String> {
        let tmp_path = path.with_extension("ts.tmp");
        let max_retries = 3;

        for attempt in 0..max_retries {
            let mut req = client.get(url);
            if let Some(r) = referer {
                req = req.header("Referer", r);
            }

            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        if attempt < max_retries - 1 {
                            tokio::time::sleep(std::time::Duration::from_secs(
                                2u64.pow(attempt as u32),
                            ))
                            .await;
                            continue;
                        }
                        return Err(format!("HTTP {}", resp.status().as_u16()));
                    }

                    let bytes = resp
                        .bytes()
                        .await
                        .map_err(|e| format!("读取响应失败: {}", e))?;

                    let mut f = tokio::fs::File::create(&tmp_path)
                        .await
                        .map_err(|e| format!("创建文件失败: {}", e))?;
                    f.write_all(&bytes)
                        .await
                        .map_err(|e| format!("写入失败: {}", e))?;
                    f.flush().await.ok();

                    let len = bytes.len() as u64;
                    tokio::fs::rename(&tmp_path, path)
                        .await
                        .map_err(|e| format!("重命名失败: {}", e))?;
                    return Ok(len);
                }
                Err(e) => {
                    if attempt < max_retries - 1 {
                        tokio::time::sleep(std::time::Duration::from_secs(
                            2u64.pow(attempt as u32),
                        ))
                        .await;
                        continue;
                    }
                    return Err(format!("请求失败: {}", e));
                }
            }
        }

        Err("超过最大重试次数".into())
    }

    /// 拼接 .ts 分片为单个文件
    fn merge_segments(seg_dir: &Path, count: usize, output: &Path) -> Result<u64, String> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }

        let mut out_file =
            std::fs::File::create(output).map_err(|e| format!("创建输出文件失败: {}", e))?;

        let mut total_size = 0u64;

        for i in 0..count {
            let seg_path = seg_dir.join(format!("seg_{:05}.ts", i));
            if !seg_path.exists() {
                return Err(format!("分片 {} 不存在", i));
            }
            let mut seg_data = Vec::new();
            let mut f = std::fs::File::open(&seg_path)
                .map_err(|e| format!("打开分片 {} 失败: {}", i, e))?;
            std::io::Read::read_to_end(&mut f, &mut seg_data)
                .map_err(|e| format!("读取分片 {} 失败: {}", i, e))?;
            total_size += seg_data.len() as u64;
            out_file
                .write_all(&seg_data)
                .map_err(|e| format!("写入分片 {} 失败: {}", i, e))?;
        }

        out_file
            .flush()
            .map_err(|e| format!("刷新输出文件失败: {}", e))?;

        Ok(total_size)
    }

    /// 获取文本内容
    async fn fetch_text(
        client: &reqwest::Client,
        url: &str,
        referer: Option<&str>,
    ) -> Result<String, String> {
        let mut req = client.get(url);
        if let Some(r) = referer {
            req = req.header("Referer", r);
        }
        let resp = req.send().await.map_err(|e| format!("请求失败: {}", e))?;
        resp.text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))
    }

    async fn set_error(
        tasks: &Arc<Mutex<HashMap<String, AnimeDownloadTask>>>,
        task_id: &str,
        error: String,
    ) {
        let mut t = tasks.lock().await;
        if let Some(task) = t.get_mut(task_id) {
            task.status = AnimeDownloadStatus::Failed;
            task.error = Some(error);
        }
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
        "anime".into()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_media_playlist() {
        let content = r#"#EXTM3U
#EXT-X-TARGETDURATION:10
#EXTINF:9.009,
http://example.com/seg1.ts
#EXTINF:9.009,
http://example.com/seg2.ts
#EXTINF:3.003,
http://example.com/seg3.ts
#EXT-X-ENDLIST"#;

        let segments = parse_media_playlist(content, "http://example.com/playlist.m3u8");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].uri, "http://example.com/seg1.ts");
        assert!((segments[0].duration - 9.009).abs() < 0.001);
    }

    #[test]
    fn test_detect_m3u8_type() {
        let master = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\nlow.m3u8";
        assert!(matches!(detect_m3u8_type(master), M3u8Type::Master));

        let media = "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXTINF:9,\nseg.ts";
        assert!(matches!(detect_m3u8_type(media), M3u8Type::Media));
    }

    #[test]
    fn test_resolve_url() {
        assert_eq!(
            resolve_url("http://cdn.com/hls/playlist.m3u8", "seg1.ts"),
            "http://cdn.com/hls/seg1.ts"
        );
        assert_eq!(
            resolve_url("http://cdn.com/hls/playlist.m3u8", "/v/seg2.ts"),
            "http://cdn.com/v/seg2.ts"
        );
        assert_eq!(
            resolve_url("http://cdn.com/hls/playlist.m3u8", "https://other.com/s.ts"),
            "https://other.com/s.ts"
        );
    }

    #[test]
    fn test_parse_master_playlist() {
        let content = r#"#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION=640x360
low.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=2560000,RESOLUTION=1280x720
mid.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=5120000,RESOLUTION=1920x1080
high.m3u8"#;

        let variant = parse_master_playlist(content, "http://example.com/master.m3u8").unwrap();
        assert_eq!(variant.bandwidth, 5120000);
        assert_eq!(variant.uri, "http://example.com/high.m3u8");
    }
}
