//! 番剧规则引擎 — 兼容 Kazumi 社区规则 JSON（XPath → CSS 自动转换）

use futures_util::StreamExt;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;

// ── 规则(Plugin)模型 — 1:1 映射 Kazumi JSON ─────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AnimeRule {
    #[serde(default, deserialize_with = "deserialize_api")]
    pub api: String,
    #[serde(default = "default_type")]
    pub r#type: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default = "bool_true")]
    pub muli_sources: bool,
    #[serde(default)]
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
    #[serde(default)]
    pub search_mode: String,
    #[serde(default)]
    pub chapter_mode: String,
    #[serde(default)]
    pub search_api_config: Option<SearchApiConfig>,
    #[serde(default)]
    pub chapter_api_config: Option<ChapterApiConfig>,
    #[serde(default)]
    pub anti_crawler_config: AntiCrawlerConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AntiCrawlerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, deserialize_with = "deserialize_stringish")]
    pub captcha_type: String,
    #[serde(default)]
    pub captcha_image: String,
    #[serde(default)]
    pub captcha_input: String,
    #[serde(default)]
    pub captcha_button: String,
    #[serde(default, deserialize_with = "deserialize_stringish")]
    pub captcha_detect_type: String,
    #[serde(default)]
    pub captcha_detect_value: String,
    #[serde(default)]
    pub captcha_script: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiRequestConfig {
    #[serde(default = "default_get_method")]
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub query: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchApiConfig {
    pub request: ApiRequestConfig,
    pub list_path: String,
    pub name_path: String,
    pub source_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct EpisodePageConfig {
    pub url: String,
    #[serde(default)]
    pub query: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChapterApiConfig {
    pub request: ApiRequestConfig,
    #[serde(default)]
    pub format: String,
    pub roads_path: String,
    pub road_name_path: String,
    pub episodes_path: String,
    pub episode_name_path: String,
    #[serde(default)]
    pub episode_url_path: String,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    #[serde(default)]
    pub episode_page: Option<EpisodePageConfig>,
}

fn default_type() -> String {
    "anime".into()
}
fn default_get_method() -> String {
    "GET".into()
}
fn bool_true() -> bool {
    true
}

fn deserialize_stringish<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::String(s)) => s,
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
        None => String::new(),
    })
}

fn deserialize_api<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_stringish(deserializer)
}

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
    if s.is_empty() {
        return String::new();
    }

    let s = if let Some(rest) = s.strip_prefix(".//") {
        rest
    } else if let Some(rest) = s.strip_prefix("//") {
        rest
    } else {
        s
    };
    let s = s.trim_start_matches('/');

    let mut css_parts = Vec::new();
    let segments: Vec<&str> = s.split('/').collect();

    for raw_seg in &segments {
        let seg: &str = raw_seg.trim();
        if seg.is_empty() {
            continue;
        }
        if seg == "text()" {
            continue;
        }
        if seg.starts_with('@') {
            continue;
        }

        if seg.contains('[') {
            // div[@class="xxx"] or div[contains(@class,'xxx')]
            if let Some(pos) = seg.find('[') {
                let tag = &seg[..pos];
                let pred = &seg[pos + 1..seg.len() - 1]; // strip []

                if pred.starts_with("@class=")
                    || pred.starts_with("@class=\"")
                    || pred.starts_with("@class='")
                {
                    let val = pred
                        .trim_start_matches("@class=")
                        .trim_matches('"')
                        .trim_matches('\'');
                    // multiple classes → .class1.class2
                    let classes: Vec<&str> = val.split_whitespace().collect();
                    let sel = format!("{}.{}", tag, classes.join("."));
                    css_parts.push(sel);
                } else if pred.starts_with("contains(@class,")
                    || pred.starts_with("contains(@class, ")
                {
                    let inner = pred
                        .trim_start_matches("contains(@class,")
                        .trim_start_matches("contains(@class, ")
                        .trim_end_matches(')')
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'');
                    css_parts.push(format!("{}[class*=\"{}\"]", tag, inner));
                } else if pred.starts_with("@id=") {
                    let val = pred
                        .trim_start_matches("@id=")
                        .trim_matches('"')
                        .trim_matches('\'');
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

fn shared_client() -> &'static reqwest::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .user_agent(random_ua())
            .danger_accept_invalid_certs(crate::http_client::insecure_tls_enabled())
            .pool_max_idle_per_host(8)
            .build()
            .unwrap_or_default()
    })
}

