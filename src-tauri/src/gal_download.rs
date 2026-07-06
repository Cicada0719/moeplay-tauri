// 萌游 MoeGame · Galgame 下载资源接入（M6）
//
// 从 TouchGAL / Kungal 等社区抓取真实下载链接：
//   1. 搜索游戏 → 获取 galgameId
//   2. 获取详情 → 提取 patch.uniqueId
//   3. 抓取 patch 页面 → 解析 HTML 中的磁力/HTTP/网盘链接
//   4. 返回结构化 DownloadEntry 供前端展示 + 接入下载管理器

use serde::{Deserialize, Serialize};

// ============================================================================
// 数据模型
// ============================================================================

/// 单条下载资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadEntry {
    pub label: String,
    pub url: String,
    #[serde(rename = "type")]
    pub kind: DownloadKind,
    pub size: Option<String>,
    pub note: Option<String>,
    /// 是否可通过下载管理器直接下载（HTTP/HTTPS）
    pub direct_download: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadKind {
    Magnet,
    Http,
    BaiduPan,
    OneDrive,
    GoogleDrive,
    Patch,
    TranslationPatch,
    OfficialSite,
    Other,
}

/// 下载搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSearchRequest {
    /// 游戏名称
    pub name: String,
    /// Kungal 已知 galgameId（如果之前刮削到）
    pub kungal_id: Option<String>,
    /// TouchGAL 已知 patch uniqueId
    pub patch_id: Option<String>,
}

/// 下载搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSearchResult {
    pub game_name: String,
    pub entries: Vec<DownloadEntry>,
    pub source: String,
    pub source_url: Option<String>,
}

// ============================================================================
// 主入口
// ============================================================================

/// 搜索 galgame 下载资源。
/// 优先使用已有的 patch_id / kungal_id，否则按名称搜索 Kungal。
pub async fn search_downloads(req: &DownloadSearchRequest) -> Result<DownloadSearchResult, String> {
    let client = crate::http_client::build_reqwest_client(
        20,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36 MoeGame/0.1",
    );

    let mut patch_id: Option<String> = req.patch_id.clone();

    // Step 1: 如果没有 patch_id，通过 Kungal 搜索获取
    if patch_id.is_none() {
        patch_id = find_patch_id(&client, &req.name, req.kungal_id.as_deref()).await;
    }

    // Step 2: 抓取 patch 页面提取下载链接
    let entries = if let Some(ref pid) = patch_id {
        scrape_patch_page(&client, pid).await.unwrap_or_else(|e| {
            tracing::warn!(patch_id = %pid, error = %e, "Failed to scrape patch page");
            vec![]
        })
    } else {
        vec![]
    };

    let source_url = patch_id
        .as_ref()
        .map(|id| format!("https://www.touchgal.io/patch/{}", id));

    Ok(DownloadSearchResult {
        game_name: req.name.clone(),
        entries,
        source: "touchgal".into(),
        source_url,
    })
}

/// 直搜路径：不依赖 Kungal API，直接用候选名在 touchgal.io 搜索下载页。
/// `candidates` 按优先级排列（如：原版名 > 英文名 > 日文名 > 中文名）。
/// 逐个尝试直到找到结果。
/// 流程：touchgal.io 搜索 → 取第一个 patch → 抓取下载链接。
pub async fn search_downloads_direct(
    candidates: &[String],
) -> Result<DownloadSearchResult, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36 MoeGame/0.1")
        .danger_accept_invalid_certs(crate::http_client::insecure_tls_enabled())
        .build()
        .map_err(|e| e.to_string())?;

    let mut last_error = String::new();

    for name in candidates {
        if name.trim().is_empty() {
            continue;
        }
        let name = name.trim();

        // Step 1: 直接在 touchgal.io 搜索
        let search_url = format!(
            "https://www.touchgal.io/search?keyword={}",
            urlencoding::encode(name)
        );
        let resp = match client.get(&search_url).send().await {
            Ok(r) => r,
            Err(e) => {
                last_error = e.to_string();
                continue;
            }
        };

        let html = match resp.text().await {
            Ok(h) => h,
            Err(e) => {
                last_error = e.to_string();
                continue;
            }
        };

        // 从搜索结果页提取第一个 patch 链接 (/patch/XXXX)
        let patch_id = extract_first_patch_id(&html);
        tracing::info!(name, ?patch_id, "TouchGAL direct search result");

        // Step 2: 抓取 patch 页面
        if let Some(ref pid) = patch_id {
            let entries = scrape_patch_page(&client, pid).await.unwrap_or_else(|e| {
                tracing::warn!(patch_id = %pid, error = %e, "Failed to scrape patch page");
                vec![]
            });
            let source_url = Some(format!("https://www.touchgal.io/patch/{}", pid));

            return Ok(DownloadSearchResult {
                game_name: name.to_string(),
                entries,
                source: "touchgal".into(),
                source_url,
            });
        }
    }

    // 所有候选名都失败了
    Err(format!(
        "TouchGAL 搜索无结果 (尝试了 {} 个名称): {}",
        candidates.len(),
        last_error
    ))
}

