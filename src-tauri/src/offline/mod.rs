//! Durable offline chapter storage.
//!
//! This module owns only the offline catalog and chapter files.  It deliberately
//! does not touch reading history: downloading a chapter is not a reading event.

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const OFFLINE_FORMAT: &str = "moeplay-offline";
pub const OFFLINE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfflineState {
    Queued,
    Downloading,
    Paused,
    Failed,
    Cancelled,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfflineResourceState {
    Pending,
    Staging,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineResourceSpec {
    pub resource_key: String,
    #[serde(default)]
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineChapterRequest {
    pub chapter_id: String,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub resources: Vec<OfflineResourceSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineEnqueueRequest {
    pub content_type: String,
    pub source_id: String,
    pub content_id: String,
    pub title: String,
    #[serde(default)]
    pub chapters: Vec<OfflineChapterRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineResourceInput {
    pub resource_key: String,
    #[serde(default)]
    pub bytes_base64: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineSupplyRequest {
    pub chapter_key: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub resources: Vec<OfflineResourceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineControlRequest {
    pub chapter_key: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineResource {
    pub resource_key: String,
    pub filename: String,
    pub bytes: u64,
    pub state: OfflineResourceState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineChapter {
    pub offline_bundle_id: String,
    pub offline_chapter_key: String,
    pub content_type: String,
    pub source_id: String,
    pub content_id: String,
    pub chapter_id: String,
    pub order: i32,
    pub title: String,
    pub state: OfflineState,
    pub resource_state: OfflineResourceState,
    pub readable: bool,
    pub bytes: u64,
    pub resources: Vec<OfflineResource>,
    pub save_path: String,
    #[serde(default)]
    pub staging_path: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OfflineManifest {
    pub format: String,
    pub version: u32,
    pub chapters: Vec<OfflineChapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineStats {
    pub chapter_count: usize,
    pub complete_count: usize,
    pub pending_count: usize,
    pub failed_count: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineChapterContent {
    pub chapter: OfflineChapter,
    pub body: Option<String>,
    pub resource_paths: Vec<String>,
}

#[derive(Clone)]
pub struct OfflineStore {
    root: PathBuf,
    manifest_path: PathBuf,
    manifest: Arc<Mutex<OfflineManifest>>,
}

impl OfflineStore {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(root.join("chapters")).map_err(io_error)?;
        fs::create_dir_all(root.join("staging")).map_err(io_error)?;
        let manifest_path = root.join("manifest.json");
        let manifest = if manifest_path.exists() {
            match fs::read(&manifest_path)
                .ok()
                .and_then(|data| serde_json::from_slice::<OfflineManifest>(&data).ok())
            {
                Some(mut value)
                    if value.format == OFFLINE_FORMAT && value.version == OFFLINE_VERSION =>
                {
                    hydrate_manifest(&root, &mut value);
                    value
                }
                _ => {
                    let backup = root.join(format!("manifest.corrupt-{}.json", now_ms()));
                    let _ = fs::rename(&manifest_path, backup);
                    OfflineManifest {
                        format: OFFLINE_FORMAT.into(),
                        version: OFFLINE_VERSION,
                        chapters: Vec::new(),
                    }
                }
            }
        } else {
            OfflineManifest {
                format: OFFLINE_FORMAT.into(),
                version: OFFLINE_VERSION,
                chapters: Vec::new(),
            }
        };
        let store = Self {
            root,
            manifest_path,
            manifest: Arc::new(Mutex::new(manifest)),
        };
        store.persist()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn enqueue(&self, request: OfflineEnqueueRequest) -> Result<Vec<OfflineChapter>, String> {
        validate_token("content_type", &request.content_type)?;
        validate_token("source_id", &request.source_id)?;
        validate_token("content_id", &request.content_id)?;
        if request.chapters.is_empty() {
            return Err("至少选择一个章节".into());
        }
        let bundle = stable_id(&[
            &request.content_type,
            &request.source_id,
            &request.content_id,
        ]);
        let mut manifest = self.manifest.lock().map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for input in request.chapters {
            validate_token("chapter_id", &input.chapter_id)?;
            let key = stable_id(&[
                &request.content_type,
                &request.source_id,
                &request.content_id,
                &input.chapter_id,
            ]);
            if let Some(existing) = manifest
                .chapters
                .iter()
                .find(|c| c.offline_chapter_key == key)
            {
                result.push(existing.clone());
                continue;
            }
            let chapter_dir = self.root.join("chapters").join(&key);
            let staging_dir = self.root.join("staging").join(&key);
            let resources = input
                .resources
                .into_iter()
                .map(|resource| OfflineResource {
                    filename: safe_filename(
                        resource
                            .filename
                            .as_deref()
                            .unwrap_or(&resource.resource_key),
                    ),
                    resource_key: resource.resource_key,
                    bytes: 0,
                    state: OfflineResourceState::Pending,
                })
                .collect::<Vec<_>>();
            let chapter = OfflineChapter {
                offline_bundle_id: bundle.clone(),
                offline_chapter_key: key.clone(),
                content_type: request.content_type.clone(),
                source_id: request.source_id.clone(),
                content_id: request.content_id.clone(),
                chapter_id: input.chapter_id,
                order: input.order,
                title: input.title,
                state: OfflineState::Queued,
                resource_state: OfflineResourceState::Pending,
                readable: false,
                bytes: 0,
                resources,
                save_path: chapter_dir.to_string_lossy().into(),
                staging_path: Some(staging_dir.to_string_lossy().into()),
                task_id: None,
                error: None,
                updated_at: now_ms(),
            };
            manifest.chapters.push(chapter.clone());
            result.push(chapter);
        }
        drop(manifest);
        self.persist()?;
        Ok(result)
    }

    pub fn attach_task(
        &self,
        chapter_key: &str,
        task_id: String,
    ) -> Result<OfflineChapter, String> {
        self.mutate(chapter_key, |chapter| {
            chapter.task_id = Some(task_id);
            Ok(())
        })
    }

    pub fn list(&self) -> Result<Vec<OfflineChapter>, String> {
        self.hydrate();
        let manifest = self.manifest.lock().map_err(|e| e.to_string())?;
        Ok(manifest.chapters.clone())
    }

    pub fn supply(&self, request: OfflineSupplyRequest) -> Result<OfflineChapter, String> {
        let current = self.find(&request.chapter_key)?;
        if current.state == OfflineState::Cancelled {
            return Err("已取消的章节不能写入".into());
        }
        let staging = self.root.join("staging").join(&current.offline_chapter_key);
        fs::create_dir_all(&staging).map_err(io_error)?;
        let mut bytes_needed: u64 = request
            .resources
            .iter()
            .filter_map(|r| r.bytes_base64.as_ref())
            .map(|v| ((v.len() * 3) / 4) as u64)
            .sum();
        bytes_needed =
            bytes_needed.saturating_add(request.body.as_ref().map_or(0, |body| body.len() as u64));
        ensure_space(&self.root, bytes_needed)?;
        for input in request.resources {
            validate_token("resource_key", &input.resource_key)?;
            let encoded = input
                .bytes_base64
                .ok_or_else(|| format!("资源 {} 缺少内容", input.resource_key))?;
            let data = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|_| "资源不是有效的 base64".to_string())?;
            let filename = safe_filename(input.filename.as_deref().unwrap_or(&input.resource_key));
            let destination = staging.join(&filename);
            atomic_write(&destination, &data)?;
            self.mutate_in_memory(&request.chapter_key, |chapter| {
                if let Some(resource) = chapter
                    .resources
                    .iter_mut()
                    .find(|r| r.resource_key == input.resource_key)
                {
                    resource.filename = filename.clone();
                    resource.bytes = data.len() as u64;
                    resource.state = OfflineResourceState::Complete;
                } else {
                    chapter.resources.push(OfflineResource {
                        resource_key: input.resource_key.clone(),
                        filename: filename.clone(),
                        bytes: data.len() as u64,
                        state: OfflineResourceState::Complete,
                    });
                }
                Ok(())
            })?;
        }
        if let Some(body) = request.body.as_deref() {
            if body.trim().is_empty() {
                return Err("小说正文不能为空".into());
            }
            atomic_write(&staging.join("body.txt"), body.as_bytes())?;
        }
        let mut chapter = self.find(&request.chapter_key)?;
        let complete = if chapter.content_type.eq_ignore_ascii_case("novel") {
            staging.join("body.txt").is_file()
                && fs::metadata(staging.join("body.txt"))
                    .map(|m| m.len())
                    .unwrap_or(0)
                    > 0
        } else {
            !chapter.resources.is_empty()
                && chapter
                    .resources
                    .iter()
                    .all(|r| r.state == OfflineResourceState::Complete)
        };
        self.mutate_in_memory(&request.chapter_key, |value| {
            value.state = if complete {
                OfflineState::Complete
            } else {
                OfflineState::Downloading
            };
            value.resource_state = if complete {
                OfflineResourceState::Complete
            } else {
                OfflineResourceState::Staging
            };
            value.readable = complete;
            value.bytes = directory_size(&staging);
            value.error = None;
            value.updated_at = now_ms();
            Ok(())
        })?;
        chapter = self.find(&request.chapter_key)?;
        if complete {
            let final_dir = self
                .root
                .join("chapters")
                .join(&chapter.offline_chapter_key);
            if final_dir.exists() {
                fs::remove_dir_all(&final_dir).map_err(io_error)?;
            }
            fs::rename(&staging, &final_dir).map_err(io_error)?;
            self.mutate(&request.chapter_key, |value| {
                value.staging_path = None;
                value.save_path = final_dir.to_string_lossy().into();
                value.bytes = directory_size(&final_dir);
                Ok(())
            })?;
            chapter = self.find(&request.chapter_key)?;
        }
        Ok(chapter)
    }

    pub fn get_chapter(&self, chapter_key: &str) -> Result<OfflineChapterContent, String> {
        let chapter = self.find(chapter_key)?;
        if !chapter.readable || chapter.state != OfflineState::Complete {
            return Err("章节尚未完整下载".into());
        }
        let dir = PathBuf::from(&chapter.save_path);
        let body = fs::read_to_string(dir.join("body.txt")).ok();
        let resource_paths = chapter
            .resources
            .iter()
            .map(|r| dir.join(&r.filename).to_string_lossy().into())
            .collect();
        Ok(OfflineChapterContent {
            chapter,
            body,
            resource_paths,
        })
    }

    pub fn control(&self, request: OfflineControlRequest) -> Result<OfflineChapter, String> {
        let current = self.find(&request.chapter_key)?;
        match request.action.trim().to_ascii_lowercase().as_str() {
            "pause" => self.mutate(&request.chapter_key, |c| {
                if c.readable {
                    return Err("已完成章节不可暂停".into());
                }
                c.state = OfflineState::Paused;
                c.updated_at = now_ms();
                Ok(())
            })?,
            "resume" => self.mutate(&request.chapter_key, |c| {
                if c.state != OfflineState::Paused && c.state != OfflineState::Failed {
                    return Err("只有暂停或失败任务可以继续".into());
                }
                c.state = OfflineState::Queued;
                c.error = None;
                c.updated_at = now_ms();
                Ok(())
            })?,
            "retry" => self.mutate(&request.chapter_key, |c| {
                if c.state != OfflineState::Failed && c.state != OfflineState::Paused {
                    return Err("只有暂停或失败任务可以重试".into());
                }
                c.state = OfflineState::Queued;
                c.error = None;
                c.updated_at = now_ms();
                Ok(())
            })?,
            "cancel" => self.mutate(&request.chapter_key, |c| {
                if c.state == OfflineState::Complete {
                    return Err("已完成章节不可取消".into());
                }
                c.state = OfflineState::Cancelled;
                c.readable = false;
                c.updated_at = now_ms();
                Ok(())
            })?,
            "delete" => {
                let path = self
                    .root
                    .join("chapters")
                    .join(&current.offline_chapter_key);
                let staging = self.root.join("staging").join(&current.offline_chapter_key);
                if path.exists() {
                    fs::remove_dir_all(&path).map_err(io_error)?;
                }
                if staging.exists() {
                    fs::remove_dir_all(&staging).map_err(io_error)?;
                }
                let mut manifest = self.manifest.lock().map_err(|e| e.to_string())?;
                manifest
                    .chapters
                    .retain(|c| c.offline_chapter_key != request.chapter_key);
                drop(manifest);
                self.persist()?;
                return Ok(OfflineChapter {
                    state: OfflineState::Cancelled,
                    readable: false,
                    ..current
                });
            }
            action => return Err(format!("不支持的离线操作: {action}")),
        };
        self.find(&request.chapter_key)
    }

    pub fn stats(&self) -> Result<OfflineStats, String> {
        let chapters = self.list()?;
        Ok(OfflineStats {
            chapter_count: chapters.len(),
            complete_count: chapters.iter().filter(|c| c.readable).count(),
            pending_count: chapters
                .iter()
                .filter(|c| {
                    matches!(
                        c.state,
                        OfflineState::Queued | OfflineState::Downloading | OfflineState::Paused
                    )
                })
                .count(),
            failed_count: chapters
                .iter()
                .filter(|c| matches!(c.state, OfflineState::Failed | OfflineState::Cancelled))
                .count(),
            bytes: chapters.iter().map(|c| c.bytes).sum(),
        })
    }

    fn hydrate(&self) {
        let _ = self.mutate_all(|chapter| {
            let final_dir = PathBuf::from(&chapter.save_path);
            if chapter.state == OfflineState::Complete
                && (!final_dir.is_dir()
                    || (chapter.content_type == "novel" && !final_dir.join("body.txt").is_file()))
            {
                chapter.state = OfflineState::Failed;
                chapter.resource_state = OfflineResourceState::Failed;
                chapter.readable = false;
                chapter.error = Some("离线文件缺失，请重试下载".into());
            } else if !chapter.readable
                && chapter.state == OfflineState::Downloading
                && chapter
                    .staging_path
                    .as_deref()
                    .map(Path::new)
                    .is_some_and(|p| !p.exists())
            {
                chapter.state = OfflineState::Queued;
                chapter.resource_state = OfflineResourceState::Pending;
            }
            Ok(())
        });
    }

    fn find(&self, key: &str) -> Result<OfflineChapter, String> {
        let manifest = self.manifest.lock().map_err(|e| e.to_string())?;
        manifest
            .chapters
            .iter()
            .find(|c| c.offline_chapter_key == key)
            .cloned()
            .ok_or_else(|| "离线章节不存在".into())
    }
    fn mutate<F>(&self, key: &str, f: F) -> Result<OfflineChapter, String>
    where
        F: FnOnce(&mut OfflineChapter) -> Result<(), String>,
    {
        self.mutate_in_memory(key, f)?;
        self.find(key)
    }
    fn mutate_in_memory<F>(&self, key: &str, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut OfflineChapter) -> Result<(), String>,
    {
        let mut manifest = self.manifest.lock().map_err(|e| e.to_string())?;
        let chapter = manifest
            .chapters
            .iter_mut()
            .find(|c| c.offline_chapter_key == key)
            .ok_or_else(|| "离线章节不存在".to_string())?;
        f(chapter)?;
        drop(manifest);
        self.persist()
    }
    fn mutate_all<F>(&self, mut f: F) -> Result<(), String>
    where
        F: FnMut(&mut OfflineChapter) -> Result<(), String>,
    {
        let mut manifest = self.manifest.lock().map_err(|e| e.to_string())?;
        for chapter in &mut manifest.chapters {
            f(chapter)?;
        }
        drop(manifest);
        self.persist()
    }
    fn persist(&self) -> Result<(), String> {
        let manifest = self.manifest.lock().map_err(|e| e.to_string())?.clone();
        atomic_write(
            &self.manifest_path,
            serde_json::to_vec_pretty(&manifest)
                .map_err(|e| e.to_string())?
                .as_slice(),
        )
    }
}

fn hydrate_manifest(root: &Path, manifest: &mut OfflineManifest) {
    // Manifest files are user data and may be damaged or hand-edited. Keep
    // only opaque single-component keys and always derive paths from the
    // configured offline root, never from persisted path strings.
    manifest
        .chapters
        .retain(|chapter| is_safe_component(&chapter.offline_chapter_key));
    for chapter in &mut manifest.chapters {
        chapter.save_path = root
            .join("chapters")
            .join(&chapter.offline_chapter_key)
            .to_string_lossy()
            .into();
        if chapter.staging_path.is_some() {
            chapter.staging_path = Some(
                root.join("staging")
                    .join(&chapter.offline_chapter_key)
                    .to_string_lossy()
                    .into(),
            );
        }
    }
}
fn is_safe_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
}
fn stable_id(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
        h.update([0]);
    }
    hex::encode(h.finalize())
}
fn validate_token(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.len() > 256
        || value.contains('/')
        || value.contains('\\')
        || value.contains("..")
    {
        Err(format!("{field} 含有非法值"))
    } else {
        Ok(())
    }
}
fn safe_filename(value: &str) -> String {
    let value = value.trim();
    let name = Path::new(value)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("resource.bin");
    if name.is_empty() || name == "." || name == ".." {
        "resource.bin".into()
    } else {
        name.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("无效存储路径")?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        Uuid::new_v4()
    ));
    fs::write(&temp, bytes).map_err(io_error)?;
    fs::rename(&temp, path).map_err(io_error)
}
fn directory_size(path: &Path) -> u64 {
    if !path.is_dir() {
        return 0;
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| {
            if e.path().is_dir() {
                directory_size(&e.path())
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}
fn ensure_space(path: &Path, required: u64) -> Result<(), String> {
    if required == 0 {
        return Ok(());
    }
    let disks = sysinfo::Disks::new_with_refreshed_list();
    if let Some(disk) = disks
        .list()
        .iter()
        .filter(|d| path.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
    {
        if disk.available_space() < required {
            return Err(format!("磁盘空间不足：需要 {required} 字节"));
        }
    }
    Ok(())
}
fn io_error(error: std::io::Error) -> String {
    error.to_string()
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn request() -> OfflineEnqueueRequest {
        OfflineEnqueueRequest {
            content_type: "manga".into(),
            source_id: "source".into(),
            content_id: "book".into(),
            title: "Book".into(),
            chapters: vec![OfflineChapterRequest {
                chapter_id: "12".into(),
                order: 12,
                title: "第12话".into(),
                resources: vec![OfflineResourceSpec {
                    resource_key: "p7".into(),
                    filename: Some("7.jpg".into()),
                }],
            }],
        }
    }
    #[test]
    fn enqueue_is_idempotent_and_uses_stable_key() {
        let dir = tempdir().unwrap();
        let store = OfflineStore::new(dir.path().to_path_buf()).unwrap();
        let first = store.enqueue(request()).unwrap();
        let second = store.enqueue(request()).unwrap();
        assert_eq!(first[0].offline_chapter_key, second[0].offline_chapter_key);
        assert_eq!(store.list().unwrap().len(), 1);
    }
    #[test]
    fn incomplete_pages_are_not_readable_and_complete_supply_is_atomic() {
        let dir = tempdir().unwrap();
        let store = OfflineStore::new(dir.path().to_path_buf()).unwrap();
        let chapter = store.enqueue(request()).unwrap().remove(0);
        let incomplete = store
            .supply(OfflineSupplyRequest {
                chapter_key: chapter.offline_chapter_key.clone(),
                body: None,
                resources: vec![],
            })
            .unwrap();
        assert!(!incomplete.readable);
        let complete = store
            .supply(OfflineSupplyRequest {
                chapter_key: chapter.offline_chapter_key.clone(),
                body: None,
                resources: vec![OfflineResourceInput {
                    resource_key: "p7".into(),
                    bytes_base64: Some(base64::engine::general_purpose::STANDARD.encode(b"image")),
                    filename: Some("7.jpg".into()),
                }],
            })
            .unwrap();
        assert!(complete.readable);
        assert!(
            store
                .get_chapter(&chapter.offline_chapter_key)
                .unwrap()
                .resource_paths
                .len()
                == 1
        );
    }
    #[test]
    fn missing_completed_file_becomes_recoverable_failure() {
        let dir = tempdir().unwrap();
        let store = OfflineStore::new(dir.path().to_path_buf()).unwrap();
        let chapter = store.enqueue(request()).unwrap().remove(0);
        store
            .supply(OfflineSupplyRequest {
                chapter_key: chapter.offline_chapter_key.clone(),
                body: None,
                resources: vec![OfflineResourceInput {
                    resource_key: "p7".into(),
                    bytes_base64: Some(base64::engine::general_purpose::STANDARD.encode(b"x")),
                    filename: Some("7.jpg".into()),
                }],
            })
            .unwrap();
        fs::remove_dir_all(PathBuf::from(
            store.find(&chapter.offline_chapter_key).unwrap().save_path,
        ))
        .unwrap();
        let restarted = OfflineStore::new(dir.path().to_path_buf()).unwrap();
        assert_eq!(restarted.list().unwrap()[0].state, OfflineState::Failed);
    }
    #[test]
    fn delete_updates_stats_without_history_side_effect() {
        let dir = tempdir().unwrap();
        let store = OfflineStore::new(dir.path().to_path_buf()).unwrap();
        let chapter = store.enqueue(request()).unwrap().remove(0);
        assert!(store
            .control(OfflineControlRequest {
                chapter_key: chapter.offline_chapter_key,
                action: "delete".into()
            })
            .is_ok());
        assert_eq!(store.stats().unwrap().chapter_count, 0);
    }

    #[test]
    fn manifest_paths_are_rebuilt_inside_offline_root() {
        let dir = tempdir().unwrap();
        let store = OfflineStore::new(dir.path().to_path_buf()).unwrap();
        let mut chapter = store.enqueue(request()).unwrap().remove(0);
        chapter.save_path = "C:\\outside".into();
        chapter.staging_path = Some("..\\outside".into());
        let mut unsafe_chapter = chapter.clone();
        unsafe_chapter.offline_chapter_key = "../outside".into();
        let mut manifest = OfflineManifest {
            format: OFFLINE_FORMAT.into(),
            version: OFFLINE_VERSION,
            chapters: vec![chapter, unsafe_chapter],
        };
        hydrate_manifest(dir.path(), &mut manifest);
        assert_eq!(manifest.chapters.len(), 1);
        assert!(Path::new(&manifest.chapters[0].save_path).starts_with(dir.path().join("chapters")));
        assert!(
            Path::new(manifest.chapters[0].staging_path.as_deref().unwrap())
                .starts_with(dir.path().join("staging"))
        );
    }
}
