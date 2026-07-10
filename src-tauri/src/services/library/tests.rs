use super::*;
use crate::models::Game;
use serde::Deserialize;
use serde_json::json;
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCase {
    source: String,
    source_record_id: String,
    title: String,
    old_path: Option<String>,
    new_path: String,
    platform_source: Option<String>,
    platform_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureSet {
    repeated_import: FixtureCase,
    same_title_different_works: FixtureCase,
    path_move: FixtureCase,
}

fn fixtures() -> FixtureSet {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/library/import-cases.json"
    ))
    .unwrap()
}

fn game(id: &str, name: &str, path: &str) -> Game {
    let mut game = Game::new(name.to_string(), path.to_string());
    game.id = id.to_string();
    game
}

fn record(case: &FixtureCase) -> ImportSourceRecord {
    ImportSourceRecord {
        source_record_id: case.source_record_id.clone(),
        title: case.title.clone(),
        launch_path: Some(case.new_path.clone()),
        install_dir: None,
        platform_id: case
            .platform_source
            .as_ref()
            .zip(case.platform_id.as_ref())
            .map(|(source, id)| PlatformIdentity {
                source: source.clone(),
                id: id.clone(),
            }),
        launch_uri: None,
        fields: BTreeMap::new(),
    }
}

#[derive(Default)]
struct MemoryBackend {
    games: RefCell<Vec<Game>>,
    ledger: RefCell<ProvenanceLedger>,
    completed: RefCell<BTreeMap<String, ApplyImportResponse>>,
}

impl LibraryImportBackend for MemoryBackend {
    fn list_games(&self) -> Result<Vec<Game>, String> {
        Ok(self.games.borrow().clone())
    }

    fn add_game(&self, game: Game) -> Result<Game, String> {
        self.games.borrow_mut().push(game.clone());
        Ok(game)
    }

    fn update_game(&self, game: Game) -> Result<Game, String> {
        let mut games = self.games.borrow_mut();
        let position = games
            .iter()
            .position(|item| item.id == game.id)
            .ok_or_else(|| "not found".to_string())?;
        games[position] = game.clone();
        Ok(game)
    }

    fn load_provenance(&self) -> Result<ProvenanceLedger, String> {
        Ok(self.ledger.borrow().clone())
    }

    fn load_completed_apply(&self, job_id: &str) -> Result<Option<ApplyImportResponse>, String> {
        Ok(self.completed.borrow().get(job_id).cloned())
    }

    fn claim_import_job(
        &self,
        job_id: &str,
        _idempotency_key: &str,
    ) -> Result<Option<ApplyImportResponse>, String> {
        Ok(self.completed.borrow().get(job_id).cloned())
    }

    fn record_import_job(
        &self,
        job_id: &str,
        _idempotency_key: &str,
        state: super::importer::ImportJobState,
        _progress: f32,
        response: Option<&ApplyImportResponse>,
        _error: Option<&str>,
    ) -> Result<(), String> {
        if state == super::importer::ImportJobState::Succeeded {
            if let Some(response) = response {
                self.completed
                    .borrow_mut()
                    .insert(job_id.to_string(), response.clone());
                for change in &response.provenance_changes {
                    self.ledger.borrow_mut().insert(
                        (change.current.game_id.clone(), change.current.field.clone()),
                        change.current.clone(),
                    );
                }
            }
        }
        Ok(())
    }
}

#[test]
fn duplicate_import_uses_strong_path_identity_and_stable_candidate_id() {
    let fixture = fixtures().repeated_import;
    let existing = game(
        "game-1",
        &fixture.title,
        fixture.old_path.as_deref().unwrap_or(&fixture.new_path),
    );
    let request = PreviewImportRequest {
        source: fixture.source.clone(),
        records: vec![record(&fixture)],
    };
    let first = preview_import(
        std::slice::from_ref(&existing),
        &ProvenanceLedger::new(),
        request.clone(),
    );
    let second = preview_import(&[existing], &ProvenanceLedger::new(), request);
    assert_eq!(first.write_count, 0);
    assert_eq!(first.candidates[0].action, ImportAction::Update);
    assert_eq!(first.candidates[0].id, second.candidates[0].id);
}

