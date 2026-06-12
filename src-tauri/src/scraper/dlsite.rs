//! DLsite 刮削客户端
//!
//! 从 DLsite (dlsite.com) 搜索同人游戏元数据。
//! DLsite 是日本最大的同人作品销售平台，覆盖海量同人 Galgame。
//!
//! 支持功能：
//! - 按标题搜索作品（JSON-LD 结构化数据 + HTML 列表回退）
//! - 按产品 ID 获取详情（社团/品牌、标签、评分、封面）
//! - 标题匹配置信度评估
//! - 请求重试机制

use regex::Regex;
use serde_json::Value;
use tokio::time::sleep;
use tokio::time::Duration;

use super::error::ScrapeError;
use super::utils;
use crate::models::{ScrapeDetail, ScrapeResult};

/// DLsite 搜索 API（JSON 格式，同人游戏分类）
const SEARCH_URL: &str = "https://www.dlsite.com/home/fsr/=/language/jp/keyword/{keyword}/order/d/per_page/10/page/1/show_type/1/genre_and_or/and/genre/11/regist_date_start/regist_date_end/lang_options[0]/ja/work_category%5B0%5D/doujin/is_order_affinity/1";

/// 备用：DLsite maniax 搜索
const MANIAX_SEARCH_URL: &str = "https://www.dlsite.com/maniax/fsr/=/language/jp/keyword/{keyword}/order/d/per_page/10/page/1/show_type/1/genre_and_or/and/genre/11";

/// 产品详情页 URL
const PRODUCT_URL: &str = "https://www.dlsite.com/maniax/work/=/product_id/{id}.html";

/// 请求间隔 (ms)
const RATE_LIMIT_MS: u64 = 300;

// ========== 内部数据结构 ==========

/// 从 DLsite 解析的临时游戏信息
#[derive(Debug, Clone, Default)]
struct DlsiteGameInfo {
    name: Option<String>,
    developer: Option<String>,
    description: Option<String>,
    cover_url: Option<String>,
    tags: Vec<String>,
    release_date: Option<String>,
    community_score: Option<i32>,
    product_url: Option<String>,
    product_id: Option<String>,
}

// ========== 速率限制 ==========

async fn rate_limit() {
    sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
}

// ========== 公共 API ==========

/// 按标题搜索 DLsite 游戏
pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    if query.trim().is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    rate_limit().await;

    let keyword = urlencoding::encode(query);
    let search_url = SEARCH_URL.replace("{keyword}", &keyword);

    let html = match utils::fetch_text_ja(&search_url).await {
        Ok(html) => html,
        Err(_) => {
            // 备用 maniax 搜索
            rate_limit().await;
            let maniax_url = MANIAX_SEARCH_URL.replace("{keyword}", &keyword);
            utils::fetch_text_ja(&maniax_url)
                .await
                .map_err(|e| ScrapeError::Network(format!("DLsite 搜索请求失败: {}", e)))?
        }
    };

    if html.is_empty() {
        return Err(ScrapeError::NotFound);
    }

    let result = parse_search_result(&html, query);
    match result {
        Some(r) => Ok(vec![r]),
        None => Err(ScrapeError::NotFound),
    }
}

/// 按产品 ID 获取 DLsite 详情
pub async fn get_product(product_id: &str) -> Result<ScrapeResult, ScrapeError> {
    if product_id.trim().is_empty() {
        return Err(ScrapeError::Config("产品 ID 不能为空".into()));
    }

    rate_limit().await;

    let url = PRODUCT_URL.replace("{id}", product_id);
    let html = utils::fetch_text_ja(&url)
        .await
        .map_err(|e| ScrapeError::Network(format!("DLsite 详情页请求失败: {}", e)))?;

    parse_product_page(&html, product_id).ok_or_else(|| ScrapeError::Parse("无法解析产品页".into()))
}

/// 简易搜索接口（返回 `Result<Vec<ScrapeResult>, String>`）
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|e| e.to_string())
}

// ========== 搜索结果解析 ==========

