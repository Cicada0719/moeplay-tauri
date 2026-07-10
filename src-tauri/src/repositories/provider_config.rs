use crate::db_sqlite::SqliteDb;
use crate::domain::ResourceKind;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigRecord {
    pub provider_id: String,
    pub resource_kind: ResourceKind,
    pub provider_kind: String,
    pub config_version: u32,
    pub config: Value,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigUpsert {
    pub provider_id: String,
    pub resource_kind: ResourceKind,
    pub provider_kind: String,
    pub config_version: u32,
    pub config: Value,
    pub enabled: bool,
}

pub struct ProviderConfigRepository<'db> {
    db: &'db SqliteDb,
}

impl<'db> ProviderConfigRepository<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self { db }
    }

    /// Inserts or replaces the mutable fields for a provider/resource pair.
    ///
    /// The write and read-back happen in one SQLite transaction. `created_at` is
    /// retained on conflict while `updated_at` is advanced by the repository.
    pub fn upsert(&self, config: &ProviderConfigUpsert) -> Result<ProviderConfigRecord, String> {
        self.upsert_at(config, &chrono::Utc::now().to_rfc3339())
    }

    pub fn get(
        &self,
        provider_id: &str,
        resource_kind: ResourceKind,
    ) -> Result<Option<ProviderConfigRecord>, String> {
        validate_key(provider_id, "provider id")?;
        self.db.with_connection(|conn| {
            get_with_connection(conn, provider_id, resource_kind).map_err(|e| e.to_string())
        })
    }

    pub fn list(&self) -> Result<Vec<ProviderConfigRecord>, String> {
        self.db.with_connection(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at \
                     FROM provider_configs ORDER BY resource_kind,provider_id",
                )
                .map_err(|e| e.to_string())?;
            let rows = statement
                .query_map([], read_record)
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        })
    }

    pub fn list_for_resource_kind(
        &self,
        resource_kind: ResourceKind,
    ) -> Result<Vec<ProviderConfigRecord>, String> {
        self.db.with_connection(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at \
                     FROM provider_configs WHERE resource_kind=?1 ORDER BY enabled DESC,provider_id",
                )
                .map_err(|e| e.to_string())?;
            let rows = statement
                .query_map(params![enum_text(&resource_kind)?], read_record)
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        })
    }

    pub fn list_enabled_for_resource_kind(
        &self,
        resource_kind: ResourceKind,
    ) -> Result<Vec<ProviderConfigRecord>, String> {
        self.db.with_connection(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at \
                     FROM provider_configs WHERE resource_kind=?1 AND enabled=1 ORDER BY provider_id",
                )
                .map_err(|e| e.to_string())?;
            let rows = statement
                .query_map(params![enum_text(&resource_kind)?], read_record)
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        })
    }

    pub fn delete(&self, provider_id: &str, resource_kind: ResourceKind) -> Result<bool, String> {
        validate_key(provider_id, "provider id")?;
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM provider_configs WHERE provider_id=?1 AND resource_kind=?2",
                params![provider_id, enum_text(&resource_kind)?],
            )
            .map(|changed| changed > 0)
            .map_err(|e| e.to_string())
        })
    }

    fn upsert_at(
        &self,
        config: &ProviderConfigUpsert,
        now: &str,
    ) -> Result<ProviderConfigRecord, String> {
        validate_upsert(config)?;
        let config_json = serde_json::to_string(&config.config).map_err(|e| e.to_string())?;
        let resource_kind = enum_text(&config.resource_kind)?;

        self.db.with_connection_mut(|conn| {
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO provider_configs(\
                    provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at\
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?7) \
                 ON CONFLICT(provider_id,resource_kind) DO UPDATE SET \
                    provider_kind=excluded.provider_kind, \
                    config_version=excluded.config_version, \
                    config_json=excluded.config_json, \
                    enabled=excluded.enabled, \
                    updated_at=excluded.updated_at",
                params![
                    config.provider_id,
                    resource_kind,
                    config.provider_kind,
                    i64::from(config.config_version),
                    config_json,
                    i64::from(config.enabled),
                    now,
                ],
            )
            .map_err(|e| e.to_string())?;

            let stored = get_with_connection(&tx, &config.provider_id, config.resource_kind)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "provider config upsert did not produce a row".to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(stored)
        })
    }
}

