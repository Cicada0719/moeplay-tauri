use crate::db::Database;
use crate::domain::{ProviderError, ProviderErrorKind};
use crate::models::{Game, StoreLink};
use crate::secret_store::{SecretKind, SecretStore};
use crate::task_queue::{JobOperation, TaskQueue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformImportSource {
    Steam,
    Epic,
}

impl PlatformImportSource {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Epic => "epic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformImportMode {
    Local,
    Account,
    Combined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformImportStatus {
    pub steam_path: Option<String>,
    pub steam_id: Option<String>,
    pub has_steam_api_key: bool,
    pub steam_api_key_validated: bool,
    pub steam_can_sync_account: bool,
    pub epic_manifest_path: Option<String>,
    pub epic_manifest_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformGameCandidate {
    pub source: String,
    pub library_id: String,
    pub name: String,
    pub install_dir: Option<String>,
    pub launch_uri: String,
    pub cover_url: Option<String>,
    pub icon_url: Option<String>,
    pub store_url: Option<String>,
    pub playtime_minutes: Option<u32>,
    pub last_played: Option<String>,
    pub achievements_total: Option<u32>,
    pub achievements_unlocked: Option<u32>,
    pub installed: bool,
    /// 是否来自 Steam 家庭共享库（Steam Families）。导入后会打"家庭共享"标签。
    #[serde(default)]
    pub shared: bool,
    pub selected: bool,
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformScanResult {
    pub source: String,
    pub mode: String,
    pub candidates: Vec<PlatformGameCandidate>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformImportResult {
    pub source: String,
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total: usize,
    pub imported_ids: Vec<String>,
    pub updated_ids: Vec<String>,
    pub skipped_reasons: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SteamLocalConfigApp {
    appid: String,
    playtime_minutes: u32,
    last_played: Option<u64>,
}

fn imported_game_to_candidate(
    game: crate::integration::ImportedGame,
) -> Option<PlatformGameCandidate> {
    let source = normalize_platform_source(&game.platform);
    let library_id = game.app_id.clone().unwrap_or_default();
    if library_id.trim().is_empty() {
        return None;
    }
    let launch_uri = platform_launch_uri(&source, &library_id)?;
    let store_url = platform_store_url(&source, &library_id);
    let cover_url = if source == "steam" {
        Some(steam_preferred_cover(&library_id))
    } else {
        game.cover_url
    };
    Some(PlatformGameCandidate {
        source,
        library_id,
        name: game.name,
        install_dir: Some(game.install_path.to_string_lossy().to_string()),
        launch_uri,
        cover_url,
        icon_url: None,
        store_url,
        playtime_minutes: None,
        last_played: None,
        achievements_total: None,
        achievements_unlocked: None,
        installed: true,
        shared: false,
        selected: true,
        skip_reason: None,
    })
}

fn owned_game_to_candidate(game: crate::steam_openid::SteamOwnedGame) -> PlatformGameCandidate {
    let appid = game.app_id.to_string();
    PlatformGameCandidate {
        source: "steam".to_string(),
        library_id: appid.clone(),
        name: game.name,
        install_dir: None,
        launch_uri: platform_launch_uri("steam", &appid).unwrap_or_default(),
        cover_url: Some(steam_preferred_cover(&appid)),
        icon_url: steam_icon_url(&appid, game.img_icon_url.as_deref()),
        store_url: platform_store_url("steam", &appid),
        playtime_minutes: Some(game.playtime_forever),
        last_played: steam_last_played_string(game.rtime_last_played),
        achievements_total: game.achievements_total,
        achievements_unlocked: game.achievements_unlocked,
        installed: false,
        shared: false,
        selected: true,
        skip_reason: None,
    }
}

/// 从已登录 WebView 会话抓取的单个游戏（Playnite 式：无需 API Key）。
/// 字段对应 steamcommunity.com 个人游戏页里的全局 `rgGames`，由注入脚本归一化后回传。
#[derive(Debug, Clone, Deserialize)]
pub struct SessionScrapedGame {
    pub appid: u32,
    #[serde(default)]
    pub name: String,
    /// 总时长（分钟），脚本已从 playtime_forever / hours_forever 归一。
    #[serde(default)]
    pub playtime_forever: u32,
    /// 最后游玩 unix 秒；0 视为无。
    #[serde(default)]
    pub last_played: u64,
    /// 是否来自家庭共享库（Steam Families）。注入脚本对共享批次置 true。
    #[serde(default)]
    pub shared: bool,
}

/// 竖封面优先用本机 Steam `librarycache` 已缓存的图（被墙网络也能显示），否则回退 CDN URL。
fn steam_local_or_remote_cover(appid: &str) -> String {
    if let Some(steam) = crate::integration::find_steam_install_path() {
        let cache = steam.join("appcache").join("librarycache");
        // 新版布局：librarycache/{appid}/library_600x900.jpg
        let nested = cache.join(appid).join("library_600x900.jpg");
        if nested.is_file() {
            return nested.to_string_lossy().to_string();
        }
        // 旧版扁平布局：librarycache/{appid}_library_600x900.jpg
        let flat = cache.join(format!("{appid}_library_600x900.jpg"));
        if flat.is_file() {
            return flat.to_string_lossy().to_string();
        }
    }
    steam_vertical_cover_url(appid)
}

fn steam_preferred_cover(appid: &str) -> String {
    let _legacy_cover_probe: fn(&str) -> String = steam_local_or_remote_cover;
    if let Some(steam) = crate::integration::find_steam_install_path() {
        if let Some(local) = copy_steam_cached_cover_to_data(&steam, appid) {
            return local;
        }
    }
    steam_vertical_cover_url(appid)
}

fn steam_cached_cover_path(steam_path: &Path, appid: &str) -> Option<PathBuf> {
    let cache = steam_path.join("appcache").join("librarycache");
    let nested = cache.join(appid).join("library_600x900.jpg");
    if nested.is_file() {
        return Some(nested);
    }

    let flat = cache.join(format!("{appid}_library_600x900.jpg"));
    if flat.is_file() {
        return Some(flat);
    }

    None
}

fn copy_steam_cached_cover_to_data(steam_path: &Path, appid: &str) -> Option<String> {
    let src = steam_cached_cover_path(steam_path, appid)?;
    let data_dir = dirs::data_dir()?
        .join("moeplay")
        .join("covers")
        .join("steam");
    if std::fs::create_dir_all(&data_dir).is_err() {
        return None;
    }

    let dst = data_dir.join(format!("{appid}_library_600x900.jpg"));
    if should_copy_file(&src, &dst) && std::fs::copy(&src, &dst).is_err() {
        return None;
    }
    Some(dst.to_string_lossy().to_string())
}

/// 本机 Steam librarycache 里的横版 hero 背景图（library_hero.jpg）。
fn steam_cached_hero_path(steam_path: &Path, appid: &str) -> Option<PathBuf> {
    let cache = steam_path.join("appcache").join("librarycache");
    let nested = cache.join(appid).join("library_hero.jpg");
    if nested.is_file() {
        return Some(nested);
    }
    let flat = cache.join(format!("{appid}_library_hero.jpg"));
    if flat.is_file() {
        return Some(flat);
    }
    None
}

/// 把本机缓存的 hero 横版图复制到 data 目录（资产协议作用域内），返回本地路径。
/// 仅在本机确有该缓存图时返回 Some——避免持久化可能 404 的 CDN URL，CDN 兜底交给前端按 appid 派生。
fn steam_preferred_hero_local(appid: &str) -> Option<String> {
    let steam = crate::integration::find_steam_install_path()?;
    let src = steam_cached_hero_path(&steam, appid)?;
    let data_dir = dirs::data_dir()?
        .join("moeplay")
        .join("covers")
        .join("steam");
    if std::fs::create_dir_all(&data_dir).is_err() {
        return None;
    }
    let dst = data_dir.join(format!("{appid}_library_hero.jpg"));
    if should_copy_file(&src, &dst) && std::fs::copy(&src, &dst).is_err() {
        return None;
    }
    Some(dst.to_string_lossy().to_string())
}

fn should_copy_file(src: &Path, dst: &Path) -> bool {
    if !dst.is_file() {
        return true;
    }

    let src_meta = match std::fs::metadata(src) {
        Ok(meta) => meta,
        Err(_) => return true,
    };
    let dst_meta = match std::fs::metadata(dst) {
        Ok(meta) => meta,
        Err(_) => return true,
    };

    src_meta.len() != dst_meta.len()
        || src_meta
            .modified()
            .ok()
            .zip(dst_meta.modified().ok())
            .is_none_or(|(src_mtime, dst_mtime)| src_mtime > dst_mtime)
}

/// 将远程封面 URL 下载到本地 covers/ 目录，返回本地文件路径。
/// 已存在则跳过下载直接返回。失败时返回原始 URL 兜底。
pub(crate) async fn fetch_cover_to_local(url: &str, game_id: &str) -> String {
    if url.is_empty() || (!url.starts_with("http://") && !url.starts_with("https://")) {
        return url.to_string();
    }

    let Some(data_dir) = dirs::data_dir() else {
        return url.to_string();
    };
    let covers_dir = data_dir.join("moeplay").join("covers");
    if std::fs::create_dir_all(&covers_dir).is_err() {
        return url.to_string();
    }

    let ext = if url.contains(".jpg") || url.contains(".jpeg") {
        "jpg"
    } else if url.contains(".png") {
        "png"
    } else if url.contains(".webp") {
        "webp"
    } else {
        "jpg"
    };

    let dst = covers_dir.join(format!("{}.{}", game_id, ext));

    // 已缓存 → 直接用
    if dst.is_file() && dst.metadata().map(|m| m.len()).unwrap_or(0) > 512 {
        return dst.to_string_lossy().to_string();
    }

    // 下载
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(_) => return url.to_string(),
    };

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(url, error = %e, "Cover download failed, keeping remote URL");
            return url.to_string();
        }
    };

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(url, error = %e, "Cover read failed");
            return url.to_string();
        }
    };

    if bytes.len() < 512 {
        return url.to_string();
    }

    if std::fs::write(&dst, &bytes).is_err() {
        return url.to_string();
    }

    tracing::info!(url, local = %dst.display(), "Cover cached locally");
    dst.to_string_lossy().to_string()
}

fn session_game_to_candidate(game: SessionScrapedGame) -> PlatformGameCandidate {
    let appid = game.appid.to_string();
    let name = if game.name.trim().is_empty() {
        format!("Steam App {appid}")
    } else {
        game.name
    };
    PlatformGameCandidate {
        source: "steam".to_string(),
        library_id: appid.clone(),
        name,
        install_dir: None,
        launch_uri: platform_launch_uri("steam", &appid).unwrap_or_default(),
        cover_url: Some(steam_preferred_cover(&appid)),
        icon_url: None,
        store_url: platform_store_url("steam", &appid),
        playtime_minutes: Some(game.playtime_forever),
        last_played: steam_last_played_string((game.last_played > 0).then_some(game.last_played)),
        achievements_total: None,
        achievements_unlocked: None,
        installed: false,
        shared: game.shared,
        selected: true,
        skip_reason: None,
    }
}

/// 导入「网页会话抓取」得到的 Steam 全库（Playnite 式，无需 API Key）。
/// 复用 import_platform_library 的去重/合并逻辑，不覆盖用户手改的中文名与已有竖封面。
#[tauri::command]
pub fn import_steam_session_games(
    db: State<'_, Database>,
    games: Vec<SessionScrapedGame>,
) -> PlatformImportResult {
    crate::crash_log(&format!(
        "import_steam_session_games: START ({} games)",
        games.len()
    ));
    // 账号自有库（rgGames 网页会话抓取）
    let owned: Vec<PlatformGameCandidate> = games
        .into_iter()
        .filter(|g| g.appid != 0)
        .map(session_game_to_candidate)
        .collect();
    let owned_ids: std::collections::HashSet<String> =
        owned.iter().map(|c| c.library_id.clone()).collect();

    // 与 Playnite 同思路：合并本机 appmanifest 扫描（已安装游戏，含家庭共享已下载的）。
    // 已安装但不在自有库里的 = 家庭共享（borrowed）→ 标 shared，导入时打"家庭共享"标签。
    let mut local = scan_local_platform_candidates("steam").candidates;
    let mut shared_count = 0usize;
    for c in local.iter_mut() {
        if !owned_ids.contains(&c.library_id) {
            c.shared = true;
            shared_count += 1;
        }
    }
    tracing::info!(
        owned = owned.len(),
        local = local.len(),
        shared = shared_count,
        "Steam session import: merging owned library with local-installed (incl. family-shared)"
    );

    let merged = merge_platform_candidates(local, owned);
    import_platform_library(db, PlatformImportSource::Steam, merged)
}

fn merge_platform_candidates(
    local: Vec<PlatformGameCandidate>,
    account: Vec<PlatformGameCandidate>,
) -> Vec<PlatformGameCandidate> {
    let mut by_id: HashMap<String, PlatformGameCandidate> = HashMap::new();
    for candidate in account {
        by_id.insert(candidate.library_id.clone(), candidate);
    }
    for candidate in local {
        by_id
            .entry(candidate.library_id.clone())
            .and_modify(|existing| {
                existing.installed = true;
                existing.install_dir = candidate.install_dir.clone();
                if existing.cover_url.is_none() {
                    existing.cover_url = candidate.cover_url.clone();
                }
                if existing.icon_url.is_none() {
                    existing.icon_url = candidate.icon_url.clone();
                }
                if existing.last_played.is_none() {
                    existing.last_played = candidate.last_played.clone();
                }
                if existing.achievements_total.is_none() {
                    existing.achievements_total = candidate.achievements_total;
                }
                if existing.achievements_unlocked.is_none() {
                    existing.achievements_unlocked = candidate.achievements_unlocked;
                }
            })
            .or_insert(candidate);
    }
    let mut merged: Vec<_> = by_id.into_values().collect();
    merged.sort_by(|a, b| {
        b.installed
            .cmp(&a.installed)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    merged
}

fn scan_local_platform_candidates(source: &str) -> PlatformScanResult {
    let mut skipped = Vec::new();
    let candidates = match source {
        "epic" => crate::integration::scan_epic_games(),
        _ => crate::integration::scan_steam_games(),
    }
    .into_iter()
    .filter_map(|game| {
        let name = game.name.clone();
        let id = game.app_id.clone();
        let candidate = imported_game_to_candidate(game);
        if candidate.is_none() {
            skipped.push(format!("{}: 缺少平台启动 ID，已跳过", name));
        } else if id.as_deref().unwrap_or_default().trim().is_empty() {
            skipped.push(format!("{}: 缺少平台 ID，已跳过", name));
        }
        candidate
    })
    .collect();

    if source == "steam" {
        return scan_steam_local_full_candidates(candidates, skipped);
    }

    PlatformScanResult {
        source: source.to_string(),
        mode: "local".to_string(),
        candidates,
        skipped,
        errors: Vec::new(),
    }
}

fn scan_steam_local_full_candidates(
    installed: Vec<PlatformGameCandidate>,
    mut skipped: Vec<String>,
) -> PlatformScanResult {
    let Some(steam_path) = crate::integration::find_steam_install_path() else {
        return PlatformScanResult {
            source: "steam".to_string(),
            mode: "local".to_string(),
            candidates: installed,
            skipped,
            errors: Vec::new(),
        };
    };

    let installed_names = installed
        .iter()
        .map(|candidate| (candidate.library_id.clone(), candidate.name.clone()))
        .collect::<HashMap<_, _>>();
    let account_id = read_most_recent_steamid64(&steam_path)
        .and_then(|sid| steamid64_to_account_id(&sid))
        .or_else(|| first_userdata_account_id(&steam_path));

    let account_candidates = account_id.as_deref().and_then(|account| {
        read_steam_localconfig_candidates(&steam_path, account, &installed_names)
    });

    let candidates = if let Some(account_candidates) = account_candidates {
        merge_platform_candidates(installed, account_candidates)
    } else {
        skipped.push("未找到 Steam localconfig.vdf，已退回仅扫描本机已安装游戏".to_string());
        installed
    };

    PlatformScanResult {
        source: "steam".to_string(),
        mode: "local".to_string(),
        candidates,
        skipped,
        errors: Vec::new(),
    }
}

fn read_steam_localconfig_candidates(
    steam_path: &Path,
    account_id: &str,
    installed_names: &HashMap<String, String>,
) -> Option<Vec<PlatformGameCandidate>> {
    let localconfig = steam_path
        .join("userdata")
        .join(account_id)
        .join("config")
        .join("localconfig.vdf");
    let content = std::fs::read_to_string(localconfig).ok()?;
    let mut candidates = parse_steam_localconfig_apps(&content)
        .into_iter()
        .filter(|app| !app.appid.trim().is_empty())
        .map(|app| {
            let appid = app.appid;
            let name = installed_names
                .get(&appid)
                .cloned()
                .unwrap_or_else(|| format!("Steam App {appid}"));
            PlatformGameCandidate {
                source: "steam".to_string(),
                library_id: appid.clone(),
                name,
                install_dir: None,
                launch_uri: platform_launch_uri("steam", &appid).unwrap_or_default(),
                cover_url: Some(steam_preferred_cover(&appid)),
                icon_url: None,
                store_url: platform_store_url("steam", &appid),
                playtime_minutes: Some(app.playtime_minutes),
                last_played: steam_last_played_string(app.last_played),
                achievements_total: None,
                achievements_unlocked: None,
                installed: false,
                shared: false,
                selected: true,
                skip_reason: None,
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|a| a.name.to_lowercase());
    Some(candidates)
}

fn parse_steam_localconfig_apps(content: &str) -> Vec<SteamLocalConfigApp> {
    let lines = content.lines().collect::<Vec<_>>();
    let Some(mut i) = lines.iter().position(|line| {
        quoted_fields(line.trim())
            .first()
            .is_some_and(|field| field == "apps")
    }) else {
        return Vec::new();
    };

    i += 1;
    while i < lines.len() && lines[i].trim() != "{" {
        i += 1;
    }
    if i >= lines.len() {
        return Vec::new();
    }
    i += 1;

    let mut apps = Vec::new();
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with('}') {
            break;
        }

        let fields = quoted_fields(line);
        let Some(appid) = fields
            .first()
            .filter(|value| value.chars().all(|c| c.is_ascii_digit()))
        else {
            i += 1;
            continue;
        };

        let mut j = i + 1;
        while j < lines.len() && lines[j].trim().is_empty() {
            j += 1;
        }
        if j >= lines.len() || lines[j].trim() != "{" {
            i += 1;
            continue;
        }

        j += 1;
        let mut playtime = None;
        let mut disconnected = None;
        let mut last_played = None;
        while j < lines.len() {
            let inner = lines[j].trim();
            if inner.starts_with('}') {
                break;
            }
            let pair = quoted_fields(inner);
            if pair.len() >= 2 {
                match pair[0].as_str() {
                    "Playtime" => playtime = pair[1].parse::<u32>().ok(),
                    "PlaytimeDisconnected" => disconnected = pair[1].parse::<u32>().ok(),
                    "LastPlayed" => {
                        last_played = pair[1].parse::<u64>().ok().filter(|value| *value > 0)
                    }
                    _ => {}
                }
            }
            j += 1;
        }

        apps.push(SteamLocalConfigApp {
            appid: appid.clone(),
            playtime_minutes: playtime.or(disconnected).unwrap_or(0),
            last_played,
        });
        i = j + 1;
    }

    apps
}

fn read_most_recent_steamid64(steam_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(steam_path.join("config").join("loginusers.vdf")).ok()?;
    parse_most_recent_steamid64(&content)
}

fn parse_most_recent_steamid64(content: &str) -> Option<String> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut first_user = None;
    let mut i = 0;
    while i < lines.len() {
        let fields = quoted_fields(lines[i].trim());
        let Some(steamid) = fields
            .first()
            .filter(|value| value.len() >= 16 && value.chars().all(|c| c.is_ascii_digit()))
        else {
            i += 1;
            continue;
        };

        first_user.get_or_insert_with(|| steamid.clone());
        let mut j = i + 1;
        while j < lines.len() && lines[j].trim().is_empty() {
            j += 1;
        }
        if j >= lines.len() || lines[j].trim() != "{" {
            i += 1;
            continue;
        }

        j += 1;
        while j < lines.len() {
            let inner = lines[j].trim();
            if inner.starts_with('}') {
                break;
            }
            let pair = quoted_fields(inner);
            if pair.len() >= 2 && pair[0] == "MostRecent" && pair[1] == "1" {
                return Some(steamid.clone());
            }
            j += 1;
        }
        i = j + 1;
    }
    first_user
}

fn steamid64_to_account_id(steamid64: &str) -> Option<String> {
    steamid64
        .trim()
        .parse::<u64>()
        .ok()
        .map(|sid| (sid & 0xFFFF_FFFF).to_string())
}

fn first_userdata_account_id(steam_path: &Path) -> Option<String> {
    let mut ids = std::fs::read_dir(steam_path.join("userdata"))
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            name.chars().all(|c| c.is_ascii_digit()).then_some(name)
        })
        .collect::<Vec<_>>();
    ids.sort();
    ids.into_iter().next()
}

fn quoted_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }

        let mut value = String::new();
        let mut escaped = false;
        for inner in chars.by_ref() {
            if escaped {
                value.push(inner);
                escaped = false;
            } else if inner == '\\' {
                escaped = true;
            } else if inner == '"' {
                break;
            } else {
                value.push(inner);
            }
        }
        fields.push(value);
    }
    fields
}

