use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const SYNC_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncEnvelopeV1 {
    pub schema_version: u32,
    pub device_id: String,
    pub generated_at: String,
    pub entities: SyncEntitiesV1,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncEntitiesV1 {
    #[serde(default)]
    pub games: Vec<SyncEntity>,
    #[serde(default)]
    pub activity_events: Vec<SyncEntity>,
    #[serde(default)]
    pub progress: Vec<SyncEntity>,
    #[serde(default)]
    pub anime_collections: Vec<SyncEntity>,
    #[serde(default)]
    pub comic_collections: Vec<SyncEntity>,
    #[serde(default)]
    pub portable_settings: Vec<SyncEntity>,
    #[serde(default)]
    pub tombstones: Vec<SyncTombstone>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncEntity {
    pub id: String,
    pub updated_at: String,
    pub device_id: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncTombstone {
    pub entity_type: String,
    pub id: String,
    pub deleted_at: String,
    pub device_id: String,
}

fn revision_key<'a>(updated_at: &'a str, device_id: &'a str) -> (&'a str, &'a str) {
    (updated_at, device_id)
}

fn merge_entities(groups: impl IntoIterator<Item = Vec<SyncEntity>>) -> Vec<SyncEntity> {
    let mut merged = BTreeMap::<String, SyncEntity>::new();
    for group in groups {
        for entity in group {
            let replace = merged
                .get(&entity.id)
                .map(|current| {
                    revision_key(&entity.updated_at, &entity.device_id)
                        > revision_key(&current.updated_at, &current.device_id)
                })
                .unwrap_or(true);
            if replace {
                merged.insert(entity.id.clone(), entity);
            }
        }
    }
    merged.into_values().collect()
}

fn merge_tombstones(groups: impl IntoIterator<Item = Vec<SyncTombstone>>) -> Vec<SyncTombstone> {
    let mut merged = BTreeMap::<(String, String), SyncTombstone>::new();
    for group in groups {
        for item in group {
            let key = (item.entity_type.clone(), item.id.clone());
            let replace = merged
                .get(&key)
                .map(|current| {
                    revision_key(&item.deleted_at, &item.device_id)
                        > revision_key(&current.deleted_at, &current.device_id)
                })
                .unwrap_or(true);
            if replace {
                merged.insert(key, item);
            }
        }
    }
    merged.into_values().collect()
}

fn apply_tombstones(
    entity_type: &str,
    entities: &mut Vec<SyncEntity>,
    tombstones: &[SyncTombstone],
) {
    let deleted = tombstones
        .iter()
        .filter(|item| item.entity_type == entity_type)
        .map(|item| {
            (
                item.id.as_str(),
                revision_key(&item.deleted_at, &item.device_id),
            )
        })
        .collect::<BTreeMap<_, _>>();
    entities.retain(|entity| {
        deleted
            .get(entity.id.as_str())
            .map(|deleted_at| revision_key(&entity.updated_at, &entity.device_id) > *deleted_at)
            .unwrap_or(true)
    });
}

pub fn merge_envelopes(
    envelopes: &[SyncEnvelopeV1],
    local_device_id: &str,
) -> Result<SyncEnvelopeV1, String> {
    if envelopes
        .iter()
        .any(|item| item.schema_version != SYNC_SCHEMA_VERSION)
    {
        return Err("unsupported sync schema version".to_string());
    }
    let generated_at = envelopes
        .iter()
        .map(|item| item.generated_at.as_str())
        .max()
        .unwrap_or_default()
        .to_string();
    let tombstones = merge_tombstones(
        envelopes
            .iter()
            .map(|item| item.entities.tombstones.clone()),
    );
    let mut entities = SyncEntitiesV1 {
        games: merge_entities(envelopes.iter().map(|item| item.entities.games.clone())),
        activity_events: merge_entities(
            envelopes
                .iter()
                .map(|item| item.entities.activity_events.clone()),
        ),
        progress: merge_entities(envelopes.iter().map(|item| item.entities.progress.clone())),
        anime_collections: merge_entities(
            envelopes
                .iter()
                .map(|item| item.entities.anime_collections.clone()),
        ),
        comic_collections: merge_entities(
            envelopes
                .iter()
                .map(|item| item.entities.comic_collections.clone()),
        ),
        portable_settings: merge_entities(
            envelopes
                .iter()
                .map(|item| item.entities.portable_settings.clone()),
        ),
        tombstones,
    };
    apply_tombstones("game", &mut entities.games, &entities.tombstones);
    apply_tombstones(
        "activity_event",
        &mut entities.activity_events,
        &entities.tombstones,
    );
    apply_tombstones("progress", &mut entities.progress, &entities.tombstones);
    apply_tombstones(
        "anime_collection",
        &mut entities.anime_collections,
        &entities.tombstones,
    );
    apply_tombstones(
        "comic_collection",
        &mut entities.comic_collections,
        &entities.tombstones,
    );
    apply_tombstones(
        "portable_setting",
        &mut entities.portable_settings,
        &entities.tombstones,
    );
    Ok(SyncEnvelopeV1 {
        schema_version: SYNC_SCHEMA_VERSION,
        device_id: local_device_id.to_string(),
        generated_at,
        entities,
    })
}

pub fn webdav_snapshot_path(device_id: &str) -> Result<String, String> {
    let valid = !device_id.is_empty()
        && device_id.len() <= 128
        && device_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'));
    if !valid {
        return Err("invalid sync device id".to_string());
    }
    Ok(format!("MoePlaySync/v1/devices/{device_id}/snapshot.json"))
}

pub fn changed_entity_count(before: &SyncEnvelopeV1, after: &SyncEnvelopeV1) -> usize {
    fn keys(items: &[SyncEntity]) -> BTreeSet<(&str, &str, &str)> {
        items
            .iter()
            .map(|item| {
                (
                    item.id.as_str(),
                    item.updated_at.as_str(),
                    item.device_id.as_str(),
                )
            })
            .collect()
    }
    [
        (&before.entities.games, &after.entities.games),
        (
            &before.entities.activity_events,
            &after.entities.activity_events,
        ),
        (&before.entities.progress, &after.entities.progress),
        (
            &before.entities.anime_collections,
            &after.entities.anime_collections,
        ),
        (
            &before.entities.comic_collections,
            &after.entities.comic_collections,
        ),
        (
            &before.entities.portable_settings,
            &after.entities.portable_settings,
        ),
    ]
    .into_iter()
    .map(|(left, right)| keys(left).symmetric_difference(&keys(right)).count())
    .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entity(id: &str, updated_at: &str, device_id: &str, value: i32) -> SyncEntity {
        SyncEntity {
            id: id.into(),
            updated_at: updated_at.into(),
            device_id: device_id.into(),
            value: json!(value),
        }
    }

    fn envelope(
        device_id: &str,
        games: Vec<SyncEntity>,
        tombstones: Vec<SyncTombstone>,
    ) -> SyncEnvelopeV1 {
        SyncEnvelopeV1 {
            schema_version: 1,
            device_id: device_id.into(),
            generated_at: "2026-07-16T00:00:00Z".into(),
            entities: SyncEntitiesV1 {
                games,
                tombstones,
                ..Default::default()
            },
        }
    }

    #[test]
    fn latest_revision_wins_with_device_id_tie_breaker() {
        let merged = merge_envelopes(
            &[
                envelope(
                    "a",
                    vec![entity("g", "2026-07-16T00:00:00Z", "a", 1)],
                    vec![],
                ),
                envelope(
                    "b",
                    vec![entity("g", "2026-07-16T00:00:00Z", "b", 2)],
                    vec![],
                ),
            ],
            "local",
        )
        .unwrap();
        assert_eq!(merged.entities.games[0].value, json!(2));
    }

    #[test]
    fn newer_tombstone_removes_entity() {
        let tombstone = SyncTombstone {
            entity_type: "game".into(),
            id: "g".into(),
            deleted_at: "2026-07-17T00:00:00Z".into(),
            device_id: "b".into(),
        };
        let merged = merge_envelopes(
            &[envelope(
                "a",
                vec![entity("g", "2026-07-16T00:00:00Z", "a", 1)],
                vec![tombstone],
            )],
            "local",
        )
        .unwrap();
        assert!(merged.entities.games.is_empty());
    }

    #[test]
    fn snapshot_path_rejects_path_traversal() {
        assert!(webdav_snapshot_path("../../bad").is_err());
        assert_eq!(
            webdav_snapshot_path("phone-01").unwrap(),
            "MoePlaySync/v1/devices/phone-01/snapshot.json"
        );
    }
    #[test]
    fn newer_entity_survives_an_older_tombstone() {
        let tombstone = SyncTombstone {
            entity_type: "game".into(),
            id: "g".into(),
            deleted_at: "2026-07-16T00:00:00Z".into(),
            device_id: "desktop".into(),
        };
        let merged = merge_envelopes(
            &[envelope(
                "phone",
                vec![entity("g", "2026-07-17T00:00:00Z", "phone", 2)],
                vec![tombstone],
            )],
            "local",
        )
        .unwrap();
        assert_eq!(merged.entities.games.len(), 1);
    }

    #[test]
    fn changed_count_reports_cross_device_revisions() {
        let before = envelope(
            "desktop",
            vec![entity("g", "2026-07-16T00:00:00Z", "desktop", 1)],
            vec![],
        );
        let after = envelope(
            "phone",
            vec![entity("g", "2026-07-17T00:00:00Z", "phone", 2)],
            vec![],
        );
        assert_eq!(changed_entity_count(&before, &after), 2);
    }
}
