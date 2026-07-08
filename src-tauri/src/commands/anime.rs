//! Tauri commands for the anime rule engine

use crate::anime::{self, AnimeRule, AnimeState};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, State, WebviewUrl, WebviewWindowBuilder};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleWebviewQueryResult {
    pub status: String,
    pub message: String,
    pub url: String,
    pub items: Vec<anime::SearchItem>,
    pub roads: Vec<anime::Road>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthEvent {
    pub success: bool,
    #[serde(default)]
    pub failure_kind: Option<String>,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
    #[serde(default)]
    pub anime_name: Option<String>,
    #[serde(default)]
    pub timestamp: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct SourceHealthRecord {
    success: bool,
    #[serde(default)]
    failure_kind: Option<String>,
    #[serde(default)]
    elapsed_ms: Option<u64>,
    #[serde(default)]
    anime_name: Option<String>,
    timestamp: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthSummary {
    pub rule_name: String,
    pub recent_success_at: Option<i64>,
    pub failure_rate: f64,
    pub consecutive_failures: u32,
    pub avg_extract_ms: u64,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

fn source_health_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("moeplay")
        .join("anime_source_health.json")
}

fn read_source_health() -> HashMap<String, Vec<SourceHealthRecord>> {
    let path = source_health_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_source_health(map: &HashMap<String, Vec<SourceHealthRecord>>) -> Result<(), String> {
    let path = source_health_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

// ── 规则管理 ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_get_rules(state: State<'_, AnimeState>) -> Result<Vec<AnimeRule>, String> {
    let rules = state.rules.lock().map_err(|e| e.to_string())?;
    Ok(rules.clone())
}

#[tauri::command]
pub async fn anime_set_rules(
    state: State<'_, AnimeState>,
    rules: Vec<AnimeRule>,
) -> Result<(), String> {
    let mut store = state.rules.lock().map_err(|e| e.to_string())?;
    *store = rules;
    Ok(())
}

#[tauri::command]
pub async fn anime_add_rule(state: State<'_, AnimeState>, rule: AnimeRule) -> Result<(), String> {
    let mut store = state.rules.lock().map_err(|e| e.to_string())?;
    if let Some(pos) = store.iter().position(|r| r.name == rule.name) {
        store[pos] = rule;
    } else {
        store.push(rule);
    }
    Ok(())
}

#[tauri::command]
pub async fn anime_remove_rule(state: State<'_, AnimeState>, name: String) -> Result<(), String> {
    let mut store = state.rules.lock().map_err(|e| e.to_string())?;
    store.retain(|r| r.name != name);
    Ok(())
}

#[tauri::command]
pub async fn anime_import_rules(
    state: State<'_, AnimeState>,
    json: String,
) -> Result<usize, String> {
    let imported: Vec<AnimeRule> =
        serde_json::from_str(&json).map_err(|e| format!("JSON 解析失败: {}", e))?;
    let count = imported.len();
    let mut store = state.rules.lock().map_err(|e| e.to_string())?;
    for rule in imported {
        if let Some(pos) = store.iter().position(|r| r.name == rule.name) {
            store[pos] = rule;
        } else {
            store.push(rule);
        }
    }
    Ok(count)
}

// ── 搜索 & 章节 ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_search(
    state: State<'_, AnimeState>,
    rule_name: String,
    keyword: String,
) -> Result<Vec<anime::SearchItem>, String> {
    let rule = {
        let store = state.rules.lock().map_err(|e| e.to_string())?;
        let count = store.len();
        let found = store.iter().find(|r| r.name == rule_name).cloned();
        eprintln!(
            "[anime_search] rule='{}' keyword='{}' backend_rules={} found={}",
            rule_name,
            keyword,
            count,
            found.is_some()
        );
        found.ok_or_else(|| format!("规则 '{}' 不存在 (backend has {} rules)", rule_name, count))?
    };
    match tokio::time::timeout(
        std::time::Duration::from_secs(12),
        anime::search_anime(&rule, &keyword),
    )
    .await
    {
        Ok(Ok(items)) => {
            eprintln!(
                "[anime_search] rule='{}' → {} results",
                rule_name,
                items.len()
            );
            Ok(items)
        }
        Ok(Err(e)) => {
            eprintln!("[anime_search] rule='{}' → error: {}", rule_name, e);
            Err(e)
        }
        Err(_) => {
            eprintln!("[anime_search] rule='{}' → TIMEOUT 12s", rule_name);
            Err(format!("规则 '{}' 搜索超时", rule_name))
        }
    }
}

#[tauri::command]
pub async fn anime_search_all(
    app: tauri::AppHandle,
    state: State<'_, AnimeState>,
    keyword: String,
) -> Result<Vec<(String, Vec<anime::SearchItem>)>, String> {
    let rules = {
        let store = state.rules.lock().map_err(|e| e.to_string())?;
        store.clone()
    };
    let futures: Vec<_> = rules
        .iter()
        .map(|rule| {
            let rule = rule.clone();
            let kw = keyword.clone();
            let app = app.clone();
            async move {
                // 每条规则独立硬超时；一出结果就「流式」推给前端 —— 边搜边显示，不等全部完成（Kazumi 式体验）
                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    anime::search_anime(&rule, &kw),
                )
                .await
                {
                    Ok(Ok(items)) if !items.is_empty() => {
                        let _ = app.emit("anime-search-result", (rule.name.clone(), items.clone()));
                        Some((rule.name.clone(), items))
                    }
                    _ => None,
                }
            }
        })
        .collect();
    let all = futures_util::future::join_all(futures).await;
    let _ = app.emit("anime-search-done", ());
    Ok(all.into_iter().flatten().collect())
}

#[tauri::command]
pub async fn anime_fetch_roads(
    state: State<'_, AnimeState>,
    rule_name: String,
    page_url: String,
) -> Result<Vec<anime::Road>, String> {
    let rule = {
        let store = state.rules.lock().map_err(|e| e.to_string())?;
        store
            .iter()
            .find(|r| r.name == rule_name)
            .cloned()
            .ok_or_else(|| format!("规则 '{}' 不存在", rule_name))?
    };
    // 硬超时 15s — 防止 TLS 握手/响应卡死导致前端永远「获取线路中」
    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        anime::fetch_roads(&rule, &page_url),
    )
    .await
    {
        Ok(res) => res,
        Err(_) => Err(format!("规则 '{}' 获取线路超时 (15s)", rule_name)),
    }
}

#[tauri::command]
pub async fn anime_build_url(
    state: State<'_, AnimeState>,
    rule_name: String,
    url: String,
) -> Result<String, String> {
    let rule = {
        let store = state.rules.lock().map_err(|e| e.to_string())?;
        store
            .iter()
            .find(|r| r.name == rule_name)
            .cloned()
            .ok_or_else(|| format!("规则 '{}' 不存在", rule_name))?
    };
    Ok(anime::build_full_url(&rule, &url))
}

#[tauri::command]
pub async fn anime_verify_rule_webview(
    app: tauri::AppHandle,
    state: State<'_, AnimeState>,
    rule_name: String,
    keyword_or_url: String,
    mode: String,
) -> Result<RuleWebviewQueryResult, String> {
    let rule = {
        let store = state.rules.lock().map_err(|e| e.to_string())?;
        store
            .iter()
            .find(|r| r.name == rule_name)
            .cloned()
            .ok_or_else(|| format!("规则 '{}' 不存在", rule_name))?
    };

    let target_url = if mode == "roads" {
        anime::build_full_url(&rule, &keyword_or_url)
    } else {
        let search_path = rule
            .search_url
            .replace("@keyword", &urlencoding::encode(&keyword_or_url));
        anime::build_full_url(&rule, &search_path)
    };
    let parsed = target_url
        .parse()
        .map_err(|e| format!("验证页 URL 无效: {}", e))?;
    let label = format!(
        "anime-verify-{}-{}",
        sanitize_label(&rule.name),
        now_millis()
    );
    let init_script = verification_init_script(&rule);
    let mut builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed))
        .title(format!("源站验证 · {}", rule.name))
        .inner_size(980.0, 720.0)
        .min_inner_size(720.0, 520.0)
        .resizable(true)
        .center()
        .initialization_script(&init_script);
    if !rule.user_agent.is_empty() {
        builder = builder.user_agent(&rule.user_agent);
    }

    builder
        .build()
        .map_err(|e| format!("打开验证窗口失败: {}", e))?;

    Ok(RuleWebviewQueryResult {
        status: "opened".into(),
        message: "已打开源站验证窗口，完成后请重试该源".into(),
        url: target_url,
        items: Vec::new(),
        roads: Vec::new(),
    })
}

