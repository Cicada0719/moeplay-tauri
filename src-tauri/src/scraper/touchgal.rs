//! TouchGAL / Kungal 刮削客户端
//!
//! 从 kungal.com 获取 Galgame 元数据。
//! TouchGAL 已更名为 Kungal，是中文 Galgame 社区的重要数据源。
//!
//! 支持功能：
//! - 按标题搜索（搜索 API + 列表 API 回退）
//! - 游戏详情查询（多语言名称、标签、别名、开发商等）
//! - VNDB/Bangumi 外链提取
//! - 请求重试机制

use serde_json::Value;
use tokio::time::sleep;
use tokio::time::Duration;

use super::error::ScrapeError;
use super::utils;
use crate::models::{ScrapeDetail, ScrapeResult};

/// Kungal API 基础地址
const BASE_URL: &str = "https://www.kungal.com";

/// 请求间隔 (ms)
const RATE_LIMIT_MS: u64 = 200;

// ========== 速率限制 ==========

async fn rate_limit() {
    sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
}

// ========== 公共 API ==========

/// 按标题搜索 Kungal 游戏
pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    rate_limit().await;

    let galgames = match search_api(query).await {
        Ok(games) if !games.is_empty() => games,
        _ => {
            // 回退到列表 API
            list_api().await?
        }
    };

    if galgames.is_empty() {
        return Err(ScrapeError::NotFound);
    }

    // 从列表中找最佳匹配
    let matched = find_best_match(&galgames, query);
    let matched = matched.ok_or(ScrapeError::NotFound)?;

    // 获取详情页
    let id = matched
        .get("id")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .ok_or_else(|| ScrapeError::Parse("无游戏 ID".into()))?;

    rate_limit().await;

    let detail_url = format!("{}/api/galgame/detail?galgameId={}", BASE_URL, id);
    let detail_json = utils::fetch_text_with_retry(&detail_url)
        .await
        .map_err(|e| ScrapeError::Network(format!("Kungal 详情请求失败: {}", e)))?;

    let detail: Value = serde_json::from_str(&detail_json)
        .map_err(|e| ScrapeError::Parse(format!("详情 JSON 解析失败: {}", e)))?;

    let info = parse_touchgal_result(&detail, query);
    let _conf = utils::confidence(query, &info.title);
    Ok(vec![info])
}

/// 简易搜索接口（返回 `Result<Vec<ScrapeResult>, String>`）
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|e| e.to_string())
}

// ========== 内部搜索 ==========

/// 调用搜索 API
async fn search_api(query: &str) -> Result<Vec<Value>, ScrapeError> {
    let url = format!(
        "{}/api/search?keywords={}&type=galgame&page=1&limit=12",
        BASE_URL,
        urlencoding::encode(query)
    );

    let text = utils::fetch_text_with_retry(&url).await?;
    let token: Value =
        serde_json::from_str(&text).map_err(|e| ScrapeError::Parse(e.to_string()))?;

    if let Some(arr) = token.as_array() {
        Ok(arr.clone())
    } else {
        Ok(vec![])
    }
}

/// 调用列表 API（搜索回退）
async fn list_api() -> Result<Vec<Value>, ScrapeError> {
    let url = format!(
        "{}/api/galgame?page=1&limit=24&sortField=created&sortOrder=desc&type=all&language=all&platform=all",
        BASE_URL
    );

    let text = utils::fetch_text_with_retry(&url).await?;
    let token: Value =
        serde_json::from_str(&text).map_err(|e| ScrapeError::Parse(e.to_string()))?;

    if let Some(galgames) = token.get("galgames").and_then(|v| v.as_array()) {
        Ok(galgames.clone())
    } else {
        Ok(vec![])
    }
}

/// 从 galgames 列表中找最佳匹配项
fn find_best_match<'a>(galgames: &'a [Value], query: &str) -> Option<&'a Value> {
    if galgames.is_empty() || query.is_empty() {
        return galgames.first();
    }

    let q = query.trim().to_lowercase();
    let mut best: Option<&Value> = None;
    let mut best_score = 0.0f64;

    for item in galgames {
        let name_token = item.get("name")?;

        let mut names: Vec<String> = Vec::new();
        if let Some(name_obj) = name_token.as_object() {
            for val in name_obj.values() {
                if let Some(s) = val.as_str() {
                    if !s.is_empty() {
                        names.push(s.trim().to_lowercase());
                    }
                }
            }
        } else if let Some(s) = name_token.as_str() {
            if !s.is_empty() {
                names.push(s.trim().to_lowercase());
            }
        }

        if names.is_empty() {
            continue;
        }

        let mut score = 0.0;
        for n in &names {
            let s = if n == &q {
                1.0
            } else if n.contains(&q) || q.contains(n) {
                0.8
            } else {
                0.0
            };
            if s > score {
                score = s;
            }
        }

        if score > best_score {
            best_score = score;
            best = Some(item);
        }
    }

    best
}

// ========== 结果解析 ==========

