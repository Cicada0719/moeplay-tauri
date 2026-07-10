use crate::db_sqlite::SqliteDb;
use crate::domain::{ProgressRecord, ResourceKind};
use rusqlite::params;
use serde_json::Value;

pub struct ProgressRepository<'db> {
    db: &'db SqliteDb,
}

impl<'db> ProgressRepository<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self { db }
    }

    pub fn upsert(&self, record: &ProgressRecord) -> Result<(), String> {
        let position_json = serde_json::to_string(&record.position).map_err(|e| e.to_string())?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO progress_records(resource_kind,resource_id,provider_id,position_json,updated_at,completed) \
                 VALUES(?1,?2,?3,?4,?5,?6) \
                 ON CONFLICT(resource_kind,resource_id,provider_id) DO UPDATE SET \
                 position_json=excluded.position_json, updated_at=excluded.updated_at, completed=excluded.completed",
                params![
                    enum_text(&record.resource_kind)?,
                    record.resource_id,
                    provider_key(record.provider_id.as_deref()),
                    position_json,
                    record.updated_at,
                    i64::from(record.completed),
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
    }

    pub fn get(
        &self,
        resource_kind: ResourceKind,
        resource_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Option<ProgressRecord>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT resource_kind,resource_id,provider_id,position_json,updated_at,completed \
                     FROM progress_records WHERE resource_kind=?1 AND resource_id=?2 AND provider_id=?3",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt
                .query(params![enum_text(&resource_kind)?, resource_id, provider_key(provider_id)])
                .map_err(|e| e.to_string())?;
            match rows.next().map_err(|e| e.to_string())? {
                Some(row) => read_record(row).map(Some).map_err(|e| e.to_string()),
                None => Ok(None),
            }
        })
    }

    pub fn list_for_resource(
        &self,
        resource_kind: ResourceKind,
        resource_id: &str,
    ) -> Result<Vec<ProgressRecord>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT resource_kind,resource_id,provider_id,position_json,updated_at,completed \
                     FROM progress_records WHERE resource_kind=?1 AND resource_id=?2 ORDER BY updated_at DESC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![enum_text(&resource_kind)?, resource_id], read_record)
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        })
    }

    pub fn delete(
        &self,
        resource_kind: ResourceKind,
        resource_id: &str,
        provider_id: Option<&str>,
    ) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM progress_records WHERE resource_kind=?1 AND resource_id=?2 AND provider_id=?3",
                params![enum_text(&resource_kind)?, resource_id, provider_key(provider_id)],
            )
            .map(|changed| changed > 0)
            .map_err(|e| e.to_string())
        })
    }
}

fn read_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProgressRecord> {
    let resource_kind: String = row.get(0)?;
    let provider_id: String = row.get(2)?;
    let position_json: String = row.get(3)?;
    let position = serde_json::from_str::<Value>(&position_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(ProgressRecord {
        resource_kind: enum_from_text(&resource_kind).map_err(conversion_error)?,
        resource_id: row.get(1)?,
        provider_id: (!provider_id.is_empty()).then_some(provider_id),
        position,
        updated_at: row.get(4)?,
        completed: row.get::<_, i64>(5)? != 0,
    })
}

fn provider_key(provider_id: Option<&str>) -> String {
    provider_id.unwrap_or_default().to_owned()
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
    use serde_json::json;
    #[test]
    fn progress_crud_preserves_json_and_none_provider_identity() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = ProgressRepository::new(&db);
        let record = ProgressRecord {
            resource_kind: ResourceKind::Anime,
            resource_id: "anime-1".to_owned(),
            provider_id: None,
            position: json!({"episode": 3, "seconds": 17.5, "extras": [true, null]}),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
            completed: false,
        };
        repository.upsert(&record).unwrap();
        assert_eq!(
            repository
                .get(ResourceKind::Anime, "anime-1", None)
                .unwrap(),
            Some(record.clone())
        );
        assert_eq!(
            repository
                .list_for_resource(ResourceKind::Anime, "anime-1")
                .unwrap(),
            vec![record]
        );
        assert!(repository
            .delete(ResourceKind::Anime, "anime-1", None)
            .unwrap());
    }
}