pub async fn search_anime(rule: &AnimeRule, keyword: &str) -> Result<Vec<SearchItem>, String> {
    if rule.search_mode.eq_ignore_ascii_case("api") {
        return search_anime_api(rule, keyword).await;
    }
    let query_path = rule
        .search_url
        .replace("@keyword", &urlencoding::encode(keyword));
    let query_url = build_full_url(rule, &query_path);

    let client = shared_client();
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("{}/", rule.base_url)) {
        headers.insert(reqwest::header::REFERER, v);
    }
    if !rule.user_agent.is_empty() {
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&rule.user_agent) {
            headers.insert(reqwest::header::USER_AGENT, v);
        }
    }

    let html = if rule.use_post {
        let uri: url::Url = url::Url::parse(&query_url).map_err(|e| e.to_string())?;
        let params: Vec<(String, String)> = uri
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let post_url = url_origin_and_path(&uri);
        client
            .post(&post_url)
            .headers(headers)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("网络错误: {}", e))?
            .text()
            .await
            .map_err(|e| e.to_string())?
    } else {
        client
            .get(&query_url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("网络错误: {}", e))?
            .text()
            .await
            .map_err(|e| e.to_string())?
    };

    if is_captcha_page(rule, &html) {
        return Err("CAPTCHA_REQUIRED: 源站需要验证".into());
    }

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
                elem.select(&sel)
                    .next()
                    .map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string())
                    .unwrap_or_default()
            } else {
                continue;
            };

            let url = if result_css.is_empty() {
                elem.value().attr("href").unwrap_or("").to_string()
            } else if let Ok(sel) = scraper::Selector::parse(&result_css) {
                elem.select(&sel)
                    .next()
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
    if rule.chapter_mode.eq_ignore_ascii_case("api") {
        return fetch_roads_api(rule, page_url).await;
    }
    let full_url = build_full_url(rule, page_url);

    let client = shared_client();
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("{}/", rule.base_url)) {
        headers.insert(reqwest::header::REFERER, v);
    }
    if !rule.user_agent.is_empty() {
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&rule.user_agent) {
            headers.insert(reqwest::header::USER_AGENT, v);
        }
    }

    let html = client
        .get(&full_url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    if is_captcha_page(rule, &html) {
        return Err("CAPTCHA_REQUIRED: 源站需要验证".into());
    }

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
                    let name = ch
                        .text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .to_string()
                        .replace(char::is_whitespace, "");
                    if !href.is_empty() && !name.is_empty() {
                        episodes.push(Episode { name, url: href });
                    }
                }
            }
            if !episodes.is_empty() {
                roads.push(Road {
                    name: format!("播放线路{}", count),
                    episodes,
                });
                count += 1;
            }
        }
        Ok(roads)
    })
    .await
    .map_err(|e| format!("解析任务失败: {}", e))??;

    Ok(roads)
}

async fn search_anime_api(rule: &AnimeRule, keyword: &str) -> Result<Vec<SearchItem>, String> {
    let config = rule
        .search_api_config
        .as_ref()
        .ok_or_else(|| "API search config is missing".to_string())?;
    let substitutions = BTreeMap::from([("keyword", keyword.to_string())]);
    let body = execute_rule_api_request(rule, &config.request, &substitutions).await?;
    let mut items = Vec::new();
    for entry in select_json_path(&body, &config.list_path)? {
        let name = json_path_scalar(entry, &config.name_path).unwrap_or_default();
        let source = json_path_scalar(entry, &config.source_path).unwrap_or_default();
        if !name.trim().is_empty() && !source.trim().is_empty() {
            items.push(SearchItem { name, url: source });
        }
    }
    Ok(items)
}

async fn fetch_roads_api(rule: &AnimeRule, source: &str) -> Result<Vec<Road>, String> {
    let config = rule
        .chapter_api_config
        .as_ref()
        .ok_or_else(|| "API chapter config is missing".to_string())?;
    if !config.format.is_empty() && !config.format.eq_ignore_ascii_case("nested") {
        return Err(format!("unsupported API chapter format: {}", config.format));
    }
    let substitutions = BTreeMap::from([("source", source.to_string())]);
    let body = execute_rule_api_request(rule, &config.request, &substitutions).await?;
    let mut variables = substitutions;
    for (name, path) in &config.variables {
        if let Some(value) = json_path_scalar(&body, path) {
            variables.insert(name.as_str(), value);
        }
    }

    let mut roads = Vec::new();
    for (road_index, road_value) in select_json_path(&body, &config.roads_path)?
        .into_iter()
        .enumerate()
    {
        let road_name = json_path_scalar(road_value, &config.road_name_path)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("播放线路{}", road_index + 1));
        let mut episodes = Vec::new();
        for (episode_index, episode_value) in select_json_path(road_value, &config.episodes_path)?
            .into_iter()
            .enumerate()
        {
            let name = json_path_scalar(episode_value, &config.episode_name_path)
                .unwrap_or_else(|| format!("第{}集", episode_index + 1));
            let url = if !config.episode_url_path.trim().is_empty() {
                json_path_scalar(episode_value, &config.episode_url_path).unwrap_or_default()
            } else if let Some(page) = &config.episode_page {
                build_episode_page_url(page, &variables, road_index, episode_index)?
            } else {
                String::new()
            };
            if !name.trim().is_empty() && !url.trim().is_empty() {
                episodes.push(Episode { name, url });
            }
        }
        if !episodes.is_empty() {
            roads.push(Road {
                name: road_name,
                episodes,
            });
        }
    }
    Ok(roads)
}

async fn execute_rule_api_request(
    rule: &AnimeRule,
    request: &ApiRequestConfig,
    substitutions: &BTreeMap<&str, String>,
) -> Result<serde_json::Value, String> {
    let url = substitute_rule_variables(&request.url, substitutions);
    let mut builder = match request.method.to_ascii_uppercase().as_str() {
        "GET" => shared_client().get(url),
        "POST" => shared_client().post(url),
        method => return Err(format!("unsupported API request method: {method}")),
    };
    let mut query = Vec::<(String, String)>::new();
    for (name, value) in &request.query {
        query.push((
            name.clone(),
            substitute_rule_variables(&json_value_text(value), substitutions),
        ));
    }
    if !query.is_empty() {
        builder = query_or_form(builder, &request.method, &query);
    }
    if !rule.referer.is_empty() {
        builder = builder.header(reqwest::header::REFERER, &rule.referer);
    }
    if !rule.user_agent.is_empty() {
        builder = builder.header(reqwest::header::USER_AGENT, &rule.user_agent);
    }
    let response = builder
        .send()
        .await
        .map_err(|error| format!("API source request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("API source returned HTTP {}", response.status()));
    }
    response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("API source JSON is invalid: {error}"))
}

