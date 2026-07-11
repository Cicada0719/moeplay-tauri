use crate::db_sqlite::SqliteDb;
use crate::domain::{BackgroundJob, BackgroundJobStatus, ProviderError};
use chrono::Utc;
use regex::Regex;
use rusqlite::{params, params_from_iter, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

/// Severity recorded for an operational job event. The serialized form is a
/// stable lowercase string so frontend clients can render it without depending
/// on backend implementation details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundJobEventLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A durable, redacted point on a background job's timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundJobEvent {
    pub job_id: String,
    pub sequence: i64,
    pub level: BackgroundJobEventLevel,
    pub code: String,
    pub message: String,
    pub progress: Option<f64>,
    pub created_at: String,
}

const MAX_EVENTS_PER_JOB: i64 = 200;
const MAX_EVENT_MESSAGE_CHARS: usize = 2_048;

/// Redacts event data at the persistence boundary. This is deliberately kept
/// here (rather than only in a command) so every caller of the repository,
/// including future workers, receives the same no-secret guarantee.
pub fn redact_event_message(input: &str) -> String {
    static USERINFO_URL: OnceLock<Regex> = OnceLock::new();
    static BEARER_VALUE: OnceLock<Regex> = OnceLock::new();
    static SENSITIVE_VALUE: OnceLock<Regex> = OnceLock::new();
    static REQUEST_HEADERS: OnceLock<Regex> = OnceLock::new();
    static AI_PAYLOAD: OnceLock<Regex> = OnceLock::new();

    if input.chars().count() > MAX_EVENT_MESSAGE_CHARS {
        return "[REDACTED LONG PAYLOAD]".to_string();
    }
    if REQUEST_HEADERS
        .get_or_init(|| {
            Regex::new(r"(?is)\b(?:request[_ -]?headers?|headers)\b\s*[:=]")
                .expect("request header regex is valid")
        })
        .is_match(input)
    {
        return "[REDACTED REQUEST HEADERS]".to_string();
    }
    if AI_PAYLOAD
        .get_or_init(|| {
            Regex::new(r"(?is)\b(?:prompt|response|completion|messages)\b\s*[:=]")
                .expect("AI payload regex is valid")
        })
        .is_match(input)
    {
        return "[REDACTED AI PAYLOAD]".to_string();
    }

    let mut value = USERINFO_URL
        .get_or_init(|| {
            Regex::new(r"(?i)(https?://)[^/@\s]+@").expect("URL userinfo regex is valid")
        })
        .replace_all(input, "$1[REDACTED]@")
        .into_owned();
    value = BEARER_VALUE
        .get_or_init(|| {
            Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9._~+/=\-]+").expect("bearer token regex is valid")
        })
        .replace_all(&value, "$1[REDACTED]")
        .into_owned();
    SENSITIVE_VALUE
        .get_or_init(|| {
            Regex::new(
                r#"(?i)("?(?:api[_-]?key|access[_-]?token|token|password|secret|authorization|cookie|set-cookie)"?\s*[:=]\s*"?)[^",\s;&]+"#,
            )
            .expect("sensitive value regex is valid")
        })
        .replace_all(&value, "$1[REDACTED]")
        .into_owned()
}

pub struct BackgroundJobRepository<'db> {
    db: &'db SqliteDb,
}