fn import_platform_candidate(
    db: &Database,
    candidate: &PlatformGameCandidate,
) -> Result<(Game, bool), String> {
    let source = normalize_platform_source(&candidate.source);
    let library_id = candidate.library_id.trim();
    if library_id.is_empty() {
        return Err("缺少平台游戏 ID".to_string());
    }
    let launch_uri = if candidate.launch_uri.trim().is_empty() {
        platform_launch_uri(&source, library_id)
    } else {
        Some(candidate.launch_uri.clone())
    };
    let exe_str = launch_uri.clone().unwrap_or_default();
    let existing = db.get_games();
    if let Some(mut game) = find_existing_platform_game(
        &existing,
        &source,
        Some(library_id),
        launch_uri.as_deref(),
        candidate.install_dir.as_deref(),
        Some(&exe_str),
    ) {
        let before = game.updated_at.clone();
        apply_platform_import_fields(
            &mut game,
            &source,
            Some(library_id),
            launch_uri.as_deref(),
            candidate.install_dir.as_deref(),
            Some(&exe_str),
            candidate.store_url.as_deref(),
            candidate.cover_url.as_deref(),
            candidate.icon_url.as_deref(),
            candidate.playtime_minutes,
            candidate.last_played.as_deref(),
            candidate.achievements_total,
            candidate.achievements_unlocked,
        );
        if game.name.trim().is_empty() || game.name.starts_with("steam_") {
            game.name = candidate.name.clone();
        }
        if candidate.shared && !game.tags.iter().any(|t| t == "家庭共享") {
            game.tags.push("家庭共享".to_string());
        }
        let updated = db.update_game(game)?;
        return Ok((updated, before.is_empty()));
    }

    let mut game = Game::new(candidate.name.clone(), exe_str);
    apply_platform_import_fields(
        &mut game,
        &source,
        Some(library_id),
        launch_uri.as_deref(),
        candidate.install_dir.as_deref(),
        launch_uri.as_deref(),
        candidate.store_url.as_deref(),
        candidate.cover_url.as_deref(),
        candidate.icon_url.as_deref(),
        candidate.playtime_minutes,
        candidate.last_played.as_deref(),
        candidate.achievements_total,
        candidate.achievements_unlocked,
    );
    if candidate.shared && !game.tags.iter().any(|t| t == "家庭共享") {
        game.tags.push("家庭共享".to_string());
    }
    let created = db.add_game(game)?;
    Ok((created, true))
}