#[test]
fn same_title_is_recalled_but_never_auto_merged() {
    let fixture = fixtures().same_title_different_works;
    let existing = game(
        "work-a",
        &fixture.title,
        fixture.old_path.as_deref().unwrap(),
    );
    let preview = preview_import(
        &[existing],
        &ProvenanceLedger::new(),
        PreviewImportRequest {
            source: fixture.source.clone(),
            records: vec![record(&fixture)],
        },
    );
    let candidate = &preview.candidates[0];
    assert_eq!(candidate.action, ImportAction::Conflict);
    assert_eq!(candidate.reason.code, ImportReasonCode::TitleRecallOnly);
    assert_eq!(candidate.reason.recalled_game_ids, vec!["work-a"]);
    assert!(candidate.target_game_id.is_none());
}

#[test]
fn platform_identity_recognizes_path_move_and_plans_path_update() {
    let fixture = fixtures().path_move;
    let mut existing = game(
        "game-moved",
        &fixture.title,
        fixture.old_path.as_deref().unwrap(),
    );
    existing.library_source = fixture.platform_source.clone();
    existing.library_id = fixture.platform_id.clone();
    let preview = preview_import(
        &[existing],
        &ProvenanceLedger::new(),
        PreviewImportRequest {
            source: fixture.source.clone(),
            records: vec![record(&fixture)],
        },
    );
    let candidate = &preview.candidates[0];
    assert_eq!(candidate.action, ImportAction::Update);
    assert_eq!(candidate.reason.code, ImportReasonCode::PlatformIdMatch);
    let path = candidate
        .field_diff
        .iter()
        .find(|diff| diff.field == "exe_path")
        .unwrap();
    assert_eq!(path.disposition, FieldDiffDisposition::ReplaceImported);
    assert!(path.will_apply);
}

#[test]
fn field_provenance_updates_import_owned_values_and_preserves_user_edits() {
    let mut existing = game("game-1", "Example", "c:/games/example.exe");
    existing.description = Some("old imported".to_string());
    let previous = FieldProvenance {
        game_id: existing.id.clone(),
        field: "description".to_string(),
        source: "steam".to_string(),
        source_record_id: "10".to_string(),
        imported_at: "2026-01-01T00:00:00Z".to_string(),
        applied_value: json!("old imported"),
        value_hash: "old".to_string(),
    };
    let mut ledger = ProvenanceLedger::new();
    ledger.insert((existing.id.clone(), "description".to_string()), previous);
    let mut incoming = ImportSourceRecord {
        source_record_id: "10".to_string(),
        title: existing.name.clone(),
        launch_path: Some(existing.exe_path.clone()),
        install_dir: None,
        platform_id: None,
        launch_uri: None,
        fields: BTreeMap::from([("description".to_string(), json!("new imported"))]),
    };
    let owned = plan_field_diffs(
        Some(&existing),
        Some(&existing.id),
        "steam",
        &incoming,
        &ledger,
        false,
    );
    assert_eq!(
        owned
            .iter()
            .find(|item| item.field == "description")
            .unwrap()
            .disposition,
        FieldDiffDisposition::ReplaceImported
    );

    existing.description = Some("my personal note".to_string());
    incoming
        .fields
        .insert("description".to_string(), json!("provider refresh"));
    let user_edit = plan_field_diffs(
        Some(&existing),
        Some(&existing.id),
        "steam",
        &incoming,
        &ledger,
        false,
    );
    let description = user_edit
        .iter()
        .find(|item| item.field == "description")
        .unwrap();
    assert_eq!(description.disposition, FieldDiffDisposition::PreserveUser);
    assert!(!description.will_apply);
}

#[test]
fn apply_replays_completed_batch_by_idempotency_key() {
    let fixture = fixtures().repeated_import;
    let backend = MemoryBackend::default();
    let preview = preview_import(
        &[],
        &ProvenanceLedger::new(),
        PreviewImportRequest {
            source: fixture.source.clone(),
            records: vec![record(&fixture)],
        },
    );
    let request = ApplyImportRequest {
        preview,
        decisions: Vec::new(),
        idempotency_key: "batch-42".to_string(),
    };
    let first = apply_import(&backend, request.clone()).unwrap();
    let second = apply_import(&backend, request).unwrap();
    assert!(!first.replayed);
    assert_eq!(first.results[0].status, ApplyItemStatus::Created);
    assert!(second.replayed);
    assert_eq!(second.results[0].status, ApplyItemStatus::AlreadyApplied);
    assert_eq!(backend.games.borrow().len(), 1);
}

#[test]
fn missing_launch_is_classified_without_spawning() {
    let outcome = launch(LaunchDescriptor::Executable {
        path: "z:/definitely-missing/moeplay.exe".to_string(),
        args: Vec::new(),
        working_dir: None,
    });
    assert!(matches!(
        outcome,
        LaunchOutcome::Failed {
            error_kind: LaunchErrorKind::NotFound,
            ..
        }
    ));
}
