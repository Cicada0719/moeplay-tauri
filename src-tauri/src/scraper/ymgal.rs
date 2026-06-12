//! 月幕 Galgame (Ymgal) 刮削客户端
//!
//! 从 ymgal.games 搜索 Galgame 元数据。
//! 月幕是中文 Galgame 数据库，中文名/开发商数据权威，中文社区覆盖率高。
//!
//! 支持功能：
//! - 按标题搜索（JSON API）
//! - 游戏详情查询（多语言名称、别名、标签、评分、发布日期）
//! - 请求重试机制

use serde_json::Value;
use tokio::time::sleep;
use tokio::time::Duration;

use super::error::ScrapeError;
use super::utils;
use crate::models::{ScrapeDetail, ScrapeResult};

/// 月幕搜索 API
const SEARCH_URL: &str =
    "https://www.ymgal.games/api/game/search?keyword={keyword}&pageNum=1&pageSize=10";

/// 游戏详情 API
const GAME_DETAIL_URL: &str = "https://www.ymgal.games/api/game/{id}";

/// 请求间隔 (ms)
const RATE_LIMIT_MS: u64 = 200;

// ========== 速率限制 ==========

async fn rate_limit() {
    sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
}

// ========== 公共 API ==========

/// 按标题搜索月幕游戏
pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    rate_limit().await;

    let keyword = urlencoding::encode(query);
    let url = SEARCH_URL.replace("{keyword}", &keyword);

    let text = utils::fetch_text_with_retry(&url)
        .await
        .map_err(|e| ScrapeError::Network(format!("Ymgal 搜索请求失败: {}", e)))?;

    let json: Value = serde_json::from_str(&text)
        .map_err(|e| ScrapeError::Parse(format!("JSON 解析失败: {}", e)))?;

    parse_search_result(&json, query)
}

/// 按游戏 ID 获取月幕详情
pub async fn get_detail(game_id: &str) -> Result<ScrapeResult, ScrapeError> {
    if game_id.trim().is_empty() {
        return Err(ScrapeError::Config("游戏 ID 不能为空".into()));
    }

    rate_limit().await;

    let url = GAME_DETAIL_URL.replace("{id}", game_id);
    let text = utils::fetch_text_with_retry(&url)
        .await
        .map_err(|e| ScrapeError::Network(format!("Ymgal 详情请求失败: {}", e)))?;

    let json: Value = serde_json::from_str(&text)
        .map_err(|e| ScrapeError::Parse(format!("JSON 解析失败: {}", e)))?;

    // 检查 API 响应状态
    let code = json.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "0" && code != "200" {
        let msg = json.get("msg").and_then(|v| v.as_str()).unwrap_or(code);
        return Err(ScrapeError::Api {
            status: 0,
            body: format!("Ymgal API 错误: {}", msg),
        });
    }

    // 数据可能在顶层或 data 字段
    let data = json.get("data").unwrap_or(&json);
    let info = parse_game_json(data);

    match info {
        Some(result) => Ok(result),
        None => Err(ScrapeError::Parse("无法解析游戏数据".into())),
    }
}

/// 简易搜索接口（返回 `Result<Vec<ScrapeResult>, String>`）
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|e| e.to_string())
}

// ========== 搜索结果解析 ==========