#[tauri::command]
pub async fn get_platform_import_status(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
) -> Result<PlatformImportStatus, String> {
    crate::crash_log("get_platform_import_status: START");
    let settings =
        super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let steam_path =
        crate::integration::find_steam_install_path().map(|p| p.to_string_lossy().to_string());
    let steam_id = settings
        .steam_id
        .clone()
        .or_else(crate::steam_openid::detect_local_steam_id);
    let key = read_optional_steam_api_key(secret_store.inner())?;
    let has_key = key.is_some();
    let key_ok = if let Some(key) = key.as_deref() {
        crate::steam_openid::verify_api_key(key).await.is_ok()
    } else {
        false
    };
    let epic_manifest_path =
        crate::integration::epic_manifests_dir().map(|p| p.to_string_lossy().to_string());

    Ok(PlatformImportStatus {
        steam_path,
        steam_id: steam_id.clone(),
        has_steam_api_key: has_key,
        steam_api_key_validated: key_ok,
        steam_can_sync_account: steam_id.is_some() && key_ok,
        epic_manifest_available: epic_manifest_path.is_some(),
        epic_manifest_path,
    })
}

#[tauri::command]
pub async fn resolve_steam_id(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
    input: String,
) -> Result<crate::steam_openid::SteamLoginResult, String> {
    let mut settings =
        super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let key = read_optional_steam_api_key(secret_store.inner())?;
    let result = resolve_steam_url_with_key(&input, key.as_deref()).await?;
    settings.steam_id = Some(result.steam_id.clone());
    db.update_settings(settings)?;
    Ok(result)
}

