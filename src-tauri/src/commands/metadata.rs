use crate::db::Database;
use crate::models::{Game, GameMetadata, GamePlatform};
use tauri::State;

// ===== Metadata =====

#[tauri::command]
pub fn update_game_metadata(
    db: State<'_, Database>,
    id: String,
    metadata: GameMetadata,
) -> Result<Game, String> {
    db.update_game_metadata(&id, metadata)
}

#[tauri::command]
pub fn update_developer(
    db: State<'_, Database>,
    id: String,
    developer: Option<String>,
) -> Result<Game, String> {
    db.update_developer(&id, developer)
}

#[tauri::command]
pub fn update_publisher(
    db: State<'_, Database>,
    id: String,
    publisher: Option<String>,
) -> Result<Game, String> {
    db.update_publisher(&id, publisher)
}

#[tauri::command]
pub fn update_platform(
    db: State<'_, Database>,
    id: String,
    platform: Option<GamePlatform>,
) -> Result<Game, String> {
    db.update_platform(&id, platform)
}

#[tauri::command]
pub fn update_engine(
    db: State<'_, Database>,
    id: String,
    engine: Option<String>,
) -> Result<Game, String> {
    db.update_engine(&id, engine)
}

#[tauri::command]
pub fn update_game_version(
    db: State<'_, Database>,
    id: String,
    version: Option<String>,
) -> Result<Game, String> {
    db.update_game_version(&id, version)
}

#[tauri::command]
pub fn update_original_name(
    db: State<'_, Database>,
    id: String,
    original_name: Option<String>,
) -> Result<Game, String> {
    db.update_original_name(&id, original_name)
}

#[tauri::command]
pub fn update_homepage(
    db: State<'_, Database>,
    id: String,
    homepage: Option<String>,
) -> Result<Game, String> {
    db.update_homepage(&id, homepage)
}

#[tauri::command]
pub fn update_developer_homepage(
    db: State<'_, Database>,
    id: String,
    homepage: Option<String>,
) -> Result<Game, String> {
    db.update_developer_homepage(&id, homepage)
}

#[tauri::command]
pub fn update_age_rating(
    db: State<'_, Database>,
    id: String,
    age_rating: Option<String>,
) -> Result<Game, String> {
    db.update_age_rating(&id, age_rating)
}

#[tauri::command]
pub fn update_series(
    db: State<'_, Database>,
    id: String,
    series: Option<String>,
) -> Result<Game, String> {
    db.update_series(&id, series)
}

#[tauri::command]
pub fn update_release_date(
    db: State<'_, Database>,
    id: String,
    release_date: Option<String>,
) -> Result<Game, String> {
    db.update_release_date(&id, release_date)
}

#[tauri::command]
pub fn update_release_year(
    db: State<'_, Database>,
    id: String,
    release_year: Option<u32>,
) -> Result<Game, String> {
    db.update_release_year(&id, release_year)
}

#[tauri::command]
pub fn update_estimated_hours(
    db: State<'_, Database>,
    id: String,
    hours: Option<f64>,
) -> Result<Game, String> {
    db.update_estimated_hours(&id, hours)
}

#[tauri::command]
pub fn update_vndb_rating(
    db: State<'_, Database>,
    id: String,
    rating: Option<f64>,
) -> Result<Game, String> {
    db.update_vndb_rating(&id, rating)
}

#[tauri::command]
pub fn update_bangumi_rating(
    db: State<'_, Database>,
    id: String,
    rating: Option<f64>,
) -> Result<Game, String> {
    db.update_bangumi_rating(&id, rating)
}

#[tauri::command]
pub fn update_vndb_id(
    db: State<'_, Database>,
    id: String,
    vndb_id: Option<String>,
) -> Result<Game, String> {
    db.update_vndb_id(&id, vndb_id)
}

#[tauri::command]
pub fn update_bangumi_id(
    db: State<'_, Database>,
    id: String,
    bangumi_id: Option<String>,
) -> Result<Game, String> {
    db.update_bangumi_id(&id, bangumi_id)
}

#[tauri::command]
pub fn set_genres(
    db: State<'_, Database>,
    id: String,
    genres: Vec<String>,
) -> Result<Game, String> {
    db.set_genres(&id, genres)
}

#[tauri::command]
pub fn set_languages(
    db: State<'_, Database>,
    id: String,
    languages: Vec<String>,
) -> Result<Game, String> {
    db.set_languages(&id, languages)
}

#[tauri::command]
pub fn set_voice_languages(
    db: State<'_, Database>,
    id: String,
    voice_languages: Vec<String>,
) -> Result<Game, String> {
    db.set_voice_languages(&id, voice_languages)
}