impl<'db> BackgroundJobRepository<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self { db }
    }

    pub fn insert(&self, job: &BackgroundJob) -> Result<(), String> {
        self.write(job, false)
    }

    pub fn upsert(&self, job: &BackgroundJob) -> Result<(), String> {
        self.write(job, true)
    }

    pub fn get(&self, id: &str) -> Result<Option<BackgroundJob>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id,kind,title,status,progress,created_at,updated_at,error_json,metadata_json \
                     FROM background_jobs WHERE id=?1",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
            match rows.next().map_err(|e| e.to_string())? {
                Some(row) => read_job(row).map(Some).map_err(|e| e.to_string()),
                None => Ok(None),
            }
        })
    }

    pub fn list(
        &self,
        statuses: &[BackgroundJobStatus],
        limit: usize,
    ) -> Result<Vec<BackgroundJob>, String> {
        self.list_filtered(statuses, None, limit)
    }

    /// Lists jobs with optional exact storage-kind filtering. Higher-level
    /// category filtering (for example all `ai_v2.*` jobs as `ai`) belongs in
    /// TaskQueue's public projection mapper.
    pub fn list_filtered(
        &self,
        statuses: &[BackgroundJobStatus],
        kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<BackgroundJob>, String> {
        let limit = limit.clamp(1, 500);
        self.db.with_connection(|conn| {
            let mut values = Vec::<SqlValue>::new();
            let mut predicates = Vec::<String>::new();
            if !statuses.is_empty() {
                let placeholders = (0..statuses.len()).map(|_| "?").collect::<Vec<_>>().join(",");
                for status in statuses {
                    values.push(SqlValue::Text(enum_text(status)?));
                }
                predicates.push(format!("status IN ({placeholders})"));
            }
            if let Some(kind) = kind.map(str::trim).filter(|kind| !kind.is_empty()) {
                predicates.push("kind = ?".to_string());
                values.push(SqlValue::Text(kind.to_string()));
            }
            let where_clause = if predicates.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", predicates.join(" AND "))
            };
            values.push(SqlValue::Integer(limit as i64));
            let sql = format!(
                "SELECT id,kind,title,status,progress,created_at,updated_at,error_json,metadata_json \
                 FROM background_jobs{where_clause} ORDER BY updated_at DESC,id DESC LIMIT ?"
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params_from_iter(values.iter()), read_job)
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        })
    }

    /// Appends one event using a per-job monotonically increasing sequence.
    /// Events are sanitized and the transaction retains only the most recent
    /// 200 events for each job.
    pub fn append_event(
        &self,
        job_id: &str,
        level: BackgroundJobEventLevel,
        code: &str,
        message: &str,
        progress: Option<f64>,
    ) -> Result<BackgroundJobEvent, String> {
        let code = normalize_event_code(code)?;
        let progress = normalize_event_progress(progress)?;
        let message = redact_event_message(message);
        self.db.with_connection_mut(|conn| {
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM background_jobs WHERE id=?1)",
                    params![job_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if !exists {
                return Err("任务不存在".to_string());
            }
            let sequence: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(sequence), 0) + 1 FROM background_job_events WHERE job_id=?1",
                    params![job_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            let created_at = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT INTO background_job_events(job_id,sequence,level,code,message,progress,created_at) \
                 VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    job_id,
                    sequence,
                    enum_text(&level)?,
                    code,
                    message,
                    progress,
                    created_at,
                ],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM background_job_events
                  WHERE job_id=?1
                    AND sequence NOT IN (
                        SELECT sequence FROM background_job_events
                         WHERE job_id=?1 ORDER BY sequence DESC LIMIT ?2
                    )",
                params![job_id, MAX_EVENTS_PER_JOB],
            )
            .map_err(|e| e.to_string())?;
            prune_terminal_events_older_than_now(&tx)?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(BackgroundJobEvent {
                job_id: job_id.to_string(),
                sequence,
                level,
                code,
                message,
                progress,
                created_at,
            })
        })
    }

    /// Returns events in ascending sequence order. `after_sequence` is an
    /// exclusive keyset cursor, making it safe for polling without clearing or
    /// duplicating events already rendered by the Task Center.
    pub fn list_events(
        &self,
        job_id: &str,
        after_sequence: Option<i64>,
        limit: usize,
    ) -> Result<Vec<BackgroundJobEvent>, String> {
        let limit = limit.clamp(1, MAX_EVENTS_PER_JOB as usize);
        let after_sequence = after_sequence.unwrap_or(0).max(0);
        self.db.with_connection(|conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM background_jobs WHERE id=?1)",
                    params![job_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if !exists {
                return Err("任务不存在".to_string());
            }
            let mut stmt = conn
                .prepare(
                    "SELECT job_id,sequence,level,code,message,progress,created_at
                       FROM background_job_events
                      WHERE job_id=?1 AND sequence>?2
                      ORDER BY sequence ASC
                      LIMIT ?3",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![job_id, after_sequence, limit as i64], read_event)
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        })
    }

    /// Removes timeline events belonging to jobs that were already terminal
    /// before the supplied RFC3339 cutoff. This is public for scheduled cleanup
    /// and deterministic retention tests; normal app opens and event writes run
    /// the fixed 30-day retention automatically.
    pub fn purge_terminal_events_before(&self, cutoff: &str) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM background_job_events
                  WHERE job_id IN (
                    SELECT id FROM background_jobs
                     WHERE status IN ('succeeded','failed','cancelled')
                       AND julianday(updated_at) < julianday(?1)
                  )",
                params![cutoff],
            )
            .map_err(|e| e.to_string())
        })
    }

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            conn.execute("DELETE FROM background_jobs WHERE id=?1", params![id])
                .map(|changed| changed > 0)
                .map_err(|e| e.to_string())
        })
    }

    /// Atomic guard used by Task Center cleanup: active/paused jobs can never
    /// be removed even if another producer changes state between list/delete.
    pub fn delete_if_terminal(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM background_jobs WHERE id=?1 AND status IN ('succeeded','failed','cancelled')",
                params![id],
            )
            .map(|changed| changed > 0)
            .map_err(|e| e.to_string())
        })
    }

    fn write(&self, job: &BackgroundJob, upsert: bool) -> Result<(), String> {
        if !(0.0..=1.0).contains(&job.progress) {
            return Err("background job progress must be between 0.0 and 1.0".to_string());
        }
        let error_json = job
            .error
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| e.to_string())?;
        let metadata_json = serde_json::to_string(&job.metadata).map_err(|e| e.to_string())?;
        let sql = if upsert {
            "INSERT INTO background_jobs(id,kind,title,status,progress,created_at,updated_at,error_json,metadata_json) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET \
             kind=excluded.kind,title=excluded.title,status=excluded.status,progress=excluded.progress,updated_at=excluded.updated_at,error_json=excluded.error_json,metadata_json=excluded.metadata_json"
        } else {
            "INSERT INTO background_jobs(id,kind,title,status,progress,created_at,updated_at,error_json,metadata_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)"
        };
        self.db.with_connection(|conn| {
            conn.execute(
                sql,
                params![
                    job.id,
                    job.kind,
                    job.title,
                    enum_text(&job.status)?,
                    job.progress,
                    job.created_at,
                    job.updated_at,
                    error_json,
                    metadata_json
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
    }
}

fn normalize_event_code(code: &str) -> Result<String, String> {
    let code = code.trim();
    if code.is_empty() || code.len() > 128 {
        return Err("任务事件代码必须为 1 到 128 个字符".to_string());
    }
    if !code
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
    {
        return Err("任务事件代码只能包含字母、数字、点、下划线、连字符或冒号".to_string());
    }
    Ok(code.to_string())
}

fn normalize_event_progress(progress: Option<f64>) -> Result<Option<f64>, String> {
    match progress {
        Some(value) if !value.is_finite() || !(0.0..=1.0).contains(&value) => {
            Err("任务事件进度必须是 0 到 1 之间的有限数字".to_string())
        }
        Some(value) => Ok(Some(value)),
        None => Ok(None),
    }
}

fn prune_terminal_events_older_than_now(conn: &rusqlite::Transaction<'_>) -> Result<(), String> {
    conn.execute(
        "DELETE FROM background_job_events
          WHERE job_id IN (
            SELECT id FROM background_jobs
             WHERE status IN ('succeeded','failed','cancelled')
               AND julianday(updated_at) < julianday('now', '-30 days')
          )",
        [],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn read_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<BackgroundJob> {
    let status: String = row.get(3)?;
    let error_json: Option<String> = row.get(7)?;
    let metadata_json: String = row.get(8)?;
    let error = error_json
        .as_deref()
        .map(serde_json::from_str::<ProviderError>)
        .transpose()
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    let metadata = serde_json::from_str::<Value>(&metadata_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(BackgroundJob {
        id: row.get(0)?,
        kind: row.get(1)?,
        title: row.get(2)?,
        status: enum_from_text(&status).map_err(conversion_error)?,
        progress: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        error,
        metadata,
    })
}

fn read_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<BackgroundJobEvent> {
    let level: String = row.get(2)?;
    Ok(BackgroundJobEvent {
        job_id: row.get(0)?,
        sequence: row.get(1)?,
        level: enum_from_text(&level).map_err(conversion_error)?,
        code: row.get(3)?,
        message: row.get(4)?,
        progress: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn enum_text<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_value(value)
        .map_err(|e| e.to_string())?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "domain enum did not serialize to a string".to_string())
}
fn enum_from_text<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, String> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(|e| e.to_string())
}
fn conversion_error(error: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::other(error)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_sqlite::SqliteDb;
    use crate::domain::{ProviderError, ProviderErrorKind};
    use serde_json::json;

    fn job(id: &str, status: BackgroundJobStatus, updated_at: &str) -> BackgroundJob {
        BackgroundJob {
            id: id.to_owned(),
            kind: "metadata_refresh".to_owned(),
            title: "Refresh metadata".to_owned(),
            status,
            progress: 0.5,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: updated_at.to_owned(),
            error: None,
            metadata: json!({}),
        }
    }

    #[test]
    fn background_jobs_crud_preserves_error_and_metadata_json() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = BackgroundJobRepository::new(&db);
        let mut job = job("job-1", BackgroundJobStatus::Failed, "2026-01-01T00:00:01Z");
        job.error = Some(ProviderError {
            kind: ProviderErrorKind::Timeout,
            message: "timed out".to_owned(),
            retryable: true,
            retry_after_ms: Some(500),
            provider_id: Some("provider-a".to_owned()),
            operation: Some("detail".to_owned()),
        });
        job.metadata = json!({"retry": {"count": 2}, "labels": ["safe", null]});
        repository.insert(&job).unwrap();
        assert_eq!(repository.get("job-1").unwrap(), Some(job.clone()));
        assert_eq!(
            repository.list(&[BackgroundJobStatus::Failed], 20).unwrap(),
            vec![job]
        );
        assert!(repository.delete("job-1").unwrap());
    }

    #[test]
    fn filtered_list_and_terminal_delete_are_guarded_in_sql() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = BackgroundJobRepository::new(&db);
        for (id, kind, status) in [
            ("active", "download", BackgroundJobStatus::Running),
            ("paused", "download", BackgroundJobStatus::Paused),
            ("done", "download", BackgroundJobStatus::Succeeded),
            (
                "ai-failed",
                "ai_v2.recommendation",
                BackgroundJobStatus::Failed,
            ),
        ] {
            let mut item = job(id, status, &format!("2026-01-01T00:00:0{}Z", id.len()));
            item.kind = kind.to_string();
            item.title = id.to_string();
            repository.insert(&item).unwrap();
        }

        let downloads = repository.list_filtered(&[], Some("download"), 10).unwrap();
        assert_eq!(downloads.len(), 3);
        let terminal_downloads = repository
            .list_filtered(&[BackgroundJobStatus::Succeeded], Some("download"), 10)
            .unwrap();
        assert_eq!(terminal_downloads[0].id, "done");

        assert!(!repository.delete_if_terminal("active").unwrap());
        assert!(!repository.delete_if_terminal("paused").unwrap());
        assert!(repository.delete_if_terminal("done").unwrap());
        assert!(repository.get("active").unwrap().is_some());
        assert!(repository.get("paused").unwrap().is_some());
    }

    #[test]
    fn event_timeline_is_ordered_keyset_paginated_and_bounded() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = BackgroundJobRepository::new(&db);
        repository
            .insert(&job(
                "event-job",
                BackgroundJobStatus::Running,
                "2026-01-01T00:00:00Z",
            ))
            .unwrap();

        for index in 1..=205 {
            let event = repository
                .append_event(
                    "event-job",
                    BackgroundJobEventLevel::Info,
                    "progress",
                    &format!("step {index}"),
                    Some(index as f64 / 205.0),
                )
                .unwrap();
            assert_eq!(event.sequence, index);
        }

        let all = repository.list_events("event-job", None, 500).unwrap();
        assert_eq!(all.len(), 200);
        assert_eq!(all.first().unwrap().sequence, 6);
        assert_eq!(all.last().unwrap().sequence, 205);
        let next = repository.list_events("event-job", Some(202), 20).unwrap();
        assert_eq!(
            next.iter().map(|event| event.sequence).collect::<Vec<_>>(),
            vec![203, 204, 205]
        );

        db.with_connection(|conn| {
            let indexed: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_background_job_events_job_sequence_desc')",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            assert!(indexed);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn event_redaction_and_terminal_retention_never_store_sensitive_payloads() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = BackgroundJobRepository::new(&db);
        repository
            .insert(&job(
                "secret-job",
                BackgroundJobStatus::Running,
                "2026-01-01T00:00:00Z",
            ))
            .unwrap();
        repository
            .append_event(
                "secret-job",
                BackgroundJobEventLevel::Error,
                "request_failed",
                "Authorization: Bearer super-secret https://alice:pa55@example.test/path?token=query-secret api_key=key-secret",
                None,
            )
            .unwrap();
        repository
            .append_event(
                "secret-job",
                BackgroundJobEventLevel::Debug,
                "ai_response",
                "response={\"choices\":[\"secret completion\"]}",
                None,
            )
            .unwrap();
        repository
            .append_event(
                "secret-job",
                BackgroundJobEventLevel::Debug,
                "request_context",
                "request_headers={Authorization: Bearer header-secret}",
                None,
            )
            .unwrap();

        let events = repository.list_events("secret-job", None, 20).unwrap();
        assert!(!events[0].message.contains("super-secret"));
        assert!(!events[0].message.contains("pa55"));
        assert!(!events[0].message.contains("query-secret"));
        assert!(!events[0].message.contains("key-secret"));
        assert_eq!(events[1].message, "[REDACTED AI PAYLOAD]");
        assert_eq!(events[2].message, "[REDACTED REQUEST HEADERS]");

        let mut old_terminal = job(
            "old-terminal",
            BackgroundJobStatus::Running,
            "2026-07-01T00:00:00Z",
        );
        repository.insert(&old_terminal).unwrap();
        repository
            .append_event(
                "old-terminal",
                BackgroundJobEventLevel::Info,
                "complete",
                "finished",
                Some(1.0),
            )
            .unwrap();
        old_terminal.status = BackgroundJobStatus::Succeeded;
        old_terminal.updated_at = "2025-01-01T00:00:00Z".to_string();
        repository.upsert(&old_terminal).unwrap();
        assert_eq!(
            repository
                .purge_terminal_events_before("2025-02-01T00:00:00Z")
                .unwrap(),
            1
        );
        assert!(repository
            .list_events("old-terminal", None, 20)
            .unwrap()
            .is_empty());
    }
}