#[tauri::command]
pub async fn validate_steam_api_key(
    secret_store: State<'_, SecretStore>,
    api_key: String,
) -> Result<String, String> {
    verify_and_store_steam_api_key(secret_store.inner(), &api_key).await
}

#[tauri::command]
pub async fn steam_login_openid(app: tauri::AppHandle) -> Result<String, String> {
    steam_login_webview(app).await
}

#[tauri::command]
pub async fn scan_platform_library(
    source: PlatformImportSource,
    mode: PlatformImportMode,
    steam_id: Option<String>,
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
) -> Result<PlatformScanResult, String> {
    let source_name = source.as_str();
    if source_name == "epic" {
        let mut result = scan_local_platform_candidates("epic");
        result.mode = "local".to_string();
        if result.candidates.is_empty() && result.skipped.is_empty() {
            result.skipped.push("未发现 Epic 本机安装清单".to_string());
        }
        return Ok(result);
    }

    let include_local = matches!(
        mode,
        PlatformImportMode::Local | PlatformImportMode::Combined
    );
    let include_account = matches!(
        mode,
        PlatformImportMode::Account | PlatformImportMode::Combined
    );
    let local = if include_local {
        scan_local_platform_candidates("steam")
    } else {
        PlatformScanResult {
            source: "steam".to_string(),
            mode: "local".to_string(),
            candidates: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        }
    };

    if !include_account {
        return Ok(PlatformScanResult {
            source: "steam".to_string(),
            mode: "local".to_string(),
            candidates: local.candidates,
            skipped: local.skipped,
            errors: local.errors,
        });
    }

    let settings =
        super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let sid = steam_id
        .filter(|s| !s.trim().is_empty())
        .or(settings.steam_id)
        .or_else(crate::steam_openid::detect_local_steam_id)
        .ok_or_else(|| "缺少 SteamID64，请先检测本地账号、网页登录或手动输入".to_string())?;
    let key = read_steam_api_key(secret_store.inner())?;

    let account_resp = crate::steam_openid::fetch_owned_games(&sid, &key).await?;
    let account_candidates = account_resp
        .games
        .into_iter()
        .map(owned_game_to_candidate)
        .collect::<Vec<_>>();
    let candidates = if include_local {
        merge_platform_candidates(local.candidates, account_candidates)
    } else {
        account_candidates
    };
    let mut skipped = local.skipped;
    if candidates.is_empty() {
        skipped.push("Steam API 未返回任何游戏，请检查隐私设置和 API Key".to_string());
    }

    Ok(PlatformScanResult {
        source: "steam".to_string(),
        mode: match mode {
            PlatformImportMode::Account => "account",
            PlatformImportMode::Combined => "combined",
            PlatformImportMode::Local => "local",
        }
        .to_string(),
        candidates,
        skipped,
        errors: local.errors,
    })
}

#[tauri::command]
pub fn import_platform_library(
    db: State<'_, Database>,
    source: PlatformImportSource,
    candidates: Vec<PlatformGameCandidate>,
) -> PlatformImportResult {
    let source_name = source.as_str().to_string();
    let mut result = PlatformImportResult {
        source: source_name,
        imported: 0,
        updated: 0,
        skipped: 0,
        failed: 0,
        total: candidates.len(),
        imported_ids: Vec::new(),
        updated_ids: Vec::new(),
        skipped_reasons: Vec::new(),
        errors: Vec::new(),
    };

    for candidate in candidates {
        if let Some(reason) = candidate
            .skip_reason
            .as_ref()
            .filter(|s| !s.trim().is_empty())
        {
            result.skipped += 1;
            result
                .skipped_reasons
                .push(format!("{}: {}", candidate.name, reason));
            continue;
        }
        match import_platform_candidate(&db, &candidate) {
            Ok((game, created)) => {
                if created {
                    result.imported += 1;
                    result.imported_ids.push(game.id.clone());
                    // 自动刮削：新游戏入库 + 用户开启 auto_scrape 时异步搜索元数据
                    let settings = db.get_settings();
                    if settings.auto_scrape {
                        let gid = game.id.clone();
                        let gname = game.name.clone();
                        // 必须用 tauri::async_runtime::spawn：本命令是同步 #[tauri::command]，
                        // 运行线程没有 Tokio reactor，直接 tokio::spawn 会 panic→abort 闪退。
                        tauri::async_runtime::spawn(async move {
                            let db2 = crate::db::Database::new();
                            let s = db2.get_settings();
                            let proxy = if s.scraper_proxy.trim().is_empty() {
                                None
                            } else {
                                Some(s.scraper_proxy.clone())
                            };
                            crate::scraper::utils::set_proxy(proxy);
                            let (raw, _) = crate::scraper::search_all(
                                &gname,
                                s.vndb_enabled,
                                s.bangumi_enabled,
                                s.dlsite_enabled,
                                s.getchu_enabled,
                                s.touchgal_enabled,
                                s.erogamescape_enabled,
                                s.ymgal_enabled,
                                s.kungal_enabled,
                                s.steam_enabled,
                                s.pcgw_enabled,
                            )
                            .await;
                            if !raw.is_empty() {
                                let merged = crate::scraper::merge::merge_results(
                                    raw,
                                    &crate::scraper::merge::MergeConfig {
                                        max_results: 1,
                                        ..Default::default()
                                    },
                                );
                                if let Some(best) = merged.first() {
                                    let cover = if let Some(ref url) = best.result.cover {
                                        Some(fetch_cover_to_local(url, &gid).await)
                                    } else {
                                        None
                                    };
                                    let background = if let Some(ref url) = best.result.background {
                                        Some(fetch_cover_to_local(url, &format!("{gid}_bg")).await)
                                    } else {
                                        None
                                    };
                                    let _ = db2.apply_scrape_result_ext(
                                        &gid,
                                        Some(best.result.title.clone()),
                                        best.result.description.clone(),
                                        cover,
                                        background,
                                        Some(best.result.tags.clone()),
                                        best.result.rating,
                                        best.result.release_year,
                                        Some(&best.result.source),
                                        Some(best.result.source_id.clone()),
                                        best.result
                                            .detail
                                            .as_ref()
                                            .and_then(|d| d.developer.clone()),
                                        best.result
                                            .detail
                                            .as_ref()
                                            .and_then(|d| d.publisher.clone()),
                                        best.result.detail.as_ref().map(|d| d.genres.clone()),
                                        best.result.detail.as_ref().map(|d| d.languages.clone()),
                                        best.result.detail.as_ref().and_then(|d| d.engine.clone()),
                                        best.result
                                            .detail
                                            .as_ref()
                                            .and_then(|d| d.age_rating.clone()),
                                        best.result.detail.as_ref().and_then(|d| d.series.clone()),
                                        best.result
                                            .detail
                                            .as_ref()
                                            .and_then(|d| d.release_date.clone()),
                                        best.result
                                            .detail
                                            .as_ref()
                                            .map(|d| d.voice_languages.clone()),
                                        best.result.detail.as_ref().map(|d| d.aliases.clone()),
                                        best.result.detail.as_ref().map(|d| d.screenshots.clone()),
                                        best.result
                                            .detail
                                            .as_ref()
                                            .and_then(|d| d.homepage.clone()),
                                    );
                                }
                            }
                        });
                    }
                } else {
                    result.updated += 1;
                    result.updated_ids.push(game.id);
                }
            }
            Err(e) => {
                result.failed += 1;
                result.errors.push(format!("{}: {}", candidate.name, e));
            }
        }
    }
    result
}