#[tauri::command]
pub fn anime_record_source_health(
    rule_name: String,
    result: SourceHealthEvent,
) -> Result<(), String> {
    if rule_name.trim().is_empty() {
        return Ok(());
    }
    let mut map = read_source_health();
    let records = map.entry(rule_name).or_default();
    records.push(SourceHealthRecord {
        success: result.success,
        failure_kind: result.failure_kind,
        elapsed_ms: result.elapsed_ms,
        anime_name: result.anime_name,
        timestamp: result.timestamp.unwrap_or_else(now_millis),
    });
    if records.len() > 20 {
        let keep_from = records.len().saturating_sub(20);
        records.drain(0..keep_from);
    }
    write_source_health(&map)
}

#[tauri::command]
pub fn anime_get_source_health() -> Result<Vec<SourceHealthSummary>, String> {
    let map = read_source_health();
    let summaries = map
        .into_iter()
        .map(|(rule_name, records)| summarize_source_health(rule_name, &records))
        .collect();
    Ok(summaries)
}

fn sanitize_label(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(40)
        .collect()
}

fn verification_init_script(rule: &AnimeRule) -> String {
    let config = &rule.anti_crawler_config;
    let button = serde_json::to_string(&config.captcha_button).unwrap_or_else(|_| "\"\"".into());
    let script = serde_json::to_string(&config.captcha_script).unwrap_or_else(|_| "\"\"".into());
    let captcha_type =
        serde_json::to_string(&config.captcha_type).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"
(() => {{
  const captchaType = {captcha_type};
  const buttonXPath = {button};
  const customScript = {script};
  const firstByXPath = (xpath) => {{
    if (!xpath) return null;
    try {{
      return document.evaluate(xpath, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue;
    }} catch (_) {{ return null; }}
  }};
  window.addEventListener('DOMContentLoaded', () => {{
    window.setTimeout(() => {{
      if (captchaType === '2') {{
        const button = firstByXPath(buttonXPath);
        if (button && typeof button.click === 'function') button.click();
      }}
      if (captchaType === '3' && customScript) {{
        try {{ (0, eval)(customScript); }} catch (_) {{}}
      }}
    }}, 900);
  }});
}})();
"#
    )
}

