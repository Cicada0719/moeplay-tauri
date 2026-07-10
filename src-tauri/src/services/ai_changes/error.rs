use serde::{Deserialize, Serialize};
use std::fmt;

pub type AiChangesResult<T> = Result<T, AiChangesError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiChangesErrorCode {
    InvalidChangeSet,
    InvalidSelection,
    UnsupportedOperation,
    DatabaseUnavailable,
    UndoStorageUnavailable,
    UndoNotFound,
    UndoScopeMismatch,
    UndoConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChangesError {
    pub code: AiChangesErrorCode,
    pub message: String,
}

impl AiChangesError {
    pub(crate) fn invalid_change_set() -> Self {
        Self::new(
            AiChangesErrorCode::InvalidChangeSet,
            "The AI change set is invalid or no longer applicable.",
        )
    }

    pub(crate) fn invalid_selection() -> Self {
        Self::new(
            AiChangesErrorCode::InvalidSelection,
            "The selected AI change operations are invalid.",
        )
    }

    pub(crate) fn unsupported_operation() -> Self {
        Self::new(
            AiChangesErrorCode::UnsupportedOperation,
            "The selected AI operation is advisory and cannot be applied.",
        )
    }

    pub(crate) fn database() -> Self {
        Self::new(
            AiChangesErrorCode::DatabaseUnavailable,
            "The library could not be updated.",
        )
    }

    pub(crate) fn storage() -> Self {
        Self::new(
            AiChangesErrorCode::UndoStorageUnavailable,
            "The AI undo record could not be stored or verified.",
        )
    }

    pub(crate) fn undo_not_found() -> Self {
        Self::new(
            AiChangesErrorCode::UndoNotFound,
            "The requested AI undo record was not found.",
        )
    }

    pub(crate) fn undo_scope_mismatch() -> Self {
        Self::new(
            AiChangesErrorCode::UndoScopeMismatch,
            "The AI undo record does not belong to this change set.",
        )
    }

    pub(crate) fn undo_conflict() -> Self {
        Self::new(
            AiChangesErrorCode::UndoConflict,
            "The affected library fields changed after the AI change was applied.",
        )
    }

    fn new(code: AiChangesErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for AiChangesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AiChangesError {}
