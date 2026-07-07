//! Kungal 刮削客户端
//!
//! 从 https://www.kungal.com 搜索中文 Galgame 元数据。这个模块保留
//! `touchgal` 兼容源，同时以独立的 `kungal` source 暴露给前端和聚合刮削。

use serde_json::Value;
use tokio::time::{sleep, Duration};

use super::error::ScrapeError;
use super::utils;
use crate::models::{ScrapeDetail, ScrapeResult};

const BASE_URL: &str = "https://www.kungal.com";
const RATE_LIMIT_MS: u64 = 200;

async fn rate_limit() {
    sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
}

/// 按标题搜索 Kungal 游戏。
pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    rate_limit().await;

    let games = search_api(query).await?;
    if games.is_empty() {
        return Err(ScrapeError::NotFound);
    }

    let matched = find_best_match(&games, query).ok_or(ScrapeError::NotFound)?;
    let id = extract_id(matched).ok_or_else(|| ScrapeError::Parse("无游戏 ID".into()))?;

    match get_detail(&id.to_string()).await {
        Ok(result) => Ok(vec![result]),
        Err(_) => Ok(vec![parse_search_item(matched, query)]),
    }
}

/// 按 Kungal 游戏 ID 获取详情。
pub async fn get_detail(game_id: &str) -> Result<ScrapeResult, ScrapeError> {
    if game_id.trim().is_empty() {
        return Err(ScrapeError::Config("Kungal 游戏 ID 不能为空".into()));
    }

    rate_limit().await;

    let detail = fetch_detail_json(game_id).await?;
    if detail.get("code").and_then(|v| v.as_i64()) == Some(233) {
        return Err(ScrapeError::NotFound);
    }
    // 现行 API 把字段包在 data 下：{"code":0,"data":{...}}；兼容直接返回对象的旧形态。
    let data = detail
        .get("data")
        .filter(|d| d.is_object())
        .unwrap_or(&detail);
    Ok(parse_detail(data, game_id))
}

/// 简易搜索接口（返回 `Result<Vec<ScrapeResult>, String>`）。
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|e| e.to_string())
}

async fn search_api(query: &str) -> Result<Vec<Value>, ScrapeError> {
    let url = format!(
        "{}/api/search?keywords={}&type=galgame&page=1&limit=10",
        BASE_URL,
        urlencoding::encode(query)
    );

    let text = utils::fetch_text_with_retry(&url).await?;
    let token: Value =
        serde_json::from_str(&text).map_err(|e| ScrapeError::Parse(e.to_string()))?;

    // 现行 API: {"code":0,"data":{"items":[...],"total":N}}
    // 兼容旧形态: {"data":[...]} 或顶层数组。
    let items = token
        .pointer("/data/items")
        .and_then(|v| v.as_array())
        .or_else(|| token.get("data").and_then(|v| v.as_array()))
        .or_else(|| token.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(items)
}

async fn fetch_detail_json(game_id: &str) -> Result<Value, ScrapeError> {
    let endpoints = [
        format!(
            "{}/api/galgame/{}?galgameId={}",
            BASE_URL,
            urlencoding::encode(game_id),
            urlencoding::encode(game_id)
        ),
        format!(
            "{}/api/galgame/detail?galgameId={}",
            BASE_URL,
            urlencoding::encode(game_id)
        ),
    ];

    let mut last_error = None;
    for url in endpoints {
        match utils::fetch_text_with_retry(&url).await {
            Ok(text) => {
                let value: Value =
                    serde_json::from_str(&text).map_err(|e| ScrapeError::Parse(e.to_string()))?;
                return Ok(value);
            }
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap_or(ScrapeError::NotFound))
}

fn find_best_match<'a>(games: &'a [Value], query: &str) -> Option<&'a Value> {
    if games.is_empty() {
        return None;
    }

    let mut best = games.first();
    let mut best_score = 0.0;
    for game in games {
        let names = extract_names(game.get("name"));
        let score = names
            .iter()
            .map(|name| confidence_with_cjk(query, name))
            .fold(0.0, f64::max);

        if score > best_score {
            best = Some(game);
            best_score = score;
        }
    }

    best
}

