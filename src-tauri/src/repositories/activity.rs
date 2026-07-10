use crate::db_sqlite::SqliteDb;
use crate::domain::{ActivityEvent, ActivityEventType, ResourceKind};
use rusqlite::{params, params_from_iter, types::Value as SqlValue};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityCursor {
    pub started_at: String,
    pub id: String,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityQuery {
    pub resource_kind: Option<ResourceKind>,
    pub resource_id: Option<String>,
    pub event_type: Option<ActivityEventType>,
    pub started_at_from: Option<String>,
    pub started_at_to: Option<String>,
    /// Descending keyset cursor. The cursor row itself is excluded.
    pub cursor: Option<ActivityCursor>,
    /// Bounded to 1..=500; callers receive no more than this many DTOs.
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivityPage {
    pub events: Vec<ActivityEvent>,
    pub next_cursor: Option<ActivityCursor>,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityAggregateQuery {
    pub resource_kind: Option<ResourceKind>,
    pub started_at_from: Option<String>,
    pub started_at_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityDayAggregate {
    pub day: String,
    pub event_count: u64,
    pub duration_seconds: u64,
}

/// SQLite access for the activity timeline. Aggregate queries deliberately select
/// only indexed/scalar columns; event payload JSON is decoded only for page rows.
pub struct ActivityRepository<'db> {
    db: &'db SqliteDb,
}

impl<'db> ActivityRepository<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self { db }
    }

    pub fn insert(&self, event: &ActivityEvent) -> Result<(), String> {
        self.db
            .with_connection(|conn| insert_event(conn, event, false))
    }

    pub fn upsert(&self, event: &ActivityEvent) -> Result<(), String> {
        self.db
            .with_connection(|conn| insert_event(conn, event, true))
    }

    pub fn insert_many(&self, events: &[ActivityEvent]) -> Result<(), String> {
        self.db.with_connection_mut(|conn| {
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            for event in events {
                insert_event(&tx, event, false)?;
            }
            tx.commit().map_err(|e| e.to_string())
        })
    }

    pub fn get(&self, id: &str) -> Result<Option<ActivityEvent>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, resource_kind, resource_id, event_type, started_at, ended_at, \
                     duration_seconds, provider_id, payload_json FROM activity_events WHERE id=?1",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
            match rows.next().map_err(|e| e.to_string())? {
                Some(row) => read_event(row).map(Some).map_err(|e| e.to_string()),
                None => Ok(None),
            }
        })
    }

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            conn.execute("DELETE FROM activity_events WHERE id=?1", params![id])
                .map(|changed| changed > 0)
                .map_err(|e| e.to_string())
        })
    }

    pub fn page(&self, query: &ActivityQuery) -> Result<ActivityPage, String> {
        let limit = query.limit.clamp(1, 500);
        self.db.with_connection(|conn| {
            let mut conditions = Vec::new();
            let mut values = Vec::<SqlValue>::new();
            if let Some(kind) = query.resource_kind {
                conditions.push("resource_kind=?");
                values.push(SqlValue::Text(enum_text(&kind)?));
            }
            if let Some(resource_id) = &query.resource_id {
                conditions.push("resource_id=?");
                values.push(SqlValue::Text(resource_id.clone()));
            }
            if let Some(event_type) = query.event_type {
                conditions.push("event_type=?");
                values.push(SqlValue::Text(enum_text(&event_type)?));
            }
            if let Some(from) = &query.started_at_from {
                conditions.push("started_at>=?");
                values.push(SqlValue::Text(from.clone()));
            }
            if let Some(to) = &query.started_at_to {
                conditions.push("started_at<=?");
                values.push(SqlValue::Text(to.clone()));
            }
            if let Some(cursor) = &query.cursor {
                conditions.push("(started_at < ? OR (started_at = ? AND id < ?))");
                values.push(SqlValue::Text(cursor.started_at.clone()));
                values.push(SqlValue::Text(cursor.started_at.clone()));
                values.push(SqlValue::Text(cursor.id.clone()));
            }

            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", conditions.join(" AND "))
            };
            values.push(SqlValue::Integer((limit + 1) as i64));
            let sql = format!(
                "SELECT id, resource_kind, resource_id, event_type, started_at, ended_at, \
                 duration_seconds, provider_id, payload_json FROM activity_events{where_clause} \
                 ORDER BY started_at DESC, id DESC LIMIT ?"
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params_from_iter(values.iter()), read_event)
                .map_err(|e| e.to_string())?;
            let mut events = rows
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let next_cursor = if events.len() > limit {
                events.pop();
                events.last().map(|event| ActivityCursor {
                    started_at: event.started_at.clone(),
                    id: event.id.clone(),
                })
            } else {
                None
            };
            Ok(ActivityPage {
                events,
                next_cursor,
            })
        })
    }

    /// SQL-only daily aggregation. This never reads or parses `payload_json`.
    pub fn aggregate_by_day(
        &self,
        query: &ActivityAggregateQuery,
    ) -> Result<Vec<ActivityDayAggregate>, String> {
        self.db.with_connection(|conn| {
            let mut conditions = Vec::new();
            let mut values = Vec::<SqlValue>::new();
            if let Some(kind) = query.resource_kind {
                conditions.push("resource_kind=?");
                values.push(SqlValue::Text(enum_text(&kind)?));
            }
            if let Some(from) = &query.started_at_from {
                conditions.push("started_at>=?");
                values.push(SqlValue::Text(from.clone()));
            }
            if let Some(to) = &query.started_at_to {
                conditions.push("started_at<=?");
                values.push(SqlValue::Text(to.clone()));
            }
            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", conditions.join(" AND "))
            };
            let sql = format!(
                "SELECT substr(started_at, 1, 10) AS day, COUNT(*) AS event_count, \
                 COALESCE(SUM(duration_seconds), 0) AS duration_seconds \
                 FROM activity_events{where_clause} GROUP BY day ORDER BY day DESC"
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params_from_iter(values.iter()), |row| {
                    Ok(ActivityDayAggregate {
                        day: row.get(0)?,
                        event_count: u64::try_from(row.get::<_, i64>(1)?).map_err(|_| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                rusqlite::types::Type::Integer,
                                Box::new(std::io::Error::other(
                                    "activity event count cannot be negative",
                                )),
                            )
                        })?,
                        duration_seconds: u64::try_from(row.get::<_, i64>(2)?).map_err(|_| {
                            rusqlite::Error::FromSqlConversionFailure(
                                2,
                                rusqlite::types::Type::Integer,
                                Box::new(std::io::Error::other(
                                    "activity duration total cannot be negative",
                                )),
                            )
                        })?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        })
    }
}

