use super::error::{AiChangesError, AiChangesResult};
use super::AiChangeProvenance;
use crate::ai::change_set::{AiChangeSetPreview, ChangeSetState};
use crate::ai::contracts::LibraryOperation;
use crate::models::AppDatabase;
use serde_json::Value;
use std::collections::BTreeSet;

const MAX_OPERATIONS: usize = 100;
const MAX_SELECTED_OPERATIONS: usize = 100;

pub(crate) fn validate_change_set(
    change_set: &AiChangeSetPreview,
    snapshot: &AppDatabase,
) -> AiChangesResult<()> {
    validate_text(&change_set.id, 128, false)?;
    validate_text(&change_set.task_id, 128, false)?;
    validate_text(&change_set.summary, 1000, true)?;
    if change_set.state != ChangeSetState::AwaitingConfirmation
        || !change_set.confidence.is_finite()
        || !(0.0..=1.0).contains(&change_set.confidence)
        || change_set.operations.len() > MAX_OPERATIONS
    {
        return Err(AiChangesError::invalid_change_set());
    }

    let game_ids: BTreeSet<&str> = snapshot.games.iter().map(|game| game.id.as_str()).collect();
    for preview_operation in &change_set.operations {
        validate_operation(&preview_operation.operation, &game_ids)?;
    }
    Ok(())
}

pub(crate) fn validate_selection(
    selected: &[usize],
    operation_count: usize,
) -> AiChangesResult<Vec<usize>> {
    if selected.len() > MAX_SELECTED_OPERATIONS {
        return Err(AiChangesError::invalid_selection());
    }
    let mut seen = BTreeSet::new();
    for index in selected {
        if *index >= operation_count || !seen.insert(*index) {
            return Err(AiChangesError::invalid_selection());
        }
    }
    let mut ordered = selected.to_vec();
    ordered.sort_unstable();
    Ok(ordered)
}

pub(crate) fn validate_provenance(provenance: &AiChangeProvenance) -> AiChangesResult<()> {
    validate_text(&provenance.provider_id, 128, true)?;
    validate_text(&provenance.model, 200, true)?;
    validate_text(&provenance.prompt_id, 128, false)?;
    validate_text(&provenance.prompt_version, 64, false)?;
    for value in [
        &provenance.provider_id,
        &provenance.model,
        &provenance.prompt_id,
        &provenance.prompt_version,
    ] {
        if looks_secret_like(value) {
            return Err(AiChangesError::invalid_change_set());
        }
    }
    Ok(())
}

pub(crate) fn validate_undo_id(value: &str) -> AiChangesResult<()> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(AiChangesError::undo_not_found());
    }
    Ok(())
}

fn validate_operation(
    operation: &LibraryOperation,
    game_ids: &BTreeSet<&str>,
) -> AiChangesResult<()> {
    match operation {
        LibraryOperation::SetField {
            game_id,
            field,
            value,
            reason,
        } => {
            require_game(game_id, game_ids)?;
            validate_text(reason, 1000, true)?;
            validate_field_value(field, value)
        }
        LibraryOperation::AddTag {
            game_id,
            tag,
            reason,
        } => {
            require_game(game_id, game_ids)?;
            validate_tag(tag)?;
            validate_text(reason, 1000, true)
        }
        LibraryOperation::PossibleDuplicate {
            game_ids: ids,
            reason,
        } => {
            validate_text(reason, 1000, true)?;
            if ids.len() < 2 || ids.len() > 10 {
                return Err(AiChangesError::invalid_change_set());
            }
            let mut unique = BTreeSet::new();
            for game_id in ids {
                require_game(game_id, game_ids)?;
                if !unique.insert(game_id) {
                    return Err(AiChangesError::invalid_change_set());
                }
            }
            Ok(())
        }
        LibraryOperation::NeedsReview { game_id, reason } => {
            require_game(game_id, game_ids)?;
            validate_text(reason, 1000, true)
        }
    }
}

fn validate_field_value(field: &str, value: &Value) -> AiChangesResult<()> {
    match field {
        "title" | "developer" | "publisher" => value
            .as_str()
            .ok_or_else(AiChangesError::invalid_change_set)
            .and_then(|text| validate_text(text, 200, true)),
        "description" => value
            .as_str()
            .ok_or_else(AiChangesError::invalid_change_set)
            .and_then(|text| validate_text(text, 5000, true)),
        "contentRating" => match value.as_str() {
            Some("all_ages" | "teen" | "mature" | "adult" | "unknown") => Ok(()),
            _ => Err(AiChangesError::invalid_change_set()),
        },
        "estimatedHours" => match value.as_f64() {
            Some(hours) if hours.is_finite() && (0.0..=10_000.0).contains(&hours) => Ok(()),
            _ => Err(AiChangesError::invalid_change_set()),
        },
        _ => Err(AiChangesError::invalid_change_set()),
    }
}

fn validate_tag(tag: &str) -> AiChangesResult<()> {
    if tag != tag.trim()
        || tag.is_empty()
        || tag.chars().count() > 80
        || tag.chars().any(char::is_control)
        || tag.contains(['/', '\\'])
    {
        return Err(AiChangesError::invalid_change_set());
    }
    Ok(())
}

fn require_game(game_id: &str, game_ids: &BTreeSet<&str>) -> AiChangesResult<()> {
    if game_id.is_empty() || !game_ids.contains(game_id) {
        return Err(AiChangesError::invalid_change_set());
    }
    Ok(())
}

fn validate_text(value: &str, max_chars: usize, allow_spaces: bool) -> AiChangesResult<()> {
    if value.is_empty()
        || value != value.trim()
        || value.chars().count() > max_chars
        || value.chars().any(char::is_control)
        || (!allow_spaces && value.chars().any(char::is_whitespace))
    {
        return Err(AiChangesError::invalid_change_set());
    }
    Ok(())
}

fn looks_secret_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("key-")
        || lower.contains("bearer ")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("access_token")
}
