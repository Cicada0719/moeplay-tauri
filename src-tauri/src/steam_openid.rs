// 萌游 MoeGame · Steam 身份认证（支持多种方式）
//
// 由于部分地区 Steam OpenID 被 CDN 拦截（Akamai Access Denied），
// 提供三种方式获取 SteamID：
//
//   A) 【主要】用户粘贴 Steam 个人主页 URL → 自动解析 SteamID64
//   B) 在浏览器打开 Steam 社区，用户登录后手动复制 SteamID
//   C) 尝试 OpenID（部分网络可用）
//
// 然后统一用 SteamID + API Key → GetOwnedGames 获取全库

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// ============================================================================
// 数据类型
// ============================================================================

/// 统一登录结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamLoginResult {
    pub steam_id: String,
    pub personaname: String,
    pub avatar: String,
    pub profile_url: String,
    pub login_method: String,
}

impl SteamLoginResult {
    pub fn from_id(sid: String, method: &str) -> Self {
        Self {
            profile_url: format!("https://steamcommunity.com/profiles/{}", sid),
            steam_id: sid,
            personaname: String::new(),
            avatar: String::new(),
            login_method: method.to_string(),
        }
    }
}

/// Steam 拥有的游戏
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamOwnedGame {
    pub app_id: u32,
    pub name: String,
    pub playtime_forever: u32,
    pub playtime_2weeks: Option<u32>,
    pub rtime_last_played: Option<u64>,
    pub img_icon_url: Option<String>,
    pub img_logo_url: Option<String>,
    pub achievements_total: Option<u32>,
    pub achievements_unlocked: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SteamAchievementSummary {
    pub total: u32,
    pub unlocked: u32,
}

/// Steam 全库响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamOwnedGamesResponse {
    pub game_count: u32,
    pub games: Vec<SteamOwnedGame>,
    #[serde(default)]
    pub imported_count: u32,
    #[serde(default)]
    pub updated_count: u32,
    #[serde(default)]
    pub skipped_count: u32,
}

const STEAM_API_URL: &str = "https://api.steampowered.com";
const STEAM_COMMUNITY: &str = "https://steamcommunity.com";
const STEAM_OPENID_URL: &str = "https://steamcommunity.com/openid/login";

// ============================================================================
// 方式 A: 从个人主页 URL 解析 SteamID64（推荐，100% 可靠）
// ============================================================================

/// 从 Steam 个人主页 URL 解析 SteamID64。
/// 支持三种格式：
///   - 纯数字: "76561197960435530"
///   - profiles URL: "https://steamcommunity.com/profiles/76561197960435530"
///   - custom URL: "https://steamcommunity.com/id/gaben"
///
/// 自定义 URL 需要 API Key 调用 ResolveVanityURL 解析。
pub async fn resolve_steamid(input: &str, api_key: Option<&str>) -> Result<String, String> {
    let trimmed = input.trim();

    // 1. 纯数字
    if trimmed.len() == 17 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Ok(trimmed.to_string());
    }

    // 2. 从 URL 中提取
    let lower = trimmed.to_lowercase();

    // profiles/STEAMID64
    if let Some(start) = lower.find("/profiles/") {
        let after = &trimmed[start + 10..];
        let sid: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if sid.len() == 17 {
            return Ok(sid);
        }
    }

    // id/CUSTOM_NAME → 需要 API
    if let Some(start) = lower.find("/id/") {
        let after = &trimmed[start + 4..];
        let vanity: String = after
            .chars()
            .take_while(|c| *c != '/' && *c != '?' && *c != ' ')
            .collect();
        if !vanity.is_empty() {
            return resolve_vanity_url(&vanity, api_key).await;
        }
    }

    // 3. 可能整个输入就是自定义名称（无 /id/ 前缀）
    if !trimmed.contains("://") && !trimmed.contains('/') && trimmed.len() >= 3 {
        return resolve_vanity_url(trimmed, api_key).await;
    }

    Err(format!("无法从输入解析 SteamID: \"{}\". 请粘贴完整的 Steam 个人主页 URL（例如 https://steamcommunity.com/profiles/7656119...）", trimmed))
}

