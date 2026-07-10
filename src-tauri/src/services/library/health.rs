use super::contracts::GameIdentity;
use super::contracts::{LibraryHealthIssue, LibraryHealthSnapshot, LibraryHealthState};
use super::provenance::ProvenanceLedger;
use crate::models::Game;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn library_health(
    games: &[Game],
    ledger: &ProvenanceLedger,
    unresolved_import_conflicts: usize,
) -> LibraryHealthSnapshot {
    let mut issues = Vec::new();
    let mut missing = 0usize;
    for game in games {
        let has_uri = game
            .launch_uri
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let path_missing = game.exe_path.trim().is_empty() || !Path::new(&game.exe_path).exists();
        if !has_uri && path_missing {
            missing += 1;
            issues.push(LibraryHealthIssue {
                code: "missing_launch_target".to_string(),
                severity: "error".to_string(),
                message: format!("{} has no reachable launch target", game.name),
                game_ids: vec![game.id.clone()],
            });
        }
    }

    let duplicate_groups = strong_duplicate_groups(games);
    for ids in &duplicate_groups {
        issues.push(LibraryHealthIssue {
            code: "duplicate_strong_identity".to_string(),
            severity: "error".to_string(),
            message: "multiple games share a launch path or platform ID".to_string(),
            game_ids: ids.clone(),
        });
    }

    let title_groups = title_recall_groups(games);
    for ids in &title_groups {
        issues.push(LibraryHealthIssue {
            code: "title_recall_group".to_string(),
            severity: "info".to_string(),
            message: "same-title games are recall-only and remain separate".to_string(),
            game_ids: ids.clone(),
        });
    }

    if unresolved_import_conflicts > 0 {
        issues.push(LibraryHealthIssue {
            code: "unresolved_import_conflicts".to_string(),
            severity: "warning".to_string(),
            message: format!("{unresolved_import_conflicts} import candidates need a decision"),
            game_ids: Vec::new(),
        });
    }

    let games_with_provenance = ledger
        .keys()
        .map(|(game_id, _)| game_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let provenance_coverage = if games.is_empty() {
        1.0
    } else {
        games_with_provenance as f32 / games.len() as f32
    };
    let state = if !duplicate_groups.is_empty() || missing > 0 {
        LibraryHealthState::Degraded
    } else if unresolved_import_conflicts > 0 || !title_groups.is_empty() {
        LibraryHealthState::NeedsAttention
    } else {
        LibraryHealthState::Healthy
    };

    LibraryHealthSnapshot {
        state,
        total_games: games.len(),
        missing_launch_targets: missing,
        duplicate_identity_groups: duplicate_groups.len(),
        title_recall_groups: title_groups.len(),
        unresolved_import_conflicts,
        provenance_coverage,
        issues,
    }
}

fn strong_duplicate_groups(games: &[Game]) -> Vec<Vec<String>> {
    let mut identities = BTreeMap::<String, Vec<String>>::new();
    for game in games {
        let identity = GameIdentity::from_game(game);
        if let Some(platform) = identity.platform_id {
            identities
                .entry(format!("platform:{}:{}", platform.source, platform.id))
                .or_default()
                .push(game.id.clone());
        }
        if let Some(path) = identity.launch_path {
            identities
                .entry(format!("path:{path}"))
                .or_default()
                .push(game.id.clone());
        }
    }
    unique_groups(identities.into_values().filter(|ids| ids.len() > 1))
}

fn title_recall_groups(games: &[Game]) -> Vec<Vec<String>> {
    let mut identities = BTreeMap::<String, Vec<String>>::new();
    for game in games {
        let fingerprint = GameIdentity::from_game(game).title_fingerprint;
        if !fingerprint.is_empty() {
            identities
                .entry(fingerprint)
                .or_default()
                .push(game.id.clone());
        }
    }
    unique_groups(identities.into_values().filter(|ids| ids.len() > 1))
}

fn unique_groups(groups: impl Iterator<Item = Vec<String>>) -> Vec<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for mut group in groups {
        group.sort();
        let key = group.join("|");
        if seen.insert(key) {
            output.push(group);
        }
    }
    output
}
