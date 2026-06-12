use crate::db::Database;
use crate::models::Game;
use tauri::State;

#[tauri::command]
pub fn add_screenshot(db: State<'_, Database>, id: String, path: String) -> Result<Game, String> {
    db.add_screenshot(&id, path)
}

#[tauri::command]
pub fn remove_screenshot(
    db: State<'_, Database>,
    id: String,
    index: usize,
) -> Result<Game, String> {
    db.remove_screenshot(&id, index)
}

#[tauri::command]
pub fn remove_screenshot_by_path(
    db: State<'_, Database>,
    id: String,
    path: String,
) -> Result<Game, String> {
    db.remove_screenshot_by_path(&id, &path)
}

#[tauri::command]
pub fn set_screenshots(
    db: State<'_, Database>,
    id: String,
    screenshots: Vec<String>,
) -> Result<Game, String> {
    db.set_screenshots(&id, screenshots)
}
