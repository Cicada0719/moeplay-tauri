//! 番剧规则引擎 — 兼容 Kazumi 社区规则 JSON（XPath → CSS 自动转换）

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// ── 规则(Plugin)模型 — 1:1 映射 Kazumi JSON ─────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AnimeRule {
    #[serde(default)]
    pub api: String,
    #[serde(default = "default_type")]
    pub r#type: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default = "bool_true")]
    pub muli_sources: bool,
    #[serde(default = "bool_true")]
    pub use_webview: bool,
    #[serde(default = "bool_true")]
    pub use_native_player: bool,
    #[serde(default)]
    pub use_post: bool,
    #[serde(default)]
    pub use_legacy_parser: bool,
    #[serde(default)]
    pub ad_blocker: bool,
    #[serde(default)]
    pub user_agent: String,
    #[serde(alias = "baseURL")]
    pub base_url: String,
    #[serde(alias = "searchURL")]
    pub search_url: String,
    pub search_list: String,
    pub search_name: String,
    pub search_result: String,
    pub chapter_roads: String,
    pub chapter_result: String,
    #[serde(default)]
    pub referer: String,
}

fn default_type() -> String { "anime".into() }
fn bool_true() -> bool { true }

#[derive(Serialize, Clone, Debug)]
pub struct SearchItem {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Road {
    pub name: String,
    pub episodes: Vec<Episode>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Episode {
    pub name: String,
    pub url: String,
}

// ── 收藏 & 历史（本地持久化）────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimeCollect {
    pub bangumi_id: Option<i64>,
    pub name: String,
    pub name_cn: String,
    pub image: String,
    pub air_date: String,
    pub rating: f64,
    pub collect_type: i32, // 1=在看 2=想看 3=搁置 4=看过 5=抛弃
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimeHistory {
    pub key: String,
    pub name: String,
    pub name_cn: String,
    pub image: String,
    pub rule_name: String,
    pub last_episode: i32,
    pub last_episode_name: String,
    pub last_road: i32,
    pub progress_ms: i64,
    pub last_src: String,
    pub updated_at: String,
}

// ── 托管状态 ─────────────────────────────────────────────────────────────

pub struct AnimeState {
    pub rules: Mutex<Vec<AnimeRule>>,
}

impl Default for AnimeState {
    fn default() -> Self {
        Self {
            rules: Mutex::new(Vec::new()),
        }
    }
}

// ── XPath → CSS 转换（覆盖 Kazumi 社区规则常用模式）─────────────────────

pub fn xpath_to_css(xpath: &str) -> String {
    let s = xpath.trim();
    if s.is_empty() { return String::new(); }

    let s = if s.starts_with(".//") { &s[3..] } else if s.starts_with("//") { &s[2..] } else { s };
    let s = s.trim_start_matches('/');

    let mut css_parts = Vec::new();
    let segments: Vec<&str> = s.split('/').collect();

    for raw_seg in &segments {
        let seg: &str = raw_seg.trim();
        if seg.is_empty() { continue; }
        if seg == "text()" { continue; }
        if seg.starts_with('@') { continue; }

        if seg.contains('[') {
            // div[@class="xxx"] or div[contains(@class,'xxx')]
            if let Some(pos) = seg.find('[') {
                let tag = &seg[..pos];
                let pred = &seg[pos+1..seg.len()-1]; // strip []

                if pred.starts_with("@class=") || pred.starts_with("@class=\"") || pred.starts_with("@class='") {
                    let val = pred.trim_start_matches("@class=")
                        .trim_matches('"').trim_matches('\'');
                    // multiple classes → .class1.class2
                    let classes: Vec<&str> = val.split_whitespace().collect();
                    let sel = format!("{}.{}", tag, classes.join("."));
                    css_parts.push(sel);
                } else if pred.starts_with("contains(@class,") || pred.starts_with("contains(@class, ") {
                    let inner = pred.trim_start_matches("contains(@class,")
                        .trim_start_matches("contains(@class, ")
                        .trim_end_matches(')')
                        .trim().trim_matches('"').trim_matches('\'');
                    css_parts.push(format!("{}[class*=\"{}\"]", tag, inner));
                } else if pred.starts_with("@id=") {
                    let val = pred.trim_start_matches("@id=")
                        .trim_matches('"').trim_matches('\'');
                    css_parts.push(format!("{}#{}", tag, val));
                } else {
                    css_parts.push(tag.to_string());
                }
            }
        } else {
            css_parts.push(seg.to_string());
        }
    }

    css_parts.join(" ")
}

// ── 搜索引擎 ─────────────────────────────────────────────────────────────

fn random_ua() -> &'static str {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36"
}

fn build_client(rule: &AnimeRule) -> reqwest::Client {
    let ua = if rule.user_agent.is_empty() { random_ua().to_string() } else { rule.user_agent.clone() };
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent(ua)
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default()
}

pub async fn search_anime(rule: &AnimeRule, keyword: &str) -> Result<Vec<SearchItem>, String> {
    let query_url = rule.search_url.replace("@keyword", &urlencoding::encode(keyword));

    let client = build_client(rule);
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("{}/", rule.base_url)) {
        headers.insert(reqwest::header::REFERER, v);
    }