/// 发现 Steam 安装路径。
#[tauri::command]
pub fn find_steam_path() -> Option<String> {
    crate::integration::find_steam_install_path().map(|p| p.to_string_lossy().to_string())
}

/// 扫描本地 Steam 库中已安装的游戏。
#[tauri::command]
pub fn scan_steam_library() -> Vec<crate::integration::ImportedGame> {
    crate::integration::scan_steam_games()
}

/// 扫描本地 Epic Games 库中已安装的游戏。
#[tauri::command]
pub fn scan_epic_library() -> Vec<crate::integration::ImportedGame> {
    crate::integration::scan_epic_games()
}

/// 将 Steam/Epic 游戏导入到本地库（自动去重 + 引擎检测 + 封面URL）。
#[tauri::command]
pub fn import_steam_game(
    db: State<'_, Database>,
    name: String,
    install_path: String,
    app_id: Option<String>,
    cover_url: Option<String>,
    platform: Option<String>,
) -> Result<Game, String> {
    let install_dir = PathBuf::from(&install_path);
    let source = normalize_platform_source(platform.as_deref().unwrap_or("steam"));
    let library_id = app_id.clone();
    let launch_uri = library_id
        .as_deref()
        .and_then(|id| platform_launch_uri(&source, id));
    let exe_str = launch_uri.clone().unwrap_or_else(|| {
        crate::archive::find_best_exe(&install_dir)
            .map(|c| c.path.to_string_lossy().to_string())
            .unwrap_or_else(|| install_dir.join("game.exe").to_string_lossy().to_string())
    });

    let existing = db.get_games();
    if let Some(mut game) = find_existing_platform_game(
        &existing,
        &source,
        library_id.as_deref(),
        launch_uri.as_deref(),
        Some(&install_path),
        Some(&exe_str),
    ) {
        apply_platform_import_fields(
            &mut game,
            &source,
            library_id.as_deref(),
            launch_uri.as_deref(),
            Some(&install_path),
            Some(&exe_str),
            None,
            cover_url.as_deref(),
            None,
            None,
            None,
            None,
            None,
        );
        if game.name.trim().is_empty() || game.name.starts_with("steam_") {
            game.name = name;
        }
        return db.update_game(game);
    }

    if launch_uri.is_none() && crate::archive::is_duplicate(&name, &exe_str, &existing) {
        return Err("游戏已存在".into());
    }

    let mut game = Game::new(name, exe_str.clone());
    apply_platform_import_fields(
        &mut game,
        &source,
        library_id.as_deref(),
        launch_uri.as_deref(),
        Some(&install_path),
        Some(&exe_str),
        None,
        cover_url.as_deref(),
        None,
        None,
        None,
        None,
        None,
    );

    if launch_uri.is_none() {
        if let Some(ec) = crate::locale::EngineLibrary::detect_engine(&install_dir) {
            game.metadata.engine = Some(format!("{:?}", ec.engine));
        }
    }

    db.add_game(game)
}

// ===== M6 自动入库刮削管线 =====

/// 对指定目录执行完整自动入库管线：检测 → 去重 → 标题推断 → 入库
#[tauri::command]
pub fn run_auto_scrape_pipeline(
    db: State<'_, Database>,
    dir: String,
    auto_scrape: Option<bool>,
) -> Result<crate::auto_scrape::PipelineState, String> {
    let path = PathBuf::from(&dir);
    if !path.is_dir() {
        return Err("指定路径不是有效目录".into());
    }
    let mut state = crate::auto_scrape::PipelineState::default();
    crate::auto_scrape::run_full_pipeline(&db, &path, auto_scrape.unwrap_or(true), &mut state);
    Ok(state)
}

// ===== M6 Steam 身份认证 + Web API =====

/// 方式 A: 打开浏览器让用户手动获取 SteamID（最可靠）
#[tauri::command]
pub fn steam_open_community(mode: Option<String>) -> Result<String, String> {
    match mode.as_deref() {
        Some("edit") => crate::steam_openid::open_steam_edit_profile()?,
        _ => crate::steam_openid::open_steam_community()?,
    }
    Ok("已在浏览器打开 Steam 页面".to_string())
}

/// 方式 D: 【推荐】Tauri WebView 窗口打开 Steam 登录（像 Playnite，支持扫码）
#[tauri::command]
pub async fn steam_login_webview(app: tauri::AppHandle) -> Result<String, String> {
    crate::steam_openid::open_login_webview(&app)?;
    Ok("已打开 Steam 登录窗口（支持扫码）".to_string())
}

/// 方式 B: 从用户粘贴的 URL 自动解析 SteamID64
#[tauri::command]
pub async fn steam_resolve_url(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
    url: String,
) -> Result<crate::steam_openid::SteamLoginResult, String> {
    super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let key = read_optional_steam_api_key(secret_store.inner())?;
    resolve_steam_url_with_key(&url, key.as_deref()).await
}

async fn resolve_steam_url_with_key(
    url: &str,
    api_key: Option<&str>,
) -> Result<crate::steam_openid::SteamLoginResult, String> {
    let sid = crate::steam_openid::resolve_steamid(url, api_key).await?;
    if let Some(key) = api_key.filter(|key| !key.trim().is_empty()) {
        if let Ok(info) = crate::steam_openid::fetch_player_summary(&sid, key).await {
            return Ok(info);
        }
    }
    Ok(crate::steam_openid::SteamLoginResult::from_id(
        sid, "resolved",
    ))
}

/// 方式 C: 尝试 OpenID 一键登录（部分网络可能被拦截）
#[tauri::command]
pub async fn steam_openid_login() -> Result<crate::steam_openid::SteamLoginResult, String> {
    crate::steam_openid::login_via_openid().await
}

/// 验证 Steam API Key；只有远端验证成功后才写入 SecretStore。
#[tauri::command]
pub async fn steam_verify_api_key(
    secret_store: State<'_, SecretStore>,
    api_key: String,
) -> Result<String, String> {
    verify_and_store_steam_api_key(secret_store.inner(), &api_key).await
}

/// 检测本地 Steam 客户端是否已登录，返回 SteamID64
#[tauri::command]
pub fn steam_detect_local() -> Result<Option<String>, String> {
    Ok(crate::steam_openid::detect_local_steam_id())
}

/// 获取 Steam 用户拥有的全部游戏（API Key 仅从 SecretStore 读取）
#[tauri::command]
pub async fn steam_fetch_owned_games(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
    steam_id: String,
) -> Result<crate::steam_openid::SteamOwnedGamesResponse, String> {
    super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let api_key = read_steam_api_key(secret_store.inner())?;
    crate::steam_openid::fetch_owned_games(&steam_id, &api_key).await
}

/// Enqueues one user-initiated Steam import run.
///
/// Steam library syncs are deliberately not keyed by a stable idempotency key:
/// after a task reaches a terminal state, the next explicit sync must have a
/// new job ID so it can transition through its own lifecycle. This preserves
/// reliable repeat operations instead of attaching them to a completed task.
fn enqueue_steam_import_run(
    queue: &TaskQueue,
    title: String,
    source: &str,
    reference_id: String,
) -> Result<crate::task_queue::TaskCenterJob, String> {
    queue.enqueue_operation(
        title,
        JobOperation::Import {
            source: source.to_string(),
            reference_id,
        },
        None,
    )
}