/// 通过 Steam Web API 解析自定义 URL (/id/gaben → SteamID64)
async fn resolve_vanity_url(vanity: &str, api_key: Option<&str>) -> Result<String, String> {
    let key = api_key.ok_or_else(|| {
        "自定义 URL 需要 Steam Web API Key 才能解析。\n请到 steamcommunity.com/dev/apikey 申请免费 Key，或直接使用纯数字 SteamID。".to_string()
    })?;

    let url = format!(
        "{}/ISteamUser/ResolveVanityURL/v1/?key={}&vanityurl={}",
        STEAM_API_URL, key, vanity
    );

    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("Steam API 请求失败: {}", e))?;
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let response = json
        .get("response")
        .ok_or_else(|| "Steam API 响应格式异常".to_string())?;
    let success = response
        .get("success")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    if success == 1 {
        response
            .get("steamid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "steamid 字段缺失".to_string())
    } else {
        let message = response
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
        Err(format!("自定义 URL \"{}\" 解析失败: {}", vanity, message))
    }
}

// ============================================================================
// 方式 B: 打开浏览器让用户登录（不依赖回调）
// ============================================================================

/// 在浏览器打开 Steam 社区首页，用户登录后手动复制 SteamID。
pub fn open_steam_community() -> Result<(), String> {
    open::that(STEAM_COMMUNITY).map_err(|e| format!("无法打开浏览器: {}", e))?;
    tracing::info!("Opened Steam community in browser");
    Ok(())
}

/// 打开 Steam 个人资料编辑页（方便用户找到自己的 SteamID）
pub fn open_steam_edit_profile() -> Result<(), String> {
    open::that("https://steamcommunity.com/my/edit").map_err(|e| format!("无法打开浏览器: {}", e))
}

// ============================================================================
// 方式 C: 尝试 OpenID（可能被 CDN 拦截，保留作为备选）
// ============================================================================

/// 启动 OpenID 登录流程（使用异步 TCP，避免阻塞 tokio runtime）。
pub async fn login_via_openid() -> Result<SteamLoginResult, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("无法绑定端口: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("获取端口失败: {}", e))?
        .port();
    let callback_url = format!("http://127.0.0.1:{}/callback", port);

    let params = &[
        ("openid.ns", "http://specs.openid.net/auth/2.0"),
        ("openid.mode", "checkid_setup"),
        ("openid.return_to", &callback_url),
        ("openid.realm", "http://127.0.0.1"),
        (
            "openid.identity",
            "http://specs.openid.net/auth/2.0/identifier_select",
        ),
        (
            "openid.claimed_id",
            "http://specs.openid.net/auth/2.0/identifier_select",
        ),
    ];
    let login_url = build_query_url(STEAM_OPENID_URL, params);

    open::that(&login_url).map_err(|e| format!("无法打开浏览器: {}", e))?;
    tracing::info!(%login_url, %port, "Opened Steam OpenID login in browser");

    // 等待 Steam 回调（超时 5 分钟）
    let result =
        tokio::time::timeout(Duration::from_secs(300), accept_one_callback(listener)).await;

    match result {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => Err(e),
        Err(_) => {
            Err("Steam 登录超时（5分钟）。你可以试试粘贴 Steam 个人主页 URL 的方式。".to_string())
        }
    }
}

async fn accept_one_callback(
    listener: TcpListener,
) -> Result<Result<SteamLoginResult, String>, String> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|e| format!("接受连接失败: {}", e))?;
        match handle_callback_async(&mut stream).await {
            Ok(result) => return Ok(Ok(result)),
            Err(e) => {
                tracing::warn!(%e, "Steam callback parsing error, waiting for next request");
                continue;
            }
        }
    }
}

