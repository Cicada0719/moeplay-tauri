use super::error::{AiChangesError, AiChangesResult};
use super::store::{
    hash_json, AiUndoMutation, AiUndoRecordBody, UndoStore, AI_UNDO_RECORD_VERSION,
};
use super::validation::{
    validate_change_set, validate_provenance, validate_selection, validate_undo_id,
};
use super::{
    AiChangeOperationPreview, AiChangesApplyStatus, AiChangesUndoStatus, ApplyAiChangesRequest,
    ApplyAiChangesResponse, PreviewAiChangesRequest, PreviewAiChangesResponse,
    UndoAiChangesRequest, UndoAiChangesResponse,
};
use crate::ai::contracts::LibraryOperation;
use crate::db::Database;
use crate::models::{AppDatabase, Game};
use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

pub struct AiChangesService {
    store: UndoStore,
    mutation_lock: Mutex<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FieldTarget {
    game_id: String,
    field: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashedFieldValue<'a> {
    game_id: &'a str,
    field: &'a str,
    value: &'a Value,
}

impl AiChangesService {
    pub fn new(undo_directory: PathBuf) -> Self {
        Self {
            store: UndoStore::new(undo_directory),
            mutation_lock: Mutex::new(()),
        }
    }

    pub fn preview(
        &self,
        database: &Database,
        request: PreviewAiChangesRequest,
    ) -> AiChangesResult<PreviewAiChangesResponse> {
        let snapshot = export_snapshot(database)?;
        validate_change_set(&request.change_set, &snapshot)?;

        let operations = request
            .change_set
            .operations
            .iter()
            .enumerate()
            .map(|(operation_index, preview)| {
                preview_operation(operation_index, &preview.operation, &snapshot)
            })
            .collect::<AiChangesResult<Vec<_>>>()?;

        Ok(PreviewAiChangesResponse {
            change_set_id: request.change_set.id,
            task_id: request.change_set.task_id,
            operations,
            write_count: 0,
        })
    }

    pub fn apply(
        &self,
        database: &Database,
        request: ApplyAiChangesRequest,
    ) -> AiChangesResult<ApplyAiChangesResponse> {
        let _guard = self
            .mutation_lock
            .lock()
            .map_err(|_| AiChangesError::database())?;
        let mut snapshot = export_snapshot(database)?;
        validate_change_set(&request.change_set, &snapshot)?;
        validate_provenance(&request.provenance)?;
        let selected = validate_selection(
            &request.selected_operation_indices,
            request.change_set.operations.len(),
        )?;

        let mut before_values = BTreeMap::<FieldTarget, Value>::new();
        let mut set_field_targets = BTreeSet::<FieldTarget>::new();
        let mut selected_tags = BTreeSet::<(String, String)>::new();

        for operation_index in &selected {
            let operation = &request.change_set.operations[*operation_index].operation;
            match operation {
                LibraryOperation::SetField {
                    game_id,
                    field,
                    value,
                    ..
                } => {
                    let target = FieldTarget {
                        game_id: game_id.clone(),
                        field: field.clone(),
                    };
                    if !set_field_targets.insert(target.clone()) {
                        return Err(AiChangesError::invalid_selection());
                    }
                    capture_before(&snapshot, &target, &mut before_values)?;
                    let game = find_game_mut(&mut snapshot, game_id)?;
                    write_field(game, field, value.clone())?;
                }
                LibraryOperation::AddTag { game_id, tag, .. } => {
                    let normalized = tag.to_lowercase();
                    if !selected_tags.insert((game_id.clone(), normalized)) {
                        return Err(AiChangesError::invalid_selection());
                    }
                    let target = FieldTarget {
                        game_id: game_id.clone(),
                        field: "tags".to_string(),
                    };
                    capture_before(&snapshot, &target, &mut before_values)?;
                    let game = find_game_mut(&mut snapshot, game_id)?;
                    if !game
                        .tags
                        .iter()
                        .any(|existing| existing.eq_ignore_ascii_case(tag))
                    {
                        game.tags.push(tag.clone());
                    }
                }
                LibraryOperation::PossibleDuplicate { .. }
                | LibraryOperation::NeedsReview { .. } => {
                    return Err(AiChangesError::unsupported_operation());
                }
            }
        }

        let mut mutations = Vec::new();
        for (target, before) in before_values {
            let after = read_field(find_game(&snapshot, &target.game_id)?, &target.field)?;
            if before != after {
                mutations.push(AiUndoMutation {
                    game_id: target.game_id,
                    field: target.field,
                    before,
                    after,
                });
            }
        }

        let applied_at = Utc::now().to_rfc3339();
        if mutations.is_empty() {
            return Ok(ApplyAiChangesResponse {
                status: AiChangesApplyStatus::NoChanges,
                change_set_id: request.change_set.id,
                selected_operation_count: selected.len(),
                changed_field_count: 0,
                undo_id: None,
                applied_at,
            });
        }

        let changed_game_ids: BTreeSet<&str> =
            mutations.iter().map(|item| item.game_id.as_str()).collect();
        let updated_at = Utc::now().format("%Y-%m-%d %H:%M").to_string();
        for game in snapshot
            .games
            .iter_mut()
            .filter(|game| changed_game_ids.contains(game.id.as_str()))
        {
            game.updated_at.clone_from(&updated_at);
        }

        let undo_id = Uuid::new_v4().to_string();
        let body = AiUndoRecordBody {
            version: AI_UNDO_RECORD_VERSION,
            undo_id: undo_id.clone(),
            change_set_id: request.change_set.id.clone(),
            task_id: request.change_set.task_id.clone(),
            change_set_hash: hash_json(&request.change_set)?,
            before_hash: hash_mutation_side(&mutations, true)?,
            after_hash: hash_mutation_side(&mutations, false)?,
            selected_operation_indices: selected.clone(),
            mutations: mutations.clone(),
            provenance: request.provenance,
            applied_at: applied_at.clone(),
        };
        self.store.write_record(body)?;

        let changed_games: Vec<Game> = snapshot
            .games
            .into_iter()
            .filter(|game| changed_game_ids.contains(game.id.as_str()))
            .collect();
        if database.sqlite().import_games(&changed_games).is_err() {
            self.store.remove_record(&undo_id);
            return Err(AiChangesError::database());
        }

        Ok(ApplyAiChangesResponse {
            status: AiChangesApplyStatus::Applied,
            change_set_id: request.change_set.id,
            selected_operation_count: selected.len(),
            changed_field_count: mutations.len(),
            undo_id: Some(undo_id),
            applied_at,
        })
    }