/// 一步完成：获取+导入 Steam 全库游戏
#[tauri::command]
pub async fn steam_fetch_and_import(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
    queue: State<'_, TaskQueue>,
    steam_id: String,
    app: tauri::AppHandle,
) -> Result<crate::steam_openid::SteamOwnedGamesResponse, String> {
    let job = enqueue_steam_import_run(
        queue.inner(),
        "同步并导入 Steam 游戏库".to_string(),
        "steam_account",
        "owned_games".to_string(),
    )?;
    queue.mark_running(
        &job.id,
        Some("正在获取 Steam 已拥有游戏".to_string()),
        Some(0.05),
    )?;

    let outcome = async {
        super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
        let api_key = read_steam_api_key(secret_store.inner())?;
        let resp = crate::steam_openid::fetch_owned_games(&steam_id, &api_key).await?;
        if resp.games.is_empty() {
            return Err("未获取到任何游戏。请检查 Steam 游戏详情公开状态和 API Key".to_string());
        }
        queue.append_event(
            &job.id,
            crate::task_queue::TaskEventLevel::Info,
            "steam_library_loaded".to_string(),
            format!("已读取 {} 个 Steam 游戏", resp.games.len()),
            Some(0.35),
        )?;
        let imported = import_steam_games(&db, &resp.games, &app)?;
        Ok::<_, String>(imported)
    }
    .await;

    match outcome {
        Ok(response) => {
            queue.mark_succeeded(
                &job.id,
                Some(format!(
                    "Steam 导入完成：新增 {}，更新 {}",
                    response.imported_count, response.updated_count
                )),
            )?;
            Ok(response)
        }
        Err(message) => {
            let _ = queue.mark_failed(
                &job.id,
                ProviderError {
                    kind: ProviderErrorKind::Unknown,
                    message: message.clone(),
                    retryable: true,
                    retry_after_ms: None,
                    provider_id: Some("steam".to_string()),
                    operation: Some("import".to_string()),
                },
            );
            Err(message)
        }
    }
}

/// 批量导入 Steam 全库游戏（按 app_id 去重）
#[tauri::command]
pub async fn steam_import_owned_games(
    db: State<'_, Database>,
    queue: State<'_, TaskQueue>,
    games: Vec<crate::steam_openid::SteamOwnedGame>,
    app: tauri::AppHandle,
) -> Result<crate::steam_openid::SteamOwnedGamesResponse, String> {
    let total = games.len();
    let job = enqueue_steam_import_run(
        queue.inner(),
        format!("导入 {total} 个 Steam 游戏"),
        "steam_owned_games",
        total.to_string(),
    )?;
    queue.mark_running(
        &job.id,
        Some("正在写入 Steam 游戏库".to_string()),
        Some(0.1),
    )?;
    match import_steam_games(&db, &games, &app) {
        Ok(response) => {
            queue.mark_succeeded(
                &job.id,
                Some(format!(
                    "Steam 导入完成：新增 {}，更新 {}",
                    response.imported_count, response.updated_count
                )),
            )?;
            Ok(response)
        }
        Err(message) => {
            let _ = queue.mark_failed(
                &job.id,
                ProviderError {
                    kind: ProviderErrorKind::Unknown,
                    message: message.clone(),
                    retryable: true,
                    retry_after_ms: None,
                    provider_id: Some("steam".to_string()),
                    operation: Some("import".to_string()),
                },
            );
            Err(message)
        }
    }
}

fn import_steam_games(
    db: &Database,
    games: &[crate::steam_openid::SteamOwnedGame],
    _app: &tauri::AppHandle,
) -> Result<crate::steam_openid::SteamOwnedGamesResponse, String> {
    let mut existing = db.get_games();
    let mut imported_count = 0u32;
    let mut updated_count = 0u32;
    let mut skipped_count = 0u32;

    for game in games {
        let aid = game.app_id.to_string();
        let launch_uri = platform_launch_uri("steam", &aid);
        let cover_url = steam_preferred_cover(&aid);
        let icon_url = steam_icon_url(&aid, game.img_icon_url.as_deref());
        let last_played = steam_last_played_string(game.rtime_last_played);

        if let Some(mut existing_game) = find_existing_platform_game(
            &existing,
            "steam",
            Some(&aid),
            launch_uri.as_deref(),
            None,
            launch_uri.as_deref(),
        ) {
            apply_platform_import_fields(
                &mut existing_game,
                "steam",
                Some(&aid),
                launch_uri.as_deref(),
                None,
                launch_uri.as_deref(),
                None,
                Some(&cover_url),
                icon_url.as_deref(),
                Some(game.playtime_forever),
                last_played.as_deref(),
                game.achievements_total,
                game.achievements_unlocked,
            );
            if existing_game.name.trim().is_empty() || existing_game.name.starts_with("steam_") {
                existing_game.name = game.name.clone();
            }
            match db.update_game(existing_game.clone()) {
                Ok(updated) => {
                    replace_cached_game(&mut existing, updated);
                    updated_count += 1;
                }
                Err(e) => {
                    skipped_count += 1;
                    tracing::warn!(app_id = %game.app_id, name = %game.name, error = %e, "Steam import update skipped");
                }
            }
            continue;
        }

        let mut new_game = Game::new(game.name.clone(), launch_uri.clone().unwrap_or_default());
        apply_platform_import_fields(
            &mut new_game,
            "steam",
            Some(&aid),
            launch_uri.as_deref(),
            None,
            launch_uri.as_deref(),
            None,
            Some(&cover_url),
            icon_url.as_deref(),
            Some(game.playtime_forever),
            last_played.as_deref(),
            game.achievements_total,
            game.achievements_unlocked,
        );

        match db.add_game(new_game) {
            Ok(created) => {
                existing.push(created);
                imported_count += 1;
            }
            Err(e) => {
                skipped_count += 1;
                tracing::warn!(app_id = %game.app_id, name = %game.name, error = %e, "Steam import create skipped");
            }
        }
    }

    tracing::info!(
        imported = imported_count,
        updated = updated_count,
        skipped = skipped_count,
        total = games.len(),
        "Steam library import complete"
    );
    Ok(crate::steam_openid::SteamOwnedGamesResponse {
        game_count: imported_count + updated_count,
        games: games.to_vec(),
        imported_count,
        updated_count,
        skipped_count,
    })
}

fn normalize_platform_source(platform: &str) -> String {
    match platform.trim().to_ascii_lowercase().as_str() {
        "epic" | "epic games" => "epic".to_string(),
        _ => "steam".to_string(),
    }
}

fn platform_launch_uri(source: &str, library_id: &str) -> Option<String> {
    let id = library_id.trim();
    if id.is_empty() {
        return None;
    }
    match source {
        "steam" => Some(format!("steam://rungameid/{}", id)),
        "epic" => Some(format!(
            "com.epicgames.launcher://apps/{}?action=launch&silent=true",
            id
        )),
        _ => None,
    }
}

fn platform_display_name(source: &str) -> String {
    match source {
        "epic" => "Epic".to_string(),
        _ => "Steam".to_string(),
    }
}

fn platform_store_url(source: &str, library_id: &str) -> Option<String> {
    match source {
        "steam" => Some(format!(
            "https://store.steampowered.com/app/{}/",
            library_id
        )),
        "epic" => Some("https://store.epicgames.com/library".to_string()),
        _ => None,
    }
}

fn steam_vertical_cover_url(appid: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg")
}

fn steam_generated_horizontal_cover(appid: &str, cover: &str) -> bool {
    let appid = appid.trim();
    if appid.is_empty() {
        return false;
    }

    let cover = cover.trim().to_ascii_lowercase();
    let app_path = format!("/steam/apps/{appid}/");
    if !cover.contains(&app_path) {
        return false;
    }

    cover.ends_with("/header.jpg") || cover.contains("/capsule_")
}

fn should_apply_platform_cover(
    source: &str,
    library_id: Option<&str>,
    current: Option<&str>,
) -> bool {
    let Some(current) = current.filter(|cover| !cover.trim().is_empty()) else {
        return true;
    };

    source == "steam"
        && library_id
            .map(|id| steam_generated_horizontal_cover(id, current))
            .unwrap_or(false)
}

