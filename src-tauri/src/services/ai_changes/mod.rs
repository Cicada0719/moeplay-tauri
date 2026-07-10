mod error;
mod service;
mod store;
mod validation;

pub use error::{AiChangesError, AiChangesErrorCode, AiChangesResult};
pub use service::AiChangesService;
pub use store::{AiUndoRecord, AI_UNDO_RECORD_VERSION};

use crate::ai::change_set::AiChangeSetPreview;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiChangeProvenance {
    pub provider_id: String,
    pub model: String,
    pub prompt_id: String,
    pub prompt_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewAiChangesRequest {
    pub change_set: AiChangeSetPreview,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyAiChangesRequest {
    pub change_set: AiChangeSetPreview,
    pub selected_operation_indices: Vec<usize>,
    pub provenance: AiChangeProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UndoAiChangesRequest {
    pub undo_id: String,
    pub change_set_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChangeOperationPreview {
    pub operation_index: usize,
    pub kind: String,
    pub game_ids: Vec<String>,
    pub field: Option<String>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub reason: String,
    pub applicable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAiChangesResponse {
    pub change_set_id: String,
    pub task_id: String,
    pub operations: Vec<AiChangeOperationPreview>,
    pub write_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiChangesApplyStatus {
    Applied,
    NoChanges,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyAiChangesResponse {
    pub status: AiChangesApplyStatus,
    pub change_set_id: String,
    pub selected_operation_count: usize,
    pub changed_field_count: usize,
    pub undo_id: Option<String>,
    pub applied_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiChangesUndoStatus {
    Undone,
    AlreadyUndone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoAiChangesResponse {
    pub status: AiChangesUndoStatus,
    pub undo_id: String,
    pub change_set_id: String,
    pub restored_field_count: usize,
    pub undone_at: String,
}