/// 从 HTML 搜索结果中提取第一个 patch ID
fn extract_first_patch_id(html: &str) -> Option<String> {
    // 匹配 href="/patch/XXXX" 或 href="https://www.touchgal.io/patch/XXXX"
    let re =
        regex::Regex::new(r#"href=["'](?:https?://www\.touchgal\.io)?/patch/([A-Za-z0-9]+)["']"#)
            .ok()?;
    re.captures(html)?.get(1).map(|m| m.as_str().to_string())
}

// ============================================================================
// Step 1: 找到 patch_id
// ============================================================================

async fn find_patch_id(
    client: &reqwest::Client,
    name: &str,
    kungal_id: Option<&str>,
) -> Option<String> {
    let galgame_id = if let Some(id) = kungal_id {
        id.to_string()
    } else {
        search_kungal_for_game(client, name).await?
    };

    // 调用 Kungal detail API
    let detail_url = format!(
        "https://www.kungal.com/api/galgame/detail?galgameId={}",
        galgame_id
    );
    let resp = client
        .get(&detail_url)
        .header("User-Agent", "MoeGame/0.1")
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = resp.json().await.ok()?;

    // 提取 patch.uniqueId
    json.get("patch")
        .and_then(|p| p.get("uniqueId"))
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .map(|id| id.to_string())
        .or_else(|| {
            // 备选：直接找 resources 中的链接
            json.get("resources")
                .and_then(|r| r.as_array())
                .and_then(|arr| {
                    arr.iter().find_map(|item| {
                        let url = item
                            .get("url")
                            .or(item.get("link"))
                            .and_then(|v| v.as_str())?;
                        if url.contains("touchgal.io/patch/") {
                            url.split('/').next_back().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
        })
}

/// 按名称搜索 Kungal，返回第一个匹配的 galgameId。
async fn search_kungal_for_game(client: &reqwest::Client, name: &str) -> Option<String> {
    let url = format!(
        "https://www.kungal.com/api/search?keywords={}&type=galgame&page=1&limit=5",
        urlencoding::encode(name)
    );
    let resp = client.get(&url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    let arr = json.as_array()?;

    // 最佳匹配
    let best = arr.iter().max_by(|a, b| {
        let sa = match_score(a, name);
        let sb = match_score(b, name);
        sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
    })?;

    best.get("id")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .map(|id| id.to_string())
}

fn match_score(item: &serde_json::Value, query: &str) -> f64 {
    let q = query.trim().to_lowercase();
    let names = item.get("name");
    if let Some(name_obj) = names.and_then(|n| n.as_object()) {
        for (_, v) in name_obj {
            if let Some(s) = v.as_str() {
                let s = s.trim().to_lowercase();
                if s == q {
                    return 1.0;
                }
                if s.contains(&q) || q.contains(&s) {
                    return 0.85;
                }
            }
        }
    }
    if let Some(s) = names.and_then(|v| v.as_str()) {
        let s = s.trim().to_lowercase();
        if s == q {
            return 0.9;
        }
        if s.contains(&q) || q.contains(&s) {
            return 0.75;
        }
    }
    0.0
}

// ============================================================================
// Step 2: 抓取 patch 页面获取下载链接
// ============================================================================

async fn scrape_patch_page(
    client: &reqwest::Client,
    patch_id: &str,
) -> Result<Vec<DownloadEntry>, String> {
    let url = format!("https://www.touchgal.io/patch/{}", patch_id);
    let resp = client
        .get(&url)
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|e| format!("请求 patch 页面失败: {}", e))?;

    let html = resp
        .text()
        .await
        .map_err(|e| format!("读取 HTML 失败: {}", e))?;

    let mut entries = Vec::new();

    // 1. 提取 magnet: 链接（magnet:?xt=urn:btih:...）
    let magnet_re = regex::Regex::new(r#"magnet:\?xt=urn:btih:[A-Za-z0-9]+"#).ok();
    if let Some(re) = magnet_re {
        for m in re.find_iter(&html) {
            let url = m.as_str().trim_matches('"').to_string();
            if !entries.iter().any(|e: &DownloadEntry| e.url == url) {
                entries.push(DownloadEntry {
                    label: "磁力链接 (BT)".into(),
                    url,
                    kind: DownloadKind::Magnet,
                    size: None,
                    note: None,
                    direct_download: false,
                });
            }
        }
    }

    // 2. 提取百度网盘链接 (pan.baidu.com)
    let pan_re = regex::Regex::new(r#"https?://pan\.baidu\.com/s/[^\s"'<>]+"#).ok();
    if let Some(re) = pan_re {
        for m in re.find_iter(&html) {
            let url = m.as_str().trim_matches('"').to_string();
            let label = extract_pan_label(&html, &url);
            let note = extract_pan_code(&html, &url);
            if !entries.iter().any(|e: &DownloadEntry| e.url == url) {
                entries.push(DownloadEntry {
                    label: label.unwrap_or_else(|| "百度网盘".into()),
                    url,
                    kind: DownloadKind::BaiduPan,
                    size: None,
                    note,
                    direct_download: false,
                });
            }
        }
    }

    // 3. 提取 OneDrive / Google Drive
    for (domain_re, kind) in &[
        (
            r#"https?://onedrive\.live\.com/[^\s"'<>]+"#,
            DownloadKind::OneDrive,
        ),
        (
            r#"https?://drive\.google\.com/[^\s"'<>]+"#,
            DownloadKind::GoogleDrive,
        ),
    ] {
        if let Ok(re) = regex::Regex::new(domain_re) {
            for m in re.find_iter(&html) {
                let url = m.as_str().trim_matches('"').to_string();
                if !entries.iter().any(|e: &DownloadEntry| e.url == url) {
                    entries.push(DownloadEntry {
                        label: format!("{:?}", kind),
                        url,
                        kind: kind.clone(),
                        size: None,
                        note: None,
                        direct_download: false,
                    });
                }
            }
        }
    }

    // 4. 提取直接 HTTP 下载链接（常见文件托管）
    let http_re = regex::Regex::new(r#"https?://[^\s"'<>]+\.(?:7z|zip|rar|exe|iso)"#).ok();
    if let Some(re) = http_re {
        for m in re.find_iter(&html) {
            let raw = m.as_str().trim_matches('"').to_string();
            if raw.starts_with("http")
                && !raw.contains("pan.baidu")
                && !raw.contains("drive.google")
                && !raw.contains("onedrive")
                && !entries.iter().any(|e: &DownloadEntry| e.url == raw)
            {
                entries.push(DownloadEntry {
                    label: "HTTP 直链下载".into(),
                    url: raw.clone(),
                    kind: DownloadKind::Http,
                    size: None,
                    note: None,
                    direct_download: true,
                });
            }
        }
    }

    // 5. 如果没有任何链接，尝试抓取 HTML 中所有链接并分类
    if entries.is_empty() {
        entries = fallback_extract_links(&html);
    }

    Ok(entries)
}

/// 回退方案：提取 HTML 中所有 `<a href=...>` 链接并分类。
fn fallback_extract_links(html: &str) -> Vec<DownloadEntry> {
    let link_re = regex::Regex::new(r#"<a[^>]+href=["']([^"']+)["'][^>]*>(.*?)</a>"#).ok();
    let mut entries = Vec::new();
    if let Some(re) = link_re {
        for cap in re.captures_iter(html) {
            let url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let text = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            // 跳过不是下载的链接
            if url.starts_with("#") || url.starts_with("/") || url.is_empty() {
                continue;
            }
            if url.contains("magnet:") {
                entries.push(DownloadEntry {
                    label: text.to_string(),
                    url: url.to_string(),
                    kind: DownloadKind::Magnet,
                    size: None,
                    note: None,
                    direct_download: false,
                });
            } else if url.contains("pan.baidu.com") {
                entries.push(DownloadEntry {
                    label: text.to_string(),
                    url: url.to_string(),
                    kind: DownloadKind::BaiduPan,
                    size: None,
                    note: None,
                    direct_download: false,
                });
            } else if url.contains("drive.google.com") || url.contains("onedrive.live.com") {
                entries.push(DownloadEntry {
                    label: text.to_string(),
                    url: url.to_string(),
                    kind: if url.contains("google") {
                        DownloadKind::GoogleDrive
                    } else {
                        DownloadKind::OneDrive
                    },
                    size: None,
                    note: None,
                    direct_download: false,
                });
            } else if url.starts_with("http") {
                entries.push(DownloadEntry {
                    label: text.to_string(),
                    url: url.to_string(),
                    kind: DownloadKind::Http,
                    size: None,
                    note: None,
                    direct_download: true,
                });
            }
        }
    }
    entries
}

fn extract_pan_label(html: &str, pan_url: &str) -> Option<String> {
    // 百度网盘链接附近通常有提取码或说明文字
    let idx = html.find(pan_url)?;
    let snippet = &html[idx.saturating_sub(200)..(idx + pan_url.len() + 100).min(html.len())];
    // 尝试取上一行的纯文本
    snippet.lines().rev().find_map(|line| {
        let cleaned = line.trim().replace(['<', '>'], "");
        if cleaned.len() > 2 && cleaned.len() < 40 && !cleaned.contains("href") {
            Some(cleaned.to_string())
        } else {
            None
        }
    })
}

fn extract_pan_code(html: &str, pan_url: &str) -> Option<String> {
    // 提取码通常在"提取码"或"访问码"后面的 4-6 位字母数字
    let idx = html.find(pan_url)?;
    let snippet = &html[idx.saturating_sub(300)..(idx + pan_url.len() + 200).min(html.len())];
    let code_re =
        regex::Regex::new(r#"(?:提取码|访问码|密码|pass(?:word)?|code)[:：\s]*([A-Za-z0-9]{4,6})"#)
            .ok()?;
    code_re
        .captures(snippet)
        .and_then(|c| c.get(1))
        .map(|m| format!("提取码: {}", m.as_str()))
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_extract_magnet() {
        let html = r#"<a href="magnet:?xt=urn:btih:ABCDEF1234567890">游戏下载</a>"#;
        let entries = fallback_extract_links(html);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, DownloadKind::Magnet);
        assert_eq!(entries[0].url, "magnet:?xt=urn:btih:ABCDEF1234567890");
    }

    #[test]
    fn test_fallback_extract_baidu() {
        let html = r#"<a href="https://pan.baidu.com/s/1abc123">百度网盘</a>"#;
        let entries = fallback_extract_links(html);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, DownloadKind::BaiduPan);
    }

    #[test]
    fn test_fallback_extract_http() {
        let html = r#"<a href="https://example.com/game.zip">HTTP下载</a>"#;
        let entries = fallback_extract_links(html);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, DownloadKind::Http);
        assert!(entries[0].direct_download);
    }

    #[test]
    fn test_fallback_skips_relative() {
        let html = "<a href=\"/about\">关于</a><a href=\"#top\">顶部</a>";
        let entries = fallback_extract_links(html);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_magnet_regex() {
        let html = r#"<a href="magnet:?xt=urn:btih:ABCDEF1234567890ABCDEF1234567890ABCDEF12&dn=game">下载</a>"#;
        let re = regex::Regex::new(r#"magnet:\?xt=urn:btih:[A-Za-z0-9]+"#).unwrap();
        let found: Vec<_> = re.find_iter(html).map(|m| m.as_str()).collect();
        assert!(!found.is_empty());
    }
}
