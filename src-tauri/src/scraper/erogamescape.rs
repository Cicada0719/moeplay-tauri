//! ErogameScape (批评空间) 刮削客户端
//!
//! 从 ErogameScape (erogamescape.dyndns.org) 搜索 Galgame 元数据。
//! 批评空间是日本最权威的 Galgame 评分网站，评分数据参考价值极高。
//!
//! 注意：
//! - 该网站使用 **EUC-JP** 编码，需要解码处理
//! - 速率限制较严格（每 60 秒约 15 次请求）
//! - 搜索结果为 HTML 表格格式
//!
//! 支持功能：
//! - 按标题搜索（HTML 表格解析）
//! - 按游戏 ID 获取详情（品牌、评分、标签、简介）
//! - EUC-JP → UTF-8 编码转换
//! - 请求重试机制

use regex::Regex;
use tokio::time::sleep;
use tokio::time::Duration;

use super::error::ScrapeError;
use super::utils;
use crate::models::{ScrapeDetail, ScrapeResult};

/// ErogameScape 搜索 API
const SEARCH_URL: &str =
    "https://erogamescape.dyndns.org/~ap2/ero/toukei_kaiseki/kensaku.php?word={word}&mode=normal";

/// 游戏详情页 URL
const GAME_URL: &str = "https://erogamescape.dyndns.org/~ap2/ero/toukei_kaiseki/game.php?game={id}";

/// 请求间隔 (ms)，批评空间较敏感
const RATE_LIMIT_MS: u64 = 500;

// ========== 内部数据结构 ==========

/// 从 ErogameScape 解析的临时游戏信息
#[derive(Debug, Clone, Default)]
struct EgsGameInfo {
    name: Option<String>,
    developer: Option<String>,
    description: Option<String>,
    community_score: Option<i32>,
    tags: Vec<String>,
    links: Vec<String>,
    game_id: Option<String>,
}

// ========== 速率限制 ==========

async fn rate_limit() {
    sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
}

// ========== 公共 API ==========

/// 按标题搜索 ErogameScape 游戏
pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    rate_limit().await;

    let keyword = urlencoding::encode(query);
    let url = SEARCH_URL.replace("{word}", &keyword);

    let html = fetch_eucjp(&url)
        .await
        .map_err(|e| ScrapeError::Network(format!("ErogameScape 搜索请求失败: {}", e)))?;

    let result = parse_search_result(&html, query);
    match result {
        Some(r) => Ok(vec![r]),
        None => Err(ScrapeError::NotFound),
    }
}

/// 按游戏 ID 获取 ErogameScape 详情
pub async fn get_game(game_id: &str) -> Result<ScrapeResult, ScrapeError> {
    if game_id.trim().is_empty() {
        return Err(ScrapeError::Config("游戏 ID 不能为空".into()));
    }

    rate_limit().await;

    let url = GAME_URL.replace("{id}", game_id);
    let html = fetch_eucjp(&url)
        .await
        .map_err(|e| ScrapeError::Network(format!("ErogameScape 详情页请求失败: {}", e)))?;

    parse_game_page(&html, game_id).ok_or_else(|| ScrapeError::Parse("无法解析游戏详情页".into()))
}

/// 简易搜索接口（返回 `Result<Vec<ScrapeResult>, String>`）
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|e| e.to_string())
}

// ========== EUC-JP 编码处理 ==========

/// 获取 URL 内容并使用 EUC-JP 解码
async fn fetch_eucjp(url: &str) -> Result<String, ScrapeError> {
    let bytes = utils::fetch_bytes_with_retry(url).await?;

    // 尝试 EUC-JP 解码
    let (decoded, _, had_errors) = encoding_rs::EUC_JP.decode(&bytes);
    if had_errors {
        // 如果 EUC-JP 解码有错误，尝试 UTF-8
        if let Ok(utf8) = String::from_utf8(bytes.clone()) {
            return Ok(utf8);
        }
        // 回退：使用 EUC-JP 解码结果（即使有部分错误）
    }

    Ok(decoded.into_owned())
}

// ========== 搜索结果解析 ==========

