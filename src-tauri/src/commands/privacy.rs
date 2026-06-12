use crate::db::Database;
use crate::models::{Game, Settings};
use crate::nsfw;
use tauri::State;

#[tauri::command]
pub fn get_nsfw_decision(
    db: State<'_, Database>,
    game_id: String,
    mode: Option<String>,
) -> Result<nsfw::NsfwDecision, String> {
    let game = db.get_game(&game_id)?;
    Ok(nsfw::decide(&game, resolve_nsfw_mode(&db, mode)))
}

#[tauri::command]
pub fn classify_nsfw_game(game: Game, mode: Option<String>) -> nsfw::NsfwDecision {
    nsfw::decide(&game, nsfw::NsfwDisplayMode::parse(mode.as_deref()))
}

#[tauri::command]
pub fn get_games_nsfw_filtered(db: State<'_, Database>, mode: Option<String>) -> Vec<Game> {
    let mode = resolve_nsfw_mode(&db, mode);
    nsfw::filter_games(db.get_games(), mode)
}

#[tauri::command]
pub fn update_nsfw_display_mode(db: State<'_, Database>, mode: String) -> Result<Settings, String> {
    let parsed = nsfw::NsfwDisplayMode::parse(Some(&mode));
    let mut settings = db.get_settings();
    settings.nsfw_display_mode = parsed.as_str().to_string();
    db.update_settings(settings)
}

fn resolve_nsfw_mode(db: &Database, mode: Option<String>) -> nsfw::NsfwDisplayMode {
    let settings = db.get_settings();
    nsfw::NsfwDisplayMode::parse(mode.as_deref().or(Some(&settings.nsfw_display_mode)))
}
