use super::contracts::{
    FieldDiff, FieldDiffDisposition, FieldProvenance, FieldProvenanceChange, ImportSourceRecord,
};
use super::identity::stable_hash;
use crate::models::Game;
use chrono::Utc;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub type ProvenanceLedger = BTreeMap<(String, String), FieldProvenance>;

const IMPORTABLE_FIELDS: &[&str] = &[
    "name",
    "exe_path",
    "install_dir",
    "game_type",
    "library_source",
    "library_id",
    "launch_uri",
    "description",
    "cover",
    "background",
    "icon",
    "metadata.developer",
    "metadata.publisher",
    "metadata.engine",
    "metadata.genres",
    "metadata.languages",
    "metadata.voice_languages",
    "metadata.version",
    "metadata.original_name",
    "metadata.homepage",
    "metadata.developer_homepage",
    "metadata.age_rating",
    "metadata.series",
    "metadata.release_date",
    "metadata.release_year",
    "metadata.estimated_hours",
    "metadata.vndb_rating",
    "metadata.bangumi_rating",
    "metadata.vndb_id",
    "metadata.bangumi_id",
    "metadata.cover",
    "metadata.background",
];

pub fn record_field_values(record: &ImportSourceRecord) -> BTreeMap<String, Value> {
    let mut values = BTreeMap::new();
    values.insert(
        "name".to_string(),
        Value::String(record.title.trim().to_string()),
    );
    if let Some(path) = meaningful_string(record.launch_path.as_deref()) {
        values.insert("exe_path".to_string(), Value::String(path));
    }
    if let Some(path) = meaningful_string(record.install_dir.as_deref()) {
        values.insert("install_dir".to_string(), Value::String(path));
    }
    if let Some(platform) = &record.platform_id {
        if let Some(source) = meaningful_string(Some(&platform.source)) {
            values.insert("library_source".to_string(), Value::String(source));
        }
        if let Some(id) = meaningful_string(Some(&platform.id)) {
            values.insert("library_id".to_string(), Value::String(id));
        }
    }
    if let Some(uri) = meaningful_string(record.launch_uri.as_deref()) {
        values.insert("launch_uri".to_string(), Value::String(uri));
    }
    for (field, value) in &record.fields {
        if IMPORTABLE_FIELDS.contains(&field.as_str()) && !value.is_null() {
            values.insert(field.clone(), value.clone());
        }
    }
    values
}

pub fn plan_field_diffs(
    game: Option<&Game>,
    game_id: Option<&str>,
    source: &str,
    record: &ImportSourceRecord,
    ledger: &ProvenanceLedger,
    platform_identity_match: bool,
) -> Vec<FieldDiff> {
    let current_json = game
        .and_then(|game| serde_json::to_value(game).ok())
        .unwrap_or_else(|| Value::Object(Map::new()));
    let values = record_field_values(record);
    values
        .into_iter()
        .map(|(field, incoming)| {
            let current = get_path(&current_json, &field)
                .cloned()
                .unwrap_or(Value::Null);
            let current_provenance = game_id
                .and_then(|id| ledger.get(&(id.to_string(), field.clone())))
                .cloned();
            let (disposition, will_apply) = disposition_for(
                &field,
                &current,
                &incoming,
                current_provenance.as_ref(),
                platform_identity_match,
                game.is_none(),
            );
            FieldDiff {
                field,
                current,
                incoming,
                disposition,
                will_apply,
                current_provenance,
                incoming_source: source.to_string(),
            }
        })
        .collect()
}

fn disposition_for(
    field: &str,
    current: &Value,
    incoming: &Value,
    provenance: Option<&FieldProvenance>,
    platform_identity_match: bool,
    creating: bool,
) -> (FieldDiffDisposition, bool) {
    if current == incoming {
        return (FieldDiffDisposition::Unchanged, false);
    }
    if creating || is_empty(current) {
        return (FieldDiffDisposition::FillEmpty, true);
    }
    if provenance.is_some_and(|item| item.applied_value == *current) {
        return (FieldDiffDisposition::ReplaceImported, true);
    }
    if platform_identity_match && is_platform_managed_identity_field(field) {
        return (FieldDiffDisposition::ReplaceImported, true);
    }
    (FieldDiffDisposition::PreserveUser, false)
}

fn is_platform_managed_identity_field(field: &str) -> bool {
    matches!(
        field,
        "exe_path" | "install_dir" | "library_source" | "library_id" | "launch_uri"
    )
}

pub fn apply_field_diffs(
    game: &mut Game,
    source: &str,
    source_record_id: &str,
    item_idempotency_key: &str,
    diffs: &[FieldDiff],
    ledger: &mut ProvenanceLedger,
) -> Result<Vec<FieldProvenanceChange>, String> {
    let mut json = serde_json::to_value(&*game).map_err(|error| error.to_string())?;
    let imported_at = Utc::now().to_rfc3339();
    let mut changes = Vec::new();

    for diff in diffs.iter().filter(|diff| diff.will_apply) {
        let before = get_path(&json, &diff.field).cloned().unwrap_or(Value::Null);
        set_path(&mut json, &diff.field, diff.incoming.clone())?;
        let current = FieldProvenance {
            game_id: game.id.clone(),
            field: diff.field.clone(),
            source: source.to_string(),
            source_record_id: source_record_id.to_string(),
            imported_at: imported_at.clone(),
            applied_value: diff.incoming.clone(),
            value_hash: value_hash(&diff.incoming),
        };
        let key = (game.id.clone(), diff.field.clone());
        let previous = ledger.insert(key, current.clone());
        changes.push(FieldProvenanceChange {
            item_idempotency_key: item_idempotency_key.to_string(),
            game_id: game.id.clone(),
            field: diff.field.clone(),
            before,
            after: diff.incoming.clone(),
            previous,
            current,
        });
    }

    if !changes.is_empty() {
        *game = serde_json::from_value(json).map_err(|error| error.to_string())?;
        game.touch_updated();
    }
    Ok(changes)
}

pub fn value_hash(value: &Value) -> String {
    let serialized = serde_json::to_string(value).unwrap_or_default();
    stable_hash(&[&serialized])
}

fn meaningful_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.trim().is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) => value.is_empty(),
        _ => false,
    }
}

fn get_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for segment in path.split('.') {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

fn set_path(root: &mut Value, path: &str, value: Value) -> Result<(), String> {
    let segments = path.split('.').collect::<Vec<_>>();
    if segments.is_empty() {
        return Err("field path is empty".to_string());
    }
    let mut current = root;
    for segment in &segments[..segments.len() - 1] {
        let object = current
            .as_object_mut()
            .ok_or_else(|| format!("field parent is not an object: {path}"))?;
        current = object
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    current
        .as_object_mut()
        .ok_or_else(|| format!("field parent is not an object: {path}"))?
        .insert(segments[segments.len() - 1].to_string(), value);
    Ok(())
}