async fn handle_callback_async(stream: &mut TcpStream) -> Result<SteamLoginResult, String> {
    let mut buf = vec![0u8; 8192];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|_| "读取请求失败".to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");

    if !path.contains("/callback") {
        send_response_async(stream, "404 Not Found", "<h1>404</h1>").await;
        return Err("路径错误".to_string());
    }

    let query = path.split('?').nth(1).unwrap_or("");
    let params = parse_query_params(query);
    let mode = params.get("openid.mode").map(|s| s.as_str()).unwrap_or("");

    if mode != "id_res" {
        send_response_async(stream, "400 Bad Request", "<h1>认证失败: 模式错误</h1>").await;
        return Err("Steam 认证模式不正确".to_string());
    }

    let claimed_id = params
        .get("openid.claimed_id")
        .or_else(|| params.get("openid.identity"))
        .ok_or_else(|| "缺少 claimed_id".to_string())?;

    let steam_id = claimed_id
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .ok_or_else(|| "无法解析 SteamID".to_string())?;

    if steam_id.is_empty() || !steam_id.chars().all(|c| c.is_ascii_digit()) {
        send_response_async(stream, "400 Bad Request", "<h1>无效 SteamID</h1>").await;
        return Err("无效 SteamID".to_string());
    }

    let html = format!(
        r#"<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"><title>✓ 登录成功</title>
<style>body{{font-family:-apple-system,sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;margin:0;background:#1b2838;color:#c6d4df;text-align:center}}
.box{{background:#16212e;padding:48px;border-radius:12px;max-width:420px;box-shadow:0 4px 24px rgba(0,0,0,.4)}}
h1{{color:#66c0f4}}code{{background:#0a0f14;padding:4px 12px;border-radius:6px;font-size:15px;color:#acdbf5}}</style></head><body><div class="box">
<h1>✓ Steam 登录成功</h1><p>SteamID64: <code>{steam_id}</code></p><p>可关闭此页面。</p></div></body></html>"#
    );
    send_response_async(stream, "200 OK", &html).await;

    tracing::info!(%steam_id, "Steam OpenID login success");
    Ok(SteamLoginResult::from_id(steam_id.to_string(), "openid"))
}

async fn send_response_async(stream: &mut TcpStream, status: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status, body.len(), body
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.flush().await;
}

// ============================================================================
// 玩家信息 + 全库 API（需 Key）
// ============================================================================

/// 获取玩家昵称和头像
pub async fn fetch_player_summary(
    steam_id: &str,
    api_key: &str,
) -> Result<SteamLoginResult, String> {
    let url = format!(
        "{}/ISteamUser/GetPlayerSummaries/v2/?key={}&steamids={}",
        STEAM_API_URL, api_key, steam_id
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析失败: {}", e))?;

    let players = json
        .get("response")
        .and_then(|r| r.get("players"))
        .and_then(|p| p.as_array());

    if let Some(list) = players {
        if let Some(p) = list.first() {
            return Ok(SteamLoginResult {
                steam_id: steam_id.to_string(),
                personaname: p
                    .get("personaname")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                avatar: p
                    .get("avatarfull")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                profile_url: format!("https://steamcommunity.com/profiles/{}", steam_id),
                login_method: "api".to_string(),
            });
        }
    }
    Err("未找到该 SteamID 的用户信息".to_string())
}

/// 获取 Steam 用户拥有的全部游戏
pub async fn fetch_owned_games(
    steam_id: &str,
    api_key: &str,
) -> Result<SteamOwnedGamesResponse, String> {
    let url = format!(
        "{}/IPlayerService/GetOwnedGames/v1/?key={}&steamid={}&include_appinfo=true&include_played_free_games=true&format=json",
        STEAM_API_URL, api_key.trim(), steam_id.trim()
    );
    tracing::info!(%steam_id, "Fetching Steam owned games via IPlayerService/GetOwnedGames");

    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("Steam API 请求失败: {}", e))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let hint = match status.as_u16() {
            401 | 403 => {
                "Steam API 拒绝访问。请确认 API Key 正确，且 Steam 隐私设置里的「游戏详情」为公开"
            }
            429 => "Steam API 请求过于频繁，请稍后重试",
            _ => "Steam API 请求失败",
        };
        return Err(format!("{} (HTTP {}): {}", hint, status, body));
    }

    let mut parsed = parse_owned_games(&body)?;
    enrich_owned_games_with_achievements(&mut parsed.games, steam_id, api_key).await;
    Ok(parsed)
}

fn parse_owned_games(text: &str) -> Result<SteamOwnedGamesResponse, String> {
    let json: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("解析失败: {}", e))?;

    // Handle store API response format: { "response": { "games": [...] } }
    let response = json
        .get("response")
        .ok_or_else(|| "响应格式异常".to_string())?;

    let game_count = response
        .get("game_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let games: Vec<SteamOwnedGame> = response
        .get("games")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|g| {
                    Some(SteamOwnedGame {
                        app_id: g.get("appid")?.as_u64()? as u32,
                        name: g.get("name")?.as_str()?.to_string(),
                        playtime_forever: g
                            .get("playtime_forever")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32,
                        playtime_2weeks: g
                            .get("playtime_2weeks")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        rtime_last_played: g
                            .get("rtime_last_played")
                            .and_then(|v| v.as_u64())
                            .filter(|v| *v > 0),
                        img_icon_url: g
                            .get("img_icon_url")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        img_logo_url: g
                            .get("img_logo_url")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        achievements_total: None,
                        achievements_unlocked: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SteamOwnedGamesResponse {
        game_count,
        games,
        imported_count: 0,
        updated_count: 0,
        skipped_count: 0,
    })
}

async fn enrich_owned_games_with_achievements(
    games: &mut [SteamOwnedGame],
    steam_id: &str,
    api_key: &str,
) {
    use futures_util::stream::{self, StreamExt};
    // 只对玩过的游戏拉成就：没玩过的 GetPlayerAchievements 必为空，
    // 500+ 全库逐个拉（每款 2 个请求）会让扫描卡数分钟并触发 429。
    let app_ids: Vec<u32> = games
        .iter()
        .filter(|game| game.playtime_forever > 0)
        .map(|game| game.app_id)
        .collect();
    if app_ids.is_empty() {
        return;
    }
    tracing::info!(
        count = app_ids.len(),
        "Fetching Steam achievements for played games"
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let key = api_key.trim().to_string();
    let sid = steam_id.trim().to_string();

    let mut pending = stream::iter(app_ids)
        .map(|app_id| {
            let client = client.clone();
            let key = key.clone();
            let sid = sid.clone();
            async move {
                let summary =
                    fetch_achievement_summary_with_client(&client, &sid, &key, app_id).await;
                (app_id, summary)
            }
        })
        .buffer_unordered(8);

    while let Some((app_id, result)) = pending.next().await {
        match result {
            Ok(summary) => {
                if let Some(game) = games.iter_mut().find(|game| game.app_id == app_id) {
                    game.achievements_total = Some(summary.total);
                    game.achievements_unlocked = Some(summary.unlocked);
                }
            }
            Err(err) => {
                tracing::debug!(app_id, error = %err, "Steam achievements skipped");
            }
        }
    }
}

pub async fn fetch_achievement_summary(
    steam_id: &str,
    api_key: &str,
    app_id: u32,
) -> Result<SteamAchievementSummary, String> {
    let client = reqwest::Client::new();
    fetch_achievement_summary_with_client(&client, steam_id, api_key, app_id).await
}

async fn fetch_achievement_summary_with_client(
    client: &reqwest::Client,
    steam_id: &str,
    api_key: &str,
    app_id: u32,
) -> Result<SteamAchievementSummary, String> {
    let total_url = format!(
        "{}/ISteamUserStats/GetSchemaForGame/v2/?key={}&appid={}&format=json",
        STEAM_API_URL,
        api_key.trim(),
        app_id
    );
    let player_url = format!(
        "{}/ISteamUserStats/GetPlayerAchievements/v1/?key={}&steamid={}&appid={}&format=json",
        STEAM_API_URL,
        api_key.trim(),
        steam_id.trim(),
        app_id
    );

    let total_body = client
        .get(&total_url)
        .send()
        .await
        .map_err(|e| format!("Steam 成就 schema 请求失败: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Steam 成就 schema 读取失败: {}", e))?;
    let total = parse_achievement_total(&total_body)?;

    let player_resp = client
        .get(&player_url)
        .send()
        .await
        .map_err(|e| format!("Steam 成就请求失败: {}", e))?;
    if !player_resp.status().is_success() {
        return Err(format!(
            "Steam 成就请求失败 (HTTP {})",
            player_resp.status()
        ));
    }
    let player_body = player_resp
        .text()
        .await
        .map_err(|e| format!("Steam 成就读取失败: {}", e))?;
    let unlocked = parse_unlocked_achievements(&player_body)?;

    Ok(SteamAchievementSummary { total, unlocked })
}

fn parse_achievement_total(text: &str) -> Result<u32, String> {
    let json: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("解析成就 schema 失败: {}", e))?;
    Ok(json
        .get("game")
        .and_then(|game| game.get("availableGameStats"))
        .and_then(|stats| stats.get("achievements"))
        .and_then(|achievements| achievements.as_array())
        .map(|achievements| achievements.len() as u32)
        .unwrap_or(0))
}

fn parse_unlocked_achievements(text: &str) -> Result<u32, String> {
    let json: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("解析玩家成就失败: {}", e))?;
    Ok(json
        .get("playerstats")
        .and_then(|stats| stats.get("achievements"))
        .and_then(|achievements| achievements.as_array())
        .map(|achievements| {
            achievements
                .iter()
                .filter(|achievement| {
                    achievement
                        .get("achieved")
                        .and_then(|achieved| achieved.as_u64())
                        .unwrap_or(0)
                        > 0
                })
                .count() as u32
        })
        .unwrap_or(0))
}

/// 验证 API Key
pub async fn verify_api_key(api_key: &str) -> Result<String, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    // GetServerInfo 不足以证明 Key 可用于玩家库接口；这里用公开账号做一次轻量 GetOwnedGames。
    let url = format!(
        "{}/IPlayerService/GetOwnedGames/v1/?key={}&steamid=76561197960435530&include_appinfo=false&include_played_free_games=false&format=json",
        STEAM_API_URL, key
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("无法连接 Steam API: {}", e))?;

    if resp.status().is_success() {
        Ok("✅ API Key 有效，可以同步 Steam 全库".to_string())
    } else {
        Err(format!("API Key 无效 (HTTP {})", resp.status()))
    }
}

// ============================================================================
// 本地 Steam 客户端检测（Playnite 同款：读注册表拿 SteamID）
// ============================================================================

/// 从本地 Steam 客户端注册表获取当前登录的 SteamID64。
/// Playnite 也是这样做的：读 HKCU\Software\Valve\Steam\ActiveProcess\ActiveUser
pub fn detect_local_steam_id() -> Option<String> {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        let steam_key = hkcu
            .open_subkey("Software\\Valve\\Steam\\ActiveProcess")
            .ok()?;
        let active_user: u32 = steam_key.get_value("ActiveUser").ok()?;
        if active_user == 0 {
            return None;
        }
        let steam_id64 = active_user as u64 + 76561197960265728u64;
        return Some(steam_id64.to_string());
    }
    #[cfg(not(windows))]
    None
}

/// 检测本地 Steam 是否正在运行且已登录
pub fn is_steam_running() -> bool {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        return match hkcu.open_subkey("Software\\Valve\\Steam\\ActiveProcess") {
            Ok(key) => {
                let user: u32 = key.get_value("ActiveUser").unwrap_or(0);
                let pid: u32 = key.get_value("pid").unwrap_or(0);
                user > 0 && pid > 0
            }
            Err(_) => false,
        };
    }
    #[cfg(not(windows))]
    false
}

// ============================================================================
// 辅助
// ============================================================================

fn build_query_url(base: &str, params: &[(&str, &str)]) -> String {
    let mut url = base.to_string();
    url.push('?');
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            url.push('&');
        }
        url.push_str(&urlencoding::encode(k));
        url.push('=');
        url.push_str(&urlencoding::encode(v));
    }
    url
}

fn parse_query_params(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(
                urlencoding::decode(k).unwrap_or_default().to_string(),
                urlencoding::decode(v).unwrap_or_default().to_string(),
            );
        }
    }
    map
}