/// 解析搜索结果页
fn parse_search_result(html: &str, query_title: &str) -> Option<ScrapeResult> {
    let items = extract_search_items(html);
    if items.is_empty() {
        return None;
    }

    // 找最佳匹配
    let best = items.iter().max_by(|a, b| {
        let name_a = a.name.as_deref().unwrap_or("");
        let name_b = b.name.as_deref().unwrap_or("");
        let conf_a = utils::confidence(query_title, name_a);
        let conf_b = utils::confidence(query_title, name_b);
        conf_a
            .partial_cmp(&conf_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;

    let name = best.name.as_deref().unwrap_or("");
    let conf = utils::confidence(query_title, name);
    Some(info_to_result(best, conf))
}

/// 从搜索结果 HTML 提取游戏列表
fn extract_search_items(html: &str) -> Vec<EgsGameInfo> {
    let mut items = Vec::new();

    // 匹配搜索结果行：
    // <tr><td><a href="game.php?game=123">游戏名</a></td><td>品牌名</td><td>评分</td>
    let row_re = Regex::new(
        r#"<tr[^>]*>\s*<td[^>]*>\s*<a[^>]*href="game\.php\?game=(\d+)"[^>]*>(.*?)</a>\s*</td>\s*<td[^>]*>(.*?)</td>\s*<td[^>]*>(.*?)</td>"#,
    );

    let re = match row_re {
        Ok(r) => r,
        Err(_) => return items,
    };

    for caps in re.captures_iter(html) {
        let game_id = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let name = caps
            .get(2)
            .map(|m| utils::clean_html(m.as_str()))
            .unwrap_or_default();
        let developer = caps
            .get(3)
            .map(|m| utils::clean_html(m.as_str()))
            .unwrap_or_default();
        let score_text = caps.get(4).map(|m| m.as_str().trim()).unwrap_or("");

        if name.is_empty() {
            continue;
        }

        let mut info = EgsGameInfo::default();
        info.name = Some(name);
        info.developer = if developer.is_empty() {
            None
        } else {
            Some(developer)
        };
        info.game_id = Some(game_id.clone());

        // 评分
        if let Ok(score) = score_text.parse::<i32>() {
            if score > 0 {
                info.community_score = Some(score);
            }
        }

        info.links.push(format!(
            "https://erogamescape.dyndns.org/~ap2/ero/toukei_kaiseki/game.php?game={}",
            game_id
        ));

        items.push(info);
    }

    items
}

// ========== 游戏详情页解析 ==========

/// 解析游戏详情页
fn parse_game_page(html: &str, game_id: &str) -> Option<ScrapeResult> {
    let mut info = EgsGameInfo::default();
    info.game_id = Some(game_id.to_string());

    // 标题
    let title_re = Regex::new(r"<h2[^>]*>(.*?)</h2>").ok()?;
    if let Some(caps) = title_re.captures(html) {
        info.name = Some(utils::clean_html(caps.get(1)?.as_str()));
    }

    // 开发商（品牌）
    let brand_re = Regex::new(r"ブランド.*?<a[^>]*>(.*?)</a>").ok();
    if let Some(re) = &brand_re {
        if let Some(caps) = re.captures(html) {
            info.developer = Some(utils::clean_html(caps.get(1)?.as_str()));
        }
    }

    // 评分（批评空间点数）
    let score_re = Regex::new(r#"<td[^>]*id="score"[^>]*>(\d+)</td>"#).ok();
    if let Some(re) = &score_re {
        if let Some(caps) = re.captures(html) {
            if let Ok(score) = caps.get(1)?.as_str().parse::<i32>() {
                info.community_score = Some(score);
            }
        }
    }

    // 简介
    let desc_re = Regex::new(r#"<div[^>]*id="game_summary"[^>]*>(.*?)</div>"#).ok();
    if let Some(re) = &desc_re {
        if let Some(caps) = re.captures(html) {
            let desc = utils::clean_html(caps.get(1)?.as_str());
            if !desc.is_empty() {
                info.description = Some(utils::truncate(&desc, 500));
            }
        }
    }

    // 标签
    let tag_re = Regex::new(r#"<span[^>]*class="tag"[^>]*>(.*?)</span>"#).ok();
    if let Some(re) = &tag_re {
        info.tags = re
            .captures_iter(html)
            .filter_map(|caps| {
                let tag = utils::clean_html(caps.get(1)?.as_str());
                if tag.is_empty() || tag.len() >= 30 {
                    None
                } else {
                    Some(tag)
                }
            })
            .take(10)
            .collect();
    }

    // 链接
    info.links.push(format!(
        "https://erogamescape.dyndns.org/~ap2/ero/toukei_kaiseki/game.php?game={}",
        game_id
    ));

    // 必须有名称
    info.name.as_ref()?;

    Some(info_to_result(&info, 0.85))
}

// ========== 辅助函数 ==========

/// 将 EgsGameInfo 转为 ScrapeResult
fn info_to_result(info: &EgsGameInfo, _confidence: f64) -> ScrapeResult {
    let community_score = info.community_score.map(|s| s as f64);

    let mut detail = ScrapeDetail::default();
    if let Some(ref dev) = info.developer {
        detail.developer = Some(dev.clone());
    }

    let source_id = info
        .game_id
        .clone()
        .unwrap_or_else(|| info.name.clone().unwrap_or_default());

    ScrapeResult {
        title: info.name.clone().unwrap_or_default(),
        description: info.description.clone(),
        cover: None,
        background: None,
        tags: info.tags.clone(),
        rating: community_score,
        release_year: None,
        source: "erogamescape".to_string(),
        source_id,
        detail: Some(detail),
    }
}