    pub fn undo(
        &self,
        database: &Database,
        request: UndoAiChangesRequest,
    ) -> AiChangesResult<UndoAiChangesResponse> {
        let _guard = self
            .mutation_lock
            .lock()
            .map_err(|_| AiChangesError::database())?;
        validate_undo_id(&request.undo_id)?;
        let record = self.store.read_record(&request.undo_id)?;
        if record.body.change_set_id != request.change_set_id {
            return Err(AiChangesError::undo_scope_mismatch());
        }

        let undone_at = Utc::now().to_rfc3339();
        if self.store.marker_exists(&request.undo_id) {
            return Ok(UndoAiChangesResponse {
                status: AiChangesUndoStatus::AlreadyUndone,
                undo_id: request.undo_id,
                change_set_id: request.change_set_id,
                restored_field_count: record.body.mutations.len(),
                undone_at,
            });
        }

        if record.body.before_hash != hash_mutation_side(&record.body.mutations, true)?
            || record.body.after_hash != hash_mutation_side(&record.body.mutations, false)?
        {
            return Err(AiChangesError::storage());
        }

        let mut snapshot = export_snapshot(database)?;
        let current_values = record
            .body
            .mutations
            .iter()
            .map(|mutation| read_field(find_game(&snapshot, &mutation.game_id)?, &mutation.field))
            .collect::<AiChangesResult<Vec<_>>>()?;
        let all_after = current_values
            .iter()
            .zip(&record.body.mutations)
            .all(|(current, mutation)| current == &mutation.after);
        let all_before = current_values
            .iter()
            .zip(&record.body.mutations)
            .all(|(current, mutation)| current == &mutation.before);

        if all_before {
            self.store.write_marker(&record, undone_at.clone())?;
            return Ok(UndoAiChangesResponse {
                status: AiChangesUndoStatus::AlreadyUndone,
                undo_id: request.undo_id,
                change_set_id: request.change_set_id,
                restored_field_count: record.body.mutations.len(),
                undone_at,
            });
        }
        if !all_after {
            return Err(AiChangesError::undo_conflict());
        }

        let mut changed_game_ids = BTreeSet::new();
        for mutation in &record.body.mutations {
            let game = find_game_mut(&mut snapshot, &mutation.game_id)?;
            write_field(game, &mutation.field, mutation.before.clone())?;
            changed_game_ids.insert(mutation.game_id.as_str());
        }
        let updated_at = Utc::now().format("%Y-%m-%d %H:%M").to_string();
        for game in snapshot
            .games
            .iter_mut()
            .filter(|game| changed_game_ids.contains(game.id.as_str()))
        {
            game.updated_at.clone_from(&updated_at);
        }
        let changed_games: Vec<Game> = snapshot
            .games
            .into_iter()
            .filter(|game| changed_game_ids.contains(game.id.as_str()))
            .collect();
        database
            .sqlite()
            .import_games(&changed_games)
            .map_err(|_| AiChangesError::database())?;
        self.store.write_marker(&record, undone_at.clone())?;

        Ok(UndoAiChangesResponse {
            status: AiChangesUndoStatus::Undone,
            undo_id: request.undo_id,
            change_set_id: request.change_set_id,
            restored_field_count: record.body.mutations.len(),
            undone_at,
        })
    }
}

fn preview_operation(
    operation_index: usize,
    operation: &LibraryOperation,
    snapshot: &AppDatabase,
) -> AiChangesResult<AiChangeOperationPreview> {
    match operation {
        LibraryOperation::SetField {
            game_id,
            field,
            value,
            reason,
        } => Ok(AiChangeOperationPreview {
            operation_index,
            kind: "set_field".to_string(),
            game_ids: vec![game_id.clone()],
            field: Some(field.clone()),
            before: Some(read_field(find_game(snapshot, game_id)?, field)?),
            after: Some(value.clone()),
            reason: reason.clone(),
            applicable: true,
        }),
        LibraryOperation::AddTag {
            game_id,
            tag,
            reason,
        } => {
            let already_present = find_game(snapshot, game_id)?
                .tags
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(tag));
            Ok(AiChangeOperationPreview {
                operation_index,
                kind: "add_tag".to_string(),
                game_ids: vec![game_id.clone()],
                field: Some("tags".to_string()),
                before: Some(Value::Bool(already_present)),
                after: Some(Value::Bool(true)),
                reason: reason.clone(),
                applicable: true,
            })
        }
        LibraryOperation::PossibleDuplicate { game_ids, reason } => Ok(AiChangeOperationPreview {
            operation_index,
            kind: "possible_duplicate".to_string(),
            game_ids: game_ids.clone(),
            field: None,
            before: None,
            after: None,
            reason: reason.clone(),
            applicable: false,
        }),
        LibraryOperation::NeedsReview { game_id, reason } => Ok(AiChangeOperationPreview {
            operation_index,
            kind: "needs_review".to_string(),
            game_ids: vec![game_id.clone()],
            field: None,
            before: None,
            after: None,
            reason: reason.clone(),
            applicable: false,
        }),
    }
}

