use crate::sync_envelope::{
    changed_entity_count, merge_envelopes, webdav_snapshot_path, SyncEnvelopeV1,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMergePreview {
    pub merged: SyncEnvelopeV1,
    pub changed_entities: usize,
    pub destructive_change: bool,
}

#[tauri::command]
pub fn merge_sync_envelopes(
    envelopes: Vec<SyncEnvelopeV1>,
    local_device_id: String,
    current: Option<SyncEnvelopeV1>,
) -> Result<SyncMergePreview, String> {
    let merged = merge_envelopes(&envelopes, &local_device_id)?;
    let changed_entities = current
        .as_ref()
        .map(|before| changed_entity_count(before, &merged))
        .unwrap_or_else(|| {
            merged.entities.games.len()
                + merged.entities.activity_events.len()
                + merged.entities.progress.len()
                + merged.entities.anime_collections.len()
                + merged.entities.comic_collections.len()
                + merged.entities.portable_settings.len()
                + merged.entities.tombstones.len()
        });
    let current_count = current
        .as_ref()
        .map(|item| {
            item.entities.games.len()
                + item.entities.activity_events.len()
                + item.entities.progress.len()
                + item.entities.anime_collections.len()
                + item.entities.comic_collections.len()
                + item.entities.portable_settings.len()
        })
        .unwrap_or(0);
    let tombstone_count = merged.entities.tombstones.len();
    let destructive_change = current_count >= 20 && tombstone_count * 4 >= current_count;
    Ok(SyncMergePreview {
        merged,
        changed_entities,
        destructive_change,
    })
}

#[tauri::command]
pub fn get_sync_snapshot_path(device_id: String) -> Result<String, String> {
    webdav_snapshot_path(&device_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync_envelope::{SyncEntitiesV1, SYNC_SCHEMA_VERSION};

    #[test]
    fn merge_preview_rejects_unknown_schema() {
        let envelope = SyncEnvelopeV1 {
            schema_version: SYNC_SCHEMA_VERSION + 1,
            device_id: "desktop".into(),
            generated_at: "2026-07-16T00:00:00Z".into(),
            entities: SyncEntitiesV1::default(),
        };
        assert!(merge_sync_envelopes(vec![envelope], "phone".into(), None).is_err());
    }
}