fn query_or_form(
    builder: reqwest::RequestBuilder,
    method: &str,
    values: &[(String, String)],
) -> reqwest::RequestBuilder {
    if method.eq_ignore_ascii_case("POST") {
        builder.form(values)
    } else {
        builder.query(values)
    }
}

fn build_episode_page_url(
    page: &EpisodePageConfig,
    variables: &BTreeMap<&str, String>,
    road_index: usize,
    episode_index: usize,
) -> Result<String, String> {
    let mut substitutions = variables.clone();
    substitutions.insert("roadIndex", road_index.to_string());
    substitutions.insert("episodeIndex", episode_index.to_string());
    let base = substitute_rule_variables(&page.url, &substitutions);
    let mut url = url::Url::parse(&base).map_err(|error| error.to_string())?;
    {
        let mut pairs = url.query_pairs_mut();
        for (name, value) in &page.query {
            pairs.append_pair(
                name,
                &substitute_rule_variables(&json_value_text(value), &substitutions),
            );
        }
    }
    Ok(url.to_string())
}

fn substitute_rule_variables(input: &str, variables: &BTreeMap<&str, String>) -> String {
    variables
        .iter()
        .fold(input.to_string(), |value, (name, replacement)| {
            value.replace(&format!("@{name}"), replacement)
        })
}

fn json_value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn json_path_scalar(value: &serde_json::Value, path: &str) -> Option<String> {
    select_json_path(value, path)
        .ok()?
        .first()
        .map(|value| json_value_text(value))
}

fn select_json_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<Vec<&'a serde_json::Value>, String> {
    let path = path.trim();
    if path.is_empty() || path == "$" {
        return Ok(vec![value]);
    }
    let mut current = vec![value];
    for raw_segment in path
        .strip_prefix("$.")
        .or_else(|| path.strip_prefix('.'))
        .unwrap_or(path)
        .split('.')
    {
        let (name, expand_array) = raw_segment
            .strip_suffix("[*]")
            .map(|name| (name, true))
            .unwrap_or((raw_segment, false));
        if name.is_empty() {
            return Err(format!("invalid JSON path: {path}"));
        }
        let mut next = Vec::new();
        for candidate in current {
            let Some(child) = candidate.get(name) else {
                continue;
            };
            if expand_array {
                if let Some(values) = child.as_array() {
                    next.extend(values);
                }
            } else {
                next.push(child);
            }
        }
        current = next;
    }
    Ok(current)
}

pub fn build_full_url(rule: &AnimeRule, url: &str) -> String {
    join_rule_url(&rule.base_url, url)
}

pub fn join_rule_url(base_url: &str, url: &str) -> String {
    let raw = url.trim();
    if raw.is_empty() {
        return base_url.to_string();
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return raw.to_string();
    }
    if raw.starts_with("//") {
        let scheme = url::Url::parse(base_url)
            .ok()
            .map(|u| u.scheme().to_string())
            .unwrap_or_else(|| "https".into());
        return format!("{}:{}", scheme, raw);
    }

    if let Ok(base) = url::Url::parse(base_url) {
        if raw.starts_with('/') {
            let mut joined = format!("{}://{}", base.scheme(), base.host_str().unwrap_or(""));
            if let Some(port) = base.port() {
                joined.push_str(&format!(":{}", port));
            }
            joined.push_str(raw);
            return joined;
        }

        if base_url.ends_with('/') {
            return format!("{}{}", base_url, raw);
        }
        return format!("{}/{}", base_url.trim_end_matches('/'), raw);
    }

    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        raw.trim_start_matches('/')
    )
}

fn url_origin_and_path(uri: &url::Url) -> String {
    let mut output = format!("{}://{}", uri.scheme(), uri.host_str().unwrap_or(""));
    if let Some(port) = uri.port() {
        output.push_str(&format!(":{}", port));
    }
    output.push_str(uri.path());
    output
}

pub fn is_captcha_page(rule: &AnimeRule, html: &str) -> bool {
    let config = &rule.anti_crawler_config;
    if !config.enabled || html.trim().is_empty() {
        return false;
    }

    let detect_value = config.captcha_detect_value.trim();
    let detect_type = config.captcha_detect_type.trim();
    if !detect_value.is_empty() {
        if detect_type == "2" || detect_type.eq_ignore_ascii_case("text") {
            if html.contains(detect_value) {
                return true;
            }
        } else if detect_type == "3" || detect_type.eq_ignore_ascii_case("regex") {
            if regex::Regex::new(detect_value)
                .map(|re| re.is_match(html))
                .unwrap_or(false)
            {
                return true;
            }
        } else if captcha_selector_matches(html, detect_value) {
            return true;
        }
    }

    [
        &config.captcha_image,
        &config.captcha_input,
        &config.captcha_button,
    ]
    .iter()
    .any(|selector| captcha_selector_matches(html, selector))
}

