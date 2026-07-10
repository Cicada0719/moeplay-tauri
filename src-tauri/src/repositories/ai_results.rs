use crate::db_sqlite::SqliteDb;
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiTaskResultRecord {
    pub task_id: String,
    pub outcome_kind: String,
    pub outcome_json: String,
    pub created_at: String,
    pub expires_at: String,
}

pub struct AiTaskResultRepository<'db> {
    db: &'db SqliteDb,
}

impl<'db> AiTaskResultRepository<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self { db }
    }

    pub fn upsert(&self, record: &AiTaskResultRecord) -> Result<(), String> {
        validate_record(record)?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO ai_task_results(task_id,outcome_kind,outcome_json,schema_version,created_at,expires_at) \
                 VALUES(?1,?2,?3,1,?4,?5) ON CONFLICT(task_id) DO UPDATE SET \
                 outcome_kind=excluded.outcome_kind,outcome_json=excluded.outcome_json, \
                 schema_version=1,created_at=excluded.created_at,expires_at=excluded.expires_at",
                params![
                    record.task_id,
                    record.outcome_kind,
                    record.outcome_json,
                    record.created_at,
                    record.expires_at,
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
    }

    pub fn get(
        &self,
        task_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<AiTaskResultRecord>, String> {
        validate_task_id(task_id)?;
        let record = self.db.with_connection(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT task_id,outcome_kind,outcome_json,created_at,expires_at \
                     FROM ai_task_results WHERE task_id=?1",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = statement
                .query(params![task_id])
                .map_err(|e| e.to_string())?;
            match rows.next().map_err(|e| e.to_string())? {
                Some(row) => Ok(Some(AiTaskResultRecord {
                    task_id: row.get(0).map_err(|e| e.to_string())?,
                    outcome_kind: row.get(1).map_err(|e| e.to_string())?,
                    outcome_json: row.get(2).map_err(|e| e.to_string())?,
                    created_at: row.get(3).map_err(|e| e.to_string())?,
                    expires_at: row.get(4).map_err(|e| e.to_string())?,
                })),
                None => Ok(None),
            }
        })?;
        let Some(record) = record else {
            return Ok(None);
        };
        validate_record(&record)?;
        let expires_at = DateTime::parse_from_rfc3339(&record.expires_at)
            .map_err(|_| "AI task result expiry is invalid".to_string())?
            .with_timezone(&Utc);
        if expires_at <= now {
            let _ = self.delete(task_id);
            return Ok(None);
        }
        Ok(Some(record))
    }

    pub fn delete(&self, task_id: &str) -> Result<bool, String> {
        validate_task_id(task_id)?;
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM ai_task_results WHERE task_id=?1",
                params![task_id],
            )
            .map(|changed| changed > 0)
            .map_err(|e| e.to_string())
        })
    }

    pub fn prune_expired(&self, now: DateTime<Utc>) -> Result<usize, String> {
        let now = now.to_rfc3339();
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM ai_task_results WHERE expires_at<=?1",
                params![now],
            )
            .map_err(|e| e.to_string())
        })
    }
}

fn validate_record(record: &AiTaskResultRecord) -> Result<(), String> {
    validate_task_id(&record.task_id)?;
    if !matches!(record.outcome_kind.as_str(), "succeeded" | "failed") {
        return Err("AI task result kind is invalid".to_string());
    }
    let value: Value = serde_json::from_str(&record.outcome_json)
        .map_err(|_| "AI task result JSON is invalid".to_string())?;
    if !value.is_object() {
        return Err("AI task result JSON must be an object".to_string());
    }
    DateTime::parse_from_rfc3339(&record.created_at)
        .map_err(|_| "AI task result creation time is invalid".to_string())?;
    DateTime::parse_from_rfc3339(&record.expires_at)
        .map_err(|_| "AI task result expiry is invalid".to_string())?;
    Ok(())
}

fn validate_task_id(task_id: &str) -> Result<(), String> {
    if task_id.trim().is_empty() || task_id.len() > 128 {
        return Err("AI task result task ID is invalid".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn record(task_id: &str, now: DateTime<Utc>) -> AiTaskResultRecord {
        AiTaskResultRecord {
            task_id: task_id.to_string(),
            outcome_kind: "succeeded".to_string(),
            outcome_json: r#"{"status":"succeeded","payload":{"taskId":"task-1"}}"#.to_string(),
            created_at: now.to_rfc3339(),
            expires_at: (now + Duration::days(7)).to_rfc3339(),
        }
    }

    #[test]
    fn roundtrip_survives_repository_recreation_and_prunes_expired_rows() {
        let db = SqliteDb::open_in_memory().unwrap();
        let now = Utc::now();
        AiTaskResultRepository::new(&db)
            .upsert(&record("task-1", now))
            .unwrap();

        let restored = AiTaskResultRepository::new(&db)
            .get("task-1", now + Duration::minutes(1))
            .unwrap()
            .unwrap();
        assert_eq!(restored.outcome_kind, "succeeded");

        assert!(AiTaskResultRepository::new(&db)
            .get("task-1", now + Duration::days(8))
            .unwrap()
            .is_none());
    }

    #[test]
    fn rejects_non_json_or_unbounded_identifiers() {
        let db = SqliteDb::open_in_memory().unwrap();
        let now = Utc::now();
        let mut invalid = record("task-1", now);
        invalid.outcome_json = "secret=raw".to_string();
        assert!(AiTaskResultRepository::new(&db).upsert(&invalid).is_err());

        invalid = record(&"x".repeat(129), now);
        assert!(AiTaskResultRepository::new(&db).upsert(&invalid).is_err());
    }
}
