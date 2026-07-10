// 萌游 MoeGame · 截图/立绘下载器（M3）
//
// 从 VNDB/Steam 等源获取截图 URL，下载到本地缓存并返回路径。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 下载结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadedImage {
    pub url: String,
    pub local_path: String,
    pub size: u64,
    pub kind: ImageKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
    Screenshot,
    Cg,
    Cover,
    Background,
    CharacterStanding, // 立绘
}

/// 批量下载截图/立绘到本地缓存。
/// `urls` 为远程 URL 列表，`game_id` 用于命名。
/// 并发下载（最多 4 并发），失败跳过不阻断。
pub async fn download_images(
    urls: &[String],
    game_id: &str,
    kind: ImageKind,
) -> Vec<DownloadedImage> {
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(4));
    let dir = screenshots_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!(error = %e, "Failed to create screenshots dir");
        return vec![];
    }

    let mut handles = vec![];

    for (i, url) in urls.iter().enumerate() {
        let url = url.clone();
        let gid = game_id.to_string();
        let dir = dir.clone();
        let k = kind.clone();
        let sem = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            download_single(&url, &gid, i, &dir, k).await
        }));
    }

    let mut results = vec![];
    for h in handles {
        if let Ok(Some(img)) = h.await {
            results.push(img);
        }
    }

    results
}

async fn download_single(
    url: &str,
    game_id: &str,
    index: usize,
    dir: &std::path::Path,
    kind: ImageKind,
) -> Option<DownloadedImage> {
    let ext = guess_ext_from_url(url);
    let filename = format!("{}_{:03}.{}", sanitize_for_filename(game_id), index, ext);
    let dest = dir.join(&filename);

    // 已缓存则跳过
    if dest.exists() {
        let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        return Some(DownloadedImage {
            url: url.to_string(),
            local_path: dest.to_string_lossy().to_string(),
            size,
            kind,
        });
    }

    // 下载
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    let resp = match client
        .get(url)
        .header("User-Agent", crate::http_client::app_user_agent())
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(url, error = %e, "Download image failed");
            return None;
        }
    };

    if !resp.status().is_success() {
        return None;
    }

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(url, error = %e, "Read image bytes failed");
            return None;
        }
    };

    if let Err(e) = std::fs::write(&dest, &bytes) {
        tracing::warn!(dest = %dest.display(), error = %e, "Write image failed");
        return None;
    }

    let size = bytes.len() as u64;
    tracing::debug!(url, dest = %dest.display(), size, "Image downloaded");

    Some(DownloadedImage {
        url: url.to_string(),
        local_path: dest.to_string_lossy().to_string(),
        size,
        kind,
    })
}

fn screenshots_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("moeplay")
        .join("screenshots")
}

fn guess_ext_from_url(url: &str) -> &'static str {
    let lower = url.to_lowercase();
    if lower.contains(".jpg") || lower.contains(".jpeg") {
        return "jpg";
    }
    if lower.contains(".png") {
        return "png";
    }
    if lower.contains(".webp") {
        return "webp";
    }
    if lower.contains(".gif") {
        return "gif";
    }
    "jpg" // 默认
}

fn sanitize_for_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(60)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_ext() {
        assert_eq!(guess_ext_from_url("https://x.com/img.png"), "png");
        assert_eq!(guess_ext_from_url("https://x.com/img.jpg?w=800"), "jpg");
        assert_eq!(guess_ext_from_url("https://x.com/img.webp"), "webp");
    }

    #[test]
    fn test_sanitize_filename() {
        let s = sanitize_for_filename("Steins;Gate シュタインズ・ゲート");
        assert!(!s.contains(';'));
        assert!(!s.contains(' '));
        assert!(!s.contains('・'));
    }
}