fn summarize_source_health(
    rule_name: String,
    records: &[SourceHealthRecord],
) -> SourceHealthSummary {
    let total = records.len().max(1) as f64;
    let failures = records.iter().filter(|r| !r.success).count() as f64;
    let recent_success_at = records
        .iter()
        .rev()
        .find(|r| r.success)
        .map(|r| r.timestamp);
    let consecutive_failures = records.iter().rev().take_while(|r| !r.success).count() as u32;
    let elapsed: Vec<u64> = records.iter().filter_map(|r| r.elapsed_ms).collect();
    let avg_extract_ms = if elapsed.is_empty() {
        0
    } else {
        elapsed.iter().sum::<u64>() / elapsed.len() as u64
    };
    SourceHealthSummary {
        rule_name,
        recent_success_at,
        failure_rate: failures / total,
        consecutive_failures,
        avg_extract_ms,
    }
}

// ── GitHub 规则仓库 ────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_github_rules_index() -> Result<Vec<anime::RuleCatalogItem>, String> {
    anime::fetch_rules_index().await
}

#[tauri::command]
pub async fn anime_install_github_rule(
    name: String,
    state: State<'_, AnimeState>,
) -> Result<anime::AnimeRule, String> {
    let rule = anime::fetch_rule_by_name(&name).await?;
    let mut store = state.rules.lock().map_err(|e| e.to_string())?;
    if let Some(pos) = store.iter().position(|r| r.name == rule.name) {
        store[pos] = rule.clone();
    } else {
        store.push(rule.clone());
    }
    Ok(rule)
}

