//! 缩略图缓存（M0 增强版）
//!
//! - 磁盘缓存：图片缩放后存 `thumbs/{hash}.jpg`
//! - 内存 LRU：最近访问的缩略图保持在内存中
//! - 异步生成：`image` crate 解码 + 缩放 + JPEG 编码
//! - 过期清理：超过 30 天未访问的缩略图自动清理

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

/// 内存 LRU 缓存最大条目数
const MEMORY_CACHE_SIZE: usize = 200;
/// 缩略图最大宽度（等比缩放）
const THUMB_MAX_WIDTH: u32 = 400;
/// 缩略图最大高度
const THUMB_MAX_HEIGHT: u32 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailInfo {
    pub key: String,
    pub source: String,
    pub path: String,
    pub size: u64,
    pub cached: bool,
}

/// 内存 LRU 缓存条目
#[allow(dead_code)]
struct CacheEntry {
    data: Vec<u8>,
    info: ThumbnailInfo,
}

/// 线程安全的缩略图管理器
pub struct ThumbnailCache {
    /// 内存 LRU（最近访问的在前面）
    memory: Mutex<Vec<(String, CacheEntry)>>,
    /// 最大内存条目
    max_memory: usize,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self {
            memory: Mutex::new(Vec::new()),
            max_memory: MEMORY_CACHE_SIZE,
        }
    }

    /// 从内存或磁盘获取缩略图，不存在则异步生成。
    pub async fn get_or_generate(&self, key: &str, source: &str) -> Result<ThumbnailInfo, String> {
        let cache_key = cache_identity(key, source);
        // 1. 查内存 LRU
        {
            let mut mem = self.memory.lock().map_err(|e| e.to_string())?;
            if let Some(pos) = mem.iter().position(|(k, _)| k == &cache_key) {
                let entry = mem.remove(pos);
                let info = entry.1.info.clone();
                mem.insert(0, (cache_key.clone(), entry.1));
                return Ok(info);
            }
        }

        // 2. 查磁盘缓存
        let disk_path = disk_path_for(&cache_key);
        if disk_path.is_file() {
            let info = make_info(key, source, &disk_path);
            // 加载到内存 LRU
            if let Ok(data) = fs::read(&disk_path) {
                self.insert_memory(&cache_key, &info, data);
            }
            return Ok(info);
        }

        // 3. 生成缩略图
        self.generate(key, source).await
    }

    /// 强制生成缩略图（覆盖已有）。
    pub async fn generate(&self, key: &str, source: &str) -> Result<ThumbnailInfo, String> {
        let dir = cache_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let cache_key = cache_identity(key, source);
        let disk_path = disk_path_for(&cache_key);

        // 下载或读取原图
        let raw_bytes = if source.starts_with("http://") || source.starts_with("https://") {
            reqwest::get(source)
                .await
                .map_err(|e| format!("下载失败: {}", e))?
                .bytes()
                .await
                .map_err(|e| format!("读取响应失败: {}", e))?
                .to_vec()
        } else {
            fs::read(source).map_err(|e| format!("读取文件失败: {}", e))?
        };

        // 解码 + 缩放 + 编码
        let thumb_bytes = tokio::task::spawn_blocking(move || {
            resize_image(&raw_bytes, THUMB_MAX_WIDTH, THUMB_MAX_HEIGHT)
        })
        .await
        .map_err(|e| format!("缩放任务失败: {}", e))?
        .map_err(|e| e.to_string())?;

        // 写入磁盘
        fs::write(&disk_path, &thumb_bytes).map_err(|e| format!("写入缓存失败: {}", e))?;

        let info = make_info(key, source, &disk_path);

        // 插入内存 LRU
        self.insert_memory(&cache_key, &info, thumb_bytes);

        Ok(info)
    }

    /// 仅从内存或磁盘读取，不生成。
    pub fn get(&self, key: &str) -> Option<ThumbnailInfo> {
        // 查内存
        {
            let mut mem = self.memory.lock().ok()?;
            if let Some(pos) = mem
                .iter()
                .position(|(k, _)| k == key || k.starts_with(&format!("{key}|")))
            {
                let entry = mem.remove(pos);
                let info = entry.1.info.clone();
                mem.insert(0, entry);
                return Some(info);
            }
        }

        // 查磁盘
        let dir = cache_dir();
        let prefix = disk_stem_prefix(key);
        let entries = fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let stem = path.file_stem().and_then(|s| s.to_str())?;
            if stem.starts_with(&prefix) && path.is_file() {
                return Some(make_info(key, "", &path));
            }
        }
        None
    }

    /// 清除全部缓存。
    pub fn clear(&self) -> Result<u32, String> {
        // 清内存
        {
            let mut mem = self.memory.lock().map_err(|e| e.to_string())?;
            mem.clear();
        }
        // 清磁盘
        let dir = cache_dir();
        let mut removed = 0u32;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    fs::remove_file(path).map_err(|e| e.to_string())?;
                    removed += 1;
                }
            }
        }
        tracing::info!(removed, "Thumbnail cache cleared");
        Ok(removed)
    }

    /// 获取内存缓存统计。
    pub fn memory_stats(&self) -> (usize, usize) {
        let mem = self.memory.lock().unwrap();
        (mem.len(), self.max_memory)
    }

    // ---- 内部 ----

    fn insert_memory(&self, key: &str, info: &ThumbnailInfo, data: Vec<u8>) {
        let mut mem = self.memory.lock().unwrap();
        // 去重：移除旧条目
        mem.retain(|(k, _)| k != key);
        // 插入到最前面（最近使用）
        mem.insert(
            0,
            (
                key.to_string(),
                CacheEntry {
                    data,
                    info: info.clone(),
                },
            ),
        );
        // 超过容量则移除最旧的
        while mem.len() > self.max_memory {
            mem.pop();
        }
    }
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---- 兼容旧 API（供 commands.rs 调用）----

