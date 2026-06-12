use crate::db::Database;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn migrate_from_csharp(
    db: State<'_, Database>,
    source_path: String,
    app_handle: tauri::AppHandle,
) -> Result<crate::csharp_migration::MigrationReport, String> {
    let source = std::path::PathBuf::from(&source_path);
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("moeplay");

    let on_progress = {
        let handle = app_handle.clone();
        move |p: crate::csharp_migration::MigrationProgress| {
            let _ = handle.emit("migration-progress", &p);
        }
    };

    let result = crate::csharp_migration::migrate_from_csharp(
        &source,
        db.sqlite(),
        &data_dir,
        &on_progress,
    )?;

    tracing::info!(
        imported = result.imported,
        updated = result.updated,
        skipped = result.skipped,
        "C# migration complete"
    );

    Ok(result)
}

#[tauri::command]
pub fn verify_migration(
    db: State<'_, Database>,
    expected_count: usize,
) -> Result<crate::csharp_migration::MigrationVerifyReport, String> {
    crate::csharp_migration::verify_migration(db.sqlite(), expected_count)
}

#[tauri::command]
pub fn verify_migration_ids(
    db: State<'_, Database>,
    expected_count: usize,
    source_ids: Vec<String>,
) -> Result<crate::csharp_migration::MigrationVerifyReport, String> {
    crate::csharp_migration::verify_migration_for_ids(db.sqlite(), expected_count, &source_ids)
}