/// 解析 TouchGAL/Kungal 详情为 ScrapeResult
fn parse_touchgal_result(detail: &Value, query: &str) -> ScrapeResult {
    // 处理 legacy TouchGal 格式（patch 包裹）
    let detail = if detail.get("patch").is_some() {
        // legacy 格式：提取 patch 和 intro 合并
        &merge_legacy_format(detail)
    } else {
        detail
    };

    // 解析多语言名称
    let name = extract_multilang_field(detail.get("name"), "zh-cn", "ja-jp", "en-us")
        .unwrap_or_else(|| query.to_string());

    // 解析多语言简介
    let description =
        extract_multilang_field(detail.get("introduction"), "zh-cn", "ja-jp", "en-us");

    // 别名
    let aliases: Vec<String> = detail
        .get("alias")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // 标签
    let tags: Vec<String> = detail
        .get("tag")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    t.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .filter(|s| !s.is_empty())
                .take(10)
                .collect()
        })
        .unwrap_or_default();

    // 发行日期
    let release_date = detail
        .get("created")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let release_year = release_date
        .as_deref()
        .and_then(|d| d.get(0..4)?.parse::<u32>().ok());

    // 开发商
    let developer = detail
        .get("official")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|o| o.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    // 封面
    let cover = detail
        .get("banner")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 构建链接
    let game_id = detail.get("id").and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    });
    let is_legacy = detail
        .get("isLegacyTouchGal")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let kungal_url = game_id.map(|id| {
        if is_legacy {
            format!("https://www.touchgal.io/patch/{}", id)
        } else {
            format!("https://www.kungal.com/galgame/{}", id)
        }
    });

    // VNDB 链接
    let _vndb_id = detail
        .get("vndbId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Bangumi 链接
    let _bgm_id = detail
        .get("bangumiId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source_id = game_id.map(|id| id.to_string()).unwrap_or_default();

    // 构建 detail
    let mut scrape_detail = ScrapeDetail::default();
    scrape_detail.developer = developer;
    scrape_detail.aliases = aliases;
    scrape_detail.homepage = kungal_url;
    if let Some(ref date) = release_date {
        scrape_detail.release_date = Some(date.clone());
    }

    ScrapeResult {
        title: name,
        description: description.map(|d| utils::truncate(&d, 500)),
        cover,
        background: None,
        tags,
        rating: None,
        release_year,
        source: "touchgal".to_string(),
        source_id,
        detail: Some(scrape_detail),
    }
}

/// 从多语言 JSON 对象中提取最佳匹配的字段值
fn extract_multilang_field(
    obj: Option<&Value>,
    first: &str,
    second: &str,
    third: &str,
) -> Option<String> {
    let obj = obj?;
    if let Some(name_obj) = obj.as_object() {
        // 按优先级尝试
        for key in &[first, second, third] {
            if let Some(val) = name_obj.get(*key).and_then(|v| v.as_str()) {
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
        // 回退：取第一个非空值
        for val in name_obj.values() {
            if let Some(s) = val.as_str() {
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            }
        }
        None
    } else if let Some(s) = obj.as_str() {
        if !s.is_empty() {
            Some(s.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

/// 合并 legacy TouchGal 格式（patch + intro → 统一格式）
fn merge_legacy_format(detail: &Value) -> Value {
    let patch = match detail.get("patch") {
        Some(p) => p,
        None => return detail.clone(),
    };
    let intro = detail.get("intro").unwrap_or(&Value::Null);

    let mut merged = serde_json::Map::new();

    // ID
    if let Some(id) = patch.get("uniqueId") {
        merged.insert("id".to_string(), id.clone());
    }
    merged.insert("isLegacyTouchGal".to_string(), Value::Bool(true));

    // 名称
    if let Some(name) = patch.get("name") {
        if name.is_object() {
            merged.insert("name".to_string(), name.clone());
        } else if name.is_string() {
            let mut name_obj = serde_json::Map::new();
            name_obj.insert("zh-cn".to_string(), name.clone());
            merged.insert("name".to_string(), Value::Object(name_obj));
        }
    }

    // 简介
    if let Some(intro_text) = patch.get("introduction") {
        let mut intro_obj = serde_json::Map::new();
        intro_obj.insert("zh-cn".to_string(), intro_text.clone());
        merged.insert("introduction".to_string(), Value::Object(intro_obj));
    }

    // Banner
    if let Some(banner) = patch.get("banner") {
        merged.insert("banner".to_string(), banner.clone());
    }

    // 别名
    if let Some(alias) = patch.get("alias") {
        merged.insert("alias".to_string(), alias.clone());
    }

    // 标签
    if let Some(tags) = patch.get("tags").and_then(|v| v.as_array()) {
        let tag_objs: Vec<Value> = tags
            .iter()
            .map(|tag| {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "name".to_string(),
                    tag.as_str()
                        .map(|s| Value::String(s.to_string()))
                        .unwrap_or_else(|| tag.clone()),
                );
                Value::Object(obj)
            })
            .collect();
        merged.insert("tag".to_string(), Value::Array(tag_objs));
    }

    // 发布日期
    if let Some(released) = intro.get("released") {
        merged.insert("created".to_string(), released.clone());
    }

    // VNDB ID
    if let Some(vndb) = intro.get("vndbId") {
        merged.insert("vndbId".to_string(), vndb.clone());
    }

    // Bangumi ID
    if let Some(bgm) = intro.get("bangumiId") {
        merged.insert("bangumiId".to_string(), bgm.clone());
    }

    // 开发商
    if let Some(companies) = intro.get("company").and_then(|v| v.as_array()) {
        let officials: Vec<Value> = companies
            .iter()
            .map(|c| {
                let mut obj = serde_json::Map::new();
                if let Some(name) = c.get("name") {
                    obj.insert("name".to_string(), name.clone());
                }
                Value::Object(obj)
            })
            .collect();
        merged.insert("official".to_string(), Value::Array(officials));
    }

    Value::Object(merged)
}