fn steam_icon_url(appid: &str, icon_hash: Option<&str>) -> Option<String> {
    let hash = icon_hash?.trim();
    if hash.is_empty() {
        return None;
    }
    Some(format!(
        "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/{appid}/{hash}.jpg"
    ))
}

fn steam_last_played_string(timestamp_secs: Option<u64>) -> Option<String> {
    let timestamp = timestamp_secs?;
    chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
}

fn find_existing_platform_game(
    games: &[Game],
    source: &str,
    library_id: Option<&str>,
    launch_uri: Option<&str>,
    install_dir: Option<&str>,
    exe_path: Option<&str>,
) -> Option<Game> {
    games
        .iter()
        .find(|g| {
            let source_matches = g
                .library_source
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case(source))
                .unwrap_or(false);
            if source_matches && library_id.is_some() && g.library_id.as_deref() == library_id {
                return true;
            }
            if source == "steam" && library_id.is_some() && g.vndb_id.as_deref() == library_id {
                return true;
            }
            if let Some(uri) = launch_uri {
                if !uri.is_empty() && (g.launch_uri.as_deref() == Some(uri) || g.exe_path == uri) {
                    return true;
                }
            }
            if let Some(dir) = install_dir {
                if !dir.is_empty() && g.install_dir.as_deref() == Some(dir) {
                    return true;
                }
            }
            if let Some(exe) = exe_path {
                if !exe.is_empty() && g.exe_path == exe {
                    return true;
                }
            }
            false
        })
        .cloned()
}

fn apply_platform_import_fields(
    game: &mut Game,
    source: &str,
    library_id: Option<&str>,
    launch_uri: Option<&str>,
    install_dir: Option<&str>,
    exe_path: Option<&str>,
    store_url: Option<&str>,
    cover_url: Option<&str>,
    icon_url: Option<&str>,
    steam_playtime_minutes: Option<u32>,
    last_played: Option<&str>,
    achievements_total: Option<u32>,
    achievements_unlocked: Option<u32>,
) {
    game.normalize_for_persistence();

    let display = platform_display_name(source);
    game.library_source = Some(source.to_string());
    game.game_type = Some(display.clone());
    game.metadata.platform = Some(crate::models::GamePlatform::PC);
    game.last_imported_at = Some(chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string());

    if let Some(id) = library_id.filter(|id| !id.trim().is_empty()) {
        let id = id.trim().to_string();
        game.library_id = Some(id.clone());
        if let Some(url) = store_url
            .filter(|url| !url.trim().is_empty())
            .map(str::to_string)
            .or_else(|| platform_store_url(source, &id))
        {
            if !game
                .metadata
                .stores
                .iter()
                .any(|s| s.name == display || s.url == url)
            {
                game.metadata.stores.push(StoreLink {
                    name: display.clone(),
                    url,
                    price: None,
                    currency: None,
                });
            }
        }
    }

    if let Some(uri) = launch_uri.filter(|uri| !uri.trim().is_empty()) {
        game.launch_uri = Some(uri.to_string());
        game.exe_path = uri.to_string();
    } else if let Some(exe) = exe_path.filter(|exe| !exe.trim().is_empty()) {
        game.exe_path = exe.to_string();
    }

    if let Some(dir) = install_dir.filter(|dir| !dir.trim().is_empty()) {
        game.install_dir = Some(dir.to_string());
    }

    if let Some(cu) = cover_url.filter(|cu| !cu.trim().is_empty()) {
        if should_apply_platform_cover(source, library_id, game.metadata.cover.as_deref()) {
            game.metadata.cover = Some(cu.to_string());
        }
    }

    // Steam 横版 hero 背景（大屏 / 详情全屏背景）。尚无背景且本机缓存存在时填入本地图；
    // 其余情况（无本地缓存 / 存量游戏）由前端按 appid 兜底官方 library_hero。
    if source == "steam" {
        if let Some(id) = library_id.filter(|id| !id.trim().is_empty()) {
            let has_bg = game
                .metadata
                .background
                .as_deref()
                .map(|b| !b.trim().is_empty())
                .unwrap_or(false);
            if !has_bg {
                if let Some(hero) = steam_preferred_hero_local(id.trim()) {
                    game.metadata.background = Some(hero);
                }
            }
        }
    }

    if let Some(icon) = icon_url.filter(|icon| !icon.trim().is_empty()) {
        if game.icon.is_none() {
            game.icon = Some(icon.to_string());
        }
    }

    if let Some(minutes) = steam_playtime_minutes {
        let seconds = minutes as u64 * 60;
        if seconds > game.play_tracker.total_seconds {
            game.play_tracker.total_seconds = seconds;
        }
    }

    if let Some(last) = last_played.filter(|last| !last.trim().is_empty()) {
        game.play_tracker.last_played = Some(last.to_string());
    }

    if let Some(total) = achievements_total {
        game.play_tracker.achievements_total = total;
    }
    if let Some(unlocked) = achievements_unlocked {
        game.play_tracker.achievements_unlocked = unlocked;
    }

    game.sync_to_legacy();
    game.sync_tracker_to_legacy();
    game.touch_updated();
}

fn read_optional_steam_api_key(store: &SecretStore) -> Result<Option<String>, String> {
    store
        .get(SecretKind::SteamApiKey, None)
        .map_err(|error| error.to_string())
}

fn read_steam_api_key(store: &SecretStore) -> Result<String, String> {
    read_optional_steam_api_key(store)?
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| "缺少 Steam Web API Key".to_string())
}

