use crate::db::Database;
use crate::models::{ScrapeResult, Settings};
use crate::secret_store::{SecretKind, SecretStore};
use crate::translator;
use tauri::State;

#[tauri::command]
pub async fn translate_scrape_metadata(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
    result: ScrapeResult,
    target_language: Option<String>,
) -> Result<translator::ChineseMeta, String> {
    let settings = crate::commands::settings::load_settings_with_secret_migration(
        db.inner(),
        secret_store.inner(),
    )?;
    let config =
        translation_config_from_settings(&settings, secret_store.inner(), target_language)?;
    Ok(translator::resolve_chinese_meta(&result, config.as_ref()).await)
}

#[tauri::command]
pub async fn translate_text(
    db: State<'_, Database>,
    secret_store: State<'_, SecretStore>,
    text: String,
    target_language: Option<String>,
) -> Result<String, String> {
    let settings = crate::commands::settings::load_settings_with_secret_migration(
        db.inner(),
        secret_store.inner(),
    )?;
    let config =
        translation_config_from_settings(&settings, secret_store.inner(), target_language)?
            .ok_or_else(|| "AI translation is not configured".to_string())?;
    translator::translate_text(&config, &text).await
}

#[tauri::command]
pub fn parse_chinese_metadata(text: String) -> translator::ChineseMeta {
    translator::parse_chinese_marker(&text)
}

#[tauri::command]
pub fn embed_chinese_metadata(text: Option<String>, meta: translator::ChineseMeta) -> String {
    translator::embed_chinese_marker(text.as_deref(), &meta)
}

#[tauri::command]
pub fn strip_metadata_markers(text: String) -> String {
    translator::strip_markers(&text)
}

#[tauri::command]
pub fn parse_scrape_marker(text: String) -> translator::ScrapeMarker {
    translator::parse_scrape_marker(&text)
}

#[tauri::command]
pub fn embed_scrape_marker(
    text: Option<String>,
    source: Option<String>,
    metadata_hash: Option<String>,
    cover_image: bool,
    background_image: bool,
) -> String {
    translator::embed_scrape_marker(
        text.as_deref(),
        source.as_deref(),
        metadata_hash.as_deref(),
        cover_image,
        background_image,
    )
}

fn translation_config_from_settings(
    settings: &Settings,
    secret_store: &SecretStore,
    target_language: Option<String>,
) -> Result<Option<translator::TranslationConfig>, String> {
    if !settings.ai_enabled {
        return Ok(None);
    }

    crate::scraper::ai_presets::validate_endpoint(&settings.ai_api_url)?;
    let api_key = secret_store
        .get(SecretKind::AiApiKey, Some(settings.ai_api_url.as_str()))
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    if api_key.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(translator::TranslationConfig {
        api_url: settings.ai_api_url.clone(),
        api_key,
        model: settings.ai_model.clone(),
        target_language: target_language.unwrap_or_else(|| "Simplified Chinese".to_string()),
    }))
}