#[tauri::command]
pub async fn anime_install_all_github_rules(
    names: Vec<String>,
    state: State<'_, AnimeState>,
) -> Result<usize, String> {
    let mut count = 0usize;
    for name in &names {
        match anime::fetch_rule_by_name(name).await {
            Ok(rule) => {
                let mut store = state.rules.lock().map_err(|e| e.to_string())?;
                if let Some(pos) = store.iter().position(|r| r.name == rule.name) {
                    store[pos] = rule;
                } else {
                    store.push(rule);
                }
                count += 1;
            }
            Err(e) => {
                tracing::warn!("跳过规则 {}: {}", name, e);
            }
        }
    }
    Ok(count)
}

// ── Bangumi ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_bangumi_calendar() -> Result<Vec<anime::BangumiCalendarDay>, String> {
    anime::fetch_bangumi_calendar().await
}

#[tauri::command]
pub async fn anime_bangumi_search(
    keyword: String,
    offset: Option<u32>,
    sort: Option<String>,
    air_date_gte: Option<String>,
    air_date_lte: Option<String>,
) -> Result<(Vec<anime::BangumiSubject>, i64), String> {
    anime::search_bangumi(
        &keyword,
        offset.unwrap_or(0),
        &sort.unwrap_or_default(),
        &air_date_gte.unwrap_or_default(),
        &air_date_lte.unwrap_or_default(),
    )
    .await
}

// ── 图片代理 ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_proxy_image(url: String) -> Result<String, String> {
    anime::proxy_image(&url).await
}

#[tauri::command]
pub async fn anime_proxy_images_batch(urls: Vec<String>) -> Result<Vec<(String, String)>, String> {
    Ok(anime::proxy_images_batch(urls).await)
}

// ── 收藏 & 历史（前端 localStorage 持久化，这里提供代理 fetch）─────────

#[tauri::command]
pub async fn anime_fetch_page(
    url: String,
    referer: Option<String>,
    user_agent: Option<String>,
) -> Result<String, String> {
    let ua = user_agent.unwrap_or_else(|| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36".into());
    let client = crate::http_client::build_reqwest_client(15, &ua);
    let mut req = client.get(&url);
    if let Some(ref r) = referer {
        if let Ok(v) = reqwest::header::HeaderValue::from_str(r) {
            req = req.header(reqwest::header::REFERER, v);
        }
    }
    let resp = req.send().await.map_err(|e| format!("网络错误: {}", e))?;
    resp.text().await.map_err(|e| e.to_string())
}

// ── Bangumi 详情 ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_bangumi_detail(subject_id: i64) -> Result<anime::BangumiSubjectDetail, String> {
    anime::fetch_bangumi_subject_detail(subject_id).await
}

#[tauri::command]
pub async fn anime_bangumi_rating(subject_id: i64) -> Result<anime::BangumiRatingDetail, String> {
    anime::fetch_bangumi_rating_detail(subject_id).await
}

#[tauri::command]
pub async fn anime_bangumi_characters(
    subject_id: i64,
) -> Result<Vec<anime::BangumiCharacter>, String> {
    anime::fetch_bangumi_characters(subject_id).await
}