/// 从搜索结果页 HTML 解析游戏信息
fn parse_search_result(html: &str, query_title: &str) -> Option<ScrapeResult> {
    // 方法1：尝试解析 JSON-LD 结构化数据
    if let Some(json_ld) = extract_json_ld(html) {
        if let Some(info) = parse_json_ld(&json_ld) {
            if let Some(ref name) = info.name {
                if !name.is_empty() {
                    let conf = utils::confidence(query_title, name);
                    return Some(info_to_result(&info, conf));
                }
            }
        }
    }

    // 方法2：解析搜索结果列表 HTML
    let items = extract_search_items(html);
    if !items.is_empty() {
        let best = find_best_match(&items, query_title);
        if let Some(info) = best {
            let name = info.name.as_deref().unwrap_or("");
            let conf = utils::confidence(query_title, name);
            return Some(info_to_result(info, conf));
        }
    }

    None
}

/// 提取页面中的 JSON-LD 结构化数据
fn extract_json_ld(html: &str) -> Option<Value> {
    let re = Regex::new(r#"<script[^>]*type="application/ld\+json"[^>]*>(.*?)</script>"#).ok()?;

    let caps = re.captures(html)?;
    let json_text = caps.get(1)?.as_str().trim();

    serde_json::from_str::<Value>(json_text).ok()
}

/// 从 JSON-LD 解析 GameInfo
fn parse_json_ld(json_ld: &Value) -> Option<DlsiteGameInfo> {
    let mut info = DlsiteGameInfo::default();

    info.name = json_ld
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    info.description = json_ld
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 图片
    if let Some(image) = json_ld.get("image") {
        if let Some(arr) = image.as_array() {
            if let Some(first) = arr.first() {
                info.cover_url = first.as_str().map(|s| s.to_string());
            }
        } else if let Some(s) = image.as_str() {
            info.cover_url = Some(s.to_string());
        }
    }

    // 品牌/作者
    let brand = json_ld
        .get("brand")
        .or_else(|| json_ld.get("author"))
        .or_else(|| json_ld.get("creator"));
    if let Some(brand) = brand {
        if let Some(name) = brand.get("name").and_then(|v| v.as_str()) {
            info.developer = Some(name.to_string());
        } else if let Some(s) = brand.as_str() {
            info.developer = Some(s.to_string());
        }
    }

    // 关键词/标签
    if let Some(keywords) = json_ld.get("keywords") {
        let keyword_str = if let Some(s) = keywords.as_str() {
            s.to_string()
        } else {
            keywords.to_string()
        };
        if !keyword_str.is_empty() {
            info.tags = keyword_str
                .split(&[',', ';'][..])
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty() && k.len() < 30)
                .take(10)
                .collect();
        }
    }

    // 发布日期
    if let Some(date) = json_ld.get("datePublished").and_then(|v| v.as_str()) {
        info.release_date = Some(date.to_string());
    }

    Some(info)
}

