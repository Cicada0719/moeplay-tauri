use crate::db_sqlite::SqliteDb;
use crate::domain::{ProviderHealth, ProviderHealthState};
use rusqlite::params;
use serde_json::Value;

pub struct ProviderHealthRepository<'db> {
    db: &'db SqliteDb,
}

impl<'db> ProviderHealthRepository<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self { db }
    }

    pub fn upsert(&self, health: &ProviderHealth) -> Result<(), String> {
        let success_count = i64::try_from(health.success_count)
            .map_err(|_| "provider success_count exceeds SQLite INTEGER range".to_string())?;
        let failure_count = i64::try_from(health.failure_count)
            .map_err(|_| "provider failure_count exceeds SQLite INTEGER range".to_string())?;
        let consecutive_failures = i64::from(health.consecutive_failures);
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO provider_health(provider_id,operation,state,success_count,failure_count,consecutive_failures,latency_ms_ema,last_success_at,last_failure_at,circuit_open_until,last_error_kind) \
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) \
                 ON CONFLICT(provider_id,operation) DO UPDATE SET state=excluded.state,success_count=excluded.success_count, \
                 failure_count=excluded.failure_count,consecutive_failures=excluded.consecutive_failures,latency_ms_ema=excluded.latency_ms_ema, \
                 last_success_at=excluded.last_success_at,last_failure_at=excluded.last_failure_at,circuit_open_until=excluded.circuit_open_until,last_error_kind=excluded.last_error_kind",
                params![
                    health.provider_id,
                    health.operation,
                    enum_text(&health.state)?,
                    success_count,
                    failure_count,
                    consecutive_failures,
                    health.latency_ms_ema,
                    health.last_success_at,
                    health.last_failure_at,
                    health.circuit_open_until,
                    health.last_error_kind.as_ref().map(enum_text).transpose()?,
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
    }

    pub fn get(
        &self,
        provider_id: &str,
        operation: &str,
    ) -> Result<Option<ProviderHealth>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT provider_id,operation,state,success_count,failure_count,consecutive_failures,latency_ms_ema,last_success_at,last_failure_at,circuit_open_until,last_error_kind \
                 FROM provider_health WHERE provider_id=?1 AND operation=?2"
            ).map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![provider_id, operation]).map_err(|e| e.to_string())?;
            match rows.next().map_err(|e| e.to_string())? {
                Some(row) => read_health(row).map(Some).map_err(|e| e.to_string()),
                None => Ok(None),
            }
        })
    }

    pub fn list_by_state(
        &self,
        state: Option<ProviderHealthState>,
    ) -> Result<Vec<ProviderHealth>, String> {
        self.db.with_connection(|conn| {
            let (sql, values): (&str, Vec<String>) = match state {
                Some(state) => (
                    "SELECT provider_id,operation,state,success_count,failure_count,consecutive_failures,latency_ms_ema,last_success_at,last_failure_at,circuit_open_until,last_error_kind FROM provider_health WHERE state=?1 ORDER BY provider_id,operation",
                    vec![enum_text(&state)?],
                ),
                None => (
                    "SELECT provider_id,operation,state,success_count,failure_count,consecutive_failures,latency_ms_ema,last_success_at,last_failure_at,circuit_open_until,last_error_kind FROM provider_health ORDER BY provider_id,operation",
                    Vec::new(),
                ),
            };
            let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params_from_iter(values.iter()), read_health)
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        })
    }

    pub fn delete(&self, provider_id: &str, operation: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM provider_health WHERE provider_id=?1 AND operation=?2",
                params![provider_id, operation],
            )
            .map(|changed| changed > 0)
            .map_err(|e| e.to_string())
        })
    }
}

fn read_health(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderHealth> {
    let state: String = row.get(2)?;
    let error_kind: Option<String> = row.get(10)?;
    Ok(ProviderHealth {
        provider_id: row.get(0)?,
        operation: row.get(1)?,
        state: enum_from_text(&state).map_err(conversion_error)?,
        success_count: u64::try_from(row.get::<_, i64>(3)?).map_err(|_| {
            conversion_error("provider success_count cannot be negative".to_string())
        })?,
        failure_count: u64::try_from(row.get::<_, i64>(4)?).map_err(|_| {
            conversion_error("provider failure_count cannot be negative".to_string())
        })?,
        consecutive_failures: u32::try_from(row.get::<_, i64>(5)?).map_err(|_| {
            conversion_error("provider consecutive_failures is out of range".to_string())
        })?,
        latency_ms_ema: row.get(6)?,
        last_success_at: row.get(7)?,
        last_failure_at: row.get(8)?,
        circuit_open_until: row.get(9)?,
        last_error_kind: error_kind
            .as_deref()
            .map(enum_from_text)
            .transpose()
            .map_err(conversion_error)?,
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
    #[test]
    fn health_crud_tracks_provider_operation_key() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = ProviderHealthRepository::new(&db);
        let health = ProviderHealth {
            provider_id: "provider-a".to_owned(),
            operation: "search".to_owned(),
            state: ProviderHealthState::Degraded,
            success_count: 10,
            failure_count: 2,
            consecutive_failures: 1,
            latency_ms_ema: Some(18.25),
            last_success_at: Some("2026-01-01T00:00:00Z".to_owned()),
            last_failure_at: Some("2026-01-02T00:00:00Z".to_owned()),
            circuit_open_until: None,
            last_error_kind: Some(crate::domain::ProviderErrorKind::Timeout),
        };
        repository.upsert(&health).unwrap();
        assert_eq!(
            repository.get("provider-a", "search").unwrap(),
            Some(health.clone())
        );
        assert_eq!(
            repository
                .list_by_state(Some(ProviderHealthState::Degraded))
                .unwrap(),
            vec![health]
        );
        assert!(repository.delete("provider-a", "search").unwrap());
    }
}
