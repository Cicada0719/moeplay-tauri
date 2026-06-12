use crate::db::Database;
use crate::{diagnostics, performance};
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn get_performance_snapshot(db: State<'_, Database>) -> performance::PerformanceSnapshot {
    performance::snapshot(&db)
}

#[tauri::command]
pub fn run_diagnostics(db: State<'_, Database>) -> diagnostics::DiagnosticsReport {
    let count = db.game_count() as u32;
    let data_dir = dirs::data_dir().unwrap_or_default().join("moeplay");
    let db_size = std::fs::metadata(data_dir.join("moegame.db"))
        .map(|m| m.len())
        .unwrap_or(0);
    diagnostics::run_diagnostics(&data_dir, count, db_size)
}

#[tauri::command]
pub fn export_diagnostics_zip(db: State<'_, Database>) -> Result<String, String> {
    let count = db.game_count() as u32;
    let data_dir = dirs::data_dir().unwrap_or_default().join("moeplay");
    let db_size = std::fs::metadata(data_dir.join("moegame.db"))
        .map(|m| m.len())
        .unwrap_or(0);

    let export_path = dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(format!(
            "moeplay_diagnostics_{}.zip",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ));

    diagnostics::export_diagnostics_zip(&export_path, &data_dir, count, db_size)
}
