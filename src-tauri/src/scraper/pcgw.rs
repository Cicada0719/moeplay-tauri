//! PCGamingWiki 刮削客户端
//!
//! 使用公开 MediaWiki API 获取低优先级技术资料、简介和页面链接。

use serde_json::Value;

use super::error::ScrapeError;
use super::utils;
use crate::models::{ScrapeDetail, ScrapeResult};

const SEARCH_URL: &str = "https://www.pcgamingwiki.com/w/api.php?action=query&list=search&format=json&srlimit=5&srsearch={query}";
const SUMMARY_URL: &str = "https://www.pcgamingwiki.com/w/api.php?action=query&format=json&prop=extracts|pageimages&exintro=1&explaintext=1&pithumbsize=600&titles={title}";

/// 搜索 PCGamingWiki 页面并返回摘要。
pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    let page_title = search_title(query).await?.ok_or(ScrapeError::NotFound)?;
    let result = get_summary(&page_title).await?;
    Ok(vec![result])
}

/// 按 PCGamingWiki 页面标题获取摘要。
pub async fn get_summary(title: &str) -> Result<ScrapeResult, ScrapeError> {
    if title.trim().is_empty() {
        return Err(ScrapeError::Config("PCGamingWiki 标题不能为空".into()));
    }

    let url = SUMMARY_URL.replace("{title}", &urlencoding::encode(title));
    let text = utils::fetch_text_with_retry(&url).await?;
    let json: Value = serde_json::from_str(&text).map_err(|e| ScrapeError::Parse(e.to_string()))?;
    let pages = json
        .get("query")
        .and_then(|v| v.get("pages"))
        .and_then(|v| v.as_object())
        .ok_or_else(|| ScrapeError::Parse("PCGamingWiki 响应缺少 pages".into()))?;

    for page in pages.values() {
        if page.get("missing").is_some() {
            continue;
        }

        let page_title = page
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(title)
            .to_string();
        let page_url = format!(
            "https://www.pcgamingwiki.com/wiki/{}",
            urlencoding::encode(&page_title).replace("%20", "_")
        );

        let mut detail = ScrapeDetail {
            homepage: Some(page_url),
            ..ScrapeDetail::default()
        };
        detail.aliases.push(page_title.clone());

        return Ok(ScrapeResult {
            title: page_title,
            description: page
                .get("extract")
                .and_then(|v| v.as_str())
                .map(|s| utils::truncate(s, 800)),
            cover: page
                .get("thumbnail")
                .and_then(|v| v.get("source"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            background: None,
            tags: vec!["PCGamingWiki".to_string(), "技术资料".to_string()],
            rating: None,
            release_year: None,
            source: "pcgw".to_string(),
            source_id: title.to_string(),
            detail: Some(detail),
        });
    }

    Err(ScrapeError::NotFound)
}

/// 简易搜索接口（返回 `Result<Vec<ScrapeResult>, String>`）。
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|e| e.to_string())
}

async fn search_title(query: &str) -> Result<Option<String>, ScrapeError> {
    let url = SEARCH_URL.replace("{query}", &urlencoding::encode(query));
    let text = utils::fetch_text_with_retry(&url).await?;
    let json: Value = serde_json::from_str(&text).map_err(|e| ScrapeError::Parse(e.to_string()))?;
    let items = json
        .get("query")
        .and_then(|v| v.get("search"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| ScrapeError::Parse("PCGamingWiki 响应缺少 search".into()))?;

    let mut best_title = None;
    let mut best_score = 0.0;

    for item in items {
        let Some(title) = item.get("title").and_then(|v| v.as_str()) else {
            continue;
        };
        let score = utils::confidence(query, title);
        if score > best_score {
            best_score = score;
            best_title = Some(title.to_string());
        }
    }

    if best_score >= 0.55 {
        Ok(best_title)
    } else {
        Ok(None)
    }
}