fn capture_before(
    snapshot: &AppDatabase,
    target: &FieldTarget,
    before_values: &mut BTreeMap<FieldTarget, Value>,
) -> AiChangesResult<()> {
    if !before_values.contains_key(target) {
        let before = read_field(find_game(snapshot, &target.game_id)?, &target.field)?;
        before_values.insert(target.clone(), before);
    }
    Ok(())
}

fn export_snapshot(database: &Database) -> AiChangesResult<AppDatabase> {
    database
        .sqlite()
        .export_data()
        .map_err(|_| AiChangesError::database())
}

fn find_game<'a>(snapshot: &'a AppDatabase, game_id: &str) -> AiChangesResult<&'a Game> {
    snapshot
        .games
        .iter()
        .find(|game| game.id == game_id)
        .ok_or_else(AiChangesError::invalid_change_set)
}

fn find_game_mut<'a>(
    snapshot: &'a mut AppDatabase,
    game_id: &str,
) -> AiChangesResult<&'a mut Game> {
    snapshot
        .games
        .iter_mut()
        .find(|game| game.id == game_id)
        .ok_or_else(AiChangesError::invalid_change_set)
}

fn read_field(game: &Game, field: &str) -> AiChangesResult<Value> {
    match field {
        "title" => Ok(Value::String(game.name.clone())),
        "description" => Ok(option_string_value(&game.description)),
        "developer" => Ok(option_string_value(&game.metadata.developer)),
        "publisher" => Ok(option_string_value(&game.metadata.publisher)),
        "contentRating" => Ok(option_string_value(&game.metadata.age_rating)),
        "estimatedHours" => Ok(game
            .metadata
            .estimated_hours
            .map_or(Value::Null, |hours| json!(hours))),
        "tags" => serde_json::to_value(&game.tags).map_err(|_| AiChangesError::database()),
        _ => Err(AiChangesError::invalid_change_set()),
    }
}

fn write_field(game: &mut Game, field: &str, value: Value) -> AiChangesResult<()> {
    match field {
        "title" => game.name = required_string(value)?,
        "description" => game.description = optional_string(value)?,
        "developer" => game.metadata.developer = optional_string(value)?,
        "publisher" => game.metadata.publisher = optional_string(value)?,
        "contentRating" => game.metadata.age_rating = optional_string(value)?,
        "estimatedHours" => {
            game.metadata.estimated_hours = if value.is_null() {
                None
            } else {
                Some(
                    value
                        .as_f64()
                        .ok_or_else(AiChangesError::invalid_change_set)?,
                )
            };
        }
        "tags" => {
            game.tags =
                serde_json::from_value(value).map_err(|_| AiChangesError::invalid_change_set())?;
        }
        _ => return Err(AiChangesError::invalid_change_set()),
    }
    Ok(())
}

fn required_string(value: Value) -> AiChangesResult<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(AiChangesError::invalid_change_set)
}

fn optional_string(value: Value) -> AiChangesResult<Option<String>> {
    if value.is_null() {
        Ok(None)
    } else {
        required_string(value).map(Some)
    }
}

fn option_string_value(value: &Option<String>) -> Value {
    value.clone().map_or(Value::Null, Value::String)
}

fn hash_mutation_side(mutations: &[AiUndoMutation], before: bool) -> AiChangesResult<String> {
    let values: Vec<HashedFieldValue<'_>> = mutations
        .iter()
        .map(|mutation| HashedFieldValue {
            game_id: &mutation.game_id,
            field: &mutation.field,
            value: if before {
                &mutation.before
            } else {
                &mutation.after
            },
        })
        .collect();
    hash_json(&values)
}
