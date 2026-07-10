use crate::db_sqlite::SqliteDb;
use crate::domain::{BackgroundJob, BackgroundJobStatus, ProviderError};
use rusqlite::{params, params_from_iter, types::Value as SqlValue};
use serde_json::Value;

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
        let limit = limit.clamp(1, 500);
        self.db.with_connection(|conn| {
            let mut values = Vec::<SqlValue>::new();
            let where_clause = if statuses.is_empty() {
                String::new()
            } else {
                let placeholders = (0..statuses.len()).map(|_| "?").collect::<Vec<_>>().join(",");
                for status in statuses {
                    values.push(SqlValue::Text(enum_text(status)?));
                }
                format!(" WHERE status IN ({placeholders})")
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

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            conn.execute("DELETE FROM background_jobs WHERE id=?1", params![id])
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
    #[test]
    fn background_jobs_crud_preserves_error_and_metadata_json() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = BackgroundJobRepository::new(&db);
        let job = BackgroundJob {
            id: "job-1".to_owned(),
            kind: "metadata_refresh".to_owned(),
            title: "Refresh metadata".to_owned(),
            status: BackgroundJobStatus::Failed,
            progress: 0.5,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:01Z".to_owned(),
            error: Some(ProviderError {
                kind: ProviderErrorKind::Timeout,
                message: "timed out".to_owned(),
                retryable: true,
                retry_after_ms: Some(500),
                provider_id: Some("provider-a".to_owned()),
                operation: Some("detail".to_owned()),
            }),
            metadata: json!({"retry": {"count": 2}, "labels": ["safe", null]}),
        };
        repository.insert(&job).unwrap();
        assert_eq!(repository.get("job-1").unwrap(), Some(job.clone()));
        assert_eq!(
            repository.list(&[BackgroundJobStatus::Failed], 20).unwrap(),
            vec![job]
        );
        assert!(repository.delete("job-1").unwrap());
    }
}