async fn verify_and_store_steam_api_key(
    store: &SecretStore,
    api_key: &str,
) -> Result<String, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("Steam Web API Key 不能为空".to_string());
    }
    let result = crate::steam_openid::verify_api_key(api_key).await?;
    store
        .set(SecretKind::SteamApiKey, None, api_key)
        .map_err(|error| error.to_string())?;
    match store.get(SecretKind::SteamApiKey, None) {
        Ok(Some(stored)) if stored == api_key => Ok(result),
        _ => Err("Steam API Key 保存失败".to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncAchievementsResult {
    pub synced: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn sync_steam_achievements(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
) -> Result<SyncAchievementsResult, String> {
    tracing::info!("sync_steam_achievements: START");
    crate::crash_log("sync_steam_achievements: START");
    let settings =
        super::settings::load_settings_with_secret_migration(db.inner(), secret_store.inner())?;
    let steam_id = settings
        .steam_id
        .clone()
        .or_else(crate::steam_openid::detect_local_steam_id)
        .ok_or_else(|| "缺少 SteamID64，请先设置 Steam 账号".to_string())?;
    let api_key = read_steam_api_key(secret_store.inner())?;

    let games = db.get_games();
    let steam_games: Vec<(String, u32)> = games
        .iter()
        .filter_map(|g| {
            if g.library_source.as_deref() != Some("steam") {
                return None;
            }
            let appid = g.library_id.as_deref()?.trim().parse::<u32>().ok()?;
            Some((g.id.clone(), appid))
        })
        .collect();

    let mut result = SyncAchievementsResult {
        synced: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for (game_id, appid) in &steam_games {
        match crate::steam_openid::fetch_achievement_summary(&steam_id, &api_key, *appid).await {
            Ok(summary) => {
                if summary.total == 0 {
                    result.skipped += 1;
                    continue;
                }
                match db.update_achievements(game_id, summary.total, summary.unlocked) {
                    Ok(_) => result.synced += 1,
                    Err(e) => {
                        result.failed += 1;
                        result.errors.push(format!("appid {}: {}", appid, e));
                    }
                }
            }
            Err(_) => {
                result.skipped += 1;
            }
        }
        // Rate limit: Steam API has ~100k calls/day but be polite
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }

    tracing::info!(
        synced = result.synced,
        skipped = result.skipped,
        failed = result.failed,
        "sync_steam_achievements: COMPLETE"
    );
    Ok(result)
}

fn replace_cached_game(games: &mut Vec<Game>, updated: Game) {
    if let Some(slot) = games.iter_mut().find(|g| g.id == updated.id) {
        *slot = updated;
    } else {
        games.push(updated);
    }
}

#[cfg(test)]
mod platform_import_tests {
    use super::*;
    use crate::db_sqlite::SqliteDb;
    use crate::task_queue::TaskStatus;
    use std::sync::Arc;

    #[test]
    fn repeated_steam_import_runs_get_fresh_job_ids_after_terminal_completion() {
        let queue = TaskQueue::from_database(Arc::new(SqliteDb::open_in_memory().unwrap()));
        let first = enqueue_steam_import_run(
            &queue,
            "同步并导入 Steam 游戏库".to_string(),
            "steam_account",
            "owned_games".to_string(),
        )
        .unwrap();
        queue
            .mark_running(&first.id, Some("running".to_string()), Some(0.1))
            .unwrap();
        queue
            .mark_succeeded(&first.id, Some("done".to_string()))
            .unwrap();

        let second = enqueue_steam_import_run(
            &queue,
            "同步并导入 Steam 游戏库".to_string(),
            "steam_account",
            "owned_games".to_string(),
        )
        .unwrap();

        assert_ne!(first.id, second.id);
        queue
            .mark_running(&second.id, Some("running again".to_string()), Some(0.1))
            .unwrap();
        assert_eq!(
            queue.get_task_center(&second.id).unwrap().status,
            TaskStatus::Running
        );
    }

    #[test]
    fn steam_localconfig_parser_reads_apps_playtime_and_last_played() {
        let content = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "480"
                    {
                        "LastPlayed"        "1717557871"
                        "Playtime"          "43"
                    }
                    "210970"
                    {
                        "LastPlayed"        "1761219175"
                        "PlaytimeDisconnected"      "11"
                    }
                }
            }
        }
    }
}"#;

        let apps = parse_steam_localconfig_apps(content);
        assert_eq!(apps.len(), 2);
        assert_eq!(
            apps[0],
            SteamLocalConfigApp {
                appid: "480".to_string(),
                playtime_minutes: 43,
                last_played: Some(1717557871),
            }
        );
        assert_eq!(
            apps[1],
            SteamLocalConfigApp {
                appid: "210970".to_string(),
                playtime_minutes: 11,
                last_played: Some(1761219175),
            }
        );
    }

    #[test]
    fn steam_loginusers_parser_prefers_most_recent_and_maps_account_id() {
        let content = r#""users"
{
    "76561198000000001"
    {
        "MostRecent"       "0"
    }
    "76561199220678352"
    {
        "AccountName"      "tester"
        "MostRecent"      "1"
    }
}"#;

        assert_eq!(
            parse_most_recent_steamid64(content).as_deref(),
            Some("76561199220678352")
        );
        assert_eq!(
            steamid64_to_account_id("76561199220678352").as_deref(),
            Some("1260412624")
        );
    }

    #[test]
    fn steam_platform_fields_use_protocol_and_minutes() {
        let mut game = Game::new("Half-Life".to_string(), String::new());
        apply_platform_import_fields(
            &mut game,
            "steam",
            Some("70"),
            Some("steam://rungameid/70"),
            None,
            Some("steam://rungameid/70"),
            None,
            Some("https://cdn.cloudflare.steamstatic.com/steam/apps/70/library_600x900.jpg"),
            Some("https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/70/icon.jpg"),
            Some(90),
            Some("2024-01-02 03:04"),
            Some(10),
            Some(4),
        );

        assert_eq!(game.library_source.as_deref(), Some("steam"));
        assert_eq!(game.library_id.as_deref(), Some("70"));
        assert_eq!(game.launch_uri.as_deref(), Some("steam://rungameid/70"));
        assert_eq!(game.exe_path, "steam://rungameid/70");
        assert_eq!(
            game.cover.as_deref(),
            Some("https://cdn.cloudflare.steamstatic.com/steam/apps/70/library_600x900.jpg")
        );
        assert_eq!(
            game.icon.as_deref(),
            Some("https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/70/icon.jpg")
        );
        assert_eq!(game.play_tracker.total_seconds, 5400);
        assert_eq!(
            game.play_tracker.last_played.as_deref(),
            Some("2024-01-02 03:04")
        );
        assert_eq!(game.last_played.as_deref(), Some("2024-01-02 03:04"));
        assert_eq!(game.play_tracker.achievements_total, 10);
        assert_eq!(game.play_tracker.achievements_unlocked, 4);
        assert!(game.metadata.stores.iter().any(|s| s.name == "Steam"));
        assert!(game.vndb_id.is_none());
    }

    #[test]
    fn steam_import_upgrades_generated_horizontal_cover_only() {
        let vertical = "https://cdn.cloudflare.steamstatic.com/steam/apps/70/library_600x900.jpg";
        let mut generated = Game::new("Half-Life".to_string(), String::new());
        generated.metadata.cover =
            Some("https://cdn.cloudflare.steamstatic.com/steam/apps/70/header.jpg".to_string());
        generated.sync_to_legacy();
        apply_platform_import_fields(
            &mut generated,
            "steam",
            Some("70"),
            Some("steam://rungameid/70"),
            None,
            None,
            None,
            Some(vertical),
            None,
            Some(90),
            Some("2024-01-02 03:04"),
            Some(10),
            Some(4),
        );

        assert_eq!(generated.metadata.cover.as_deref(), Some(vertical));
        assert_eq!(generated.cover.as_deref(), Some(vertical));

        let mut custom = Game::new("Custom Cover".to_string(), String::new());
        custom.metadata.cover = Some("C:/covers/custom-half-life.jpg".to_string());
        custom.play_tracker.user_rating = Some(9.0);
        custom.sync_to_legacy();
        apply_platform_import_fields(
            &mut custom,
            "steam",
            Some("70"),
            Some("steam://rungameid/70"),
            None,
            None,
            None,
            Some(vertical),
            None,
            Some(120),
            Some("2024-02-03 04:05"),
            Some(20),
            Some(8),
        );

        assert_eq!(
            custom.metadata.cover.as_deref(),
            Some("C:/covers/custom-half-life.jpg")
        );
        assert_eq!(
            custom.cover.as_deref(),
            Some("C:/covers/custom-half-life.jpg")
        );
        assert_eq!(custom.play_tracker.user_rating, Some(9.0));
        assert_eq!(custom.rating, Some(9.0));
        assert_eq!(custom.play_tracker.total_seconds, 7200);
        assert_eq!(custom.play_tracker.achievements_total, 20);
        assert_eq!(custom.play_tracker.achievements_unlocked, 8);
    }

    #[test]
    fn steam_owned_game_candidate_preserves_achievements() {
        let candidate = owned_game_to_candidate(crate::steam_openid::SteamOwnedGame {
            app_id: 70,
            name: "Half-Life".to_string(),
            playtime_forever: 90,
            playtime_2weeks: None,
            rtime_last_played: Some(1704164640),
            img_icon_url: Some("iconhash".to_string()),
            img_logo_url: None,
            achievements_total: Some(10),
            achievements_unlocked: Some(4),
        });

        assert_eq!(candidate.library_id, "70");
        assert_eq!(candidate.achievements_total, Some(10));
        assert_eq!(candidate.achievements_unlocked, Some(4));
        assert!(candidate
            .cover_url
            .as_deref()
            .unwrap_or_default()
            .ends_with("/library_600x900.jpg"));
        assert!(candidate
            .icon_url
            .as_deref()
            .unwrap_or_default()
            .contains("iconhash"));
        assert_eq!(candidate.last_played.as_deref(), Some("2024-01-02 03:04"));
    }

    #[test]
    fn old_steam_vndb_id_is_still_matched_for_update() {
        let mut old = Game::new("Old Steam Game".to_string(), String::new());
        old.vndb_id = Some("123".to_string());
        let found = find_existing_platform_game(
            &[old.clone()],
            "steam",
            Some("123"),
            Some("steam://rungameid/123"),
            None,
            None,
        );
        assert_eq!(found.unwrap().id, old.id);
    }

    #[test]
    fn platform_uri_detection_skips_file_checks() {
        assert!(super::super::is_platform_launch_uri("steam://rungameid/70"));
        assert!(super::super::is_platform_launch_uri(
            "com.epicgames.launcher://apps/Foo?action=launch&silent=true"
        ));
        assert!(!super::super::is_platform_launch_uri(
            "C:/Games/Foo/game.exe"
        ));
    }
}
