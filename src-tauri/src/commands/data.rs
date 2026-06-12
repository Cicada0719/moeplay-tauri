use crate::db::Database;
use crate::migration;
use crate::models::AppDatabase;
use std::{fs, path::PathBuf};
use tauri::State;

#[tauri::command]
pub fn get_schema_version(db: State<'_, Database>) -> u32 {
    db.schema_version()
}

#[tauri::command]
pub fn get_game_count(db: State<'_, Database>) -> usize {
    db.game_count()
}

#[tauri::command]
pub fn get_migration_status(db: State<'_, Database>) -> Vec<migration::MigrationInfo> {
    migration::migration_status(db.schema_version())
}

#[tauri::command]
pub fn export_database(
    db: State<'_, Database>,
    export_path: Option<String>,
) -> Result<String, String> {
    let data = db.export_data();
    let path = export_path.map(PathBuf::from).unwrap_or_else(|| {
        dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(format!(
                "moeplay_export_{}.json",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            ))
    });
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_database(
    db: State<'_, Database>,
    import_path: String,
    merge: Option<bool>,
) -> Result<AppDatabase, String> {
    let text = fs::read_to_string(import_path).map_err(|e| e.to_string())?;
    let mut imported: AppDatabase = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    migration::run_migrations(&mut imported)?;

    if merge.unwrap_or(true) {
        let mut current = db.export_data();
        for game in imported.games {
            if !current
                .games
                .iter()
                .any(|existing| existing.exe_path == game.exe_path || existing.id == game.id)
            {
                current.games.push(game);
            }
        }
        current.settings = imported.settings;
        current.schema_version = migration::CURRENT_SCHEMA_VERSION;
        db.replace_data(current)
    } else {
        db.replace_data(imported)
    }
}
