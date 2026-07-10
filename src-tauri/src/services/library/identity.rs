use super::contracts::{
    GameIdentity, IdentityMatch, IdentityMatchKind, ImportSourceRecord, PlatformIdentity,
};
use crate::models::Game;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

impl GameIdentity {
    pub fn from_record(record: &ImportSourceRecord) -> Self {
        Self {
            launch_path: record.launch_path.as_deref().map(normalize_launch_path),
            platform_id: record.platform_id.as_ref().map(normalize_platform_identity),
            title_fingerprint: title_fingerprint(&record.title),
        }
    }

    pub fn from_game(game: &Game) -> Self {
        let platform_id = match (&game.library_source, &game.library_id) {
            (Some(source), Some(id)) if !source.trim().is_empty() && !id.trim().is_empty() => {
                Some(normalize_platform_identity(&PlatformIdentity {
                    source: source.clone(),
                    id: id.clone(),
                }))
            }
            _ => None,
        };
        Self {
            launch_path: (!game.exe_path.trim().is_empty())
                .then(|| normalize_launch_path(&game.exe_path)),
            platform_id,
            title_fingerprint: title_fingerprint(&game.name),
        }
    }
}

pub fn normalize_platform_identity(identity: &PlatformIdentity) -> PlatformIdentity {
    PlatformIdentity {
        source: identity.source.trim().to_lowercase(),
        id: identity.id.trim().to_lowercase(),
    }
}

/// Lexical path normalization only. It deliberately does not canonicalize on disk,
/// so previews remain zero-write and work for moved/offline libraries.
pub fn normalize_launch_path(path: &str) -> String {
    let raw = path.trim().replace('\\', "/");
    let mut prefix = String::new();
    let mut rest = raw.as_str();
    if raw.len() >= 2 && raw.as_bytes()[1] == b':' {
        prefix = raw[..2].to_ascii_lowercase();
        rest = &raw[2..];
    } else if raw.starts_with("//") {
        prefix = "//".to_string();
        rest = raw.trim_start_matches('/');
    } else if raw.starts_with('/') {
        prefix = "/".to_string();
        rest = raw.trim_start_matches('/');
    }

    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split('/') {
        match part {
            "" | "." => {}
            ".." if !parts.is_empty() && parts.last() != Some(&"..") => {
                parts.pop();
            }
            ".." if prefix.is_empty() => parts.push(part),
            ".." => {}
            _ => parts.push(part),
        }
    }
    let joined = parts.join("/").to_lowercase();
    match prefix.as_str() {
        "//" => format!("//{joined}"),
        "/" => format!("/{joined}"),
        "" => joined,
        drive => format!("{drive}/{joined}")
            .trim_end_matches('/')
            .to_string(),
    }
}

pub fn title_fingerprint(title: &str) -> String {
    let mut output = String::new();
    let mut pending_space = false;
    for ch in title.trim().to_lowercase().chars() {
        if ch.is_alphanumeric() {
            if pending_space && !output.is_empty() {
                output.push(' ');
            }
            output.push(ch);
            pending_space = false;
        } else {
            pending_space = true;
        }
    }
    output
}

pub fn find_identity_matches(identity: &GameIdentity, games: &[Game]) -> Vec<IdentityMatch> {
    let mut matches = Vec::new();
    for game in games {
        let existing = GameIdentity::from_game(game);
        if identity.platform_id.is_some() && identity.platform_id == existing.platform_id {
            matches.push(IdentityMatch {
                game_id: game.id.clone(),
                game_title: game.name.clone(),
                kind: IdentityMatchKind::PlatformId,
                confidence: 1.0,
            });
            continue;
        }
        if identity.launch_path.is_some() && identity.launch_path == existing.launch_path {
            matches.push(IdentityMatch {
                game_id: game.id.clone(),
                game_title: game.name.clone(),
                kind: IdentityMatchKind::LaunchPath,
                confidence: 1.0,
            });
            continue;
        }
        if !identity.title_fingerprint.is_empty()
            && identity.title_fingerprint == existing.title_fingerprint
        {
            matches.push(IdentityMatch {
                game_id: game.id.clone(),
                game_title: game.name.clone(),
                kind: IdentityMatchKind::TitleRecall,
                confidence: 0.5,
            });
        }
    }
    matches.sort_by(|left, right| {
        match_rank(left.kind)
            .cmp(&match_rank(right.kind))
            .then_with(|| left.game_id.cmp(&right.game_id))
    });
    matches
}

pub fn strong_match_ids(matches: &[IdentityMatch]) -> Vec<String> {
    matches
        .iter()
        .filter(|item| item.kind != IdentityMatchKind::TitleRecall)
        .map(|item| item.game_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn title_recall_ids(matches: &[IdentityMatch]) -> Vec<String> {
    matches
        .iter()
        .filter(|item| item.kind == IdentityMatchKind::TitleRecall)
        .map(|item| item.game_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn stable_hash(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    hex::encode(digest.finalize())
}

pub fn stable_candidate_id(source: &str, record: &ImportSourceRecord) -> String {
    let identity = GameIdentity::from_record(record);
    let record_key = if record.source_record_id.trim().is_empty() {
        format!(
            "{}|{}|{}",
            identity
                .platform_id
                .as_ref()
                .map(|item| format!("{}:{}", item.source, item.id))
                .unwrap_or_default(),
            identity.launch_path.as_deref().unwrap_or_default(),
            identity.title_fingerprint
        )
    } else {
        record.source_record_id.trim().to_lowercase()
    };
    format!(
        "import-candidate-{}",
        stable_hash(&[source.trim(), &record_key])
    )
}

fn match_rank(kind: IdentityMatchKind) -> u8 {
    match kind {
        IdentityMatchKind::PlatformId => 0,
        IdentityMatchKind::LaunchPath => 1,
        IdentityMatchKind::TitleRecall => 2,
    }
}