fn parse_search_item(item: &Value, query: &str) -> ScrapeResult {
    let title = pick_localized_text(item.get("name")).unwrap_or_else(|| query.to_string());
    let aliases = extract_names(item.get("name"))
        .into_iter()
        .filter(|name| name != &title)
        .collect::<Vec<_>>();

    let mut detail = ScrapeDetail::default();
    detail.aliases = aliases;
    if let Some(id) = extract_id(item) {
        detail.homepage = Some(format!("{}/galgame/{}", BASE_URL, id));
    }

    ScrapeResult {
        title,
        description: pick_localized_text(item.get("introduction")),
        cover: item
            .get("banner")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        background: None,
        tags: vec![],
        rating: None,
        release_year: None,
        source: "kungal".to_string(),
        source_id: extract_id(item)
            .map(|id| id.to_string())
            .unwrap_or_default(),
        detail: Some(detail),
    }
}

fn parse_detail(detail_json: &Value, fallback_id: &str) -> ScrapeResult {
    let title =
        pick_localized_text(detail_json.get("name")).unwrap_or_else(|| fallback_id.to_string());
    let description = pick_localized_text(detail_json.get("markdown"))
        .or_else(|| pick_localized_text(detail_json.get("introduction")))
        .map(|text| utils::truncate(&text.replace("\\\n", "\n").replace("\\\r\n", "\n"), 800));

    let mut aliases = extract_names(detail_json.get("name"))
        .into_iter()
        .filter(|name| name != &title)
        .collect::<Vec<_>>();
    if let Some(arr) = detail_json.get("alias").and_then(|v| v.as_array()) {
        for alias in arr.iter().filter_map(|v| v.as_str()) {
            push_unique(&mut aliases, alias.trim());
        }
    }

    let mut tags = vec![];
    if let Some(arr) = detail_json.get("tag").and_then(|v| v.as_array()) {
        let mut items = arr.iter().collect::<Vec<_>>();
        items.sort_by(|a, b| {
            let a_count = a.get("galgameCount").and_then(|v| v.as_i64()).unwrap_or(0);
            let b_count = b.get("galgameCount").and_then(|v| v.as_i64()).unwrap_or(0);
            b_count.cmp(&a_count)
        });

        for tag in items {
            let spoiler = tag
                .get("spoilerLevel")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if spoiler > 1 {
                continue;
            }
            if let Some(name) = tag.get("name").and_then(|v| v.as_str()) {
                push_unique(&mut tags, name.trim());
            }
            if tags.len() >= 12 {
                break;
            }
        }
    }

    let id = extract_id(detail_json)
        .map(|id| id.to_string())
        .unwrap_or_else(|| fallback_id.to_string());
    let release_date = detail_json
        .get("releaseDate")
        .or_else(|| detail_json.get("created"))
        .or_else(|| detail_json.get("released"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());

    let mut detail = ScrapeDetail::default();
    detail.aliases = aliases;
    detail.developer = extract_developer(detail_json);
    detail.homepage = Some(format!("{}/galgame/{}", BASE_URL, id));
    detail.release_date.clone_from(&release_date);
    detail.age_rating = extract_age_rating(detail_json);
    detail.genres = tags.clone();
    detail.languages = extract_str_array(detail_json, "language");
    // 截图：screenshots + covers（去重）
    detail.screenshots = extract_image_array(detail_json, "screenshots");
    for c in extract_image_array(detail_json, "covers") {
        if !detail.screenshots.contains(&c) {
            detail.screenshots.push(c);
        }
    }
    let engines = extract_str_array(detail_json, "engine");
    if !engines.is_empty() {
        detail.engine = Some(engines.join("/"));
    }

    ScrapeResult {
        title,
        description,
        cover: detail_json
            .get("banner")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        background: None,
        tags,
        rating: None,
        release_year: release_date.as_deref().and_then(utils::extract_year),
        source: "kungal".to_string(),
        source_id: id,
        detail: Some(detail),
    }
}

fn extract_id(value: &Value) -> Option<i64> {
    value.get("id").and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

fn pick_localized_text(value: Option<&Value>) -> Option<String> {
    let value = value?;
    let order = ["zh-cn", "ja-jp", "en-us", "zh-tw"];

    if let Some(obj) = value.as_object() {
        for key in order {
            if let Some(text) = obj.get(key).and_then(|v| v.as_str()) {
                let text = text.trim();
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }

        for text in obj.values().filter_map(|v| v.as_str()) {
            let text = text.trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }

    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn extract_names(value: Option<&Value>) -> Vec<String> {
    let mut names = vec![];
    let Some(value) = value else {
        return names;
    };

    if let Some(obj) = value.as_object() {
        for key in ["zh-cn", "ja-jp", "en-us", "zh-tw"] {
            if let Some(name) = obj.get(key).and_then(|v| v.as_str()) {
                push_unique(&mut names, name.trim());
            }
        }
    } else if let Some(name) = value.as_str() {
        push_unique(&mut names, name.trim());
    }

    names
}

/// 提取字符串数组字段（language / engine / platform 等）。
fn extract_str_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// 提取图片数组（screenshots/covers）。元素可能是字符串 URL，也可能是 {url/src/...} 对象。
fn extract_image_array(value: &Value, key: &str) -> Vec<String> {
    let Some(arr) = value.get(key).and_then(|v| v.as_array()) else {
        return vec![];
    };
    arr.iter()
        .filter_map(|item| {
            if let Some(s) = item.as_str() {
                return Some(s.to_string());
            }
            for k in ["url", "src", "image", "cover", "full", "mini"] {
                if let Some(s) = item.get(k).and_then(|v| v.as_str()) {
                    return Some(s.to_string());
                }
            }
            None
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn extract_developer(value: &Value) -> Option<String> {
    let official = value.get("official")?.as_array()?;
    let developers = official
        .iter()
        .filter(|entry| {
            entry
                .get("category")
                .and_then(|v| v.as_str())
                .map(is_developer_category)
                .unwrap_or(true)
        })
        .filter_map(|entry| entry.get("name").and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if developers.is_empty() {
        None
    } else {
        Some(developers.join("/"))
    }
}

fn extract_age_rating(value: &Value) -> Option<String> {
    let content_limit = value.get("contentLimit").and_then(|v| v.as_str());
    let age_limit = value.get("ageLimit").and_then(|v| v.as_str());

    if content_limit == Some("nsfw") || age_limit == Some("r18") {
        Some("R18".to_string())
    } else if age_limit.is_some() {
        age_limit.map(|s| s.to_string())
    } else {
        None
    }
}

fn is_developer_category(category: &str) -> bool {
    matches!(
        category.to_lowercase().as_str(),
        "brand" | "developer" | "maker" | "circle"
    )
}

fn confidence_with_cjk(query: &str, candidate: &str) -> f64 {
    let direct = utils::confidence(query, candidate);
    let normalized = utils::confidence(
        &normalize_cjk_variants(query),
        &normalize_cjk_variants(candidate),
    );
    direct.max(normalized)
}

fn normalize_cjk_variants(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '姫' => '姬',
            '遊' => '游',
            '戰' => '战',
            '記' => '记',
            '國' => '国',
            '學' => '学',
            '無' => '无',
            '劍' => '剑',
            '愛' => '爱',
            '夢' => '梦',
            '銀' => '银',
            '龍' => '龙',
            '鳥' => '鸟',
            '華' => '华',
            '異' => '异',
            '畫' => '画',
            '話' => '话',
            '連' => '连',
            '開' => '开',
            '關' => '关',
            _ => c,
        })
        .collect()
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if value.is_empty() {
        return;
    }
    if !values.iter().any(|v| v.eq_ignore_ascii_case(value)) {
        values.push(value.to_string());
    }
}