fn captcha_selector_matches(html: &str, xpath: &str) -> bool {
    let css = xpath_to_css(xpath);
    if css.is_empty() {
        return false;
    }
    let Ok(selector) = scraper::Selector::parse(&css) else {
        return false;
    };
    let doc = scraper::Html::parse_document(html);
    doc.select(&selector).next().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_rule() -> AnimeRule {
        AnimeRule {
            api: String::new(),
            r#type: "anime".into(),
            name: "fixture".into(),
            version: String::new(),
            muli_sources: true,
            use_webview: false,
            use_native_player: true,
            use_post: false,
            use_legacy_parser: false,
            ad_blocker: false,
            user_agent: String::new(),
            base_url: "https://example.com/root".into(),
            search_url: "/search?wd=@keyword".into(),
            search_list: "//div".into(),
            search_name: ".//a".into(),
            search_result: ".//a".into(),
            chapter_roads: "//div".into(),
            chapter_result: ".//a".into(),
            referer: String::new(),
            search_mode: String::new(),
            chapter_mode: String::new(),
            search_api_config: None,
            chapter_api_config: None,
            anti_crawler_config: AntiCrawlerConfig::default(),
        }
    }

    #[test]
    fn kazumi_rule_accepts_numeric_api_and_anti_crawler_config() {
        let rule: AnimeRule = serde_json::from_value(serde_json::json!({
            "api": 13,
            "name": "fixture",
            "baseUrl": "https://example.com",
            "searchUrl": "/search?wd=@keyword",
            "searchList": "//div[@class='item']",
            "searchName": ".//a",
            "searchResult": ".//a",
            "chapterRoads": "//div[@class='road']",
            "chapterResult": ".//a",
            "antiCrawlerConfig": {
                "enabled": true,
                "captchaType": 2,
                "captchaButton": "//button[@id='verify']",
                "captchaDetectType": 2,
                "captchaDetectValue": "验证"
            }
        }))
        .expect("rule should deserialize");

        assert_eq!(rule.api, "13");
        assert!(!rule.use_webview);
        assert!(rule.use_native_player);
        assert!(rule.anti_crawler_config.enabled);
        assert_eq!(rule.anti_crawler_config.captcha_type, "2");
    }

    #[test]
    fn build_full_url_handles_kazumi_url_shapes() {
        let rule = base_rule();
        assert_eq!(build_full_url(&rule, ""), "https://example.com/root");
        assert_eq!(
            build_full_url(&rule, "https://cdn.test/a.m3u8"),
            "https://cdn.test/a.m3u8"
        );
        assert_eq!(
            build_full_url(&rule, "//cdn.test/a.m3u8"),
            "https://cdn.test/a.m3u8"
        );
        assert_eq!(
            build_full_url(&rule, "/play/1"),
            "https://example.com/play/1"
        );
        assert_eq!(
            build_full_url(&rule, "play/1"),
            "https://example.com/root/play/1"
        );
    }

    #[test]
    fn api_mode_rule_parses_nested_paths_and_builds_episode_pages() {
        let rule: AnimeRule = serde_json::from_value(serde_json::json!({
            "api": "8",
            "name": "api-fixture",
            "baseURL": "https://example.com/",
            "searchURL": "",
            "searchList": "",
            "searchName": "",
            "searchResult": "",
            "chapterRoads": "",
            "chapterResult": "",
            "searchMode": "api",
            "chapterMode": "api",
            "searchApiConfig": {
                "request": { "method": "GET", "url": "https://example.com/search", "query": { "q": "@keyword", "pageSize": 5 } },
                "listPath": "$.data.videos[*]",
                "namePath": "$.name",
                "sourcePath": "$.id"
            },
            "chapterApiConfig": {
                "request": { "method": "GET", "url": "https://example.com/videos/@source" },
                "format": "nested",
                "roadsPath": "$.data.playSources[*]",
                "roadNamePath": "$.name",
                "episodesPath": "$.episodes[*]",
                "episodeNamePath": "$.name",
                "episodeUrlPath": "",
                "variables": { "slug": "$.data.slug" },
                "episodePage": { "url": "https://example.com/video/@slug/play", "query": { "source": "@roadIndex", "episode": "@episodeIndex" } }
            }
        }))
        .unwrap();
        assert_eq!(rule.search_mode, "api");
        assert_eq!(rule.chapter_mode, "api");

        let body = serde_json::json!({"data":{"videos":[{"id":7,"name":"Fixture"}]}});
        let values = select_json_path(&body, "$.data.videos[*]").unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(
            json_path_scalar(values[0], "$.name").as_deref(),
            Some("Fixture")
        );
        assert_eq!(json_path_scalar(values[0], "$.id").as_deref(), Some("7"));

        let page = rule
            .chapter_api_config
            .as_ref()
            .and_then(|config| config.episode_page.as_ref())
            .unwrap();
        let variables = BTreeMap::from([("slug", "fixture-slug".to_string())]);
        let url = build_episode_page_url(page, &variables, 1, 3).unwrap();
        let parsed = url::Url::parse(&url).unwrap();
        assert_eq!(parsed.path(), "/video/fixture-slug/play");
        let query = parsed.query_pairs().collect::<BTreeMap<_, _>>();
        assert_eq!(query.get("source").map(|value| value.as_ref()), Some("1"));
        assert_eq!(query.get("episode").map(|value| value.as_ref()), Some("3"));
    }

    #[test]
    fn captcha_detection_uses_configured_text_or_selector() {
        let mut rule = base_rule();
        rule.anti_crawler_config = AntiCrawlerConfig {
            enabled: true,
            captcha_type: "1".into(),
            captcha_image: String::new(),
            captcha_input: "//input[@id='captcha']".into(),
            captcha_button: String::new(),
            captcha_detect_type: String::new(),
            captcha_detect_value: String::new(),
            captcha_script: String::new(),
        };
        assert!(is_captcha_page(
            &rule,
            "<html><input id='captcha' /></html>"
        ));

        rule.anti_crawler_config.captcha_detect_type = "2".into();
        rule.anti_crawler_config.captcha_detect_value = "请完成验证".into();
        assert!(is_captcha_page(&rule, "<html>请完成验证后继续</html>"));
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
            .danger_accept_invalid_certs(crate::http_client::insecure_tls_enabled())
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

pub fn prune_proxy_image_cache(max_age_days: i64, max_total_bytes: u64) -> Result<u64, String> {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("moeplay/bgm_covers");
    if !dir.exists() {
        return Ok(0);
    }
    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs((max_age_days.max(0) as u64) * 86_400);
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() {
            continue;
        }
        let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        files.push((entry.path(), meta.len(), modified));
    }
    let mut removed = 0;
    for (path, _, modified) in &files {
        if now.duration_since(*modified).unwrap_or_default() > max_age
            && std::fs::remove_file(path).is_ok()
        {
            removed += 1;
        }
    }
    files.retain(|(path, _, _)| path.exists());
    files.sort_by_key(|(_, _, modified)| *modified);
    let mut total: u64 = files.iter().map(|(_, size, _)| *size).sum();
    for (path, size, _) in files {
        if total <= max_total_bytes {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            total = total.saturating_sub(size);
            removed += 1;
        }
    }
    Ok(removed)
}

pub async fn proxy_image(url: &str) -> Result<String, String> {
    if url.is_empty() {
        return Err("空 URL".into());
    }
    let parsed = url::Url::parse(url).map_err(|_| "无效图片 URL".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("图片 URL 只允许不含凭据的 HTTP/HTTPS 地址".to_string());
    }

    let path = image_cache_path(url);
    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let _permit = proxy_semaphore()
        .acquire()
        .await
        .map_err(|e| e.to_string())?;

    // double-check after acquiring permit (another task may have downloaded it)
    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let client = proxy_client();
    let resp = client
        .get(url)
        .header("Referer", "https://bgm.tv/")
        .send()
        .await
        .map_err(|e| format!("图片下载失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
    if resp
        .content_length()
        .is_some_and(|size| size as usize > MAX_IMAGE_BYTES)
    {
        return Err("图片超过 20 MiB 限制".to_string());
    }
    let mut stream = resp.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if bytes.len().saturating_add(chunk.len()) > MAX_IMAGE_BYTES {
            return Err("图片超过 20 MiB 限制".to_string());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err("空响应".into());
    }
    image::load_from_memory(&bytes).map_err(|_| "上游响应不是有效图片".to_string())?;

    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

pub async fn proxy_images_batch(urls: Vec<String>) -> Vec<(String, String)> {
    let futs: Vec<_> = urls
        .into_iter()
        .map(|url| async move {
            match proxy_image(&url).await {
                Ok(p) => Some((url, p)),
                Err(e) => {
                    tracing::debug!("图片代理跳过 {}: {}", url, e);
                    None
                }
            }
        })
        .collect();
    futures_util::future::join_all(futs)
        .await
        .into_iter()
        .flatten()
        .collect()
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
    crate::http_client::build_reqwest_client(20, crate::http_client::app_user_agent())
}

pub async fn fetch_rules_index() -> Result<Vec<RuleCatalogItem>, String> {
    let url = format!("{}index.json", RULES_BASE);
    let resp = github_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("无法连接 GitHub: {}", e))?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let raw: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("解析 index.json 失败: {}", e))?;
    let arr = raw
        .as_array()
        .or_else(|| raw.get("value").and_then(|v| v.as_array()))
        .ok_or("index.json 格式错误")?;
    Ok(arr
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect())
}

pub async fn fetch_rule_by_name(name: &str) -> Result<AnimeRule, String> {
    let url = format!("{}{}.json", RULES_BASE, name);
    let resp = github_client()
        .get(&url)
        .send()
        .await
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
        let name = v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let name_cn = v
            .get("name_cn")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let images = v.get("images").or_else(|| v.get("image"));
        let image = images
            .and_then(|i| i.get("common").or(i.get("large")).or(i.get("medium")))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let summary = v
            .get("summary")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let air_date = v
            .get("air_date")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let air_weekday = v.get("air_weekday").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let rating = v
            .get("rating")
            .and_then(|r| r.get("score").and_then(|s| s.as_f64()))
            .unwrap_or(0.0);
        let rank = v.get("rank").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let eps_count = v
            .get("total_episodes")
            .or(v.get("eps_count"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0) as i32;
        Some(Self {
            id,
            name,
            name_cn,
            image,
            summary,
            air_date,
            air_weekday,
            rating,
            rank,
            eps_count,
        })
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
        .user_agent(crate::http_client::app_user_agent_with_context("github"))
        .build()
        .unwrap_or_default()
}

pub async fn fetch_bangumi_calendar() -> Result<Vec<BangumiCalendarDay>, String> {
    let url = format!("{}/calendar", BANGUMI_API);
    let resp = bangumi_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Bangumi 请求失败: {}", e))?
        .error_for_status()
        .map_err(|e| {
            format!(
                "Bangumi HTTP status {}",
                e.status().map(|s| s.as_u16()).unwrap_or(0)
            )
        })?;
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Bangumi 响应解析失败: {}", e))?;

    let weekday_names = [
        "",
        "星期一",
        "星期二",
        "星期三",
        "星期四",
        "星期五",
        "星期六",
        "星期日",
    ];
    let arr = data.as_array().ok_or("Bangumi calendar 格式错误")?;

    let mut days = Vec::new();
    for entry in arr {
        let weekday_id = entry
            .get("weekday")
            .and_then(|w| w.get("id"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0) as i32;
        let items_arr = entry.get("items").and_then(|x| x.as_array());
        let items: Vec<BangumiSubject> = items_arr
            .map(|arr| arr.iter().filter_map(BangumiSubject::from_value).collect())
            .unwrap_or_default();
        let cn = weekday_names
            .get(weekday_id as usize)
            .unwrap_or(&"")
            .to_string();
        days.push(BangumiCalendarDay {
            weekday: weekday_id,
            weekday_cn: cn,
            items,
        });
    }
    Ok(days)
}

pub async fn search_bangumi(
    keyword: &str,
    offset: u32,
    sort: &str,
    air_date_gte: &str,
    air_date_lte: &str,
) -> Result<(Vec<BangumiSubject>, i64), String> {
    let url = format!(
        "{}/v0/search/subjects?limit=25&offset={}",
        BANGUMI_API, offset
    );
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
        .send()
        .await
        .map_err(|e| format!("Bangumi 搜索失败: {}", e))?
        .error_for_status()
        .map_err(|e| {
            format!(
                "Bangumi search HTTP status {}",
                e.status().map(|s| s.as_u16()).unwrap_or(0)
            )
        })?;
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Bangumi 响应解析失败: {}", e))?;
    let total = data.get("total").and_then(|x| x.as_i64()).unwrap_or(0);
    let items: Vec<BangumiSubject> = data
        .get("data")
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
        .build()
        .unwrap_or_default();
    let url = format!("https://api.bgm.tv/v0/subjects/{}", subject_id);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let tags: Vec<BangumiTag> = v
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    Some(BangumiTag {
                        name: t.get("name")?.as_str()?.to_string(),
                        count: t.get("count")?.as_i64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(BangumiSubjectDetail {
        id: v.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
        name: v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        name_cn: v
            .get("name_cn")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        summary: v
            .get("summary")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        date: v
            .get("date")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        image: v
            .get("images")
            .and_then(|i| i.get("large"))
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        rating_score: v
            .get("rating")
            .and_then(|r| r.get("score"))
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        rating_total: v
            .get("rating")
            .and_then(|r| r.get("total"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0),
        rank: v.get("rank").and_then(|x| x.as_i64()).unwrap_or(0),
        tags,
    })
}

pub async fn fetch_bangumi_rating_detail(subject_id: i64) -> Result<BangumiRatingDetail, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build()
        .unwrap_or_default();
    // Bangumi 没有独立的 /rating 端点：评分在主体 /v0/subjects/{id} 的 rating 字段里，
    // 且 count 是 {"1":n,…,"10":n} 对象（不是数组）。之前请求 /rating 拿到 404 错误 JSON
    // 被当成空评分解析 → 评分透视全为 0。
    let url = format!("https://api.bgm.tv/v0/subjects/{}", subject_id);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let rating = v.get("rating");
    let mut count = [0i64; 11];
    if let Some(c) = rating.and_then(|r| r.get("count")) {
        if let Some(obj) = c.as_object() {
            // {"1": n, ... "10": n}
            for (k, val) in obj {
                if let Ok(i) = k.parse::<usize>() {
                    if i < 11 {
                        count[i] = val.as_i64().unwrap_or(0);
                    }
                }
            }
        } else if let Some(arr) = c.as_array() {
            for (i, val) in arr.iter().enumerate() {
                if i < 11 {
                    count[i] = val.as_i64().unwrap_or(0);
                }
            }
        }
    }
    Ok(BangumiRatingDetail {
        score: rating
            .and_then(|r| r.get("score"))
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        total: rating
            .and_then(|r| r.get("total"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0),
        count,
    })
}

pub async fn fetch_bangumi_characters(subject_id: i64) -> Result<Vec<BangumiCharacter>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build()
        .unwrap_or_default();
    let url = format!("https://api.bgm.tv/v0/subjects/{}/characters", subject_id);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let arr: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = arr.as_array().cloned().unwrap_or_default();
    Ok(items
        .iter()
        .map(|c| BangumiCharacter {
            id: c.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
            name: c
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            name_cn: c
                .get("name_cn")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            image: c
                .get("images")
                .and_then(|i| i.get("large"))
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            actors: c
                .get("actors")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|a| BangumiActor {
                            id: a.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
                            name: a
                                .get("name")
                                .and_then(|x| x.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            name_cn: a
                                .get("name_cn")
                                .and_then(|x| x.as_str())
                                .unwrap_or_default()
                                .to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect())
}

pub async fn fetch_bangumi_persons(subject_id: i64) -> Result<Vec<BangumiPerson>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build()
        .unwrap_or_default();
    let url = format!("https://api.bgm.tv/v0/subjects/{}/persons", subject_id);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let arr: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = arr.as_array().cloned().unwrap_or_default();
    Ok(items
        .iter()
        .map(|p| BangumiPerson {
            id: p.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
            name: p
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            name_cn: p
                .get("name_cn")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            image: p
                .get("images")
                .and_then(|i| i.get("large"))
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            jobs: p
                .get("jobs")
                .and_then(|j| j.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|j| j.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect())
}

pub async fn fetch_bangumi_comments(
    subject_id: i64,
    offset: u32,
) -> Result<Vec<BangumiComment>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build()
        .unwrap_or_default();
    // 旧的 /v0/subjects/{id}/comments 端点已返回 404（被当成空数组解析 → 吐槽永远为空）。
    // 改用 next.bgm.tv 的 p1 API（与章节评论同源）。返回 { "data": [...], "total": n }，
    // 其中 user.avatar 是对象（small/medium/large），日期为 updatedAt 时间戳。
    let url = format!(
        "https://next.bgm.tv/p1/subjects/{}/comments?offset={}&limit=20",
        subject_id, offset
    );
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = v
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(items
        .iter()
        .map(|c| BangumiComment {
            user: c
                .get("user")
                .and_then(|u| u.get("nickname"))
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            avatar: c
                .get("user")
                .and_then(|u| u.get("avatar"))
                .and_then(|a| {
                    a.get("large")
                        .or_else(|| a.get("medium"))
                        .or_else(|| a.get("small"))
                })
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            rate: c.get("rate").and_then(|x| x.as_i64()).unwrap_or(0),
            comment: c
                .get("comment")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            date: c
                .get("updatedAt")
                .and_then(|x| x.as_i64())
                .map(|ts| {
                    chrono::DateTime::from_timestamp(ts, 0)
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
        })
        .collect())
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
        .user_agent(crate::http_client::app_user_agent())
        .build()
        .unwrap_or_default();
    let url = format!("{}/v0/me", BANGUMI_API);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;
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
        .user_agent(crate::http_client::app_user_agent())
        .build()
        .unwrap_or_default();
    let url = format!(
        "{}/v0/users/{}/collections?type={}&limit={}&offset={}",
        BANGUMI_API, username, collection_type, limit, offset
    );
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("获取收藏失败: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let total = v.get("total").and_then(|x| x.as_i64()).unwrap_or(0);
    let data = v
        .get("data")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let mut entries = Vec::new();
    for item in &data {
        let subject = match item.get("subject") {
            Some(s) => s,
            None => continue,
        };
        let entry = BangumiCollectionEntry {
            subject_id: subject.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
            subject_name: subject
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            subject_name_cn: subject
                .get("name_cn")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            subject_image: subject
                .get("images")
                .and_then(|i| i.get("common").or(i.get("large")).or(i.get("medium")))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            collection_type: bangumi_to_local_type(
                item.get("type").and_then(|x| x.as_u64()).unwrap_or(0) as u8,
            )
            .unwrap_or(0),
            updated_at: item
                .get("updated_at")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
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
        let (items, total) =
            bangumi_get_collection(username, collection_type, token, offset, page_size).await?;
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
        .user_agent(crate::http_client::app_user_agent())
        .build()
        .unwrap_or_default();
    let url = format!("{}/v0/users/-/collections/{}", BANGUMI_API, subject_id);
    let body = serde_json::json!({ "type": bangumi_type });
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;
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

pub async fn fetch_bangumi_episodes_list(
    subject_id: i64,
    offset: u32,
    limit: u32,
) -> Result<Vec<BangumiEpisodeInfo>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(crate::http_client::app_user_agent())
        .build()
        .unwrap_or_default();
    let url = format!(
        "https://api.bgm.tv/v0/subjects/{}/eps?offset={}&limit={}",
        subject_id, offset, limit
    );
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let arr: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = arr.as_array().cloned().unwrap_or_default();
    Ok(items
        .iter()
        .map(|ep| BangumiEpisodeInfo {
            id: ep.get("id").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
            name: ep
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            name_cn: ep
                .get("name_cn")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            sort: ep.get("sort").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
            airdate: ep
                .get("airdate")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            duration: ep
                .get("duration")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            desc: ep
                .get("desc")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            comment_count: ep.get("comment").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
        })
        .collect())
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
        .build()
        .unwrap_or_default();
    let url = format!(
        "https://api.trace.moe/search?url={}",
        urlencoding::encode(image_url)
    );
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("trace.moe 请求失败: {}", e))?;
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("trace.moe 响应解析失败: {}", e))?;
    if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
        if !err.is_empty() {
            return Err(format!("trace.moe 错误: {}", err));
        }
    }
    let results = v
        .get("result")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(results
        .iter()
        .map(|r| {
            let ep_raw = r.get("episode");
            let episode = match ep_raw {
                Some(serde_json::Value::Number(n)) => n.to_string(),
                Some(serde_json::Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join("-"),
                Some(serde_json::Value::String(s)) => s.clone(),
                _ => String::new(),
            };
            TraceMoeResult {
                anilist_id: r.get("anilist").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                filename: r
                    .get("filename")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                episode,
                from: r.get("from").and_then(|x| x.as_f64()).unwrap_or(0.0),
                to: r.get("to").and_then(|x| x.as_f64()).unwrap_or(0.0),
                similarity: r.get("similarity").and_then(|x| x.as_f64()).unwrap_or(0.0),
                video: r
                    .get("video")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                image: r
                    .get("image")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                title_native: String::new(),
                title_chinese: String::new(),
                title_english: String::new(),
            }
        })
        .collect())
}

// ── Bangumi 章节评论 ────────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug, Default)]
pub struct BangumiEpisodeComment {
    pub user: String,
    pub avatar: String,
    pub comment: String,
    pub date: String,
}

pub async fn fetch_bangumi_episode_comments(
    episode_id: i64,
) -> Result<Vec<BangumiEpisodeComment>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(random_ua())
        .build()
        .unwrap_or_default();
    let url = format!("https://next.bgm.tv/p1/episodes/{}/comments", episode_id);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Bangumi 章节评论请求失败: {}", e))?;
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Bangumi 章节评论解析失败: {}", e))?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let mut comments = Vec::new();
    for item in &arr {
        // Top-level comment
        let user = item
            .get("user")
            .and_then(|u| u.get("nickname"))
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let avatar = item
            .get("user")
            .and_then(|u| u.get("avatar"))
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let content = item
            .get("content")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let date = item
            .get("createdAt")
            .and_then(|x| x.as_i64())
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        if !content.is_empty() {
            comments.push(BangumiEpisodeComment {
                user,
                avatar,
                comment: content,
                date,
            });
        }
        // Replies
        if let Some(replies) = item.get("replies").and_then(|x| x.as_array()) {
            for reply in replies {
                let r_user = reply
                    .get("user")
                    .and_then(|u| u.get("nickname"))
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                let r_avatar = reply
                    .get("user")
                    .and_then(|u| u.get("avatar"))
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                let r_content = reply
                    .get("content")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                let r_date = reply
                    .get("createdAt")
                    .and_then(|x| x.as_i64())
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(ts, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                if !r_content.is_empty() {
                    comments.push(BangumiEpisodeComment {
                        user: r_user,
                        avatar: r_avatar,
                        comment: r_content,
                        date: r_date,
                    });
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
    crate::http_client::build_reqwest_client(
        12,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
    )
}

pub async fn danmaku_search(keyword: &str) -> Result<Vec<DanmakuAnime>, String> {
    let url = format!("{}/api/v2/search/anime", DANDAN_API);
    let client = dandan_client();
    let resp = client
        .get(&url)
        .query(&[("keyword", keyword)])
        .send()
        .await
        .map_err(|e| format!("DanDanPlay 搜索失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("DanDanPlay HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if body.get("errorCode").and_then(|v| v.as_i64()).unwrap_or(-1) != 0 {
        let msg = body
            .get("errorMessage")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
        return Err(format!("DanDanPlay: {}", msg));
    }

    let animes_raw = body
        .get("animes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut animes = Vec::new();

    for a in &animes_raw {
        let anime_id = a.get("animeId").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let anime_title = a
            .get("animeTitle")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let eps_raw = a
            .get("episodes")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let episodes: Vec<DanmakuEpisode> = eps_raw
            .iter()
            .filter_map(|ep| {
                Some(DanmakuEpisode {
                    episode_id: ep.get("episodeId")?.as_u64()? as u32,
                    episode_title: ep.get("episodeTitle")?.as_str()?.to_string(),
                })
            })
            .collect();

        animes.push(DanmakuAnime {
            anime_id,
            anime_title,
            episodes,
        });
    }

    Ok(animes)
}

pub async fn danmaku_get_episodes(anime_id: u32) -> Result<Vec<DanmakuEpisode>, String> {
    let url = format!("{}/api/v2/bangumi/{}", DANDAN_API, anime_id);
    let client = dandan_client();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("DanDanPlay 获取分集失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("DanDanPlay HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if body.get("errorCode").and_then(|v| v.as_i64()).unwrap_or(-1) != 0 {
        let msg = body
            .get("errorMessage")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
        return Err(format!("DanDanPlay: {}", msg));
    }

    let bangumi = body.get("bangumi").ok_or("缺少 bangumi 字段")?;
    let eps_raw = bangumi
        .get("episodes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let episodes: Vec<DanmakuEpisode> = eps_raw
        .iter()
        .filter_map(|ep| {
            Some(DanmakuEpisode {
                episode_id: ep.get("episodeId")?.as_u64()? as u32,
                episode_title: ep.get("episodeTitle")?.as_str()?.to_string(),
            })
        })
        .collect();

    Ok(episodes)
}

pub async fn danmaku_get_comments(episode_id: u32) -> Result<Vec<DanmakuComment>, String> {
    let url = format!("{}/api/v2/comment/{}", DANDAN_API, episode_id);
    let client = dandan_client();
    let resp = client
        .get(&url)
        .query(&[("withRelated", "true")])
        .send()
        .await
        .map_err(|e| format!("DanDanPlay 获取弹幕失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("DanDanPlay HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if body.get("errorCode").and_then(|v| v.as_i64()).unwrap_or(-1) != 0 {
        let msg = body
            .get("errorMessage")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
        return Err(format!("DanDanPlay: {}", msg));
    }

    let comments_raw = body
        .get("comments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut comments = Vec::new();

    for c in &comments_raw {
        let p = c.get("p").and_then(|v| v.as_str()).unwrap_or("");
        let m = c.get("m").and_then(|v| v.as_str()).unwrap_or("");

        let parts: Vec<&str> = p.split(',').collect();
        if parts.len() < 3 {
            continue;
        }

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