/// Rejects JSON object keys commonly used to carry credentials. The walk covers
/// nested objects and objects contained in arrays, and intentionally reports only
/// the key path so a rejected secret value is never copied into logs.
pub fn ensure_non_secret_config(config: &Value) -> Result<(), String> {
    validate_value(config, "$")
}

fn validate_upsert(config: &ProviderConfigUpsert) -> Result<(), String> {
    validate_key(&config.provider_id, "provider id")?;
    validate_key(&config.provider_kind, "provider kind")?;
    if config.config_version == 0 {
        return Err("provider config version must be greater than zero".to_string());
    }
    ensure_non_secret_config(&config.config)
}

fn validate_key(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn validate_value(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                let child_path = format!("{path}.{key}");
                if is_obvious_secret_field(key) {
                    return Err(format!(
                        "provider config contains forbidden secret field at {child_path}"
                    ));
                }
                validate_value(child, &child_path)?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                validate_value(child, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_obvious_secret_field(field: &str) -> bool {
    let normalized = field
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();

    matches!(
        normalized.as_str(),
        "token"
            | "passwd"
            | "authorizationheader"
            | "secretkey"
            | "credential"
            | "credentials"
            | "sessioncookie"
    ) || normalized.ends_with("password")
        || normalized.ends_with("apikey")
        || normalized.ends_with("authorization")
        || normalized.ends_with("secret")
        || normalized.ends_with("privatekey")
        || normalized.ends_with("token")
}

fn get_with_connection(
    conn: &Connection,
    provider_id: &str,
    resource_kind: ResourceKind,
) -> rusqlite::Result<Option<ProviderConfigRecord>> {
    let resource_kind = enum_text(&resource_kind).map_err(conversion_error)?;
    let mut statement = conn.prepare(
        "SELECT provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at \
         FROM provider_configs WHERE provider_id=?1 AND resource_kind=?2",
    )?;
    let mut rows = statement.query(params![provider_id, resource_kind])?;
    rows.next()?.map(read_record).transpose()
}

fn read_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderConfigRecord> {
    let resource_kind: String = row.get(1)?;
    let config_version = u32::try_from(row.get::<_, i64>(3)?).map_err(|_| {
        conversion_error("provider config version is outside the supported range".to_string())
    })?;
    let config_json: String = row.get(4)?;
    let config = serde_json::from_str::<Value>(&config_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(error))
    })?;
    ensure_non_secret_config(&config).map_err(conversion_error)?;

    Ok(ProviderConfigRecord {
        provider_id: row.get(0)?,
        resource_kind: enum_from_text(&resource_kind).map_err(conversion_error)?,
        provider_kind: row.get(2)?,
        config_version,
        config,
        enabled: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
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
    use serde_json::json;

    fn input(
        provider_id: &str,
        resource_kind: ResourceKind,
        provider_kind: &str,
        version: u32,
        config: Value,
        enabled: bool,
    ) -> ProviderConfigUpsert {
        ProviderConfigUpsert {
            provider_id: provider_id.to_owned(),
            resource_kind,
            provider_kind: provider_kind.to_owned(),
            config_version: version,
            config,
            enabled,
        }
    }

    #[test]
    fn provider_config_crud_roundtrip_preserves_json_and_composite_identity() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = ProviderConfigRepository::new(&db);
        let original = input(
            "self-hosted",
            ResourceKind::Anime,
            "jellyfin",
            1,
            json!({
                "baseUrl": "https://media.example.test",
                "libraries": ["anime", null],
                "options": {"verifyTls": true, "timeoutMs": 2500}
            }),
            true,
        );
        let inserted = repository
            .upsert_at(&original, "2026-07-10T00:00:00Z")
            .unwrap();
        assert_eq!(inserted.created_at, "2026-07-10T00:00:00Z");
        assert_eq!(inserted.updated_at, "2026-07-10T00:00:00Z");
        assert_eq!(
            repository.get("self-hosted", ResourceKind::Anime).unwrap(),
            Some(inserted.clone())
        );

        let updated = input(
            "self-hosted",
            ResourceKind::Anime,
            "jellyfin",
            2,
            json!({"baseUrl": "https://new.example.test", "options": {"verifyTls": false}}),
            false,
        );
        let updated = repository
            .upsert_at(&updated, "2026-07-10T00:01:00Z")
            .unwrap();
        assert_eq!(updated.created_at, "2026-07-10T00:00:00Z");
        assert_eq!(updated.updated_at, "2026-07-10T00:01:00Z");
        assert_eq!(updated.config_version, 2);
        assert!(!updated.enabled);

        let comic = repository
            .upsert_at(
                &input(
                    "self-hosted",
                    ResourceKind::Comic,
                    "komga",
                    1,
                    json!({"baseUrl": "https://comics.example.test", "username": "reader"}),
                    true,
                ),
                "2026-07-10T00:02:00Z",
            )
            .unwrap();
        assert_eq!(repository.list().unwrap().len(), 2);
        assert_eq!(
            repository
                .list_for_resource_kind(ResourceKind::Anime)
                .unwrap(),
            vec![updated]
        );
        assert!(repository
            .list_enabled_for_resource_kind(ResourceKind::Anime)
            .unwrap()
            .is_empty());
        assert_eq!(
            repository
                .list_enabled_for_resource_kind(ResourceKind::Comic)
                .unwrap(),
            vec![comic.clone()]
        );
        assert_eq!(
            repository.get("self-hosted", ResourceKind::Comic).unwrap(),
            Some(comic)
        );
        assert!(repository
            .delete("self-hosted", ResourceKind::Anime)
            .unwrap());
        assert!(!repository
            .delete("self-hosted", ResourceKind::Anime)
            .unwrap());
    }

    #[test]
    fn provider_config_rejects_secret_fields_recursively_without_echoing_values() {
        let db = SqliteDb::open_in_memory().unwrap();
        let repository = ProviderConfigRepository::new(&db);
        for (field, config) in [
            ("token", json!({"token": "do-not-log-this"})),
            ("apiKey", json!({"nested": [{"apiKey": "do-not-log-this"}]})),
            (
                "api_token",
                json!({"nested": [{"api_token": "do-not-log-this"}]}),
            ),
            ("password", json!({"auth": {"password": "do-not-log-this"}})),
            (
                "authorization",
                json!({"headers": {"authorization": "do-not-log-this"}}),
            ),
        ] {
            let error = repository
                .upsert(&input(
                    "unsafe",
                    ResourceKind::Anime,
                    "remote",
                    1,
                    config,
                    true,
                ))
                .unwrap_err();
            assert!(error.contains(field));
            assert!(!error.contains("do-not-log-this"));
        }
        assert!(repository
            .get("unsafe", ResourceKind::Anime)
            .unwrap()
            .is_none());
    }

    #[test]
    fn provider_config_table_enforces_json_and_scalar_constraints() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.with_connection(|conn| {
            let invalid_json = conn.execute(
                "INSERT INTO provider_configs(provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at) \
                 VALUES('bad-json','anime','remote',1,'{',1,'now','now')",
                [],
            );
            assert!(invalid_json.is_err());

            let invalid_version = conn.execute(
                "INSERT INTO provider_configs(provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at) \
                 VALUES('bad-version','anime','remote',0,'{}',1,'now','now')",
                [],
            );
            assert!(invalid_version.is_err());

            let invalid_enabled = conn.execute(
                "INSERT INTO provider_configs(provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at) \
                 VALUES('bad-enabled','anime','remote',1,'{}',2,'now','now')",
                [],
            );
            assert!(invalid_enabled.is_err());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn repository_fails_closed_when_external_row_contains_secret_config() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO provider_configs(provider_id,resource_kind,provider_kind,config_version,config_json,enabled,created_at,updated_at) \
                 VALUES('unsafe','anime','remote',1,?1,1,'now','now')",
                params![r#"{"nested":{"api_key":"not-returned"}}"#],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
        .unwrap();

        let error = ProviderConfigRepository::new(&db)
            .get("unsafe", ResourceKind::Anime)
            .unwrap_err();
        assert!(error.contains("api_key"));
        assert!(!error.contains("not-returned"));
    }
}
