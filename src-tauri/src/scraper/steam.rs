//! Steam Store 刮削客户端
//!
//! 使用 Steam 公开商店端点搜索并获取基础元数据，无需 API Key。

use regex::Regex;
use serde_json::Value;

use super::error::ScrapeError;
use super::utils;
use crate::models::{ScrapeDetail, ScrapeResult};

const SEARCH_URL: &str = "https://store.steampowered.com/search/?term={query}";
const APP_DETAILS_URL: &str = "https://store.steampowered.com/api/appdetails?appids={app_id}&filters=basic,release_date,developers,publishers,genres,categories,screenshots,metacritic";

/// 搜索 Steam 商店游戏。
pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    let app_id = extract_app_id(query).or(None);
    let app_id = match app_id {
        Some(id) => id,
        None => search_app_id(query).await?.ok_or(ScrapeError::NotFound)?,
    };

    let result = get_app_details(&app_id).await?;
    Ok(vec![result])
}

/// 按 Steam App ID 获取详情。
pub async fn get_app_details(app_id: &str) -> Result<ScrapeResult, ScrapeError> {
    if app_id.trim().is_empty() {
        return Err(ScrapeError::Config("Steam App ID 不能为空".into()));
    }

    let url = APP_DETAILS_URL.replace("{app_id}", &urlencoding::encode(app_id));
    let text = utils::fetch_text_with_retry(&url).await?;
    let json: Value = serde_json::from_str(&text).map_err(|e| ScrapeError::Parse(e.to_string()))?;

    let node = json
        .get(app_id)
        .ok_or_else(|| ScrapeError::Parse("Steam appdetails 缺少应用节点".into()))?;
    if node.get("success").and_then(|v| v.as_bool()) != Some(true) {
        return Err(ScrapeError::NotFound);
    }

    let data = node
        .get("data")
        .ok_or_else(|| ScrapeError::Parse("Steam appdetails 缺少 data".into()))?;

    let title = data
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ScrapeError::Parse("Steam appdetails 缺少名称".into()))?
        .to_string();

    let release_date = data
        .get("release_date")
        .and_then(|v| v.get("date"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut tags = vec![];
    push_descriptions(data.get("genres"), &mut tags);
    push_descriptions(data.get("categories"), &mut tags);

    let mut detail = ScrapeDetail::default();
    detail.developer = first_string(data.get("developers"));
    detail.publisher = first_string(data.get("publishers"));
    detail.genres = tags.clone();
    detail.homepage = Some(format!("https://store.steampowered.com/app/{}/", app_id));
    detail.release_date.clone_from(&release_date);
    detail.screenshots = data
        .get("screenshots")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|shot| shot.get("path_full").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    Ok(ScrapeResult {
        title,
        description: data
            .get("short_description")
            .and_then(|v| v.as_str())
            .map(|s| utils::truncate(s, 800)),
        cover: data
            .get("header_image")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        background: data
            .get("background_raw")
            .or_else(|| data.get("background"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tags,
        rating: data
            .get("metacritic")
            .and_then(|v| v.get("score"))
            .and_then(|v| v.as_f64()),
        release_year: release_date.as_deref().and_then(utils::extract_year),
        source: "steam".to_string(),
        source_id: app_id.to_string(),
        detail: Some(detail),
    })
}

/// 简易搜索接口（返回 `Result<Vec<ScrapeResult>, String>`）。
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|e| e.to_string())
}

async fn search_app_id(query: &str) -> Result<Option<String>, ScrapeError> {
    let url = SEARCH_URL.replace("{query}", &urlencoding::encode(query));
    let html = utils::fetch_text_with_retry(&url).await?;
    let patterns = [
        r#"data-ds-appid="(?P<id>\d+)"[^>]*>.*?<span class="title">(?P<name>.*?)</span>"#,
        r#"href="https://store\.steampowered\.com/app/(?P<id>\d+)/[^"]*"[^>]*>.*?<span class="title">(?P<name>.*?)</span>"#,
    ];

    let mut best_id = None;
    let mut best_score = 0.0;

    for pattern in patterns {
        let re = Regex::new(pattern).map_err(|e| ScrapeError::Parse(e.to_string()))?;
        for caps in re.captures_iter(&html) {
            let Some(id) = caps.name("id").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let name = caps
                .name("name")
                .map(|m| utils::clean_html(m.as_str()))
                .unwrap_or_default();
            let score = utils::confidence(query, &name);
            if score > best_score {
                best_id = Some(id);
                best_score = score;
            }
        }
    }

    if best_score >= 0.55 {
        Ok(best_id)
    } else {
        Ok(None)
    }
}

fn extract_app_id(value: &str) -> Option<String> {
    let re = Regex::new(r#"(?i)(?:store\.steampowered\.com/app/|steam[_\s-]?app[_\s-]?id[:=\s]*|^)(\d{3,8})(?:\D|$)"#).ok()?;
    re.captures(value)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

fn first_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn push_descriptions(value: Option<&Value>, tags: &mut Vec<String>) {
    if let Some(arr) = value.and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(desc) = item.get("description").and_then(|v| v.as_str()) {
                if !tags.iter().any(|t| t.eq_ignore_ascii_case(desc)) {
                    tags.push(desc.to_string());
                }
            }
        }
    }
}