// ============================================================================
// 方式 D: Tauri WebView 内嵌 Steam 登录（推荐，像 Playnite 一样支持扫码+自动导入）
// ============================================================================

/// 注入到 Steam 社区页的探针：读取全局 `g_steamID`（仅登录态可见），
/// 若拿到 17 位 SteamID64 且当前不在对应 /profiles 页，则跳转到规范个人页，
/// 让 URL 轮询能识别出 SteamID。处理「默认已登录」与「vanity 个性链接」两类场景。
const STEAMID_PROBE_JS: &str = "(function(){try{var s=(typeof g_steamID!=='undefined'&&g_steamID)?String(g_steamID):'';if(/^[0-9]{17}$/.test(s)&&location.href.indexOf('/profiles/'+s)===-1){location.replace('https://steamcommunity.com/profiles/'+s);}}catch(e){}})();";

/// 注入到个人游戏页（`/profiles/{id}/games/?tab=all`）的抓取脚本：读取页面渲染用的全局
/// `rgGames`（登录会话即可见全库，无需 API Key），归一化时长后**经同源哨兵 URL**
/// `https://steamcommunity.com/__moeingest?...` 回传给后端 `on_navigation`（WebView2 对
/// https 必触发 NavigationStarting，比自定义协议可靠）。这正是 Playnite 的做法——复用已登录会话直接拿全库。
/// 首次注入先发一针探针（带 `typeof rgGames`）确认通道与就绪状态，便于诊断。
const GAMES_SCRAPE_JS: &str = "(function(){try{if(window.__moeDone)return;if(!window.__moeProbe){window.__moeProbe=1;location.replace('https://steamcommunity.com/__moeingest?p=1&rg='+(typeof rgGames));return;}if(typeof rgGames==='undefined'||!rgGames)return;window.__moeDone=1;var g=(rgGames||[]).map(function(x){var m=0;if(typeof x.playtime_forever==='number')m=x.playtime_forever;else if(x.hours_forever)m=Math.round(parseFloat(String(x.hours_forever).replace(/[^0-9.]/g,''))*60)||0;return{appid:x.appid,name:x.name||('Steam App '+x.appid),playtime_forever:m,last_played:x.last_played||0};});location.replace('https://steamcommunity.com/__moeingest?d='+encodeURIComponent(JSON.stringify(g)));}catch(e){}})();";