/// 全局懒初始化缩略图缓存
static THUMB_CACHE: std::sync::LazyLock<ThumbnailCache> =
    std::sync::LazyLock::new(ThumbnailCache::new);

pub async fn cache_thumbnail(key: &str, source: &str) -> Result<ThumbnailInfo, String> {
    THUMB_CACHE.get_or_generate(key, source).await
}

pub fn get_thumbnail(key: &str) -> Option<ThumbnailInfo> {
    THUMB_CACHE.get(key)
}

pub fn clear_thumbnail_cache() -> Result<u32, String> {
    THUMB_CACHE.clear()
}

/// 启动时清理：仅删除超过 `max_age_days` 天未修改的磁盘缩略图，保留近期封面缓存。
/// 取代每次启动的全量 clear()，避免 500+ 封面反复重生成拖慢首屏。
pub fn prune_thumbnails(max_age_days: u64) -> Result<u32, String> {
    let dir = cache_dir();
    let cutoff = match std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_days * 24 * 60 * 60))
    {
        Some(c) => c,
        None => return Ok(0),
    };
    let mut removed = 0u32;
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let too_old = entry
                .metadata()
                .and_then(|m| m.modified())
                .map(|mtime| mtime < cutoff)
                .unwrap_or(false);
            if too_old && fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }
    if removed > 0 {
        tracing::info!(removed, "Pruned thumbnails older than 30 days");
    }
    Ok(removed)
}

// ---- 内部函数 ----

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("moeplay")
        .join("thumbnails")
}

fn disk_path_for(cache_identity: &str) -> PathBuf {
    let dir = cache_dir();
    dir.join(format!("{}.jpg", disk_stem(cache_identity)))
}

fn make_info(key: &str, source: &str, path: &Path) -> ThumbnailInfo {
    ThumbnailInfo {
        key: key.to_string(),
        source: source.to_string(),
        path: path.to_string_lossy().to_string(),
        size: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        cached: true,
    }
}