#[tauri::command]
pub async fn anime_bangumi_persons(subject_id: i64) -> Result<Vec<anime::BangumiPerson>, String> {
    anime::fetch_bangumi_persons(subject_id).await
}

#[tauri::command]
pub async fn anime_bangumi_comments(
    subject_id: i64,
    offset: Option<u32>,
) -> Result<Vec<anime::BangumiComment>, String> {
    anime::fetch_bangumi_comments(subject_id, offset.unwrap_or(0)).await
}

#[tauri::command]
pub async fn anime_bangumi_episodes_list(
    subject_id: i64,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<Vec<anime::BangumiEpisodeInfo>, String> {
    anime::fetch_bangumi_episodes_list(subject_id, offset.unwrap_or(0), limit.unwrap_or(20)).await
}

// ── Bangumi 收藏同步 ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_bangumi_get_username(token: String) -> Result<String, String> {
    anime::bangumi_get_username(&token).await
}

#[tauri::command]
pub async fn anime_bangumi_get_user_collection(
    token: String,
    collection_type: Option<u8>,
    username: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<(Vec<anime::BangumiCollectionEntry>, i64), String> {
    // If no username, resolve it from the token first
    let resolved_username = match username {
        Some(u) if !u.is_empty() => u,
        _ => anime::bangumi_get_username(&token).await?,
    };
    // If collection_type == 0 or None, fetch all types (paginated by frontend)
    let ct = collection_type.unwrap_or(3); // default: 在看
    anime::bangumi_get_collection(
        &resolved_username,
        ct,
        &token,
        offset.unwrap_or(0),
        limit.unwrap_or(30),
    )
    .await
}

#[tauri::command]
pub async fn anime_bangumi_get_all_collections(
    token: String,
    username: Option<String>,
) -> Result<Vec<anime::BangumiCollectionEntry>, String> {
    let resolved_username = match username {
        Some(u) if !u.is_empty() => u,
        _ => anime::bangumi_get_username(&token).await?,
    };
    let mut all = Vec::new();
    // Fetch all 5 collection types (1=想看,2=看过,3=在看,4=搁置,5=抛弃)
    for bangumi_type in 1u8..=5 {
        match anime::bangumi_get_all_collections(&resolved_username, bangumi_type, &token).await {
            Ok(entries) => all.extend(entries),
            Err(e) => {
                tracing::warn!("获取收藏类型 {} 失败: {}", bangumi_type, e);
            }
        }
    }
    Ok(all)
}

#[tauri::command]
pub async fn anime_bangumi_update_collection(
    token: String,
    subject_id: i64,
    collection_type: u8,
) -> Result<bool, String> {
    anime::bangumi_update_collection(subject_id, collection_type, &token).await
}

// ── 视频代理 ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn anime_get_proxy_url(url: String, referer: Option<String>) -> String {
    let result = crate::video_proxy::to_proxy_url(&url, referer.as_deref());
    tracing::info!(
        "[前端调用] anime_get_proxy_url: url={}, referer={:?} → {}",
        &url[..url.len().min(80)],
        referer,
        &result[..result.len().min(80)]
    );
    result
}

/// 前端调试日志，写入 Rust tracing
#[tauri::command]
pub fn frontend_log(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!("[前端] {}", message),
        "warn" => tracing::warn!("[前端] {}", message),
        _ => tracing::info!("[前端] {}", message),
    }
}

// ── trace.moe 图片搜番 ────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_image_search(image_url: String) -> Result<Vec<anime::TraceMoeResult>, String> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(25),
        anime::trace_moe_search(&image_url),
    )
    .await
    {
        Ok(res) => res,
        Err(_) => Err("图片搜番超时".into()),
    }
}

// ── Bangumi 章节评论 ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_bangumi_episode_comments(
    episode_id: i64,
) -> Result<Vec<anime::BangumiEpisodeComment>, String> {
    anime::fetch_bangumi_episode_comments(episode_id).await
}

// ── DanDanPlay 弹幕 ────────────────────────────────────────────────────