    let html = if rule.use_post {
        let uri: url::Url = url::Url::parse(&query_url).map_err(|e| e.to_string())?;
        let params: Vec<(String, String)> = uri.query_pairs().map(|(k,v)| (k.into_owned(), v.into_owned())).collect();
        let post_url = format!("{}://{}{}", uri.scheme(), uri.host_str().unwrap_or(""), uri.path());
        client.post(&post_url)
            .headers(headers)
            .form(&params)
            .send().await.map_err(|e| format!("网络错误: {}", e))?
            .text().await.map_err(|e| e.to_string())?
    } else {
        client.get(&query_url)
            .headers(headers)
            .send().await.map_err(|e| format!("网络错误: {}", e))?
            .text().await.map_err(|e| e.to_string())?
    };

    let list_css = xpath_to_css(&rule.search_list);
    let name_css = xpath_to_css(&rule.search_name);
    let result_css = xpath_to_css(&rule.search_result);

    // HTML 解析是同步 CPU 密集操作。并发搜「全部源」时若在 async worker 线程上解析，
    // 十几个源会瞬间占满 tokio 工作线程池，导致 IPC / 计时器都跑不动、整库「卡死」。
    // 放到阻塞线程池：async 运行时保持响应，各源独立并发返回。
    let items = tokio::task::spawn_blocking(move || -> Result<Vec<SearchItem>, String> {
        let doc = scraper::Html::parse_document(&html);
        let list_sel = scraper::Selector::parse(&list_css)
            .map_err(|e| format!("选择器错误 (searchList: {}): {:?}", list_css, e))?;

        let mut items = Vec::new();
        for elem in doc.select(&list_sel) {
            let name = if name_css.is_empty() {
                elem.text().collect::<Vec<_>>().join("").trim().to_string()
            } else if let Ok(sel) = scraper::Selector::parse(&name_css) {
                elem.select(&sel).next()
                    .map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string())
                    .unwrap_or_default()
            } else {
                continue;
            };

            let url = if result_css.is_empty() {
                elem.value().attr("href").unwrap_or("").to_string()
            } else if let Ok(sel) = scraper::Selector::parse(&result_css) {
                elem.select(&sel).next()
                    .and_then(|e| e.value().attr("href"))
                    .unwrap_or("")
                    .to_string()
            } else {
                continue;
            };

            if !name.is_empty() && !url.is_empty() {
                items.push(SearchItem { name, url });
            }
        }
        Ok(items)
    })
    .await
    .map_err(|e| format!("解析任务失败: {}", e))??;

    Ok(items)
}

pub async fn fetch_roads(rule: &AnimeRule, page_url: &str) -> Result<Vec<Road>, String> {
    let full_url = if page_url.starts_with("http") {
        page_url.to_string()
    } else {
        format!("{}{}", rule.base_url.trim_end_matches('/'), if page_url.starts_with('/') { page_url.to_string() } else { format!("/{}", page_url) })
    };

    let client = build_client(rule);
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("{}/", rule.base_url)) {
        headers.insert(reqwest::header::REFERER, v);
    }

    let html = client.get(&full_url)
        .headers(headers)
        .send().await.map_err(|e| format!("网络错误: {}", e))?
        .text().await.map_err(|e| e.to_string())?;

    let roads_css = xpath_to_css(&rule.chapter_roads);
    let chapters_css = xpath_to_css(&rule.chapter_result);

    // 解析放到阻塞线程池，避免占用 async worker（与搜索同理，防止「获取线路中」卡死）
    let roads = tokio::task::spawn_blocking(move || -> Result<Vec<Road>, String> {
        let doc = scraper::Html::parse_document(&html);
        let roads_sel = scraper::Selector::parse(&roads_css)
            .map_err(|e| format!("选择器错误 (chapterRoads: {}): {:?}", roads_css, e))?;

        let mut roads = Vec::new();
        let mut count = 1;
        for road_elem in doc.select(&roads_sel) {
            let mut episodes = Vec::new();
            if let Ok(ch_sel) = scraper::Selector::parse(&chapters_css) {
                for ch in road_elem.select(&ch_sel) {
                    let href = ch.value().attr("href").unwrap_or("").to_string();
                    let name = ch.text().collect::<Vec<_>>().join("").trim().to_string()
                        .replace(char::is_whitespace, "");
                    if !href.is_empty() && !name.is_empty() {
                        episodes.push(Episode { name, url: href });
                    }
                }
            }
            if !episodes.is_empty() {
                roads.push(Road { name: format!("播放线路{}", count), episodes });
                count += 1;
            }
        }
        Ok(roads)
    })
    .await
    .map_err(|e| format!("解析任务失败: {}", e))??;

    Ok(roads)
}

