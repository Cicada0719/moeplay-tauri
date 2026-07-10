//! Batch 2 library commands.
//!
//! Shared registration request (intentionally not applied here because `commands/mod.rs`,
//! `lib.rs`, capabilities, and generated permissions are shared-owned):
//! - expose `services/library` from the shared service module (`pub mod services; pub mod library;`)
//! - add `mod library_v2; pub use library_v2::*;` to `commands/mod.rs`
//! - add every name in `LIBRARY_V2_SHARED_COMMAND_REGISTRATION_REQUEST` to the Tauri invoke handler
//! - generate/review the matching permissions before exposing these commands

use crate::db::Database;
use crate::db_sqlite::repositories::BackgroundJobRepository;
use crate::domain::BackgroundJobStatus;
use crate::services::library::{
    apply_import, launch, launch_descriptor, library_health, preview_import, ApplyImportRequest,
    ApplyImportResponse, DatabaseLibraryBackend, ImportPreview, LaunchDescriptor, LaunchOutcome,
    LibraryHealthSnapshot, LibraryImportBackend, PreviewImportRequest,
};
use tauri::State;

pub const LIBRARY_V2_SHARED_COMMAND_REGISTRATION_REQUEST: &[&str] = &[
    "library_v2_preview_import",
    "library_v2_apply_import",
    "library_v2_health",
    "library_v2_launch_descriptor",
    "library_v2_launch",
];

#[tauri::command]
pub fn library_v2_preview_import(
    db: State<'_, Database>,
    request: PreviewImportRequest,
) -> Result<ImportPreview, String> {
    let backend = DatabaseLibraryBackend::new(&db);
    let games = backend.list_games()?;
    let ledger = backend.load_provenance()?;
    Ok(preview_import(&games, &ledger, request))
}

#[tauri::command]
pub fn library_v2_apply_import(
    db: State<'_, Database>,
    request: ApplyImportRequest,
) -> Result<ApplyImportResponse, String> {
    apply_import(&DatabaseLibraryBackend::new(&db), request)
}

#[tauri::command]
pub fn library_v2_health(db: State<'_, Database>) -> Result<LibraryHealthSnapshot, String> {
    let backend = DatabaseLibraryBackend::new(&db);
    let games = backend.list_games()?;
    let ledger = backend.load_provenance()?;
    let jobs = BackgroundJobRepository::new(db.sqlite()).list(
        &[BackgroundJobStatus::Running, BackgroundJobStatus::Failed],
        500,
    )?;
    let conflicts = jobs
        .iter()
        .filter(|job| job.kind == "library_import_v2")
        .filter_map(|job| job.metadata.get("response"))
        .filter_map(|response| response.get("results"))
        .filter_map(|results| results.as_array())
        .flatten()
        .filter(|result| result.get("status").and_then(|value| value.as_str()) == Some("conflict"))
        .count();
    Ok(library_health(&games, &ledger, conflicts))
}

#[tauri::command]
pub fn library_v2_launch_descriptor(
    db: State<'_, Database>,
    game_id: String,
) -> Result<LaunchDescriptor, String> {
    db.get_game(&game_id).map(|game| launch_descriptor(&game))
}

#[tauri::command]
pub fn library_v2_launch(
    db: State<'_, Database>,
    game_id: String,
) -> Result<LaunchOutcome, String> {
    db.get_game(&game_id)
        .map(|game| launch(launch_descriptor(&game)))
}