fn insert_event(
    conn: &rusqlite::Connection,
    event: &ActivityEvent,
    upsert: bool,
) -> Result<(), String> {
    let duration_seconds = event
        .duration_seconds
        .map(i64::try_from)
        .transpose()
        .map_err(|_| "activity duration_seconds exceeds SQLite INTEGER range".to_string())?;
    let payload_json = serde_json::to_string(&event.payload).map_err(|e| e.to_string())?;
    let sql = if upsert {
        "INSERT INTO activity_events \
         (id,resource_kind,resource_id,event_type,started_at,ended_at,duration_seconds,provider_id,payload_json) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) \
         ON CONFLICT(id) DO UPDATE SET resource_kind=excluded.resource_kind, \
         resource_id=excluded.resource_id,event_type=excluded.event_type,started_at=excluded.started_at, \
         ended_at=excluded.ended_at,duration_seconds=excluded.duration_seconds,provider_id=excluded.provider_id, \
         payload_json=excluded.payload_json"
    } else {
        "INSERT INTO activity_events \
         (id,resource_kind,resource_id,event_type,started_at,ended_at,duration_seconds,provider_id,payload_json) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"
    };
    conn.execute(
        sql,
        params![
            event.id,
            enum_text(&event.resource_kind)?,
            event.resource_id,
            enum_text(&event.event_type)?,
            event.started_at,
            event.ended_at,
            duration_seconds,
            event.provider_id,
            payload_json,
        ],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn read_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActivityEvent> {
    let resource_kind: String = row.get(1)?;
    let event_type: String = row.get(3)?;
    let payload_json: String = row.get(8)?;
    let payload = serde_json::from_str::<Value>(&payload_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let duration_seconds = row
        .get::<_, Option<i64>>(6)?
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::other(
                        "activity duration_seconds cannot be negative",
                    )),
                )
            })
        })
        .transpose()?;
    Ok(ActivityEvent {
        id: row.get(0)?,
        resource_kind: enum_from_text(&resource_kind).map_err(conversion_error)?,
        resource_id: row.get(2)?,
        event_type: enum_from_text(&event_type).map_err(conversion_error)?,
        started_at: row.get(4)?,
        ended_at: row.get(5)?,
        duration_seconds,
        provider_id: row.get(7)?,
        payload,
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
    use serde_json::json;
    use std::time::{Duration, Instant};

    fn event(id: &str, started_at: &str) -> ActivityEvent {
        ActivityEvent {
            id: id.to_owned(),
            resource_kind: ResourceKind::Game,
            resource_id: "game-1".to_owned(),
            event_type: ActivityEventType::Progressed,
            started_at: started_at.to_owned(),
            ended_at: None,
            duration_seconds: Some(42),
            provider_id: Some("legacy".to_owned()),
            payload: json!({"nested": {"array": [1, true, null]}, "label": "保真"}),
        }
    }

    #[test]
    fn activity_crud_preserves_payload_and_uses_keyset_paging() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = ActivityRepository::new(&db);
        let first = event("event-1", "2026-01-03T00:00:00Z");
        let second = event("event-2", "2026-01-02T00:00:00Z");
        repository.insert(&first).unwrap();
        repository.insert(&second).unwrap();
        assert_eq!(repository.get("event-1").unwrap(), Some(first.clone()));
        let page = repository
            .page(&ActivityQuery {
                limit: 1,
                ..ActivityQuery::default()
            })
            .unwrap();
        assert_eq!(page.events, vec![first.clone()]);
        let next = repository
            .page(&ActivityQuery {
                limit: 1,
                cursor: page.next_cursor,
                ..ActivityQuery::default()
            })
            .unwrap();
        assert_eq!(next.events, vec![second]);
        let mut replacement = first.clone();
        replacement.duration_seconds = Some(84);
        replacement.payload = json!({"exact": ["nested", {"object": true}]});
        repository.upsert(&replacement).unwrap();
        assert_eq!(repository.get("event-1").unwrap(), Some(replacement));
        assert!(repository.delete("event-2").unwrap());
        assert!(!repository.delete("event-2").unwrap());
    }

    #[test]
    fn activity_20k_aggregate_and_page_benchmark_stays_bounded() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = ActivityRepository::new(&db);
        let events = (0..20_000)
            .map(|index| ActivityEvent {
                id: format!("event-{index:05}"),
                resource_kind: if index % 2 == 0 {
                    ResourceKind::Game
                } else {
                    ResourceKind::Anime
                },
                resource_id: format!("resource-{}", index % 500),
                event_type: ActivityEventType::Progressed,
                started_at: format!(
                    "2026-02-{:02}T{:02}:{:02}:00Z",
                    index % 28 + 1,
                    index % 24,
                    index % 60
                ),
                ended_at: None,
                duration_seconds: Some(1),
                provider_id: None,
                payload: json!({"large-but-page-local": "x".repeat(512), "index": index}),
            })
            .collect::<Vec<_>>();
        repository.insert_many(&events).unwrap();
        let started = Instant::now();
        let aggregates = repository
            .aggregate_by_day(&ActivityAggregateQuery {
                resource_kind: Some(ResourceKind::Game),
                started_at_from: Some("2026-02-01T00:00:00Z".to_owned()),
                started_at_to: Some("2026-02-28T23:59:59Z".to_owned()),
            })
            .unwrap();
        let page = repository
            .page(&ActivityQuery {
                resource_kind: Some(ResourceKind::Game),
                limit: 50,
                ..ActivityQuery::default()
            })
            .unwrap();
        let elapsed = started.elapsed();
        assert_eq!(
            aggregates
                .iter()
                .map(|bucket| bucket.event_count)
                .sum::<u64>(),
            10_000
        );
        assert_eq!(
            page.events.len(),
            50,
            "page must not deserialize the full timeline"
        );
        assert!(
            elapsed <= Duration::from_millis(800),
            "20k aggregate/page took {elapsed:?}"
        );
        db.with_connection(|conn| {
            let plan = conn.prepare("EXPLAIN QUERY PLAN SELECT COUNT(*) FROM activity_events WHERE started_at >= ?1").map_err(|e| e.to_string())?.query_map(["2026-02-01T00:00:00Z"], |row| row.get::<_, String>(3)).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            assert!(plan.iter().any(|detail| detail.contains("idx_activity_events_started_at")), "query plan: {plan:?}");
            Ok(())
        }).unwrap();
    }
}
