//! Tauri command bridge for the Activity Dashboard foundation.
//!
//! Registration intentionally remains with the owning integration change:
//! 1) expose services::activity from lib.rs; 2) re-export this module from
//!    commands/mod.rs; 3) add the commands to tauri::generate_handler!.

use crate::db::Database;
use crate::domain::{ActivityEvent, ProgressRecord, ResourceKind};
use crate::services::activity::{
    ActivityEventPatch, ActivityEventView, ActivityEventsRequest, ActivityEventsResponse,
    ActivityExportFormat, ActivityFilters, ActivityService, ActivitySummary, BackfillReport,
    ContinueCandidate, ContinueQuery, DurationQuality,
};
use tauri::State;

#[tauri::command]
pub fn get_activity_events(
    db: State<'_, Database>,
    request: ActivityEventsRequest,
) -> Result<ActivityEventsResponse, String> {
    ActivityService::new(db.sqlite()).events(request)
}
#[tauri::command]
pub fn get_activity_summary(
    db: State<'_, Database>,
    filters: ActivityFilters,
) -> Result<ActivitySummary, String> {
    ActivityService::new(db.sqlite()).summary(filters)
}
#[tauri::command]
pub fn upsert_activity_event(
    db: State<'_, Database>,
    event: ActivityEvent,
    duration_quality: DurationQuality,
) -> Result<ActivityEventView, String> {
    ActivityService::new(db.sqlite()).upsert_event(event, duration_quality)
}
#[tauri::command]
pub fn edit_activity_event(
    db: State<'_, Database>,
    id: String,
    patch: ActivityEventPatch,
) -> Result<ActivityEventView, String> {
    ActivityService::new(db.sqlite()).edit_event(&id, patch)
}
#[tauri::command]
pub fn delete_activity_event(db: State<'_, Database>, id: String) -> Result<bool, String> {
    ActivityService::new(db.sqlite()).delete_event(&id)
}
#[tauri::command]
pub fn upsert_activity_progress(
    db: State<'_, Database>,
    record: ProgressRecord,
) -> Result<(), String> {
    ActivityService::new(db.sqlite()).upsert_progress(record)
}
#[tauri::command]
pub fn get_activity_progress(
    db: State<'_, Database>,
    resource_kind: ResourceKind,
    resource_id: String,
    provider_id: Option<String>,
) -> Result<Option<ProgressRecord>, String> {
    ActivityService::new(db.sqlite()).progress(resource_kind, &resource_id, provider_id.as_deref())
}
#[tauri::command]
pub fn get_continue_candidates(
    db: State<'_, Database>,
    query: Option<ContinueQuery>,
) -> Result<Vec<ContinueCandidate>, String> {
    ActivityService::new(db.sqlite())
        .continue_candidates(&db.get_games(), query.unwrap_or_default())
}
#[tauri::command]
pub fn backfill_legacy_game_activity(db: State<'_, Database>) -> Result<BackfillReport, String> {
    ActivityService::new(db.sqlite()).backfill_legacy_game_sessions(&db.get_games())
}
#[tauri::command]
pub fn export_activity_events(
    db: State<'_, Database>,
    filters: ActivityFilters,
    format: ActivityExportFormat,
    path: String,
) -> Result<String, String> {
    let mut scope = crate::security::app_data_scope()?;
    if let Some(documents) = dirs::document_dir() {
        scope.allow(documents);
    }
    let safe_path = scope.resolve(&path)?;
    let safe_path_text = safe_path.to_string_lossy().to_string();
    ActivityService::new(db.sqlite()).export_events_to_path(filters, format, &safe_path_text)?;
    Ok(safe_path_text)
}
