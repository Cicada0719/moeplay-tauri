mod ai {
    pub mod change_set {
        pub use moeplay_lib::ai::change_set::*;
    }
    pub mod contracts {
        pub use moeplay_lib::ai::contracts::*;
    }
}
mod db {
    pub use moeplay_lib::db::*;
}
mod models {
    pub use moeplay_lib::models::*;
}

#[allow(unused_imports)]
#[path = "../src/services/ai_changes/mod.rs"]
mod ai_changes;

use ai_changes::{
    AiChangeProvenance, AiChangesApplyStatus, AiChangesErrorCode, AiChangesService,
    AiChangesUndoStatus, ApplyAiChangesRequest, PreviewAiChangesRequest, UndoAiChangesRequest,
};
use moeplay_lib::ai::{
    build_library_change_set_preview, validate_library_cleanup, LibraryCleanupInput,
    LibraryGameContext,
};
use moeplay_lib::db::Database;
use moeplay_lib::models::Game;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("moeplay-ai-changes-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> (TestDirectory, Database, AiChangesService, String, String) {
    let directory = TestDirectory::new();
    let database = Database::open_at(directory.path().join("db")).unwrap();
    let mut first = Game::new("Original One".to_string(), "C:/games/one.exe".to_string());
    first.id = "game-one".to_string();
    first.description = Some("Before description".to_string());
    first.tags = vec!["existing".to_string()];
    first.metadata.developer = Some("Old Studio".to_string());

    let mut second = Game::new("Original Two".to_string(), "C:/games/two.exe".to_string());
    second.id = "game-two".to_string();
    database.sqlite().import_games(&[first, second]).unwrap();

    let service = AiChangesService::new(directory.path().join("undo"));
    (
        directory,
        database,
        service,
        "game-one".to_string(),
        "game-two".to_string(),
    )
}

fn valid_preview(game_one: &str, game_two: &str) -> moeplay_lib::ai::AiChangeSetPreview {
    let input = LibraryCleanupInput {
        games: vec![
            LibraryGameContext {
                id: game_one.to_string(),
                title: "Original One".to_string(),
                description: None,
                tags: vec![],
                metadata: BTreeMap::new(),
            },
            LibraryGameContext {
                id: game_two.to_string(),
                title: "Original Two".to_string(),
                description: None,
                tags: vec![],
                metadata: BTreeMap::new(),
            },
        ],
    };
    let validated = validate_library_cleanup(
        json!({
            "summary": "Safe cleanup",
            "confidence": 0.9,
            "operations": [
                {
                    "type": "set_field",
                    "gameId": game_one,
                    "field": "title",
                    "value": "Renamed One",
                    "reason": "Normalize title"
                },
                {
                    "type": "add_tag",
                    "gameId": game_one,
                    "tag": "cozy",
                    "reason": "Matches metadata"
                },
                {
                    "type": "set_field",
                    "gameId": game_two,
                    "field": "developer",
                    "value": "New Studio",
                    "reason": "Normalize developer"
                },
                {
                    "type": "needs_review",
                    "gameId": game_two,
                    "reason": "Review manually"
                }
            ]
        }),
        &input,
    )
    .unwrap();
    build_library_change_set_preview("change-set-1", "task-1", validated)
}

fn provenance() -> AiChangeProvenance {
    AiChangeProvenance {
        provider_id: "local-fixture".to_string(),
        model: "fixture-model".to_string(),
        prompt_id: "library-cleanup".to_string(),
        prompt_version: "1.0.0".to_string(),
    }
}

#[test]
fn preview_is_zero_write_and_reports_advisory_operations() {
    let (directory, database, service, game_one, game_two) = fixture();
    let before = database.sqlite().export_data().unwrap();
    let response = service
        .preview(
            &database,
            PreviewAiChangesRequest {
                change_set: valid_preview(&game_one, &game_two),
            },
        )
        .unwrap();
    let after = database.sqlite().export_data().unwrap();

    assert_eq!(response.write_count, 0);
    assert_eq!(response.operations.len(), 4);
    assert!(response.operations[0].applicable);
    assert!(!response.operations[3].applicable);
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(after).unwrap()
    );
    assert!(!directory.path().join("undo").exists());
}

