use serde::{Deserialize, Serialize};

const DESKTOP_ONLY: &str = "Steam 集成仅支持桌面端";

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
            profile_url: format!("https://steamcommunity.com/profiles/{sid}"),
            steam_id: sid,
            personaname: String::new(),
            avatar: String::new(),
            login_method: method.to_string(),
        }
    }
}

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

pub async fn resolve_steamid(_input: &str, _api_key: Option<&str>) -> Result<String, String> {
    Err(DESKTOP_ONLY.to_string())
}

pub fn open_steam_community() -> Result<(), String> {
    Err(DESKTOP_ONLY.to_string())
}

pub fn open_steam_edit_profile() -> Result<(), String> {
    Err(DESKTOP_ONLY.to_string())
}

pub async fn login_via_openid() -> Result<SteamLoginResult, String> {
    Err(DESKTOP_ONLY.to_string())
}

pub async fn fetch_player_summary(
    _steam_id: &str,
    _api_key: &str,
) -> Result<SteamLoginResult, String> {
    Err(DESKTOP_ONLY.to_string())
}

pub async fn fetch_owned_games(
    _steam_id: &str,
    _api_key: &str,
) -> Result<SteamOwnedGamesResponse, String> {
    Err(DESKTOP_ONLY.to_string())
}

pub async fn fetch_achievement_summary(
    _steam_id: &str,
    _api_key: &str,
    _app_id: u32,
) -> Result<SteamAchievementSummary, String> {
    Err(DESKTOP_ONLY.to_string())
}

pub async fn verify_api_key(_api_key: &str) -> Result<String, String> {
    Err(DESKTOP_ONLY.to_string())
}

pub fn detect_local_steam_id() -> Option<String> {
    None
}

pub fn is_steam_running() -> bool {
    false
}

pub fn open_login_webview(_app_handle: &tauri::AppHandle) -> Result<(), String> {
    Err(DESKTOP_ONLY.to_string())
}