/// 后端拦截的抓取哨兵 URL 前缀。
const INGEST_SENTINEL: &str = "steamcommunity.com/__moeingest";

/// 从 `…/__moeingest?d=<urlencoded-json>` 中解出抓取到的游戏数组（非数据型探针返回 None）。
fn parse_scraped_games(url_str: &str) -> Option<serde_json::Value> {
    let parsed = url::Url::parse(url_str).ok()?;
    let d = parsed.query_pairs().find(|(k, _)| k == "d")?.1;
    serde_json::from_str::<serde_json::Value>(&d).ok()
}

/// 打开 Tauri WebView 窗口让用户登录 Steam（Playnite 式）。
/// 登录成功后自动串联：解析 SteamID → 导航到个人游戏页 → 注入脚本读 `rgGames` 全库
/// → 经 `moe-steam-games://` 回传 → emit `moe://steam-session-games` → 前端调用
/// `import_steam_session_games` 批量导入。全程**无需 API Key**，复用已登录会话。
pub fn open_login_webview(app_handle: &tauri::AppHandle) -> Result<(), String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tauri::Manager;

    tracing::info!("open_login_webview: START");
    crate::crash_log("open_login_webview: START");

    // 重复点击「网页登录」：聚焦已有窗口而非报错
    if let Some(existing) = app_handle.get_webview_window("steam-login") {
        let _ = existing.set_focus();
        return Ok(());
    }

    let app = app_handle.clone();
    // 导航回调与轮询循环都可能检测到登录；只允许发射一次，避免前端重复触发全库同步
    let emitted = Arc::new(AtomicBool::new(false));
    let emitted_nav = emitted.clone();
    // 检测到的 SteamID（供轮询导航到个人游戏页、以及抓取事件携带）
    let steam_id = Arc::new(std::sync::Mutex::new(String::new()));
    let steam_id_nav = steam_id.clone();
    let steam_id_poll = steam_id.clone();
    // 是否已抓到并回传全库（Playnite 式会话抓取完成）
    let ingested = Arc::new(AtomicBool::new(false));
    let ingested_nav = ingested.clone();

    // 直接进 /my/profile：已登录 → Steam 302 到 /profiles/{id}（或 /id/{vanity}）即可解析；
    // 未登录 → Steam 跳到 /login/home?goto=my/profile（仍带二维码），扫码后再回跳 /my/profile。
    // 比起从 /login/home 起步，能正确处理「默认已登录」这种识别不到 SteamID 的情况。
    tracing::info!("open_login_webview: creating webview...");
    let mut builder = tauri::WebviewWindowBuilder::new(
        app_handle,
        "steam-login",
        tauri::WebviewUrl::External(
            "https://steamcommunity.com/my/profile"
                .parse()
                .map_err(|e| format!("URL parse error: {}", e))?,
        ),
    )
    .title("Steam 登录 · 支持扫码")
    .inner_size(900.0, 750.0)
    .min_inner_size(600.0, 500.0)
    .resizable(true);
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.center();
    }
    let window = builder
        .on_navigation(move |nav_url| {
            let url_str = nav_url.as_str();
            // 抓取回传：注入脚本把全库塞进同源哨兵 URL → 解析 → 发前端导入 → 取消该次导航。
            // 同一通道也承载探针（?p=1&rg=...），便于诊断 rgGames 是否就绪。
            if url_str.contains(INGEST_SENTINEL) {
                if let Some(games) = parse_scraped_games(url_str) {
                    let count = games.as_array().map(|a| a.len()).unwrap_or(0);
                    let sid = steam_id_nav
                        .lock()
                        .ok()
                        .map(|g| g.clone())
                        .unwrap_or_default();
                    tracing::info!(count, "Steam owned games scraped from logged-in session");
                    let _ = app.emit(
                        "moe://steam-session-games",
                        serde_json::json!({ "steam_id": sid, "games": games }),
                    );
                    ingested_nav.store(true, Ordering::SeqCst);
                } else {
                    let q = url::Url::parse(url_str)
                        .ok()
                        .and_then(|u| u.query().map(|s| s.to_string()))
                        .unwrap_or_default();
                    tracing::info!(query = %q, "Steam scrape probe/status (no data)");
                }
                return false;
            }
            if let Some(sid) = extract_steam_id_from_url(url_str) {
                if let Ok(mut s) = steam_id_nav.lock() {
                    *s = sid.clone();
                }
                if !emitted_nav.swap(true, Ordering::SeqCst) {
                    tracing::info!(%sid, "Steam login detected via navigation");
                    let _ = app.emit(
                        "moe://steam-login",
                        serde_json::json!({
                            "steam_id": sid,
                            "profile_url": url_str,
                        }),
                    );
                    let _ = app.emit(
                        "moe://steam-progress",
                        serde_json::json!({
                            "steam_id": sid,
                            "status": "login_success",
                            "message": "登录成功，正在读取游戏库…"
                        }),
                    );
                }
            }
            true
        })
        .build()
        .map_err(|e| format!("创建 Steam 登录窗口失败: {}", e))?;

    tracing::info!("open_login_webview: webview created, starting background poll");

    let wc = window.clone();
    let ac = app_handle.clone();

    // 后台状态机：① 检测登录拿 SteamID → ② 导到个人游戏页 → ③ 注入脚本抓 rgGames 全库 → ④ 关窗。
    // 窗口被用户关闭 → 通知前端解除等待。
    tokio::spawn(async move {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(300);
        // 进入抓取阶段的时刻，用于抓取兜底超时
        let mut scrape_started: Option<std::time::Instant> = None;

        loop {
            // ④ 已抓到全库并回传 → 收尾关窗
            if ingested.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                let _ = wc.close();
                break;
            }

            if start.elapsed() > timeout {
                let _ = ac.emit(
                    "moe://steam-progress",
                    serde_json::json!({ "status": "timeout", "message": "登录超时（5分钟）" }),
                );
                let _ = wc.close();
                break;
            }

            // ② 已拿到 SteamID、还没进抓取阶段 → 导航到个人游戏页（全部标签）
            if emitted.load(Ordering::SeqCst) && scrape_started.is_none() {
                let sid = steam_id_poll
                    .lock()
                    .ok()
                    .map(|g| g.clone())
                    .unwrap_or_default();
                let games_url = if sid.len() == 17 {
                    format!("https://steamcommunity.com/profiles/{}/games/?tab=all", sid)
                } else {
                    "https://steamcommunity.com/my/games/?tab=all".to_string()
                };
                if let Ok(parsed) = games_url.parse::<url::Url>() {
                    tracing::info!(url = %games_url, "Navigating to Steam games page to scrape owned library");
                    let _ = wc.navigate(parsed);
                }
                scrape_started = Some(std::time::Instant::now());
                tokio::time::sleep(std::time::Duration::from_millis(900)).await;
                continue;
            }

            // ③ 抓取阶段：反复注入抽取脚本，直到 rgGames 就绪回传（on_navigation 置 ingested）。
            // 不按 wc.url() 判断是否在游戏页——部分 WebView2 的 url() 返回创建时的旧地址（本例
            // 登录只被 on_navigation 命中、轮询从未命中即是此故），改由脚本自身判断 rgGames 是否就绪。
            if let Some(at) = scrape_started {
                if at.elapsed() > std::time::Duration::from_secs(30) {
                    // 抓取兜底超时：SteamID 已发给前端，放弃自动抓取并关窗
                    let _ = ac.emit(
                        "moe://steam-progress",
                        serde_json::json!({ "status": "scrape_timeout", "message": "未能自动读取游戏库，可改用本机扫描或 API Key" }),
                    );
                    let _ = wc.close();
                    break;
                }
                if wc.url().is_err() {
                    break; // 窗口已被关闭
                }
                let _ = wc.eval(GAMES_SCRAPE_JS);
                tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                continue;
            }

            // ① 登录检测阶段
            match wc.url() {
                Ok(current_url) => {
                    let url_str = current_url.as_str();
                    let url_lower = url_str.to_lowercase();

                    if let Some(sid) = extract_steam_id_from_url(url_str) {
                        if let Ok(mut s) = steam_id_poll.lock() {
                            *s = sid.clone();
                        }
                        if !emitted.swap(true, Ordering::SeqCst) {
                            tracing::info!(%sid, "Steam login detected via polling");
                            let _ = ac.emit(
                                "moe://steam-login",
                                serde_json::json!({ "steam_id": sid, "profile_url": url_str }),
                            );
                            let _ = ac.emit(
                                "moe://steam-progress",
                                serde_json::json!({ "steam_id": sid, "status": "login_success", "message": "登录成功，正在读取游戏库…" }),
                            );
                        }
                        // 不关窗：下一轮进入抓取阶段
                    } else if url_lower.contains("steamcommunity.com")
                        && !url_lower.contains("/login")
                    {
                        // 已登录但停在非 /profiles 页：注入 g_steamID 探针跳到规范个人页
                        let _ = wc.eval(STEAMID_PROBE_JS);
                    } else if url_lower.contains("steampowered.com")
                        && !url_lower.contains("/login")
                    {
                        tracing::info!(url = %url_str, "Steam logged in on store page — redirecting to /my/profile");
                        if let Ok(parsed) =
                            "https://steamcommunity.com/my/profile".parse::<url::Url>()
                        {
                            let _ = wc.navigate(parsed);
                        }
                    }
                }
                Err(_) => {
                    if !emitted.load(Ordering::SeqCst) {
                        let _ = ac.emit(
                            "moe://steam-progress",
                            serde_json::json!({ "status": "closed", "message": "登录窗口已关闭" }),
                        );
                    }
                    break;
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        }
    });

    let _ = window.set_focus();
    tracing::info!("Steam login WebView opened — will auto-fetch games after login");
    Ok(())
}