#[test]
fn apply_mutates_only_explicit_selection_and_creates_versioned_undo_record() {
    let (directory, database, service, game_one, game_two) = fixture();
    let response = service
        .apply(
            &database,
            ApplyAiChangesRequest {
                change_set: valid_preview(&game_one, &game_two),
                selected_operation_indices: vec![1, 2],
                provenance: provenance(),
            },
        )
        .unwrap();

    assert_eq!(response.status, AiChangesApplyStatus::Applied);
    assert_eq!(response.selected_operation_count, 2);
    assert_eq!(response.changed_field_count, 2);
    let first = database.get_game(&game_one).unwrap();
    let second = database.get_game(&game_two).unwrap();
    assert_eq!(first.name, "Original One");
    assert!(first.tags.contains(&"cozy".to_string()));
    assert_eq!(second.metadata.developer.as_deref(), Some("New Studio"));

    let undo_id = response.undo_id.unwrap();
    let record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            directory
                .path()
                .join("undo")
                .join(format!("{undo_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(record["version"], 1);
    assert_eq!(record["provenance"]["model"], "fixture-model");
    assert_eq!(record["provenance"]["promptVersion"], "1.0.0");
    assert!(record["beforeHash"].as_str().unwrap().len() == 64);
    assert!(record["recordHash"].as_str().unwrap().len() == 64);
    let serialized = serde_json::to_string(&record).unwrap();
    assert!(!serialized.contains("one.exe"));
    assert!(!serialized.contains("two.exe"));
}

#[test]
fn apply_validation_failure_is_all_or_nothing_and_rejects_advisory_selection() {
    let (_directory, database, service, game_one, game_two) = fixture();
    let before = database.sqlite().export_data().unwrap();
    let error = service
        .apply(
            &database,
            ApplyAiChangesRequest {
                change_set: valid_preview(&game_one, &game_two),
                selected_operation_indices: vec![0, 3],
                provenance: provenance(),
            },
        )
        .unwrap_err();
    assert_eq!(error.code, AiChangesErrorCode::UnsupportedOperation);
    let after = database.sqlite().export_data().unwrap();
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(after).unwrap()
    );
}

#[test]
fn command_boundary_revalidates_ids_fields_tags_and_redacts_errors() {
    let (_directory, database, service, game_one, game_two) = fixture();
    let mut bad_id = valid_preview(&game_one, &game_two);
    if let moeplay_lib::ai::LibraryOperation::SetField { game_id, .. } =
        &mut bad_id.operations[0].operation
    {
        *game_id = "missing-game".to_string();
    }
    let id_error = service
        .preview(&database, PreviewAiChangesRequest { change_set: bad_id })
        .unwrap_err();
    assert_eq!(id_error.code, AiChangesErrorCode::InvalidChangeSet);

    let mut bad_field = valid_preview(&game_one, &game_two);
    if let moeplay_lib::ai::LibraryOperation::SetField { field, value, .. } =
        &mut bad_field.operations[0].operation
    {
        *field = "exePath".to_string();
        *value = json!("C:/malicious/replacement.exe");
    }
    let field_error = service
        .preview(
            &database,
            PreviewAiChangesRequest {
                change_set: bad_field,
            },
        )
        .unwrap_err();
    assert_eq!(field_error.code, AiChangesErrorCode::InvalidChangeSet);
    assert!(!field_error.message.contains("replacement.exe"));

    let mut bad_tag = valid_preview(&game_one, &game_two);
    if let moeplay_lib::ai::LibraryOperation::AddTag { tag, .. } =
        &mut bad_tag.operations[1].operation
    {
        *tag = "../moved".to_string();
    }
    let tag_error = service
        .preview(
            &database,
            PreviewAiChangesRequest {
                change_set: bad_tag,
            },
        )
        .unwrap_err();
    assert_eq!(tag_error.code, AiChangesErrorCode::InvalidChangeSet);
    assert_eq!(
        tag_error.message,
        "The AI change set is invalid or no longer applicable."
    );
}

#[test]
fn undo_is_scoped_idempotent_and_conflict_safe() {
    let (_directory, database, service, game_one, game_two) = fixture();
    let applied = service
        .apply(
            &database,
            ApplyAiChangesRequest {
                change_set: valid_preview(&game_one, &game_two),
                selected_operation_indices: vec![0, 1],
                provenance: provenance(),
            },
        )
        .unwrap();
    let undo_id = applied.undo_id.unwrap();

    let scope_error = service
        .undo(
            &database,
            UndoAiChangesRequest {
                undo_id: undo_id.clone(),
                change_set_id: "other-change-set".to_string(),
            },
        )
        .unwrap_err();
    assert_eq!(scope_error.code, AiChangesErrorCode::UndoScopeMismatch);
    assert_eq!(database.get_game(&game_one).unwrap().name, "Renamed One");

    let undone = service
        .undo(
            &database,
            UndoAiChangesRequest {
                undo_id: undo_id.clone(),
                change_set_id: "change-set-1".to_string(),
            },
        )
        .unwrap();
    assert_eq!(undone.status, AiChangesUndoStatus::Undone);
    let restored = database.get_game(&game_one).unwrap();
    assert_eq!(restored.name, "Original One");
    assert!(!restored.tags.contains(&"cozy".to_string()));

    let repeated = service
        .undo(
            &database,
            UndoAiChangesRequest {
                undo_id,
                change_set_id: "change-set-1".to_string(),
            },
        )
        .unwrap();
    assert_eq!(repeated.status, AiChangesUndoStatus::AlreadyUndone);
}

#[test]
fn undo_refuses_to_overwrite_a_later_user_edit() {
    let (_directory, database, service, game_one, game_two) = fixture();
    let applied = service
        .apply(
            &database,
            ApplyAiChangesRequest {
                change_set: valid_preview(&game_one, &game_two),
                selected_operation_indices: vec![0],
                provenance: provenance(),
            },
        )
        .unwrap();
    database
        .update_game_name(&game_one, "User Override".to_string())
        .unwrap();

    let error = service
        .undo(
            &database,
            UndoAiChangesRequest {
                undo_id: applied.undo_id.unwrap(),
                change_set_id: "change-set-1".to_string(),
            },
        )
        .unwrap_err();
    assert_eq!(error.code, AiChangesErrorCode::UndoConflict);
    assert_eq!(database.get_game(&game_one).unwrap().name, "User Override");
}
