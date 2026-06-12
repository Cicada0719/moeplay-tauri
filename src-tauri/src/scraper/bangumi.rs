//! Bangumi 刮削客户端
//!
//! 使用 Bangumi API (https://api.bgm.tv/) 搜索游戏元数据。
//! type=4 表示游戏类别。

use reqwest::Client;
use serde::Deserialize;
use tokio::time::{sleep, Duration};

use super::error::ScrapeError;
use crate::models::ScrapeResult;

/// Bangumi 搜索 API 地址
const BANGUMI_SEARCH_API: &str = "https://api.bgm.tv/search/subject";

/// HTTP 请求超时
const REQUEST_TIMEOUT_SECS: u64 = 15;

// ========== 响应类型 ==========

#[derive(Debug, Deserialize)]
struct BangumiResponse {
    list: Vec<BangumiResult>,
}

#[derive(Debug, Deserialize)]
struct BangumiResult {
    id: u64,
    name: String,
    summary: Option<String>,
    images: Option<BangumiImages>,
    tags: Option<Vec<BangumiTag>>,
    rating: Option<BangumiRating>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BangumiImages {
    large: Option<String>,
    medium: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BangumiTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct BangumiRating {
    score: Option<f64>,
}

// ========== 公共 API ==========

/// 搜索 Bangumi 游戏（type=4）
pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| ScrapeError::Network(e.to_string()))?;

    let url = format!(
        "{}/{}?type=4&limit=10",
        BANGUMI_SEARCH_API,
        urlencoding::encode(query)
    );

    // Bangumi API 有速率限制，简单延迟
    sleep(Duration::from_millis(200)).await;

    let resp = client
        .get(&url)
        .header("User-Agent", "MoeGame/1.0")
        .send()
        .await
        .map_err(|e| ScrapeError::Network(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ScrapeError::Api {
            status: status.as_u16(),
            body,
        });
    }

    let bgm_resp: BangumiResponse = resp
        .json()
        .await
        .map_err(|e| ScrapeError::Parse(e.to_string()))?;

    let results: Vec<ScrapeResult> = bgm_resp
        .list
        .into_iter()
        .map(|r| {
            let release_year = r
                .date
                .as_ref()
                .and_then(|d| d.get(0..4).and_then(|y| y.parse::<u32>().ok()));

            ScrapeResult {
                title: r.name,
                description: r.summary,
                cover: r
                    .images
                    .as_ref()
                    .and_then(|i| i.large.clone().or_else(|| i.medium.clone())),
                background: None,
                tags: r
                    .tags
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| t.name)
                    .collect(),
                rating: r.rating.and_then(|rat| rat.score),
                release_year,
                source: "bangumi".to_string(),
                source_id: r.id.to_string(),
                detail: None,
            }
        })
        .collect();

    Ok(results)
}

/// 简易搜索接口（兼容旧代码，返回 `Result<Vec<ScrapeResult>, String>`）
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|e| e.to_string())
}

/// Bangumi 详情查询（v0 API）。
/// 返回更丰富的元数据：中文名、详情图、标签分类、开发商等。
pub async fn detail(subject_id: &str) -> Result<ScrapeResult, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://api.bgm.tv/v0/subjects/{}", subject_id);
    sleep(Duration::from_millis(200)).await;

    let resp = client
        .get(&url)
        .header("User-Agent", "MoeGame/1.0")
        .send()
        .await
        .map_err(|e| format!("Bangumi detail request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Bangumi detail HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let title = json
        .get("name_cn")
        .or(json.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();
    let summary = json
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let cover = json
        .get("images")
        .and_then(|v| v.get("large").or(v.get("common")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let rating = json
        .get("rating")
        .and_then(|v| v.get("score"))
        .and_then(|v| v.as_f64());
    let date = json
        .get("date")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let release_year = date.as_ref().and_then(|d| d[..4].parse::<u32>().ok());

    // 标签
    let tags: Vec<String> = json
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    t.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    // 别名
    let aliases: Vec<String> = json
        .get("infobox")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let key = item.get("key").and_then(|k| k.as_str()).unwrap_or("");
                    if key == "别名" || key == "其他译名" {
                        item.get("value")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let developer = json
        .get("infobox")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter().find_map(|item| {
                item.get("key")
                    .and_then(|k| k.as_str())
                    .filter(|&k| k == "开发")
                    .and_then(|_| {
                        item.get("value")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
            })
        });

    Ok(ScrapeResult {
        title,
        description: summary,
        cover,
        background: None,
        tags,
        rating,
        release_year,
        source: "bangumi".into(),
        source_id: subject_id.to_string(),
        detail: Some(crate::models::ScrapeDetail {
            developer,
            publisher: None,
            aliases,
            genres: vec![],
            homepage: Some(format!("https://bgm.tv/subject/{}", subject_id)),
            screenshots: vec![],
            languages: vec![],
            engine: None,
            age_rating: None,
            series: None,
            release_date: date,
            voice_languages: vec![],
        }),
    })
}
