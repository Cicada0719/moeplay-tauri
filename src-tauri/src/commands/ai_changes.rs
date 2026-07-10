use crate::db::Database;
use crate::services::ai_changes::{
    AiChangesError, AiChangesService, ApplyAiChangesRequest, ApplyAiChangesResponse,
    PreviewAiChangesRequest, PreviewAiChangesResponse, UndoAiChangesRequest, UndoAiChangesResponse,
};
use tauri::State;

#[tauri::command]
pub fn ai_changes_preview(
    database: State<'_, Database>,
    service: State<'_, AiChangesService>,
    request: PreviewAiChangesRequest,
) -> Result<PreviewAiChangesResponse, AiChangesError> {
    service.preview(&database, request)
}

#[tauri::command]
pub fn ai_changes_apply(
    database: State<'_, Database>,
    service: State<'_, AiChangesService>,
    request: ApplyAiChangesRequest,
) -> Result<ApplyAiChangesResponse, AiChangesError> {
    service.apply(&database, request)
}

#[tauri::command]
pub fn ai_changes_undo(
    database: State<'_, Database>,
    service: State<'_, AiChangesService>,
    request: UndoAiChangesRequest,
) -> Result<UndoAiChangesResponse, AiChangesError> {
    service.undo(&database, request)
}