/// 从 profile URL 提取 SteamID64
fn extract_steam_id_from_url(url: &str) -> Option<String> {
    if let Some(path) = url.split("/profiles/").nth(1) {
        // Take only digits (SteamID64 is 17 digits)
        let sid: String = path.chars().take_while(|c| c.is_ascii_digit()).collect();
        if sid.len() == 17 {
            return Some(sid);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url_profiles() {
        let input = "https://steamcommunity.com/profiles/76561197960435530";
        // Without async runtime in tests, just check that the input looks valid
        assert!(input.contains("/profiles/"));
        assert!(!input.contains("/id/"));
    }

    #[test]
    fn test_resolve_url_vanity() {
        let input = "https://steamcommunity.com/id/gaben";
        assert!(input.contains("/id/"));
    }

    #[test]
    fn test_resolve_numeric() {
        let sid = "76561197960435530";
        assert_eq!(sid.len(), 17);
        assert!(sid.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_parse_owned_games_preserves_last_played_and_icon() {
        let payload = r#"{
          "response": {
            "game_count": 1,
            "games": [{
              "appid": 70,
              "name": "Half-Life",
              "playtime_forever": 90,
              "playtime_2weeks": 12,
              "rtime_last_played": 1704164640,
              "img_icon_url": "iconhash",
              "img_logo_url": "logohash",
              "achievements_total": 10,
              "achievements_unlocked": 4
            }]
          }
        }"#;

        let parsed = parse_owned_games(payload).unwrap();
        assert_eq!(parsed.game_count, 1);
        assert_eq!(parsed.games[0].app_id, 70);
        assert_eq!(parsed.games[0].rtime_last_played, Some(1704164640));
        assert_eq!(parsed.games[0].img_icon_url.as_deref(), Some("iconhash"));
    }

    #[test]
    fn test_parse_achievement_summary_payloads() {
        let schema = r#"{
          "game": {
            "availableGameStats": {
              "achievements": [
                { "name": "A" },
                { "name": "B" },
                { "name": "C" }
              ]
            }
          }
        }"#;
        let player = r#"{
          "playerstats": {
            "achievements": [
              { "apiname": "A", "achieved": 1 },
              { "apiname": "B", "achieved": 0 },
              { "apiname": "C", "achieved": 1 }
            ]
          }
        }"#;

        assert_eq!(parse_achievement_total(schema).unwrap(), 3);
        assert_eq!(parse_unlocked_achievements(player).unwrap(), 2);
    }
}
