use crate::db::Database;
use crate::models::Settings;
use tauri::State;

#[tauri::command]
pub fn get_settings(db: State<'_, Database>) -> Settings {
    db.get_settings()
}

#[tauri::command]
pub fn update_settings(db: State<'_, Database>, settings: Settings) -> Result<Settings, String> {
    db.update_settings(settings)
}

#[tauri::command]
pub fn add_watch_dir(db: State<'_, Database>, dir: String) -> Result<Settings, String> {
    let mut settings = db.get_settings();
    if !settings.watch_dirs.contains(&dir) {
        settings.watch_dirs.push(dir);
    }
    db.update_settings(settings)
}

#[tauri::command]
pub fn remove_watch_dir(db: State<'_, Database>, dir: String) -> Result<Settings, String> {
    let mut settings = db.get_settings();
    settings.watch_dirs.retain(|d| d != &dir);
    db.update_settings(settings)
}
