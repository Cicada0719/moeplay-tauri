use crate::db::Database;
use crate::models::{AppearanceSettings, ThemePackId};
use futures_util::StreamExt;
use image::ImageReader;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use tauri::State;
use url::Url;

const MANIFEST_URL: &str =
    "https://github.com/Cicada0719/moeplay-tauri/releases/latest/download/wallpapers.json";
const MAX_WALLPAPER_BYTES: u64 = 20 * 1024 * 1024;
const MAX_DECODE_PIXELS: u64 = 80_000_000;
const MIN_WIDTH: u32 = 1920;
const MIN_HEIGHT: u32 = 1080;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WallpaperRating {
    General,
    Suggestive,
    Adult,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WallpaperAsset {
    pub id: String,
    pub theme_pack: ThemePackId,
    pub title: String,
    pub download_url: String,
    pub preview_url: String,
    pub sha256: String,
    pub byte_size: u64,
    pub width: u32,
    pub height: u32,
    pub rating: WallpaperRating,
    pub author: String,
    pub source_url: String,
    pub license_id: String,
    pub license_url: String,
    pub attribution_required: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WallpaperManifest {
    pub schema_version: u32,
    pub revision: String,
    pub generated_at: String,
    pub assets: Vec<WallpaperAsset>,
}

#[derive(Debug, Serialize, Clone)]
pub struct WallpaperRecord {
    #[serde(flatten)]
    pub asset: WallpaperAsset,
    pub installed: bool,
    pub local_path: Option<String>,
    pub source: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct WallpaperAttribution {
    pub id: String,
    pub title: String,
    pub author: String,
    pub source_url: String,
    pub license_id: String,
    pub license_url: String,
    pub attribution_required: bool,
}

#[derive(Debug, Serialize)]
pub struct WallpaperSyncResult {
    pub revision: String,
    pub available: usize,
    pub downloaded: usize,
}

#[derive(Debug, Serialize)]
pub struct ThemePackSummary {
    pub id: ThemePackId,
    pub label: &'static str,
    pub description: &'static str,
    pub default_color_mode: &'static str,
}

fn root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("moeplay")
        .join("wallpapers")
}
fn manifest_path() -> PathBuf {
    root().join("manifest.json")
}
fn files_dir() -> PathBuf {
    root().join("files")
}
fn custom_dir() -> PathBuf {
    root().join("custom")
}
fn ensure_dirs() -> Result<(), String> {
    fs::create_dir_all(files_dir())
        .and_then(|_| fs::create_dir_all(custom_dir()))
        .map_err(|e| e.to_string())
}
fn load_manifest() -> Result<WallpaperManifest, String> {
    serde_json::from_slice(&fs::read(manifest_path()).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn extension_for_mime(mime: &str) -> Option<&'static str> {
    match mime.split(';').next()?.trim() {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}
fn safe_id(id: &str) -> Result<&str, String> {
    if !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
    {
        Ok(id)
    } else {
        Err("invalid wallpaper id".into())
    }
}
fn allowed_host(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some(
            "github.com"
                | "objects.githubusercontent.com"
                | "githubusercontent.com"
                | "raw.githubusercontent.com"
        )
    )
}

fn rating_allowed(rating: &WallpaperRating, mode: &str) -> bool {
    match mode {
        "hide" => matches!(rating, WallpaperRating::General),
        "blur" | "show" => true,
        _ => matches!(rating, WallpaperRating::General),
    }
}

fn validate_asset(asset: &WallpaperAsset) -> Result<(), String> {
    safe_id(&asset.id)?;
    if asset.byte_size > MAX_WALLPAPER_BYTES {
        return Err("wallpaper exceeds 20MB".into());
    }
    if asset.width < MIN_WIDTH
        || asset.height < MIN_HEIGHT
        || u64::from(asset.width) * u64::from(asset.height) > MAX_DECODE_PIXELS
    {
        return Err("wallpaper dimensions are outside allowed range".into());
    }
    if asset.sha256.len() != 64 || !asset.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid wallpaper sha256".into());
    }
    let url = Url::parse(&asset.download_url).map_err(|e| e.to_string())?;
    if url.scheme() != "https" || !allowed_host(&url) {
        return Err("wallpaper download host is not allowed".into());
    }
    Ok(())
}
fn inspect_image(path: &Path) -> Result<(u32, u32), String> {
    let reader = ImageReader::new(BufReader::new(
        fs::File::open(path).map_err(|e| e.to_string())?,
    ))
    .with_guessed_format()
    .map_err(|e| e.to_string())?;
    let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if w < MIN_WIDTH || h < MIN_HEIGHT || u64::from(w) * u64::from(h) > MAX_DECODE_PIXELS {
        return Err("image dimensions are outside allowed range".into());
    }
    Ok((w, h))
}
fn find_local(id: &str) -> Option<PathBuf> {
    fs::read_dir(files_dir())
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.file_stem().and_then(|s| s.to_str()) == Some(id))
}

#[tauri::command]
pub fn list_theme_packs() -> Vec<ThemePackSummary> {
    vec![
        ThemePackSummary {
            id: ThemePackId::Yozakura,
            label: "夜樱终端",
            description: "夜樱、日式城市与玫红终端光",
            default_color_mode: "dark",
        },
        ThemePackSummary {
            id: ThemePackId::AfterSchool,
            label: "青空放课后",
            description: "晴空、海风与校园午后",
            default_color_mode: "light",
        },
        ThemePackSummary {
            id: ThemePackId::NeonIsekai,
            label: "霓虹异界",
            description: "雨夜都市与电青霓虹",
            default_color_mode: "dark",
        },
    ]
}

#[tauri::command]
pub fn list_wallpapers(theme_pack: Option<ThemePackId>) -> Result<Vec<WallpaperRecord>, String> {
    let manifest = match load_manifest() {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };
    Ok(manifest
        .assets
        .into_iter()
        .filter(|a| theme_pack.as_ref().is_none_or(|p| p == &a.theme_pack))
        .map(|asset| {
            let local = find_local(&asset.id);
            WallpaperRecord {
                installed: local.is_some(),
                local_path: local.map(|p| p.to_string_lossy().into_owned()),
                asset,
                source: "gallery".into(),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn refresh_wallpaper_manifest(
    nsfw_mode: Option<String>,
) -> Result<WallpaperSyncResult, String> {
    ensure_dirs()?;
    let bytes = reqwest::Client::new()
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > 2 * 1024 * 1024 {
        return Err("wallpaper manifest is too large".into());
    }
    let manifest: WallpaperManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if manifest.schema_version != 1 {
        return Err("unsupported wallpaper manifest schema".into());
    }
    let mut ids = HashSet::new();
    for asset in &manifest.assets {
        validate_asset(asset)?;
        if !ids.insert(&asset.id) {
            return Err("duplicate wallpaper id".into());
        }
    }
    let tmp = manifest_path().with_extension("tmp");
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, manifest_path()).map_err(|e| e.to_string())?;
    let mode = nsfw_mode.as_deref().unwrap_or("blur");
    let available = manifest
        .assets
        .iter()
        .filter(|asset| rating_allowed(&asset.rating, mode))
        .count();
    Ok(WallpaperSyncResult {
        revision: manifest.revision,
        available,
        downloaded: 0,
    })
}

#[tauri::command]
pub async fn download_wallpaper(
    id: String,
    nsfw_mode: Option<String>,
) -> Result<WallpaperRecord, String> {
    ensure_dirs()?;
    safe_id(&id)?;
    let asset = load_manifest()?
        .assets
        .into_iter()
        .find(|a| a.id == id)
        .ok_or("wallpaper not found")?;
    validate_asset(&asset)?;
    if !rating_allowed(&asset.rating, nsfw_mode.as_deref().unwrap_or("blur")) {
        return Err("current NSFW policy does not allow this wallpaper".into());
    }
    let response = reqwest::Client::new()
        .get(&asset.download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .ok_or("missing content type")?;
    let ext = extension_for_mime(mime).ok_or("unsupported wallpaper MIME type")?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_WALLPAPER_BYTES)
    {
        return Err("wallpaper exceeds 20MB".into());
    }
    let target = files_dir().join(format!("{}.{}", asset.id, ext));
    let tmp = target.with_extension(format!("{}.tmp", ext));
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        size += chunk.len() as u64;
        if size > MAX_WALLPAPER_BYTES {
            let _ = fs::remove_file(&tmp);
            return Err("wallpaper exceeds 20MB".into());
        }
        hasher.update(&chunk);
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }
    file.sync_all().map_err(|e| e.to_string())?;
    if hex::encode(hasher.finalize()) != asset.sha256.to_ascii_lowercase() {
        let _ = fs::remove_file(&tmp);
        return Err("wallpaper checksum mismatch".into());
    }
    inspect_image(&tmp)?;
    fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
    Ok(WallpaperRecord {
        asset,
        installed: true,
        local_path: Some(target.to_string_lossy().into_owned()),
        source: "gallery".into(),
    })
}

#[tauri::command]
pub fn import_wallpaper() -> Result<Option<WallpaperRecord>, String> {
    ensure_dirs()?;
    let Some(source) = rfd::FileDialog::new()
        .add_filter("Images", &["jpg", "jpeg", "png", "webp"])
        .pick_file()
    else {
        return Ok(None);
    };
    let metadata = fs::metadata(&source).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_WALLPAPER_BYTES {
        return Err("wallpaper exceeds 20MB".into());
    }
    let (width, height) = inspect_image(&source)?;
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|e| matches!(e.as_str(), "jpg" | "jpeg" | "png" | "webp"))
        .ok_or("unsupported wallpaper type")?;
    let bytes = fs::read(&source).map_err(|e| e.to_string())?;
    let hash = hex::encode(Sha256::digest(&bytes));
    let id = format!("custom-{}", &hash[..16]);
    let target = custom_dir().join(format!("{}.{}", id, ext));
    let tmp = target.with_extension(format!("{}.tmp", ext));
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
    let asset = WallpaperAsset {
        id: id.clone(),
        theme_pack: ThemePackId::Yozakura,
        title: source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Custom wallpaper")
            .to_string(),
        download_url: String::new(),
        preview_url: String::new(),
        sha256: hash,
        byte_size: metadata.len(),
        width,
        height,
        rating: WallpaperRating::General,
        author: "Local user".into(),
        source_url: String::new(),
        license_id: "Local-Only".into(),
        license_url: String::new(),
        attribution_required: false,
    };
    Ok(Some(WallpaperRecord {
        asset,
        installed: true,
        local_path: Some(target.to_string_lossy().into_owned()),
        source: "custom".into(),
    }))
}

#[tauri::command]
pub fn delete_wallpaper(id: String, db: State<'_, Database>) -> Result<(), String> {
    safe_id(&id)?;
    for dir in [files_dir(), custom_dir()] {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.file_stem().and_then(|s| s.to_str()) == Some(&id) {
                    fs::remove_file(path).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    let mut settings = db.get_settings();
    if let Some(a) = &mut settings.appearance {
        if a.fixed_wallpaper_id.as_deref() == Some(&id) {
            a.fixed_wallpaper_id = None;
            a.wallpaper_rotation = crate::models::WallpaperRotation::StartupRandom;
        }
    }
    db.update_settings(settings)?;
    Ok(())
}

#[tauri::command]
pub fn set_active_appearance(
    mut settings: AppearanceSettings,
    db: State<'_, Database>,
) -> Result<AppearanceSettings, String> {
    if settings.color_mode == crate::models::ColorMode::Contrast {
        settings.decorative_effects = false;
    }
    let mut all = db.get_settings();
    all.appearance = Some(settings.clone());
    db.update_settings(all)?;
    Ok(settings)
}

#[tauri::command]
pub fn get_wallpaper_attribution(id: String) -> Result<WallpaperAttribution, String> {
    safe_id(&id)?;
    let a = load_manifest()?
        .assets
        .into_iter()
        .find(|a| a.id == id)
        .ok_or("wallpaper not found")?;
    Ok(WallpaperAttribution {
        id: a.id,
        title: a.title,
        author: a.author,
        source_url: a.source_url,
        license_id: a.license_id,
        license_url: a.license_url,
        attribution_required: a.attribution_required,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_ids() {
        assert!(safe_id("../x").is_err());
        assert!(safe_id("ok:id-1").is_ok());
    }
    #[test]
    fn legacy_appearance_migration() {
        let mut s = crate::models::Settings::default();
        s.appearance = None;
        s.theme = "light".into();
        s.normalize_appearance();
        let a = s.appearance.unwrap();
        assert_eq!(a.theme_pack, ThemePackId::AfterSchool);
        assert_eq!(a.color_mode, crate::models::ColorMode::Light);
    }
}
