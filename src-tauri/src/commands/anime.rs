//! Tauri commands for the anime rule engine

use crate::anime::{self, AnimeRule, AnimeState};
use tauri::{Emitter, State};

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