/// 从搜索结果 HTML 提取列表项
fn extract_search_items(html: &str) -> Vec<DlsiteGameInfo> {
    let mut items = Vec::new();

    // 匹配 work_name 和链接
    let name_re =
        Regex::new(r#"<a[^>]*class="work_name"[^>]*title="([^"]+)"[^>]*href="([^"]+)""#).unwrap();
    let maker_re = Regex::new(r#"<a[^>]*class="maker_name"[^>]*>(.*?)</a>"#).unwrap();

    let name_matches: Vec<_> = name_re.captures_iter(html).collect();
    let maker_matches: Vec<_> = maker_re.captures_iter(html).collect();
    let id_re = Regex::new(r"/product_id/(\w+)").unwrap();

    for (i, caps) in name_matches.iter().enumerate() {
        let name = utils::clean_html(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        let href = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        if name.is_empty() {
            continue;
        }

        let mut info = DlsiteGameInfo::default();
        info.name = Some(name);

        // 从 URL 提取产品 ID
        if let Some(id_caps) = id_re.captures(href) {
            let pid = id_caps.get(1).unwrap().as_str().to_string();
            info.product_id = Some(pid.clone());
            info.product_url = Some(format!("https://www.dlsite.com{}", href));
        }

        // 社团名
        if i < maker_matches.len() {
            info.developer = maker_matches[i]
                .get(1)
                .map(|m| utils::clean_html(m.as_str()));
        }

        items.push(info);
    }

    items
}

/// 从搜索结果中找到最佳匹配
fn find_best_match<'a>(
    items: &'a [DlsiteGameInfo],
    query_title: &str,
) -> Option<&'a DlsiteGameInfo> {
    if items.is_empty() {
        return None;
    }
    if query_title.is_empty() {
        return items.first();
    }

    items.iter().max_by(|a, b| {
        let name_a = a.name.as_deref().unwrap_or("");
        let name_b = b.name.as_deref().unwrap_or("");
        let conf_a = utils::confidence(query_title, name_a);
        let conf_b = utils::confidence(query_title, name_b);
        conf_a
            .partial_cmp(&conf_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

// ========== 产品详情页解析 ==========

/// 从产品详情页解析完整元数据
fn parse_product_page(html: &str, product_id: &str) -> Option<ScrapeResult> {
    let mut info = DlsiteGameInfo::default();

    // 标题
    let title_re = Regex::new(r#"<h1[^>]*id="work_name"[^>]*>(.*?)</h1>"#).ok()?;
    if let Some(caps) = title_re.captures(html) {
        info.name = Some(utils::clean_html(caps.get(1)?.as_str()));
    }

    // JSON-LD（更可靠）
    if let Some(json_ld) = extract_json_ld(html) {
        if let Some(ld_info) = parse_json_ld(&json_ld) {
            if info.name.is_none() {
                info.name = ld_info.name;
            }
            if info.developer.is_none() {
                info.developer = ld_info.developer;
            }
            if info.description.is_none() {
                info.description = ld_info.description;
            }
            if info.cover_url.is_none() {
                info.cover_url = ld_info.cover_url;
            }
            if info.tags.is_empty() {
                info.tags = ld_info.tags;
            }
            if info.release_date.is_none() {
                info.release_date = ld_info.release_date;
            }
        }
    }

    // 社团名
    if info.developer.is_none() {
        let maker_re = Regex::new(r#"<a[^>]*class="maker_name"[^>]*>(.*?)</a>"#).ok();
        if let Some(re) = &maker_re {
            if let Some(caps) = re.captures(html) {
                info.developer = Some(utils::clean_html(caps.get(1)?.as_str()));
            }
        }
    }

    // 封面图 (og:image)
    if info.cover_url.is_none() {
        let cover_re = Regex::new(r#"<meta\s+property="og:image"\s+content="([^"]+)""#).ok();
        if let Some(re) = &cover_re {
            if let Some(caps) = re.captures(html) {
                info.cover_url = Some(caps.get(1)?.as_str().to_string());
            }
        }
    }

    // 评分
    let rating_re = Regex::new(r#"<dd[^>]*class="star_rating"[^>]*>([\d.]+)</dd>"#).ok();
    if let Some(re) = &rating_re {
        if let Some(caps) = re.captures(html) {
            if let Ok(rating) = caps.get(1)?.as_str().parse::<f64>() {
                // DLsite 5分制 → 100分制
                info.community_score = Some((rating * 20.0) as i32);
            }
        }
    }

    // 产品页面链接
    info.product_url = Some(format!(
        "https://www.dlsite.com/maniax/work/=/product_id/{}.html",
        product_id
    ));
    info.product_id = Some(product_id.to_string());

    // 必须有名称
    info.name.as_ref()?;

    let mut result = info_to_result(&info, 0.9);
    result.source_id = product_id.to_string();
    Some(result)
}

// ========== 辅助函数 ==========

/// 将 DlsiteGameInfo 转为 ScrapeResult
fn info_to_result(info: &DlsiteGameInfo, _confidence: f64) -> ScrapeResult {
    let release_year = info
        .release_date
        .as_deref()
        .and_then(|d| d.get(0..4)?.parse::<u32>().ok());

    let community_score = info.community_score.map(|s| s as f64);

    let mut detail = ScrapeDetail::default();

    if let Some(ref dev) = info.developer {
        detail.developer = Some(dev.clone());
    }

    if let Some(ref date) = info.release_date {
        detail.release_date = Some(date.clone());
    }

    let source_id = info
        .product_id
        .clone()
        .unwrap_or_else(|| info.name.clone().unwrap_or_default());

    ScrapeResult {
        title: info.name.clone().unwrap_or_default(),
        description: info.description.clone().map(|d| utils::truncate(&d, 500)),
        cover: info.cover_url.clone(),
        background: None,
        tags: info.tags.clone(),
        rating: community_score,
        release_year,
        source: "dlsite".to_string(),
        source_id,
        detail: Some(detail),
    }
}