#[tauri::command]
pub async fn anime_danmaku_search(keyword: String) -> Result<Vec<anime::DanmakuAnime>, String> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(12),
        anime::danmaku_search(&keyword),
    )
    .await
    {
        Ok(res) => res,
        Err(_) => Err("弹幕搜索超时".into()),
    }
}

#[tauri::command]
pub async fn anime_danmaku_get_episodes(
    anime_id: u32,
) -> Result<Vec<anime::DanmakuEpisode>, String> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(12),
        anime::danmaku_get_episodes(anime_id),
    )
    .await
    {
        Ok(res) => res,
        Err(_) => Err("获取弹幕分集超时".into()),
    }
}

#[tauri::command]
pub async fn anime_danmaku_get_comments(
    episode_id: u32,
) -> Result<Vec<anime::DanmakuComment>, String> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(12),
        anime::danmaku_get_comments(episode_id),
    )
    .await
    {
        Ok(res) => res,
        Err(_) => Err("获取弹幕超时".into()),
    }
}

// ── 外部播放器 ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn anime_get_external_players() -> Vec<crate::external_player::ExternalPlayerInfo> {
    crate::external_player::get_available_players()
}

#[tauri::command]
pub fn anime_launch_external_player(
    url: String,
    player: String,
    referer: Option<String>,
) -> Result<String, String> {
    crate::external_player::launch_external_player(&url, &player, referer.as_deref())
}

// ── 番剧下载 ──────────────────────────────────────────────────────────────

/// 下载番剧剧集（支持 m3u8/HLS 和直链）
#[tauri::command]
pub async fn anime_download_episode(
    dl: tauri::State<'_, crate::anime_download::AnimeDownloader>,
    url: String,
    filename: String,
    output_dir: Option<String>,
    anime_name: Option<String>,
    episode_name: Option<String>,
    referer: Option<String>,
) -> Result<crate::anime_download::AnimeDownloadTask, String> {
    Ok(dl
        .enqueue(url, filename, output_dir, anime_name, episode_name, referer)
        .await)
}

/// 获取所有番剧下载任务
#[tauri::command]
pub async fn anime_get_downloads(
    dl: tauri::State<'_, crate::anime_download::AnimeDownloader>,
) -> Result<Vec<crate::anime_download::AnimeDownloadTask>, String> {
    Ok(dl.get_all().await)
}

/// 取消番剧下载
#[tauri::command]
pub async fn anime_cancel_download(
    dl: tauri::State<'_, crate::anime_download::AnimeDownloader>,
    download_id: String,
) -> Result<(), String> {
    dl.cancel(&download_id).await
}

/// 暂停番剧下载
#[tauri::command]
pub async fn anime_pause_download(
    dl: tauri::State<'_, crate::anime_download::AnimeDownloader>,
    download_id: String,
) -> Result<(), String> {
    dl.pause(&download_id).await
}

/// 恢复番剧下载
#[tauri::command]
pub async fn anime_resume_download(
    dl: tauri::State<'_, crate::anime_download::AnimeDownloader>,
    download_id: String,
) -> Result<(), String> {
    dl.resume(&download_id).await
}

/// 移除番剧下载任务
#[tauri::command]
pub async fn anime_remove_download(
    dl: tauri::State<'_, crate::anime_download::AnimeDownloader>,
    download_id: String,
) -> Result<(), String> {
    dl.remove(&download_id).await
}

/// 清除已完成/取消/失败的番剧下载
#[tauri::command]
pub async fn anime_clear_finished_downloads(
    dl: tauri::State<'_, crate::anime_download::AnimeDownloader>,
) -> Result<(), String> {
    dl.clear_finished().await;
    Ok(())
}

/// 打开下载文件所在目录
#[tauri::command]
pub async fn anime_open_download_folder(
    dl: tauri::State<'_, crate::anime_download::AnimeDownloader>,
    download_id: String,
) -> Result<(), String> {
    dl.open_download_folder(&download_id).await
}
