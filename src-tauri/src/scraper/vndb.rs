//! VNDB 刮削客户端
//!
//! 使用 VNDB kana API (https://api.vndb.org/kana/) 进行视觉小说数据刮削。
//!
//! 支持功能：
//! - 按关键词搜索 VN（带分页）
//! - 按 ID 获取详细信息（截图、外链、别名、开发者等）
//! - 速率限制（~10 req/s，安全低于 VNDB 限制）
//! - 标签分类过滤（排除剧透/色情标签）

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

use super::error::ScrapeError;
use crate::models::ScrapeResult;

// ========== 常量 ==========

/// VNDB kana API 地址
const VNDB_API: &str = "https://api.vndb.org/kana/vn";

/// 请求间隔 (ms)，约 10 req/s
const RATE_LIMIT_MS: u64 = 100;

/// HTTP 请求超时
const REQUEST_TIMEOUT_SECS: u64 = 15;

/// 搜索请求默认返回数量
const DEFAULT_PER_PAGE: u32 = 10;

/// 搜索时请求的字段（轻量，不含截图/外链）
const SEARCH_FIELDS: &str = "\
    title,description,image.url,image.sexual,\
    developers.name,released,languages,platforms,\
    length_minutes,length,tags.name,tags.category,tags.rating,tags.spoiler,\
    rating,popularity";

/// 详情时请求的字段（完整，含截图/外链/别名）
const DETAIL_FIELDS: &str = "\
    title,aliases,description,image.url,image.sexual,\
    developers.name,released,languages,platforms,\
    length_minutes,length,tags.name,tags.category,tags.rating,tags.spoiler,\
    rating,popularity,\
    screens.url,screens.nsfw,\
    extlinks.url,extlinks.label";

// ========== API 请求/响应类型 ==========

#[derive(Debug, Serialize)]
struct VndbQuery {
    filters: Vec<serde_json::Value>,
    fields: String,
    results: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<u32>,
    sort: String,
}

#[derive(Debug, Deserialize)]
struct VndbResponse {
    more: bool,
    results: Vec<VndbVn>,
}

#[derive(Debug, Deserialize)]
struct VndbVn {
    id: String,
    title: Option<String>,
    aliases: Option<String>,
    description: Option<String>,
    image: Option<VndbImage>,
    developers: Option<Vec<VndbDeveloper>>,
    released: Option<String>,
    languages: Option<Vec<String>>,
    platforms: Option<Vec<String>>,
    length_minutes: Option<u32>,
    length: Option<u32>,
    tags: Option<Vec<VndbTag>>,
    rating: Option<f64>,
    popularity: Option<f64>,
    screens: Option<Vec<VndbScreen>>,
    extlinks: Option<Vec<VndbExtLink>>,
}

#[derive(Debug, Deserialize)]
struct VndbImage {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VndbDeveloper {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VndbTag {
    name: Option<String>,
    category: Option<String>,
    rating: Option<f64>,
    spoiler: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct VndbScreen {
    url: Option<String>,
    nsfw: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct VndbExtLink {
    url: Option<String>,
    label: Option<String>,
}

// ========== 丰富详情类型 ==========

/// VNDB 详细信息（比 ScrapeResult 更丰富的中间类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VndbDetail {
    pub id: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub developers: Vec<String>,
    pub released: Option<String>,
    pub release_year: Option<u32>,
    pub languages: Vec<String>,
    pub platforms: Vec<String>,
    pub length_minutes: Option<u32>,
    pub length_category: Option<u32>,
    pub tags: Vec<VndbTagInfo>,
    pub rating: Option<f64>,
    pub popularity: Option<f64>,
    pub screenshots: Vec<String>,
    pub links: Vec<VndbLink>,
}

/// VNDB 标签（含分类和权重）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VndbTagInfo {
    pub name: String,
    /// 分类: "cont"(内容), "ero"(色情), "tech"(技术)
    pub category: String,
    /// 权重 0.0-1.0
    pub rating: f64,
}

/// VNDB 外部链接
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VndbLink {
    pub label: String,
    pub url: String,
}

// ========== 速率限制 ==========

async fn rate_limit() {
    sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
}

/// 创建带超时的 HTTP 客户端
fn build_client() -> Result<Client, ScrapeError> {
    Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| ScrapeError::Network(e.to_string()))
}

// ========== 公共 API ==========

/// 搜索 VNDB 视觉小说
///
/// 返回 `(结果列表, 是否有更多页)` 的元组，支持前端分页加载。
pub async fn search(
    query: &str,
    page: u32,
    per_page: u32,
) -> Result<(Vec<ScrapeResult>, bool), ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    rate_limit().await;

    let client = build_client()?;
    let body = VndbQuery {
        filters: vec![
            serde_json::json!("search"),
            serde_json::json!("="),
            serde_json::json!(query),
        ],
        fields: SEARCH_FIELDS.to_string(),
        results: per_page,
        page: Some(page),
        sort: "searchrank".to_string(),
    };

    let resp = client
        .post(VNDB_API)
        .json(&body)
        .send()
        .await
        .map_err(|e| ScrapeError::Network(e.to_string()))?;

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(ScrapeError::RateLimited);
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ScrapeError::Api {
            status: status.as_u16(),
            body,
        });
    }

    let text = resp
        .text()
        .await
        .map_err(|e| ScrapeError::Network(e.to_string()))?;

    let vndb_resp: VndbResponse = serde_json::from_str(&text)
        .map_err(|e| ScrapeError::Parse(format!("{} | 原文: {}", e, truncate(&text, 200))))?;

    let results: Vec<ScrapeResult> = vndb_resp
        .results
        .iter()
        .filter_map(parse_vn_to_scrape_result)
        .collect();

    Ok((results, vndb_resp.more))
}

