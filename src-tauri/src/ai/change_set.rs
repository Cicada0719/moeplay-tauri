use crate::ai::contracts::{LibraryChangeSetOutput, LibraryOperation};
use crate::ai::schema::ValidatedLibraryChangeSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSetState {
    AwaitingConfirmation,
    Applied,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewOperation {
    pub operation: LibraryOperation,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChangeSetPreview {
    pub id: String,
    pub task_id: String,
    pub summary: String,
    pub confidence: f64,
    pub state: ChangeSetState,
    pub operations: Vec<PreviewOperation>,
}

/// A preview can only be created from schema- and business-validated output.
/// All operations are unselected so no mutation is implied before confirmation.
pub fn build_library_change_set_preview(
    change_set_id: impl Into<String>,
    task_id: impl Into<String>,
    validated: ValidatedLibraryChangeSet,
) -> AiChangeSetPreview {
    let LibraryChangeSetOutput {
        summary,
        confidence,
        operations,
    } = validated.into_inner();
    AiChangeSetPreview {
        id: change_set_id.into(),
        task_id: task_id.into(),
        summary,
        confidence,
        state: ChangeSetState::AwaitingConfirmation,
        operations: operations
            .into_iter()
            .map(|operation| PreviewOperation {
                operation,
                selected: false,
            })
            .collect(),
    }
}