/// 解析搜索结果 JSON
fn parse_search_result(json: &Value, query_title: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    // 检查 API 响应状态
    let code = json.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "0" && code != "200" {
        let msg = json.get("msg").and_then(|v| v.as_str()).unwrap_or(code);
        return Err(ScrapeError::Api {
            status: 0,
            body: format!("Ymgal API 错误: {}", msg),
        });
    }

    // 提取游戏列表
    let data = json.get("data");
    let games: Vec<&Value> = if let Some(arr) = data.and_then(|d| d.as_array()) {
        arr.iter().collect()
    } else if let Some(obj) = data.and_then(|d| d.as_object()) {
        // 可能在 list/records/result 中
        obj.get("list")
            .or_else(|| obj.get("records"))
            .or_else(|| obj.get("result"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().collect())
            .unwrap_or_default()
    } else {
        return Err(ScrapeError::NotFound);
    };

    if games.is_empty() {
        return Err(ScrapeError::NotFound);
    }

    // 找最佳匹配
    let mut items: Vec<(ScrapeResult, f64)> = Vec::new();
    for game in &games {
        if let Some(info) = parse_game_json(game) {
            let name = &info.title;
            let mut conf = utils::confidence(query_title, name);

            // 也检查别名
            if let Some(ref detail) = info.detail {
                for alias in &detail.aliases {
                    let alias_conf = utils::confidence(query_title, alias);
                    if alias_conf > conf {
                        conf = alias_conf;
                    }
                }
            }

            items.push((info, conf));
        }
    }

    if items.is_empty() {
        return Err(ScrapeError::NotFound);
    }

    // 按置信度排序，取最佳
    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let best = items.into_iter().next().unwrap().0;

    Ok(vec![best])
}

// ========== 游戏数据解析 ==========

/// 解析单个游戏 JSON 对象为 ScrapeResult
fn parse_game_json(game: &Value) -> Option<ScrapeResult> {
    let game = game.as_object()?;

    // 标题（优先中文名）
    let name = game
        .get("chineseName")
        .or_else(|| game.get("name"))
        .or_else(|| game.get("gameName"))
        .or_else(|| game.get("title"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;

    // 别名
    let mut aliases: Vec<String> = Vec::new();
    if let Some(jap) = game
        .get("japaneseName")
        .or_else(|| game.get("jpName"))
        .and_then(|v| v.as_str())
    {
        if jap != name {
            aliases.push(jap.to_string());
        }
    }
    if let Some(eng) = game
        .get("englishName")
        .or_else(|| game.get("enName"))
        .and_then(|v| v.as_str())
    {
        if eng != name {
            aliases.push(eng.to_string());
        }
    }

    // 开发商
    let developer = game
        .get("developer")
        .or_else(|| game.get("brand"))
        .or_else(|| game.get("maker"))
        .or_else(|| game.get("company"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 简介
    let description = game
        .get("description")
        .or_else(|| game.get("intro"))
        .or_else(|| game.get("summary"))
        .and_then(|v| v.as_str())
        .map(|s| utils::truncate(s, 500));

    // 封面
    let cover = game
        .get("coverUrl")
        .or_else(|| game.get("image"))
        .or_else(|| game.get("cover"))
        .or_else(|| game.get("thumbnail"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 标签
    let tags: Vec<String> = game
        .get("tags")
        .or_else(|| game.get("genre"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|t| {
                    if let Some(obj) = t.as_object() {
                        obj.get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        t.as_str().unwrap_or("").to_string()
                    }
                })
                .filter(|t| !t.is_empty() && t.len() < 30)
                .take(10)
                .collect()
        })
        .unwrap_or_default();

    // 评分（可能是 10 分制或 100 分制）
    let rating = game
        .get("score")
        .or_else(|| game.get("rating"))
        .or_else(|| game.get("avgScore"))
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .map(|s| if s > 10.0 { s } else { s * 10.0 });

    // 发布日期
    let release_date = game
        .get("releaseDate")
        .or_else(|| game.get("date"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let release_year = release_date
        .as_deref()
        .and_then(|d| d.get(0..4)?.parse::<u32>().ok());

    // 链接
    let game_id = game
        .get("gameId")
        .or_else(|| game.get("id"))
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        })
        .unwrap_or_default();

    // 构建 detail
    let mut detail = ScrapeDetail::default();
    detail.developer = developer;
    detail.aliases.clone_from(&aliases);
    if let Some(ref date) = release_date {
        detail.release_date = Some(date.clone());
    }

    Some(ScrapeResult {
        title: name,
        description,
        cover,
        background: None,
        tags,
        rating,
        release_year,
        source: "ymgal".to_string(),
        source_id: game_id.clone(),
        detail: Some(detail),
    })
}