fn cache_identity(key: &str, source: &str) -> String {
    let source = source.trim();
    if source.is_empty() {
        return key.to_string();
    }

    match local_file_signature(source) {
        Some(signature) => format!("{key}|source:{source}|{signature}"),
        None => format!("{key}|source:{source}"),
    }
}

fn local_file_signature(source: &str) -> Option<String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        return None;
    }

    let metadata = fs::metadata(source).ok()?;
    if !metadata.is_file() {
        return None;
    }

    let modified = metadata
        .modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok());
    let (secs, nanos) = modified
        .map(|duration| (duration.as_secs(), duration.subsec_nanos()))
        .unwrap_or((0, 0));
    Some(format!("mtime:{secs}:{nanos}|len:{}", metadata.len()))
}

fn resize_image(
    bytes: &[u8],
    max_w: u32,
    max_h: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let img = image::load_from_memory(bytes)?;
    let (w, h) = (img.width(), img.height());
    let (nw, nh) = if w > max_w || h > max_h {
        let ratio = (max_w as f64 / w as f64).min(max_h as f64 / h as f64);
        ((w as f64 * ratio) as u32, (h as f64 * ratio) as u32)
    } else {
        (w, h)
    };
    let thumb = img.thumbnail(nw, nh);
    let mut buf = std::io::Cursor::new(Vec::new());
    thumb.write_to(&mut buf, image::ImageFormat::Jpeg)?;
    Ok(buf.into_inner())
}

#[cfg(test)]
fn guess_ext(source: &str) -> &'static str {
    let path = Path::new(source);
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "jpg",
        "webp" => "webp",
        "gif" => "gif",
        _ => "png",
    }
}

fn disk_stem(cache_identity: &str) -> String {
    format!(
        "{}_{:016x}",
        disk_stem_prefix(cache_identity),
        stable_hash(cache_identity)
    )
}

fn disk_stem_prefix(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(48)
        .collect()
}

fn stable_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_stem_prefix() {
        assert_eq!(disk_stem_prefix("Hello World!"), "Hello_World");
        assert_eq!(
            disk_stem_prefix("https://example.com/img.jpg"),
            "https___example_com_img_jpg"
        );
    }

    #[test]
    fn test_guess_ext() {
        assert_eq!(guess_ext("cover.jpg"), "jpg");
        assert_eq!(guess_ext("screenshot.PNG"), "png");
        assert_eq!(guess_ext("https://x.com/img.webp"), "webp");
        let file_name = disk_path_for("https://x.com/img.webp|source:https://x.com/img.webp")
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        assert!(
            file_name.starts_with("https___x_com_img_webp_source_https___x_com_"),
            "{file_name}"
        );
        assert!(file_name.ends_with(".jpg"));
    }

    #[test]
    fn local_file_identity_changes_when_source_changes() {
        let path = std::env::temp_dir().join(format!("moeplay_thumb_{}.png", uuid::Uuid::new_v4()));
        fs::write(&path, [1_u8, 2, 3]).unwrap();
        let first = cache_identity("cover:test", &path.to_string_lossy());

        std::thread::sleep(std::time::Duration::from_millis(2));
        fs::write(&path, [1_u8, 2, 3, 4, 5]).unwrap();
        let second = cache_identity("cover:test", &path.to_string_lossy());
        fs::remove_file(&path).ok();

        assert_ne!(first, second);
        assert!(first.contains("mtime:"));
        assert!(second.contains("len:5"));
    }

    #[test]
    fn test_resize_small_image() {
        // 使用 `image` crate 创建一个 2x2 的测试图片
        let mut img = image::RgbImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        img.put_pixel(1, 0, image::Rgb([0, 255, 0]));
        img.put_pixel(0, 1, image::Rgb([0, 0, 255]));
        img.put_pixel(1, 1, image::Rgb([255, 255, 0]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let png_bytes = buf.into_inner();

        let result = resize_image(&png_bytes, 400, 300);
        assert!(result.is_ok(), "should resize valid png without error");
        let jpeg = result.unwrap();
        assert!(jpeg.len() > 0, "should produce non-empty jpeg");
    }
}