pub fn build_full_url(rule: &AnimeRule, url: &str) -> String {
    if url.contains(&rule.base_url) || url.starts_with("http") {
        url.to_string()
    } else {
        format!("{}{}", rule.base_url.trim_end_matches('/'),
            if url.starts_with('/') { url.to_string() } else { format!("/{}", url) })
    }
}

// ── 图片代理（Bangumi 封面等外链图片通过 Rust 下载并缓存到本地）────────

fn proxy_client() -> &'static reqwest::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(8))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .danger_accept_invalid_certs(true)
            .pool_max_idle_per_host(8)
            .build()
            .unwrap_or_default()
    })
}

fn proxy_semaphore() -> &'static tokio::sync::Semaphore {
    use std::sync::OnceLock;
    static SEM: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    SEM.get_or_init(|| tokio::sync::Semaphore::new(6))
}

fn image_cache_path(url: &str) -> std::path::PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    url.hash(&mut h);
    let hash = h.finish();
    let cache_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("moeplay")
        .join("bgm_covers");
    let _ = std::fs::create_dir_all(&cache_dir);
    cache_dir.join(format!("{:016x}.jpg", hash))
}

pub async fn proxy_image(url: &str) -> Result<String, String> {
    if url.is_empty() { return Err("空 URL".into()); }

    let path = image_cache_path(url);
    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let _permit = proxy_semaphore().acquire().await.map_err(|e| e.to_string())?;

    // double-check after acquiring permit (another task may have downloaded it)
    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let client = proxy_client();
    let resp = client.get(url)
        .header("Referer", "https://bgm.tv/")
        .send().await
        .map_err(|e| format!("图片下载失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.is_empty() { return Err("空响应".into()); }

    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

pub async fn proxy_images_batch(urls: Vec<String>) -> Vec<(String, String)> {
    let futs: Vec<_> = urls.into_iter().map(|url| {
        async move {
            match proxy_image(&url).await {
                Ok(p) => Some((url, p)),
                Err(e) => {
                    tracing::debug!("图片代理跳过 {}: {}", url, e);
                    None
                }
            }
        }
    }).collect();
    futures_util::future::join_all(futs).await.into_iter().flatten().collect()
}

// ── GitHub 规则仓库 ─────────────────────────────────────────────────────

const RULES_BASE: &str = "https://raw.githubusercontent.com/Predidit/KazumiRules/main/";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RuleCatalogItem {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub use_native_player: bool,
    #[serde(default)]
    pub anti_crawler_enabled: bool,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub last_update: i64,
}

fn github_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("MoeGame/1.0")
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default()
}

pub async fn fetch_rules_index() -> Result<Vec<RuleCatalogItem>, String> {
    let url = format!("{}index.json", RULES_BASE);
    let resp = github_client().get(&url).send().await
        .map_err(|e| format!("无法连接 GitHub: {}", e))?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let raw: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("解析 index.json 失败: {}", e))?;
    let arr = raw.as_array().or_else(|| raw.get("value").and_then(|v| v.as_array()))
        .ok_or("index.json 格式错误")?;
    Ok(arr.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect())
}

pub async fn fetch_rule_by_name(name: &str) -> Result<AnimeRule, String> {
    let url = format!("{}{}.json", RULES_BASE, name);
    let resp = github_client().get(&url).send().await
        .map_err(|e| format!("下载规则失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("规则 {} 不存在 (HTTP {})", name, resp.status()));
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("解析规则 {} 失败: {}", name, e))
}

// ── Bangumi API ─────────────────────────────────────────────────────────

const BANGUMI_API: &str = "https://api.bgm.tv";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BangumiSubject {
    pub id: i64,
    pub name: String,
    pub name_cn: String,
    pub image: String,
    pub summary: String,
    pub air_date: String,
    pub air_weekday: i32,
    pub rating: f64,
    pub rank: i32,
    pub eps_count: i32,
}

impl BangumiSubject {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        let id = v.get("id")?.as_i64()?;
        let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let name_cn = v.get("name_cn").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let images = v.get("images").or_else(|| v.get("image"));
        let image = images
            .and_then(|i| i.get("common").or(i.get("large")).or(i.get("medium")))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let summary = v.get("summary").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let air_date = v.get("air_date").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let air_weekday = v.get("air_weekday").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let rating = v.get("rating")
            .and_then(|r| r.get("score").and_then(|s| s.as_f64()))
            .unwrap_or(0.0);
        let rank = v.get("rank").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let eps_count = v.get("total_episodes")
            .or(v.get("eps_count"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0) as i32;
        Some(Self { id, name, name_cn, image, summary, air_date, air_weekday, rating, rank, eps_count })
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct BangumiCalendarDay {
    pub weekday: i32,
    pub weekday_cn: String,
    pub items: Vec<BangumiSubject>,
}

// ── Bangumi 详情类型 ─────────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiSubjectDetail {
    pub id: i64,
    pub name: String,
    pub name_cn: String,
    pub summary: String,
    pub date: String,
    pub image: String,
    pub rating_score: f64,
    pub rating_total: i64,
    pub rank: i64,
    pub tags: Vec<BangumiTag>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiTag {
    pub name: String,
    pub count: i64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiRatingDetail {
    pub score: f64,
    pub total: i64,
    pub count: [i64; 11], // index 0 unused, 1-10 for scores
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiCharacter {
    pub id: i64,
    pub name: String,
    pub name_cn: String,
    pub image: String,
    pub actors: Vec<BangumiActor>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiActor {
    pub id: i64,
    pub name: String,
    pub name_cn: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiPerson {
    pub id: i64,
    pub name: String,
    pub name_cn: String,
    pub image: String,
    pub jobs: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiComment {
    pub user: String,
    pub avatar: String,
    pub rate: i64,
    pub comment: String,
    pub date: String,
}

fn bangumi_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("MoeGame/1.0 (https://github.com)")
        .build()
        .unwrap_or_default()
}

pub async fn fetch_bangumi_calendar() -> Result<Vec<BangumiCalendarDay>, String> {
    let url = format!("{}/calendar", BANGUMI_API);
    let resp = bangumi_client().get(&url).send().await
        .map_err(|e| format!("Bangumi 请求失败: {}", e))?;
    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("Bangumi 响应解析失败: {}", e))?;

    let weekday_names = ["", "星期一", "星期二", "星期三", "星期四", "星期五", "星期六", "星期日"];
    let arr = data.as_array().ok_or("Bangumi calendar 格式错误")?;

    let mut days = Vec::new();
    for entry in arr {
        let weekday_id = entry.get("weekday")
            .and_then(|w| w.get("id"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0) as i32;
        let items_arr = entry.get("items").and_then(|x| x.as_array());
        let items: Vec<BangumiSubject> = items_arr
            .map(|arr| arr.iter().filter_map(BangumiSubject::from_value).collect())
            .unwrap_or_default();
        let cn = weekday_names.get(weekday_id as usize).unwrap_or(&"").to_string();
        days.push(BangumiCalendarDay { weekday: weekday_id, weekday_cn: cn, items });
    }
    Ok(days)
}

pub async fn search_bangumi(keyword: &str, offset: u32, sort: &str, air_date_gte: &str, air_date_lte: &str) -> Result<(Vec<BangumiSubject>, i64), String> {
    let url = format!("{}/v0/search/subjects?limit=25&offset={}", BANGUMI_API, offset);
    let mut filter = serde_json::json!({ "type": [2] });
    let mut date_conditions: Vec<String> = Vec::new();
    if !air_date_gte.is_empty() {
        date_conditions.push(format!(">={}", air_date_gte));
    }
    if !air_date_lte.is_empty() {
        date_conditions.push(format!("<={}", air_date_lte));
    }
    if !date_conditions.is_empty() {
        filter["air_date"] = serde_json::json!(date_conditions);
    }
    let mut body = serde_json::json!({
        "keyword": keyword,
        "filter": filter,
    });
    if !sort.is_empty() {
        body["sort"] = serde_json::Value::String(sort.to_string());
    }
    let resp = bangumi_client()
        .post(&url)
        .json(&body)
        .send().await
        .map_err(|e| format!("Bangumi 搜索失败: {}", e))?;
    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("Bangumi 响应解析失败: {}", e))?;
    let total = data.get("total").and_then(|x| x.as_i64()).unwrap_or(0);
    let items: Vec<BangumiSubject> = data.get("data")
        .and_then(|x| x.as_array())
        .map(|arr| arr.iter().filter_map(BangumiSubject::from_value).collect())
        .unwrap_or_default();
    Ok((items, total))
}

// ── Bangumi 详情 API ────────────────────────────────────────────────────

pub async fn fetch_bangumi_subject_detail(subject_id: i64) -> Result<BangumiSubjectDetail, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build().unwrap_or_default();
    let url = format!("https://api.bgm.tv/v0/subjects/{}", subject_id);
    let resp = client.get(&url).header("Accept", "application/json").send().await.map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let tags: Vec<BangumiTag> = v.get("tags").and_then(|t| t.as_array()).map(|arr| {
        arr.iter().filter_map(|t| Some(BangumiTag {
            name: t.get("name")?.as_str()?.to_string(),
            count: t.get("count")?.as_i64().unwrap_or(0),
        })).collect()
    }).unwrap_or_default();
    Ok(BangumiSubjectDetail {
        id: v.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
        name: v.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        name_cn: v.get("name_cn").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        summary: v.get("summary").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        date: v.get("date").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        image: v.get("images").and_then(|i| i.get("large")).and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        rating_score: v.get("rating").and_then(|r| r.get("score")).and_then(|x| x.as_f64()).unwrap_or(0.0),
        rating_total: v.get("rating").and_then(|r| r.get("total")).and_then(|x| x.as_i64()).unwrap_or(0),
        rank: v.get("rank").and_then(|x| x.as_i64()).unwrap_or(0),
        tags,
    })
}

pub async fn fetch_bangumi_rating_detail(subject_id: i64) -> Result<BangumiRatingDetail, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build().unwrap_or_default();
    // Bangumi 没有独立的 /rating 端点：评分在主体 /v0/subjects/{id} 的 rating 字段里，
    // 且 count 是 {"1":n,…,"10":n} 对象（不是数组）。之前请求 /rating 拿到 404 错误 JSON
    // 被当成空评分解析 → 评分透视全为 0。
    let url = format!("https://api.bgm.tv/v0/subjects/{}", subject_id);
    let resp = client.get(&url).header("Accept", "application/json").send().await.map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let rating = v.get("rating");
    let mut count = [0i64; 11];
    if let Some(c) = rating.and_then(|r| r.get("count")) {
        if let Some(obj) = c.as_object() {
            // {"1": n, ... "10": n}
            for (k, val) in obj {
                if let Ok(i) = k.parse::<usize>() {
                    if i < 11 { count[i] = val.as_i64().unwrap_or(0); }
                }
            }
        } else if let Some(arr) = c.as_array() {
            for (i, val) in arr.iter().enumerate() {
                if i < 11 { count[i] = val.as_i64().unwrap_or(0); }
            }
        }
    }
    Ok(BangumiRatingDetail {
        score: rating.and_then(|r| r.get("score")).and_then(|x| x.as_f64()).unwrap_or(0.0),
        total: rating.and_then(|r| r.get("total")).and_then(|x| x.as_i64()).unwrap_or(0),
        count,
    })
}

pub async fn fetch_bangumi_characters(subject_id: i64) -> Result<Vec<BangumiCharacter>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build().unwrap_or_default();
    let url = format!("https://api.bgm.tv/v0/subjects/{}/characters", subject_id);
    let resp = client.get(&url).header("Accept", "application/json").send().await.map_err(|e| e.to_string())?;
    let arr: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = arr.as_array().cloned().unwrap_or_default();
    Ok(items.iter().map(|c| BangumiCharacter {
        id: c.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
        name: c.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        name_cn: c.get("name_cn").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        image: c.get("images").and_then(|i| i.get("large")).and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        actors: c.get("actors").and_then(|a| a.as_array()).map(|arr| arr.iter().map(|a| BangumiActor {
            id: a.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
            name: a.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            name_cn: a.get("name_cn").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        }).collect()).unwrap_or_default(),
    }).collect())
}

pub async fn fetch_bangumi_persons(subject_id: i64) -> Result<Vec<BangumiPerson>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build().unwrap_or_default();
    let url = format!("https://api.bgm.tv/v0/subjects/{}/persons", subject_id);
    let resp = client.get(&url).header("Accept", "application/json").send().await.map_err(|e| e.to_string())?;
    let arr: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = arr.as_array().cloned().unwrap_or_default();
    Ok(items.iter().map(|p| BangumiPerson {
        id: p.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
        name: p.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        name_cn: p.get("name_cn").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        image: p.get("images").and_then(|i| i.get("large")).and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        jobs: p.get("jobs").and_then(|j| j.as_array()).map(|arr| arr.iter().filter_map(|j| j.as_str().map(String::from)).collect()).unwrap_or_default(),
    }).collect())
}

pub async fn fetch_bangumi_comments(subject_id: i64, offset: u32) -> Result<Vec<BangumiComment>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build().unwrap_or_default();
    // 旧的 /v0/subjects/{id}/comments 端点已返回 404（被当成空数组解析 → 吐槽永远为空）。
    // 改用 next.bgm.tv 的 p1 API（与章节评论同源）。返回 { "data": [...], "total": n }，
    // 其中 user.avatar 是对象（small/medium/large），日期为 updatedAt 时间戳。
    let url = format!("https://next.bgm.tv/p1/subjects/{}/comments?offset={}&limit=20", subject_id, offset);
    let resp = client.get(&url).header("Accept", "application/json").send().await.map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = v.get("data").and_then(|d| d.as_array()).cloned().unwrap_or_default();
    Ok(items.iter().map(|c| BangumiComment {
        user: c.get("user").and_then(|u| u.get("nickname")).and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        avatar: c.get("user").and_then(|u| u.get("avatar"))
            .and_then(|a| a.get("large").or_else(|| a.get("medium")).or_else(|| a.get("small")))
            .and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        rate: c.get("rate").and_then(|x| x.as_i64()).unwrap_or(0),
        comment: c.get("comment").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        date: c.get("updatedAt").and_then(|x| x.as_i64()).map(|ts| {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_default()
        }).unwrap_or_default(),
    }).collect())
}

// ── Bangumi Collection Sync ──────────────────────────────────────────────
// Local collect_type: 1=在看 2=想看 3=搁置 4=看过 5=抛弃
// Bangumi type:       1=想看 2=看过 3=在看 4=搁置 5=抛弃
// Mapping local→bangumi: 1→3, 2→1, 3→4, 4→2, 5→5
// Mapping bangumi→local: 1→2, 2→4, 3→1, 4→3, 5→5

pub fn local_to_bangumi_type(local_type: u8) -> Option<u8> {
    match local_type {
        1 => Some(3), // 在看 → watching
        2 => Some(1), // 想看 → planToWatch
        3 => Some(4), // 搁置 → onHold
        4 => Some(2), // 看过 → watched
        5 => Some(5), // 抛弃 → abandoned
        _ => None,
    }
}

pub fn bangumi_to_local_type(bangumi_type: u8) -> Option<u8> {
    match bangumi_type {
        1 => Some(2), // planToWatch → 想看
        2 => Some(4), // watched → 看过
        3 => Some(1), // watching → 在看
        4 => Some(3), // onHold → 搁置
        5 => Some(5), // abandoned → 抛弃
        _ => None,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BangumiCollectionEntry {
    pub subject_id: i64,
    pub subject_name: String,
    pub subject_name_cn: String,
    pub subject_image: String,
    pub collection_type: u8,
    pub updated_at: String,
}

/// GET https://api.bgm.tv/v0/me — get username from token
pub async fn bangumi_get_username(token: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("moeplay/0.1")
        .build().unwrap_or_default();
    let url = format!("{}/v0/me", BANGUMI_API);
    let resp = client.get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send().await.map_err(|e| format!("网络错误: {}", e))?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Token 未授权，请检查你的 Bangumi Access Token".into());
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    v.get("username")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "无法获取用户名".into())
}

/// GET /v0/users/{username}/collections — get user's collection list (single page)
pub async fn bangumi_get_collection(
    username: &str,
    collection_type: u8,
    token: &str,
    offset: u32,
    limit: u32,
) -> Result<(Vec<BangumiCollectionEntry>, i64), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("moeplay/0.1")
        .build().unwrap_or_default();
    let url = format!(
        "{}/v0/users/{}/collections?type={}&limit={}&offset={}",
        BANGUMI_API, username, collection_type, limit, offset
    );
    let resp = client.get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send().await.map_err(|e| format!("网络错误: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("获取收藏失败: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let total = v.get("total").and_then(|x| x.as_i64()).unwrap_or(0);
    let data = v.get("data").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    let mut entries = Vec::new();
    for item in &data {
        let subject = match item.get("subject") {
            Some(s) => s,
            None => continue,
        };
        let entry = BangumiCollectionEntry {
            subject_id: subject.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
            subject_name: subject.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            subject_name_cn: subject.get("name_cn").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            subject_image: subject.get("images")
                .and_then(|i| i.get("common").or(i.get("large")).or(i.get("medium")))
                .and_then(|x| x.as_str()).unwrap_or("").to_string(),
            collection_type: bangumi_to_local_type(
                item.get("type").and_then(|x| x.as_u64()).unwrap_or(0) as u8
            ).unwrap_or(0),
            updated_at: item.get("updated_at").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        };
        entries.push(entry);
    }
    Ok((entries, total))
}

/// GET /v0/users/{username}/collections — fetch ALL pages for a given type
pub async fn bangumi_get_all_collections(
    username: &str,
    collection_type: u8,
    token: &str,
) -> Result<Vec<BangumiCollectionEntry>, String> {
    let page_size: u32 = 100;
    let mut offset: u32 = 0;
    let mut all = Vec::new();
    loop {
        let (items, total) = bangumi_get_collection(username, collection_type, token, offset, page_size).await?;
        let is_empty = items.is_empty();
        all.extend(items);
        if is_empty || (total > 0 && offset as i64 + page_size as i64 >= total) {
            break;
        }
        offset += page_size;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    Ok(all)
}

/// POST /v0/users/-/collections/{subject_id} — update collection type
pub async fn bangumi_update_collection(
    subject_id: i64,
    collection_type: u8,
    token: &str,
) -> Result<bool, String> {
    let bangumi_type = match local_to_bangumi_type(collection_type) {
        Some(t) => t,
        None => return Err(format!("无效的收藏类型: {}", collection_type)),
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("moeplay/0.1")
        .build().unwrap_or_default();
    let url = format!("{}/v0/users/-/collections/{}", BANGUMI_API, subject_id);
    let body = serde_json::json!({ "type": bangumi_type });
    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await.map_err(|e| format!("网络错误: {}", e))?;
    match resp.status().as_u16() {
        200..=299 => Ok(true),
        400 => Err("验证错误".into()),
        401 => Err("未经授权，请检查 Token".into()),
        404 => Err("条目不存在".into()),
        code => Err(format!("HTTP {}", code)),
    }
}

// ── Bangumi Episodes ──────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiEpisodeInfo {
    pub id: i32,
    pub name: String,
    pub name_cn: String,
    pub sort: i32,
    pub airdate: String,
    pub duration: String,
    pub desc: String,
    pub comment_count: i32,
}

pub async fn fetch_bangumi_episodes_list(subject_id: i64, offset: u32, limit: u32) -> Result<Vec<BangumiEpisodeInfo>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("moeplay/0.1.1")
        .build().unwrap_or_default();
    let url = format!("https://api.bgm.tv/v0/subjects/{}/eps?offset={}&limit={}", subject_id, offset, limit);
    let resp = client.get(&url)
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?;
    let arr: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = arr.as_array().cloned().unwrap_or_default();
    Ok(items.iter().map(|ep| BangumiEpisodeInfo {
        id: ep.get("id").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
        name: ep.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        name_cn: ep.get("name_cn").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        sort: ep.get("sort").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
        airdate: ep.get("airdate").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        duration: ep.get("duration").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        desc: ep.get("desc").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        comment_count: ep.get("comment").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
    }).collect())
}

// ── trace.moe 图片搜番 ─────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug, Default)]
pub struct TraceMoeResult {
    pub anilist_id: u32,
    pub filename: String,
    pub episode: String,
    pub from: f64,
    pub to: f64,
    pub similarity: f64,
    pub video: String,
    pub image: String,
    pub title_native: String,
    pub title_chinese: String,
    pub title_english: String,
}

pub async fn trace_moe_search(image_url: &str) -> Result<Vec<TraceMoeResult>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(random_ua())
        .build().unwrap_or_default();
    let url = format!("https://api.trace.moe/search?url={}", urlencoding::encode(image_url));
    let resp = client.get(&url)
        .header("Accept", "application/json")
        .send().await.map_err(|e| format!("trace.moe 请求失败: {}", e))?;
    let v: serde_json::Value = resp.json().await.map_err(|e| format!("trace.moe 响应解析失败: {}", e))?;
    if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
        if !err.is_empty() {
            return Err(format!("trace.moe 错误: {}", err));
        }
    }
    let results = v.get("result").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    Ok(results.iter().map(|r| {
        let ep_raw = r.get("episode");
        let episode = match ep_raw {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::Array(arr)) => {
                arr.iter().filter_map(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>().join("-")
            }
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        TraceMoeResult {
            anilist_id: r.get("anilist").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            filename: r.get("filename").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            episode,
            from: r.get("from").and_then(|x| x.as_f64()).unwrap_or(0.0),
            to: r.get("to").and_then(|x| x.as_f64()).unwrap_or(0.0),
            similarity: r.get("similarity").and_then(|x| x.as_f64()).unwrap_or(0.0),
            video: r.get("video").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            image: r.get("image").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            title_native: String::new(),
            title_chinese: String::new(),
            title_english: String::new(),
        }
    }).collect())
}

// ── Bangumi 章节评论 ────────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiEpisodeComment {
    pub user: String,
    pub avatar: String,
    pub comment: String,
    pub date: String,
}

pub async fn fetch_bangumi_episode_comments(episode_id: i64) -> Result<Vec<BangumiEpisodeComment>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build().unwrap_or_default();
    let url = format!("https://next.bgm.tv/p1/episodes/{}/comments", episode_id);
    let resp = client.get(&url)
        .header("Accept", "application/json")
        .send().await.map_err(|e| format!("Bangumi 章节评论请求失败: {}", e))?;
    let v: serde_json::Value = resp.json().await.map_err(|e| format!("Bangumi 章节评论解析失败: {}", e))?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let mut comments = Vec::new();
    for item in &arr {
        // Top-level comment
        let user = item.get("user").and_then(|u| u.get("nickname")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let avatar = item.get("user").and_then(|u| u.get("avatar")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let content = item.get("content").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let date = item.get("createdAt").and_then(|x| x.as_i64()).map(|ts| {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default()
        }).unwrap_or_default();
        if !content.is_empty() {
            comments.push(BangumiEpisodeComment { user, avatar, comment: content, date });
        }
        // Replies
        if let Some(replies) = item.get("replies").and_then(|x| x.as_array()) {
            for reply in replies {
                let r_user = reply.get("user").and_then(|u| u.get("nickname")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let r_avatar = reply.get("user").and_then(|u| u.get("avatar")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let r_content = reply.get("content").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let r_date = reply.get("createdAt").and_then(|x| x.as_i64()).map(|ts| {
                    chrono::DateTime::from_timestamp(ts, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_default()
                }).unwrap_or_default();
                if !r_content.is_empty() {
                    comments.push(BangumiEpisodeComment { user: r_user, avatar: r_avatar, comment: r_content, date: r_date });
                }
            }
        }
    }
    Ok(comments)
}

// ── DanDanPlay 弹幕 API ─────────────────────────────────────────────────

const DANDAN_API: &str = "https://api.dandanplay.net";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DanmakuComment {
    pub time: f64,
    pub mode: u8,
    pub color: u32,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DanmakuEpisode {
    pub episode_id: u32,
    pub episode_title: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DanmakuAnime {
    pub anime_id: u32,
    pub anime_title: String,
    pub episodes: Vec<DanmakuEpisode>,
}

fn dandan_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default()
}

pub async fn danmaku_search(keyword: &str) -> Result<Vec<DanmakuAnime>, String> {
    let url = format!("{}/api/v2/search/anime", DANDAN_API);
    let client = dandan_client();
    let resp = client.get(&url)
        .query(&[("keyword", keyword)])
        .send().await
        .map_err(|e| format!("DanDanPlay 搜索失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("DanDanPlay HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if body.get("errorCode").and_then(|v| v.as_i64()).unwrap_or(-1) != 0 {
        let msg = body.get("errorMessage").and_then(|v| v.as_str()).unwrap_or("未知错误");
        return Err(format!("DanDanPlay: {}", msg));
    }

    let animes_raw = body.get("animes").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut animes = Vec::new();

    for a in &animes_raw {
        let anime_id = a.get("animeId").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let anime_title = a.get("animeTitle").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let eps_raw = a.get("episodes").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let episodes: Vec<DanmakuEpisode> = eps_raw.iter().filter_map(|ep| {
            Some(DanmakuEpisode {
                episode_id: ep.get("episodeId")?.as_u64()? as u32,
                episode_title: ep.get("episodeTitle")?.as_str()?.to_string(),
            })
        }).collect();

        animes.push(DanmakuAnime { anime_id, anime_title, episodes });
    }

    Ok(animes)
}

pub async fn danmaku_get_episodes(anime_id: u32) -> Result<Vec<DanmakuEpisode>, String> {
    let url = format!("{}/api/v2/bangumi/{}", DANDAN_API, anime_id);
    let client = dandan_client();
    let resp = client.get(&url).send().await
        .map_err(|e| format!("DanDanPlay 获取分集失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("DanDanPlay HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if body.get("errorCode").and_then(|v| v.as_i64()).unwrap_or(-1) != 0 {
        let msg = body.get("errorMessage").and_then(|v| v.as_str()).unwrap_or("未知错误");
        return Err(format!("DanDanPlay: {}", msg));
    }

    let bangumi = body.get("bangumi").ok_or("缺少 bangumi 字段")?;
    let eps_raw = bangumi.get("episodes").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let episodes: Vec<DanmakuEpisode> = eps_raw.iter().filter_map(|ep| {
        Some(DanmakuEpisode {
            episode_id: ep.get("episodeId")?.as_u64()? as u32,
            episode_title: ep.get("episodeTitle")?.as_str()?.to_string(),
        })
    }).collect();

    Ok(episodes)
}

pub async fn danmaku_get_comments(episode_id: u32) -> Result<Vec<DanmakuComment>, String> {
    let url = format!("{}/api/v2/comment/{}", DANDAN_API, episode_id);
    let client = dandan_client();
    let resp = client.get(&url)
        .query(&[("withRelated", "true")])
        .send().await
        .map_err(|e| format!("DanDanPlay 获取弹幕失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("DanDanPlay HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if body.get("errorCode").and_then(|v| v.as_i64()).unwrap_or(-1) != 0 {
        let msg = body.get("errorMessage").and_then(|v| v.as_str()).unwrap_or("未知错误");
        return Err(format!("DanDanPlay: {}", msg));
    }

    let comments_raw = body.get("comments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut comments = Vec::new();

    for c in &comments_raw {
        let p = c.get("p").and_then(|v| v.as_str()).unwrap_or("");
        let m = c.get("m").and_then(|v| v.as_str()).unwrap_or("");

        let parts: Vec<&str> = p.split(',').collect();
        if parts.len() < 3 { continue; }

        let time = parts[0].parse::<f64>().unwrap_or(0.0);
        let mode = parts[1].parse::<u8>().unwrap_or(1);
        let color = parts[2].parse::<u32>().unwrap_or(0xFFFFFF);

        comments.push(DanmakuComment {
            time,
            mode,
            color,
            text: m.to_string(),
        });
    }

    Ok(comments)
}
