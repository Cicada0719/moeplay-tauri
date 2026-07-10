use super::contracts::*;
use super::identity::{
    find_identity_matches, stable_candidate_id, stable_hash, strong_match_ids, title_recall_ids,
};
use super::provenance::{apply_field_diffs, plan_field_diffs, ProvenanceLedger};
use crate::models::Game;
use chrono::Utc;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportJobState {
    Running,
    Succeeded,
    Failed,
}

pub trait LibraryImportBackend {
    fn list_games(&self) -> Result<Vec<Game>, String>;
    fn add_game(&self, game: Game) -> Result<Game, String>;
    fn update_game(&self, game: Game) -> Result<Game, String>;
    fn load_provenance(&self) -> Result<ProvenanceLedger, String>;
    fn load_completed_apply(&self, job_id: &str) -> Result<Option<ApplyImportResponse>, String>;
    /// Atomically claims a batch idempotency key. A concurrently completed job may be
    /// returned for replay; an in-progress duplicate must return an error.
    fn claim_import_job(
        &self,
        job_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<ApplyImportResponse>, String>;
    fn record_import_job(
        &self,
        job_id: &str,
        idempotency_key: &str,
        state: ImportJobState,
        progress: f32,
        response: Option<&ApplyImportResponse>,
        error: Option<&str>,
    ) -> Result<(), String>;
}

pub fn preview_import(
    games: &[Game],
    ledger: &ProvenanceLedger,
    request: PreviewImportRequest,
) -> ImportPreview {
    let source = request.source.trim().to_string();
    let mut candidates = request
        .records
        .into_iter()
        .map(|record| build_candidate(&source, record, games, ledger))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    let joined_ids = candidates
        .iter()
        .map(|candidate| candidate.id.as_str())
        .collect::<Vec<_>>();
    let mut hash_parts = vec![source.as_str()];
    hash_parts.extend(joined_ids);
    ImportPreview {
        preview_id: format!("library-preview-{}", stable_hash(&hash_parts)),
        source,
        candidates,
        created_at: Utc::now().to_rfc3339(),
        write_count: 0,
    }
}

pub fn build_candidate(
    source: &str,
    record: ImportSourceRecord,
    games: &[Game],
    ledger: &ProvenanceLedger,
) -> ImportCandidate {
    let identity = GameIdentity::from_record(&record);
    let matches = find_identity_matches(&identity, games);
    let strong_ids = strong_match_ids(&matches);
    let recalls = title_recall_ids(&matches);
    let (action, reason, target_game_id) = if record.launch_path.is_none()
        && record.platform_id.is_none()
        && record.launch_uri.is_none()
    {
        (
            ImportAction::Ignore,
            ImportReason {
                code: ImportReasonCode::NoLaunchTarget,
                message: "candidate has no executable path, platform identity, or launch URI"
                    .into(),
                recalled_game_ids: recalls.clone(),
            },
            None,
        )
    } else if strong_ids.len() > 1 {
        (
            ImportAction::Conflict,
            ImportReason {
                code: ImportReasonCode::AmbiguousStrongIdentity,
                message: "multiple games share a strong launch identity".into(),
                recalled_game_ids: strong_ids.clone(),
            },
            None,
        )
    } else if let Some(game_id) = strong_ids.first() {
        let platform = matches
            .iter()
            .any(|item| item.game_id == *game_id && item.kind == IdentityMatchKind::PlatformId);
        (
            ImportAction::Update,
            ImportReason {
                code: if platform {
                    ImportReasonCode::PlatformIdMatch
                } else {
                    ImportReasonCode::LaunchPathMatch
                },
                message: if platform {
                    "platform identity matched an existing game".into()
                } else {
                    "normalized launch path matched an existing game".into()
                },
                recalled_game_ids: vec![game_id.clone()],
            },
            Some(game_id.clone()),
        )
    } else if !recalls.is_empty() {
        (
            ImportAction::Conflict,
            ImportReason {
                code: ImportReasonCode::TitleRecallOnly,
                message:
                    "title fingerprint is recall-only; choose create or an explicit merge target"
                        .into(),
                recalled_game_ids: recalls.clone(),
            },
            None,
        )
    } else {
        (
            ImportAction::Create,
            ImportReason {
                code: ImportReasonCode::NewIdentity,
                message: "no strong identity match was found".into(),
                recalled_game_ids: Vec::new(),
            },
            None,
        )
    };

    let diff_target = target_game_id
        .as_deref()
        .or_else(|| (recalls.len() == 1).then(|| recalls[0].as_str()));
    let game = diff_target.and_then(|id| games.iter().find(|game| game.id == id));
    let platform_match = target_game_id.as_ref().is_some_and(|id| {
        matches
            .iter()
            .any(|item| item.game_id == *id && item.kind == IdentityMatchKind::PlatformId)
    });
    let field_diff = plan_field_diffs(game, diff_target, source, &record, ledger, platform_match);

    ImportCandidate {
        id: stable_candidate_id(source, &record),
        source: source.to_string(),
        identity,
        action,
        reason,
        matches,
        target_game_id,
        field_diff,
        record,
    }
}

pub fn apply_import<B: LibraryImportBackend>(
    backend: &B,
    request: ApplyImportRequest,
) -> Result<ApplyImportResponse, String> {
    let key = request.idempotency_key.trim();
    if key.is_empty() {
        return Err("idempotencyKey is required".to_string());
    }
    let job_id = format!("library-import-{}", stable_hash(&[key]));
    if let Some(mut response) = backend.load_completed_apply(&job_id)? {
        response.replayed = true;
        for item in &mut response.results {
            if !matches!(
                item.status,
                ApplyItemStatus::Failed | ApplyItemStatus::Conflict
            ) {
                item.status = ApplyItemStatus::AlreadyApplied;
            }
        }
        return Ok(response);
    }

    if let Some(mut response) = backend.claim_import_job(&job_id, key)? {
        response.replayed = true;
        for item in &mut response.results {
            if !matches!(
                item.status,
                ApplyItemStatus::Failed | ApplyItemStatus::Conflict
            ) {
                item.status = ApplyItemStatus::AlreadyApplied;
            }
        }
        return Ok(response);
    }

    let total = request.preview.candidates.len().max(1);
    let decisions = request
        .decisions
        .into_iter()
        .map(|decision| (decision.candidate_id.clone(), decision))
        .collect::<BTreeMap<_, _>>();
    let mut games = backend.list_games()?;
    let mut ledger = backend.load_provenance()?;
    let mut results = Vec::new();
    let mut provenance_changes = Vec::new();

    for (index, preview_candidate) in request.preview.candidates.into_iter().enumerate() {
        let item_key = format!(
            "library-import-item-{}",
            stable_hash(&[key, &preview_candidate.id])
        );
        let expected_id = stable_candidate_id(&request.preview.source, &preview_candidate.record);
        if expected_id != preview_candidate.id {
            results.push(item_result(
                &preview_candidate,
                &item_key,
                preview_candidate.action,
                ApplyItemStatus::Conflict,
                None,
                "candidate stable ID does not match its source record",
                Vec::new(),
                Vec::new(),
            ));
            continue;
        }

        let current_candidate = build_candidate(
            &request.preview.source,
            preview_candidate.record.clone(),
            &games,
            &ledger,
        );
        let decision = decisions.get(&preview_candidate.id);
        let action = decision
            .map(|item| item.action)
            .unwrap_or(current_candidate.action);
        let target = decision
            .and_then(|item| item.target_game_id.clone())
            .or_else(|| current_candidate.target_game_id.clone());

        let outcome = apply_candidate(
            backend,
            &mut games,
            &mut ledger,
            &current_candidate,
            decision,
            action,
            target,
            &item_key,
        );
        match outcome {
            Ok((result, changes)) => {
                provenance_changes.extend(changes);
                results.push(result);
            }
            Err(error) => results.push(item_result(
                &current_candidate,
                &item_key,
                action,
                ApplyItemStatus::Failed,
                None,
                &error,
                Vec::new(),
                Vec::new(),
            )),
        }

        let progress = (index + 1) as f32 / total as f32;
        let partial = ApplyImportResponse {
            job_id: job_id.clone(),
            idempotency_key: key.to_string(),
            replayed: false,
            results: results.clone(),
            provenance_changes: provenance_changes.clone(),
        };
        backend.record_import_job(
            &job_id,
            key,
            ImportJobState::Running,
            progress,
            Some(&partial),
            None,
        )?;
    }

    let response = ApplyImportResponse {
        job_id: job_id.clone(),
        idempotency_key: key.to_string(),
        replayed: false,
        results,
        provenance_changes,
    };
    let failed = response
        .results
        .iter()
        .any(|item| item.status == ApplyItemStatus::Failed);
    backend.record_import_job(
        &job_id,
        key,
        if failed {
            ImportJobState::Failed
        } else {
            ImportJobState::Succeeded
        },
        1.0,
        Some(&response),
        failed.then_some("one or more import candidates failed"),
    )?;
    Ok(response)
}

#[allow(clippy::too_many_arguments)]
fn apply_candidate<B: LibraryImportBackend>(
    backend: &B,
    games: &mut Vec<Game>,
    ledger: &mut ProvenanceLedger,
    candidate: &ImportCandidate,
    decision: Option<&ImportDecision>,
    action: ImportAction,
    target_game_id: Option<String>,
    item_key: &str,
) -> Result<(ApplyItemResult, Vec<FieldProvenanceChange>), String> {
    match action {
        ImportAction::Ignore => Ok((
            item_result(
                candidate,
                item_key,
                action,
                ApplyItemStatus::Ignored,
                None,
                "candidate ignored",
                Vec::new(),
                Vec::new(),
            ),
            Vec::new(),
        )),
        ImportAction::Conflict => Ok((
            item_result(
                candidate,
                item_key,
                action,
                ApplyItemStatus::Conflict,
                None,
                &candidate.reason.message,
                Vec::new(),
                Vec::new(),
            ),
            Vec::new(),
        )),
        ImportAction::Create => {
            if !strong_match_ids(&candidate.matches).is_empty() {
                return Ok((
                    item_result(
                        candidate,
                        item_key,
                        action,
                        ApplyItemStatus::Conflict,
                        None,
                        "preview is stale: a strong identity already exists",
                        Vec::new(),
                        Vec::new(),
                    ),
                    Vec::new(),
                ));
            }
            let mut game = Game::new(
                candidate.record.title.trim().to_string(),
                candidate.record.launch_path.clone().unwrap_or_default(),
            );
            let diffs = plan_field_diffs(
                None,
                Some(&game.id),
                &candidate.source,
                &candidate.record,
                ledger,
                false,
            );
            let preserved = preserved_fields(&diffs);
            let changes = apply_field_diffs(
                &mut game,
                &candidate.source,
                &candidate.record.source_record_id,
                item_key,
                &diffs,
                ledger,
            )?;
            game.last_imported_at = Some(Utc::now().to_rfc3339());
            let saved = backend.add_game(game)?;
            games.push(saved.clone());
            Ok((
                item_result(
                    candidate,
                    item_key,
                    action,
                    ApplyItemStatus::Created,
                    Some(saved.id),
                    "game created",
                    changes.iter().map(|change| change.field.clone()).collect(),
                    preserved,
                ),
                changes,
            ))
        }
        ImportAction::Update | ImportAction::Merge => {
            if action == ImportAction::Merge
                && (!decision.is_some_and(|item| item.action == ImportAction::Merge)
                    || target_game_id.is_none())
            {
                return Ok((
                    item_result(
                        candidate,
                        item_key,
                        action,
                        ApplyItemStatus::Conflict,
                        None,
                        "same-title candidates require an explicit merge decision and target",
                        Vec::new(),
                        Vec::new(),
                    ),
                    Vec::new(),
                ));
            }
            let Some(target_id) = target_game_id else {
                return Ok((
                    item_result(
                        candidate,
                        item_key,
                        action,
                        ApplyItemStatus::Conflict,
                        None,
                        "update/merge target is missing",
                        Vec::new(),
                        Vec::new(),
                    ),
                    Vec::new(),
                ));
            };
            if action == ImportAction::Update
                && !strong_match_ids(&candidate.matches).contains(&target_id)
            {
                return Ok((
                    item_result(
                        candidate,
                        item_key,
                        action,
                        ApplyItemStatus::Conflict,
                        Some(target_id),
                        "update target no longer has a strong identity match",
                        Vec::new(),
                        Vec::new(),
                    ),
                    Vec::new(),
                ));
            }
            let position = games
                .iter()
                .position(|game| game.id == target_id)
                .ok_or_else(|| format!("target game not found: {target_id}"))?;
            let mut game = games[position].clone();
            let platform_match = candidate.matches.iter().any(|item| {
                item.game_id == target_id && item.kind == IdentityMatchKind::PlatformId
            });
            let diffs = plan_field_diffs(
                Some(&game),
                Some(&target_id),
                &candidate.source,
                &candidate.record,
                ledger,
                platform_match,
            );
            let preserved = preserved_fields(&diffs);
            let changes = apply_field_diffs(
                &mut game,
                &candidate.source,
                &candidate.record.source_record_id,
                item_key,
                &diffs,
                ledger,
            )?;
            let applied = changes
                .iter()
                .map(|change| change.field.clone())
                .collect::<Vec<_>>();
            let status = if changes.is_empty() {
                ApplyItemStatus::NoChanges
            } else if action == ImportAction::Merge {
                ApplyItemStatus::Merged
            } else {
                ApplyItemStatus::Updated
            };
            if !changes.is_empty() {
                game.last_imported_at = Some(Utc::now().to_rfc3339());
                let saved = backend.update_game(game)?;
                games[position] = saved;
            }
            Ok((
                item_result(
                    candidate,
                    item_key,
                    action,
                    status,
                    Some(target_id),
                    if changes.is_empty() {
                        "no import-owned fields changed"
                    } else if action == ImportAction::Merge {
                        "candidate explicitly merged without overwriting user-owned fields"
                    } else {
                        "import-owned fields updated"
                    },
                    applied,
                    preserved,
                ),
                changes,
            ))
        }
    }
}

fn preserved_fields(diffs: &[FieldDiff]) -> Vec<String> {
    diffs
        .iter()
        .filter(|diff| {
            matches!(
                diff.disposition,
                FieldDiffDisposition::PreserveUser | FieldDiffDisposition::Conflict
            )
        })
        .map(|diff| diff.field.clone())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn item_result(
    candidate: &ImportCandidate,
    item_key: &str,
    action: ImportAction,
    status: ApplyItemStatus,
    game_id: Option<String>,
    message: &str,
    applied_fields: Vec<String>,
    preserved_fields: Vec<String>,
) -> ApplyItemResult {
    ApplyItemResult {
        candidate_id: candidate.id.clone(),
        item_idempotency_key: item_key.to_string(),
        action,
        status,
        game_id,
        message: message.to_string(),
        applied_fields,
        preserved_fields,
    }
}
