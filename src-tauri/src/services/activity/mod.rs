//! Activity Dashboard application service.
//!
//! This module owns dashboard DTO assembly, duration-quality accounting,
//! legacy-session backfill, and exports above db_sqlite repositories.

use crate::db_sqlite::{
    repositories::{
        ActivityAggregateQuery, ActivityCursor, ActivityQuery, ActivityRepository,
        ProgressRepository,
    },
    SqliteDb,
};
use crate::domain::{ActivityEvent, ActivityEventType, ProgressRecord, ResourceKind};
use crate::models::Game;
use chrono::{DateTime, FixedOffset};
use rusqlite::{params, params_from_iter, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{collections::HashMap, fs, path::Path};

const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 500;
const DEFAULT_CONTINUE_LIMIT: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurationQuality {
    Exact,
    Estimated,
    ProgressOnly,
}
impl DurationQuality {
    fn database_value(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Estimated => "estimated",
            Self::ProgressOnly => "none",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityFilters {
    pub resource_kind: Option<ResourceKind>,
    pub resource_id: Option<String>,
    pub event_type: Option<ActivityEventType>,
    pub started_at_from: Option<String>,
    pub started_at_to: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEventsRequest {
    #[serde(default)]
    pub filters: ActivityFilters,
    pub cursor: Option<ActivityCursorDto>,
    pub limit: Option<usize>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityCursorDto {
    pub started_at: String,
    pub id: String,
}
impl From<ActivityCursor> for ActivityCursorDto {
    fn from(value: ActivityCursor) -> Self {
        Self {
            started_at: value.started_at,
            id: value.id,
        }
    }
}
impl From<ActivityCursorDto> for ActivityCursor {
    fn from(value: ActivityCursorDto) -> Self {
        Self {
            started_at: value.started_at,
            id: value.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEventView {
    #[serde(flatten)]
    pub event: ActivityEvent,
    pub duration_quality: DurationQuality,
    pub source_legacy_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEventsResponse {
    pub events: Vec<ActivityEventView>,
    pub next_cursor: Option<ActivityCursorDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDaySummary {
    pub day: String,
    pub event_count: u64,
    pub exact_duration_seconds: u64,
    pub estimated_duration_seconds: u64,
    pub progress_only_count: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySummary {
    pub event_count: u64,
    pub exact_duration_seconds: u64,
    pub estimated_duration_seconds: u64,
    pub progress_only_count: u64,
    pub days: Vec<ActivityDaySummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEventPatch {
    pub event_type: Option<ActivityEventType>,
    pub started_at: Option<String>,
    pub ended_at: Option<Option<String>>,
    pub duration_seconds: Option<Option<u64>>,
    pub provider_id: Option<Option<String>>,
    pub payload: Option<Value>,
    pub duration_quality: Option<DurationQuality>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueCandidate {
    pub resource_kind: ResourceKind,
    pub resource_id: String,
    pub provider_id: Option<String>,
    pub title: String,
    pub artwork_url: Option<String>,
    pub position: Value,
    pub updated_at: String,
    pub completed: bool,
    pub duration_quality: DurationQuality,
    pub exact_duration_seconds: Option<u64>,
    pub estimated_duration_seconds: Option<u64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueQuery {
    pub limit: Option<usize>,
    pub include_completed: Option<bool>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillReport {
    pub created: u64,
    pub updated: u64,
    pub skipped: u64,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityExportFormat {
    Json,
    Csv,
}

pub struct ActivityService<'db> {
    db: &'db SqliteDb,
}
impl<'db> ActivityService<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self { db }
    }

    pub fn events(&self, request: ActivityEventsRequest) -> Result<ActivityEventsResponse, String> {
        let page = ActivityRepository::new(self.db).page(&ActivityQuery {
            resource_kind: request.filters.resource_kind,
            resource_id: request.filters.resource_id,
            event_type: request.filters.event_type,
            started_at_from: request.filters.started_at_from,
            started_at_to: request.filters.started_at_to,
            cursor: request.cursor.map(Into::into),
            limit: request
                .limit
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .clamp(1, MAX_PAGE_SIZE),
        })?;
        let metadata = self.metadata_for_ids(page.events.iter().map(|event| event.id.as_str()))?;
        Ok(ActivityEventsResponse {
            events: page
                .events
                .into_iter()
                .map(|event| {
                    let (duration_quality, source_legacy_id) = metadata
                        .get(&event.id)
                        .cloned()
                        .unwrap_or_else(|| metadata_from_payload(&event.payload));
                    ActivityEventView {
                        event,
                        duration_quality,
                        source_legacy_id,
                    }
                })
                .collect(),
            next_cursor: page.next_cursor.map(Into::into),
        })
    }

    /// Scalar aggregation is O(number of result days) in memory and never
    /// mixes exact and estimated duration into a misleading single total.
    pub fn summary(&self, filters: ActivityFilters) -> Result<ActivitySummary, String> {
        let _ = ActivityRepository::new(self.db).aggregate_by_day(&ActivityAggregateQuery {
            resource_kind: filters.resource_kind,
            started_at_from: filters.started_at_from.clone(),
            started_at_to: filters.started_at_to.clone(),
        })?;
        self.db.with_connection(|conn| {
            let (where_clause, values) = summary_where_clause(&filters)?;
            let sql = format!(
                "SELECT substr(started_at, 1, 10), COUNT(*), \
                 COALESCE(SUM(CASE WHEN duration_quality IN ('exact','baseline') THEN COALESCE(duration_seconds,0) ELSE 0 END),0), \
                 COALESCE(SUM(CASE WHEN duration_quality='estimated' THEN COALESCE(duration_seconds,0) ELSE 0 END),0), \
                 COALESCE(SUM(CASE WHEN duration_quality='none' THEN 1 ELSE 0 END),0) \
                 FROM activity_events{where_clause} GROUP BY 1 ORDER BY 1 DESC"
            );
            let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
            let days = statement.query_map(params_from_iter(values.iter()), |row| Ok(ActivityDaySummary {
                day: row.get(0)?, event_count: row.get::<_, i64>(1)?.max(0) as u64,
                exact_duration_seconds: row.get::<_, i64>(2)?.max(0) as u64,
                estimated_duration_seconds: row.get::<_, i64>(3)?.max(0) as u64,
                progress_only_count: row.get::<_, i64>(4)?.max(0) as u64,
            })).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
            let mut result = ActivitySummary { event_count: 0, exact_duration_seconds: 0, estimated_duration_seconds: 0, progress_only_count: 0, days };
            for day in &result.days { result.event_count += day.event_count; result.exact_duration_seconds += day.exact_duration_seconds; result.estimated_duration_seconds += day.estimated_duration_seconds; result.progress_only_count += day.progress_only_count; }
            Ok(result)
        })
    }

    pub fn upsert_event(
        &self,
        mut event: ActivityEvent,
        quality: DurationQuality,
    ) -> Result<ActivityEventView, String> {
        validate_event(&event)?;
        decorate_payload(&mut event.payload, quality, None);
        ActivityRepository::new(self.db).upsert(&event)?;
        self.write_metadata(&event.id, quality, None, None)?;
        Ok(ActivityEventView {
            event,
            duration_quality: quality,
            source_legacy_id: None,
        })
    }

    pub fn edit_event(
        &self,
        id: &str,
        patch: ActivityEventPatch,
    ) -> Result<ActivityEventView, String> {
        let repository = ActivityRepository::new(self.db);
        let mut event = repository
            .get(id)?
            .ok_or_else(|| format!("activity event not found: {id}"))?;
        if let Some(value) = patch.event_type {
            event.event_type = value;
        }
        if let Some(value) = patch.started_at {
            event.started_at = value;
        }
        if let Some(value) = patch.ended_at {
            event.ended_at = value;
        }
        if let Some(value) = patch.duration_seconds {
            event.duration_seconds = value;
        }
        if let Some(value) = patch.provider_id {
            event.provider_id = value;
        }
        if let Some(value) = patch.payload {
            event.payload = value;
        }
        validate_event(&event)?;
        let (old_quality, source_legacy_id) = self
            .metadata_for_ids([id])?
            .remove(id)
            .unwrap_or_else(|| metadata_from_payload(&event.payload));
        let quality = patch.duration_quality.unwrap_or(old_quality);
        decorate_payload(&mut event.payload, quality, source_legacy_id.as_deref());
        repository.upsert(&event)?;
        self.write_metadata(&event.id, quality, source_legacy_id.as_deref(), None)?;
        Ok(ActivityEventView {
            event,
            duration_quality: quality,
            source_legacy_id,
        })
    }
    pub fn delete_event(&self, id: &str) -> Result<bool, String> {
        ActivityRepository::new(self.db).delete(id)
    }
    pub fn upsert_progress(&self, record: ProgressRecord) -> Result<(), String> {
        validate_progress(&record)?;
        ProgressRepository::new(self.db).upsert(&record)
    }
    pub fn progress(
        &self,
        kind: ResourceKind,
        id: &str,
        provider: Option<&str>,
    ) -> Result<Option<ProgressRecord>, String> {
        ProgressRepository::new(self.db).get(kind, id, provider)
    }

    pub fn continue_candidates(
        &self,
        games: &[Game],
        query: ContinueQuery,
    ) -> Result<Vec<ContinueCandidate>, String> {
        let limit = query.limit.unwrap_or(DEFAULT_CONTINUE_LIMIT).clamp(1, 100);
        let include_completed = query.include_completed.unwrap_or(false);
        let game_by_id = games
            .iter()
            .map(|game| (game.id.as_str(), game))
            .collect::<HashMap<_, _>>();
        let resource_metadata = self.latest_resource_metadata()?;
        self.db.with_connection(|conn| {
            let complete_filter = if include_completed { "" } else { " WHERE completed=0" };
            let sql = format!("SELECT resource_kind,resource_id,provider_id,position_json,updated_at,completed FROM progress_records{complete_filter} ORDER BY updated_at DESC LIMIT ?1");
            let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
            let records = statement.query_map(params![limit as i64], |row| {
                let kind: String = row.get(0)?; let payload: String = row.get(3)?;
                Ok((decode_kind(&kind).map_err(to_sql_error)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, serde_json::from_str::<Value>(&payload).map_err(to_sql_error)?, row.get::<_, String>(4)?, row.get::<_, i64>(5)? != 0))
            }).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
            records.into_iter().map(|(kind, id, provider_key, position, updated_at, completed)| {
                let game = (kind == ResourceKind::Game).then(|| game_by_id.get(id.as_str()).copied()).flatten();
                let metadata = resource_metadata.get(&(kind, id.clone()));
                let title = game.map(|game| game.name.clone()).or_else(|| metadata.and_then(|meta| meta.title.clone())).unwrap_or_else(|| format!("{} {}", resource_kind_name(kind), id));
                let artwork_url = game.and_then(|game| game.cover.clone()).or_else(|| metadata.and_then(|meta| meta.artwork_url.clone()));
                let exact = if kind == ResourceKind::Game { self.exact_seconds_for_resource(kind, &id)? } else { None };
                let estimated = estimated_seconds(&position).or_else(|| game.and_then(|game| game.metadata.estimated_hours.map(|hours| (hours * 3600.0) as u64)));
                let duration_quality = if exact.unwrap_or(0) > 0 { DurationQuality::Exact } else if estimated.unwrap_or(0) > 0 { DurationQuality::Estimated } else { DurationQuality::ProgressOnly };
                Ok(ContinueCandidate { resource_kind: kind, resource_id: id, provider_id: (!provider_key.is_empty()).then_some(provider_key), title, artwork_url, position, updated_at, completed, duration_quality, exact_duration_seconds: exact, estimated_duration_seconds: estimated })
            }).collect()
        })
    }

    /// Idempotently projects legacy game PlaySession values into activity.
    /// IDs contain game + session, while payload retains sourceLegacyId for
    /// traceability and potential future uniqueness constraints.
    pub fn backfill_legacy_game_sessions(&self, games: &[Game]) -> Result<BackfillReport, String> {
        let repository = ActivityRepository::new(self.db);
        let mut report = BackfillReport::default();
        for game in games {
            for session in &game.play_tracker.sessions {
                if session.start_time.trim().is_empty() {
                    report.skipped += 1;
                    continue;
                }
                let source_legacy_id = session.id.clone();
                let id = legacy_event_id(&game.id, &source_legacy_id);
                let existed = repository.get(&id)?.is_some();
                let mut payload = json!({"sourceLegacyId": source_legacy_id, "sessionId": session.id, "durationQuality": "exact", "notes": session.notes});
                remove_null_object_fields(&mut payload);
                let event = ActivityEvent {
                    id: id.clone(),
                    resource_kind: ResourceKind::Game,
                    resource_id: game.id.clone(),
                    event_type: ActivityEventType::Progressed,
                    started_at: canonical_timestamp(&session.start_time),
                    ended_at: session.end_time.as_deref().map(canonical_timestamp),
                    duration_seconds: (session.duration_seconds > 0)
                        .then_some(session.duration_seconds),
                    provider_id: None,
                    payload,
                };
                repository.upsert(&event)?;
                self.write_metadata(
                    &id,
                    DurationQuality::Exact,
                    Some(&source_legacy_id),
                    Some(&session.id),
                )?;
                if existed {
                    report.updated += 1;
                } else {
                    report.created += 1;
                }
            }
        }
        Ok(report)
    }

    pub fn export_events_json(&self, filters: ActivityFilters) -> Result<String, String> {
        serde_json::to_string_pretty(&self.collect_all_events(filters)?)
            .map_err(|error| error.to_string())
    }
    pub fn export_events_csv(&self, filters: ActivityFilters) -> Result<String, String> {
        let mut out = String::from("id,resource_kind,resource_id,event_type,started_at,ended_at,duration_seconds,duration_quality,provider_id,source_legacy_id,payload_json\\n");
        for item in self.collect_all_events(filters)? {
            let event = item.event;
            let values = [
                event.id,
                enum_text(&event.resource_kind)?,
                event.resource_id,
                enum_text(&event.event_type)?,
                event.started_at,
                event.ended_at.unwrap_or_default(),
                event
                    .duration_seconds
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                enum_text(&item.duration_quality)?,
                event.provider_id.unwrap_or_default(),
                item.source_legacy_id.unwrap_or_default(),
                serde_json::to_string(&event.payload).map_err(|error| error.to_string())?,
            ];
            out.push_str(
                &values
                    .iter()
                    .map(|value| csv_cell(value))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            out.push('\n');
        }
        Ok(out)
    }
    pub fn export_events_to_path(
        &self,
        filters: ActivityFilters,
        format: ActivityExportFormat,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        let text = match format {
            ActivityExportFormat::Json => self.export_events_json(filters)?,
            ActivityExportFormat::Csv => self.export_events_csv(filters)?,
        };
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(path, text).map_err(|error| error.to_string())
    }

    fn collect_all_events(
        &self,
        filters: ActivityFilters,
    ) -> Result<Vec<ActivityEventView>, String> {
        let mut cursor = None;
        let mut all = Vec::new();
        loop {
            let page = self.events(ActivityEventsRequest {
                filters: filters.clone(),
                cursor: cursor.clone(),
                limit: Some(MAX_PAGE_SIZE),
            })?;
            all.extend(page.events);
            if let Some(next) = page.next_cursor {
                cursor = Some(next);
            } else {
                break;
            }
        }
        Ok(all)
    }
    fn metadata_for_ids<'a>(
        &self,
        ids: impl IntoIterator<Item = &'a str>,
    ) -> Result<HashMap<String, (DurationQuality, Option<String>)>, String> {
        let ids = ids.into_iter().map(str::to_owned).collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        self.db.with_connection(|conn| {
            let marks = std::iter::repeat_n("?", ids.len()).collect::<Vec<_>>().join(","); let sql = format!("SELECT id,duration_quality,source_legacy_id,payload_json FROM activity_events WHERE id IN ({marks})");
            let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
            let rows = statement
                .query_map(params_from_iter(ids.iter()), |row| {
                    let payload: String = row.get(3)?;
                    let parsed = serde_json::from_str::<Value>(&payload).map_err(to_sql_error)?;
                    let fallback = metadata_from_payload(&parsed);
                    let stored: String = row.get(1)?;
                    let quality = match stored.as_str() {
                        "exact" | "baseline" => DurationQuality::Exact,
                        "estimated" => DurationQuality::Estimated,
                        _ => fallback.0,
                    };
                    Ok((
                        row.get::<_, String>(0)?,
                        (
                            quality,
                            row.get::<_, Option<String>>(2)?.or(fallback.1),
                        ),
                    ))
                })
                .map_err(|error| error.to_string())?;
            rows.collect::<Result<HashMap<_, _>, _>>()
                .map_err(|error| error.to_string())
        })
    }
    fn write_metadata(
        &self,
        id: &str,
        quality: DurationQuality,
        source: Option<&str>,
        session: Option<&str>,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| conn.execute("UPDATE activity_events SET duration_quality=?2, source_legacy_id=?3, session_id=?4 WHERE id=?1", params![id, quality.database_value(), source, session]).map(|_| ()).map_err(|error| error.to_string()))
    }
    fn exact_seconds_for_resource(
        &self,
        kind: ResourceKind,
        id: &str,
    ) -> Result<Option<u64>, String> {
        self.db.with_connection(|conn| { let value: i64 = conn.query_row("SELECT COALESCE(SUM(duration_seconds),0) FROM activity_events WHERE resource_kind=?1 AND resource_id=?2 AND duration_quality IN ('exact','baseline')", params![enum_text(&kind)?, id], |row| row.get(0)).map_err(|error| error.to_string())?; Ok((value > 0).then_some(value as u64)) })
    }
    fn latest_resource_metadata(
        &self,
    ) -> Result<HashMap<(ResourceKind, String), ResourceMetadata>, String> {
        self.db.with_connection(|conn| { let mut statement = conn.prepare("SELECT resource_kind,resource_id,payload_json FROM activity_events ORDER BY started_at DESC,id DESC").map_err(|error| error.to_string())?; let rows = statement.query_map([], |row| { let kind: String = row.get(0)?; let payload: String = row.get(2)?; Ok((decode_kind(&kind).map_err(to_sql_error)?, row.get::<_, String>(1)?, serde_json::from_str::<Value>(&payload).map_err(to_sql_error)?)) }).map_err(|error| error.to_string())?; let mut result = HashMap::new(); for row in rows { let (kind,id,payload) = row.map_err(|error| error.to_string())?; result.entry((kind,id)).or_insert_with(|| ResourceMetadata { title: payload.get("title").and_then(Value::as_str).map(str::to_owned), artwork_url: payload.get("artworkUrl").or_else(|| payload.get("artwork_url")).and_then(Value::as_str).map(str::to_owned) }); } Ok(result) })
    }
}
struct ResourceMetadata {
    title: Option<String>,
    artwork_url: Option<String>,
}

fn summary_where_clause(filters: &ActivityFilters) -> Result<(String, Vec<SqlValue>), String> {
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    if let Some(value) = filters.resource_kind {
        conditions.push("resource_kind=?");
        values.push(SqlValue::Text(enum_text(&value)?));
    }
    if let Some(value) = &filters.resource_id {
        conditions.push("resource_id=?");
        values.push(SqlValue::Text(value.clone()));
    }
    if let Some(value) = filters.event_type {
        conditions.push("event_type=?");
        values.push(SqlValue::Text(enum_text(&value)?));
    }
    if let Some(value) = &filters.started_at_from {
        conditions.push("started_at>=?");
        values.push(SqlValue::Text(value.clone()));
    }
    if let Some(value) = &filters.started_at_to {
        conditions.push("started_at<=?");
        values.push(SqlValue::Text(value.clone()));
    }
    Ok((
        if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        },
        values,
    ))
}
fn validate_event(event: &ActivityEvent) -> Result<(), String> {
    if event.id.trim().is_empty()
        || event.resource_id.trim().is_empty()
        || event.started_at.trim().is_empty()
    {
        return Err("activity id, resourceId, and startedAt are required".to_string());
    }
    if let Some(end) = &event.ended_at {
        if parse_timestamp(end)? < parse_timestamp(&event.started_at)? {
            return Err("activity endedAt cannot precede startedAt".to_string());
        }
    }
    Ok(())
}
fn validate_progress(record: &ProgressRecord) -> Result<(), String> {
    if record.resource_id.trim().is_empty() || record.updated_at.trim().is_empty() {
        return Err("progress resourceId and updatedAt are required".to_string());
    }
    if !record.position.is_object() {
        return Err("progress position must be a JSON object".to_string());
    }
    Ok(())
}
fn parse_timestamp(value: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_rfc3339(value)
        .map_err(|_| format!("timestamp must be RFC3339 with an offset: {value}"))
}
fn canonical_timestamp(value: &str) -> String {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        parsed.to_rfc3339()
    } else if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M") {
        parsed.and_utc().to_rfc3339()
    } else {
        value.to_owned()
    }
}
fn metadata_from_payload(payload: &Value) -> (DurationQuality, Option<String>) {
    let quality = match payload.get("durationQuality").and_then(Value::as_str) {
        Some("exact") => DurationQuality::Exact,
        Some("estimated") => DurationQuality::Estimated,
        _ => DurationQuality::ProgressOnly,
    };
    (
        quality,
        payload
            .get("sourceLegacyId")
            .and_then(Value::as_str)
            .map(str::to_owned),
    )
}
fn decorate_payload(payload: &mut Value, quality: DurationQuality, source: Option<&str>) {
    if !payload.is_object() {
        *payload = Value::Object(Map::new());
    }
    let object = payload.as_object_mut().expect("object assigned");
    object.insert(
        "durationQuality".into(),
        Value::String(enum_text(&quality).expect("serializable quality")),
    );
    if let Some(source) = source {
        object.insert("sourceLegacyId".into(), Value::String(source.to_owned()));
    }
}
fn remove_null_object_fields(value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object.retain(|_, value| !value.is_null());
    }
}
fn legacy_event_id(game_id: &str, session_id: &str) -> String {
    format!("legacy-game-session:{game_id}:{session_id}")
}
fn estimated_seconds(position: &Value) -> Option<u64> {
    for key in [
        "estimatedSeconds",
        "estimated_seconds",
        "durationSeconds",
        "duration_seconds",
    ] {
        if let Some(value) = position
            .get(key)
            .and_then(Value::as_f64)
            .filter(|value| *value > 0.0)
        {
            return Some(value.round() as u64);
        }
    }
    None
}
fn resource_kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Game => "Game",
        ResourceKind::Anime => "Anime",
        ResourceKind::Comic => "Comic",
    }
}
fn enum_text<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_value(value)
        .map_err(|error| error.to_string())?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "enum did not serialize to a string".to_string())
}
fn decode_kind(value: &str) -> serde_json::Result<ResourceKind> {
    serde_json::from_value(Value::String(value.to_owned()))
}
fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn to_sql_error(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PlaySession;

    fn event(
        id: impl Into<String>,
        started_at: impl Into<String>,
        seconds: Option<u64>,
    ) -> ActivityEvent {
        ActivityEvent {
            id: id.into(),
            resource_kind: ResourceKind::Game,
            resource_id: "game-1".into(),
            event_type: ActivityEventType::Progressed,
            started_at: started_at.into(),
            ended_at: None,
            duration_seconds: seconds,
            provider_id: None,
            payload: json!({}),
        }
    }

    #[test]
    fn twenty_thousand_events_page_with_keyset_and_scalar_summary() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = ActivityRepository::new(&db);
        let events = (0..20_000)
            .map(|index| {
                event(
                    format!("{index:05}"),
                    format!("2026-01-{:02}T12:00:00Z", (index % 28) + 1),
                    Some(60),
                )
            })
            .collect::<Vec<_>>();
        repository.insert_many(&events).unwrap();
        db.with_connection(|conn| {
            conn.execute("UPDATE activity_events SET duration_quality='exact'", [])
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
        .unwrap();
        let service = ActivityService::new(&db);
        let first = service
            .events(ActivityEventsRequest {
                filters: ActivityFilters::default(),
                cursor: None,
                limit: Some(100),
            })
            .unwrap();
        assert_eq!(first.events.len(), 100);
        assert!(first.next_cursor.is_some());
        let second = service
            .events(ActivityEventsRequest {
                filters: ActivityFilters::default(),
                cursor: first.next_cursor,
                limit: Some(100),
            })
            .unwrap();
        assert_eq!(second.events.len(), 100);
        assert_ne!(first.events[99].event.id, second.events[0].event.id);
        let summary = service.summary(ActivityFilters::default()).unwrap();
        assert_eq!(summary.event_count, 20_000);
        assert_eq!(summary.exact_duration_seconds, 1_200_000);
        assert_eq!(summary.estimated_duration_seconds, 0);
    }

    #[test]
    fn local_offset_days_survive_dst_transition() {
        let db = SqliteDb::open_in_memory().unwrap();
        let service = ActivityService::new(&db);
        service
            .upsert_event(
                event("before", "2026-03-08T01:30:00-05:00", Some(10)),
                DurationQuality::Exact,
            )
            .unwrap();
        service
            .upsert_event(
                event("after", "2026-03-08T03:30:00-04:00", Some(20)),
                DurationQuality::Exact,
            )
            .unwrap();
        let summary = service.summary(ActivityFilters::default()).unwrap();
        assert_eq!(summary.days.len(), 1);
        assert_eq!(summary.days[0].day, "2026-03-08");
        assert_eq!(summary.days[0].exact_duration_seconds, 30);
    }

    #[test]
    fn duplicate_legacy_backfill_is_idempotent_and_retains_source_id() {
        let db = SqliteDb::open_in_memory().unwrap();
        let service = ActivityService::new(&db);
        let mut game = Game::new("Fixture".into(), "fixture.exe".into());
        game.id = "game-legacy".into();
        game.play_tracker.sessions.push(PlaySession {
            id: "session-1".into(),
            start_time: "2026-02-01 10:00".into(),
            end_time: Some("2026-02-01 11:00".into()),
            duration_seconds: 3_600,
            notes: Some("route A".into()),
        });
        assert_eq!(
            service
                .backfill_legacy_game_sessions(&[game.clone()])
                .unwrap()
                .created,
            1
        );
        assert_eq!(
            service
                .backfill_legacy_game_sessions(&[game])
                .unwrap()
                .updated,
            1
        );
        let events = service.events(ActivityEventsRequest::default()).unwrap();
        assert_eq!(events.events.len(), 1);
        assert_eq!(
            events.events[0].source_legacy_id.as_deref(),
            Some("session-1")
        );
    }

    #[test]
    fn exact_estimated_and_progress_only_never_mix() {
        let db = SqliteDb::open_in_memory().unwrap();
        let service = ActivityService::new(&db);
        service
            .upsert_event(
                event("exact", "2026-02-01T00:00:00Z", Some(60)),
                DurationQuality::Exact,
            )
            .unwrap();
        service
            .upsert_event(
                event("estimated", "2026-02-01T01:00:00Z", Some(120)),
                DurationQuality::Estimated,
            )
            .unwrap();
        service
            .upsert_event(
                event("progress", "2026-02-01T02:00:00Z", None),
                DurationQuality::ProgressOnly,
            )
            .unwrap();
        let summary = service.summary(ActivityFilters::default()).unwrap();
        assert_eq!(
            (
                summary.exact_duration_seconds,
                summary.estimated_duration_seconds,
                summary.progress_only_count
            ),
            (60, 120, 1)
        );
    }

    #[test]
    fn editing_event_rebuilds_quality_aggregation() {
        let db = SqliteDb::open_in_memory().unwrap();
        let service = ActivityService::new(&db);
        service
            .upsert_event(
                event("editable", "2026-02-01T00:00:00Z", Some(60)),
                DurationQuality::Exact,
            )
            .unwrap();
        service
            .edit_event(
                "editable",
                ActivityEventPatch {
                    duration_seconds: Some(Some(90)),
                    duration_quality: Some(DurationQuality::Estimated),
                    ..Default::default()
                },
            )
            .unwrap();
        let summary = service.summary(ActivityFilters::default()).unwrap();
        assert_eq!(
            (
                summary.exact_duration_seconds,
                summary.estimated_duration_seconds
            ),
            (0, 90)
        );
    }
}