/// 按 ID 获取 VNDB 详细信息
///
/// 返回比搜索更丰富的元数据：截图、外链、别名、开发者详情等。
pub async fn detail(id: &str) -> Result<VndbDetail, ScrapeError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(ScrapeError::Config("VNDB ID 不能为空".into()));
    }

    rate_limit().await;

    let client = build_client()?;
    let body = VndbQuery {
        filters: vec![
            serde_json::json!("id"),
            serde_json::json!("="),
            serde_json::json!(id),
        ],
        fields: DETAIL_FIELDS.to_string(),
        results: 1,
        page: None,
        sort: "id".to_string(),
    };

    let resp = client
        .post(VNDB_API)
        .json(&body)
        .send()
        .await
        .map_err(|e| ScrapeError::Network(e.to_string()))?;

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(ScrapeError::RateLimited);
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ScrapeError::Api {
            status: status.as_u16(),
            body,
        });
    }

    let text = resp
        .text()
        .await
        .map_err(|e| ScrapeError::Network(e.to_string()))?;

    let vndb_resp: VndbResponse = serde_json::from_str(&text)
        .map_err(|e| ScrapeError::Parse(format!("{} | 原文: {}", e, truncate(&text, 200))))?;

    vndb_resp
        .results
        .first()
        .and_then(parse_vn_to_detail)
        .ok_or(ScrapeError::NotFound)
}

/// 简易搜索接口（兼容旧代码，返回 `Result<Vec<ScrapeResult>, String>`）
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    let (results, _) = search(query, 1, DEFAULT_PER_PAGE)
        .await
        .map_err(|e| e.to_string())?;
    Ok(results)
}

// ========== 内部解析 ==========

/// 解析 VN 为通用刮削结果
fn parse_vn_to_scrape_result(vn: &VndbVn) -> Option<ScrapeResult> {
    let title = vn.title.clone()?;
    let release_year = vn.released.as_deref().and_then(extract_year);
    let cover = vn.image.as_ref().and_then(|i| i.url.clone());
    let tags = extract_tags(vn, 0, false);
    let rating = vn.rating.map(|v| (v * 10.0).round() / 10.0);

    Some(ScrapeResult {
        title,
        description: vn.description.clone(),
        cover,
        background: None,
        tags,
        rating,
        release_year,
        source: "vndb".to_string(),
        source_id: vn.id.clone(),
        detail: None,
    })
}

/// 解析 VN 为详细信息
fn parse_vn_to_detail(vn: &VndbVn) -> Option<VndbDetail> {
    let title = vn.title.clone()?;
    let release_year = vn.released.as_deref().and_then(extract_year);

    let aliases = vn
        .aliases
        .as_deref()
        .map(|s| {
            s.split('\n')
                .map(|a| a.trim().to_string())
                .filter(|a| !a.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let cover_url = vn.image.as_ref().and_then(|i| i.url.clone());

    let developers = vn
        .developers
        .as_ref()
        .map(|devs| devs.iter().filter_map(|d| d.name.clone()).collect())
        .unwrap_or_default();

    let tags: Vec<VndbTagInfo> = vn
        .tags
        .as_ref()
        .map(|tags| {
            tags.iter()
                .filter(|t| t.spoiler.unwrap_or(0) == 0)
                .filter(|t| t.category.as_deref() != Some("ero"))
                .filter_map(|t| {
                    Some(VndbTagInfo {
                        name: t.name.clone()?,
                        category: t.category.clone().unwrap_or_default(),
                        rating: t.rating.unwrap_or(0.0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let screenshots = vn
        .screens
        .as_ref()
        .map(|screens| {
            screens
                .iter()
                .filter(|s| !s.nsfw.unwrap_or(false))
                .filter_map(|s| s.url.clone())
                .collect()
        })
        .unwrap_or_default();

    let links = vn
        .extlinks
        .as_ref()
        .map(|links| {
            links
                .iter()
                .filter_map(|l| {
                    Some(VndbLink {
                        label: l.label.clone().unwrap_or_else(|| "Link".to_string()),
                        url: l.url.clone()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(VndbDetail {
        id: vn.id.clone(),
        title,
        aliases,
        description: vn.description.clone(),
        cover_url,
        developers,
        released: vn.released.clone(),
        release_year,
        languages: vn.languages.clone().unwrap_or_default(),
        platforms: vn.platforms.clone().unwrap_or_default(),
        length_minutes: vn.length_minutes,
        length_category: vn.length,
        tags,
        rating: vn.rating,
        popularity: vn.popularity,
        screenshots,
        links,
    })
}

/// 从日期字符串提取年份（如 "2004-04-28" → 2004）
fn extract_year(date: &str) -> Option<u32> {
    date.get(0..4)?.parse::<u32>().ok()
}

/// 提取标签名称列表，按剧透级别和分类过滤
fn extract_tags(vn: &VndbVn, max_spoiler: u8, include_ero: bool) -> Vec<String> {
    vn.tags
        .as_ref()
        .map(|tags| {
            tags.iter()
                .filter(|t| t.spoiler.unwrap_or(0) <= max_spoiler)
                .filter(|t| include_ero || t.category.as_deref() != Some("ero"))
                .filter_map(|t| t.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// 截断字符串，用于错误消息
fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}
