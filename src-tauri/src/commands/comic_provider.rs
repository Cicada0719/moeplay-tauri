use crate::db::Database;
use crate::db_sqlite::repositories::{
    ProviderConfigRepository, ProviderConfigUpsert, ProviderHealthRepository,
};
use crate::domain::{ProviderError, ProviderErrorKind, ResourceKind};
use crate::providers::comic::{
    ChapterDto, ComicProviderAuthMode, ComicProviderConfig, ComicProviderDescriptor,
    ComicProviderError, ComicProviderRegistry, ProbeDto, ResolveRequest, SearchRequest,
    SeriesDetailDto, SeriesDto,
};
use crate::secret_store::{SecretKind, SecretStore};
use serde::{Deserialize, Serialize};
use tauri::State;

const CONFIG_ERROR: &str = "漫画源配置无效";
const SECURITY_ERROR: &str = "漫画源配置被安全策略拒绝";
const AUTH_ERROR: &str = "漫画源凭据缺失或无效";
const NETWORK_ERROR: &str = "漫画源网络请求失败";
const PROVIDER_ERROR: &str = "漫画源请求失败";
const SECRET_STORE_ERROR: &str = "安全凭据存储操作失败";
const CONFIG_STORAGE_ERROR: &str = "漫画源配置存储操作失败";

/// Command input may carry a one-time credential. It is written directly to
/// SecretStore and is never passed into registry metadata or command results.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ComicProviderConfigureRequest {
    Local {
        root: String,
    },
    Komga {
        #[serde(rename = "baseUrl")]
        base_url: String,
        #[serde(rename = "authMode")]
        auth_mode: ComicProviderAuthMode,
        username: Option<String>,
        secret: Option<String>,
    },
    Kavita {
        #[serde(rename = "baseUrl")]
        base_url: String,
        #[serde(rename = "authMode")]
        auth_mode: ComicProviderAuthMode,
        username: Option<String>,
        secret: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComicProviderCommandError {
    pub kind: ProviderErrorKind,
    pub message: String,
    pub retryable: bool,
    pub provider_id: Option<String>,
    pub operation: Option<String>,
}

#[tauri::command]
pub async fn comic_provider_configure(
    registry: State<'_, ComicProviderRegistry>,
    secrets: State<'_, SecretStore>,
    database: State<'_, Database>,
    request: ComicProviderConfigureRequest,
) -> Result<ComicProviderDescriptor, ComicProviderCommandError> {
    let (config, incoming_secret) = split_configure_request(request);
    config.validate().map_err(redact_provider_error)?;
    let (provider_kind, persisted_config) = persisted_config(&config);
    let origin = config.origin().map_err(redact_provider_error)?;
    let auth_mode = config.auth_mode();

    let credential = if auth_mode.requires_secret() {
        let origin = origin.ok_or_else(|| {
            command_error(
                ProviderErrorKind::PolicyBlocked,
                CONFIG_ERROR,
                false,
                None,
                Some("configure"),
            )
        })?;
        read_or_store_credential(secrets.inner().clone(), origin, incoming_secret).await?
    } else {
        if incoming_secret.is_some() {
            return Err(command_error(
                ProviderErrorKind::PolicyBlocked,
                CONFIG_ERROR,
                false,
                None,
                Some("configure"),
            ));
        }
        None
    };

    let descriptor = registry
        .configure(config, credential)
        .map_err(redact_provider_error)?;
    let upsert = ProviderConfigUpsert {
        provider_id: descriptor.id.clone(),
        resource_kind: ResourceKind::Comic,
        provider_kind,
        config_version: 1,
        config: persisted_config,
        enabled: true,
    };
    if persist_provider_config(database.inner(), upsert)
        .await
        .is_err()
    {
        let _ = registry.remove(&descriptor.id);
        return Err(config_storage_error());
    }
    Ok(descriptor)
}

#[tauri::command]
pub async fn comic_provider_list(
    registry: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
) -> Result<Vec<ComicProviderDescriptor>, ComicProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    registry.list().map_err(redact_provider_error)
}

#[tauri::command]
pub async fn comic_provider_remove(
    registry: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    provider_id: String,
) -> Result<bool, ComicProviderCommandError> {
    // Deliberately does not delete RuntimeConnectorToken. Credential lifecycle
    // remains an explicit SecretStore operation.
    let removed_persisted = delete_provider_config(database.inner(), provider_id.clone()).await?;
    let removed_runtime = registry
        .remove(&provider_id)
        .map_err(redact_provider_error)?;
    Ok(removed_persisted || removed_runtime)
}

#[tauri::command]
pub async fn comic_provider_probe(
    registry: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    provider_id: String,
) -> Result<ProbeDto, ComicProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.probe(&provider_id).await;
    persist_health(&registry, &database, &provider_id, "probe").await;
    result.map_err(redact_provider_error)
}

#[tauri::command]
pub async fn comic_provider_search(
    registry: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    provider_id: String,
    request: SearchRequest,
) -> Result<Vec<SeriesDto>, ComicProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.search(&provider_id, request).await;
    persist_health(&registry, &database, &provider_id, "search").await;
    result.map_err(redact_provider_error)
}

#[tauri::command]
pub async fn comic_provider_detail(
    registry: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    provider_id: String,
    series_id: String,
) -> Result<SeriesDetailDto, ComicProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.detail(&provider_id, series_id).await;
    persist_health(&registry, &database, &provider_id, "detail").await;
    result.map_err(redact_provider_error)
}

#[tauri::command]
pub async fn comic_provider_chapters(
    registry: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    provider_id: String,
    series_id: String,
) -> Result<Vec<ChapterDto>, ComicProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.chapters(&provider_id, series_id).await;
    persist_health(&registry, &database, &provider_id, "chapters").await;
    result.map_err(redact_provider_error)
}

#[tauri::command]
pub async fn comic_provider_resolve(
    registry: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    provider_id: String,
    request: ResolveRequest,
) -> Result<crate::domain::ResolvedTarget, ComicProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.resolve(&provider_id, request).await;
    persist_health(&registry, &database, &provider_id, "resolve").await;
    result.map_err(redact_provider_error)
}

fn persisted_config(config: &ComicProviderConfig) -> (String, serde_json::Value) {
    match config {
        ComicProviderConfig::Local { root } => {
            ("local".to_string(), serde_json::json!({ "root": root }))
        }
        ComicProviderConfig::Komga {
            base_url,
            auth_mode,
            username,
        } => (
            "komga".to_string(),
            serde_json::json!({
                "baseUrl": base_url,
                "authMode": auth_mode,
                "username": username,
            }),
        ),
        ComicProviderConfig::Kavita {
            base_url,
            auth_mode,
            username,
        } => (
            "kavita".to_string(),
            serde_json::json!({
                "baseUrl": base_url,
                "authMode": auth_mode,
                "username": username,
            }),
        ),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedRemoteConfig {
    base_url: String,
    auth_mode: ComicProviderAuthMode,
    username: Option<String>,
}

fn decode_persisted_config(
    provider_kind: &str,
    config: serde_json::Value,
) -> Result<ComicProviderConfig, ComicProviderCommandError> {
    match provider_kind {
        "local" => {
            let root = config
                .get("root")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(config_storage_error)?;
            Ok(ComicProviderConfig::Local {
                root: root.to_string(),
            })
        }
        "komga" | "kavita" => {
            let remote: PersistedRemoteConfig =
                serde_json::from_value(config).map_err(|_| config_storage_error())?;
            if provider_kind == "komga" {
                Ok(ComicProviderConfig::Komga {
                    base_url: remote.base_url,
                    auth_mode: remote.auth_mode,
                    username: remote.username,
                })
            } else {
                Ok(ComicProviderConfig::Kavita {
                    base_url: remote.base_url,
                    auth_mode: remote.auth_mode,
                    username: remote.username,
                })
            }
        }
        _ => Err(config_storage_error()),
    }
}

async fn persist_provider_config(
    database: &Database,
    config: ProviderConfigUpsert,
) -> Result<(), ()> {
    let sqlite = database.sqlite_arc();
    tauri::async_runtime::spawn_blocking(move || {
        ProviderConfigRepository::new(sqlite.as_ref())
            .upsert(&config)
            .map(|_| ())
            .map_err(|_| ())
    })
    .await
    .map_err(|_| ())?
}

async fn delete_provider_config(
    database: &Database,
    provider_id: String,
) -> Result<bool, ComicProviderCommandError> {
    let sqlite = database.sqlite_arc();
    tauri::async_runtime::spawn_blocking(move || {
        ProviderConfigRepository::new(sqlite.as_ref()).delete(&provider_id, ResourceKind::Comic)
    })
    .await
    .map_err(|_| config_storage_error())?
    .map_err(|_| config_storage_error())
}

async fn restore_persisted_configs(
    registry: &ComicProviderRegistry,
    database: &Database,
    secrets: &SecretStore,
) -> Result<(), ComicProviderCommandError> {
    let configured = registry
        .list()
        .map_err(redact_provider_error)?
        .into_iter()
        .map(|provider| provider.id)
        .collect::<std::collections::BTreeSet<_>>();
    let sqlite = database.sqlite_arc();
    let records = tauri::async_runtime::spawn_blocking(move || {
        ProviderConfigRepository::new(sqlite.as_ref())
            .list_enabled_for_resource_kind(ResourceKind::Comic)
    })
    .await
    .map_err(|_| config_storage_error())?
    .map_err(|_| config_storage_error())?;

    for record in records {
        if configured.contains(&record.provider_id) {
            continue;
        }
        if record.config_version != 1 {
            tracing::warn!(provider_id = %record.provider_id, "unsupported persisted comic provider config version");
            continue;
        }
        let Ok(config) = decode_persisted_config(&record.provider_kind, record.config) else {
            tracing::warn!(provider_id = %record.provider_id, "persisted comic provider config is invalid");
            continue;
        };
        let origin = config.origin().map_err(redact_provider_error)?;
        let credential = if config.auth_mode().requires_secret() {
            let Some(origin) = origin else { continue };
            let store = secrets.clone();
            tauri::async_runtime::spawn_blocking(move || {
                store.get(SecretKind::RuntimeConnectorToken, Some(&origin))
            })
            .await
            .map_err(|_| secret_store_error())?
            .map_err(|_| secret_store_error())?
        } else {
            None
        };
        if config.auth_mode().requires_secret() && credential.is_none() {
            tracing::warn!(provider_id = %record.provider_id, "persisted comic provider credential is unavailable");
            continue;
        }
        match registry.configure(config, credential) {
            Ok(descriptor) if descriptor.id == record.provider_id => {}
            Ok(descriptor) => {
                let _ = registry.remove(&descriptor.id);
                tracing::warn!(provider_id = %record.provider_id, "persisted comic provider id mismatch");
            }
            Err(_) => {
                tracing::warn!(provider_id = %record.provider_id, "persisted comic provider could not be restored")
            }
        }
    }
    Ok(())
}

fn split_configure_request(
    request: ComicProviderConfigureRequest,
) -> (ComicProviderConfig, Option<String>) {
    match request {
        ComicProviderConfigureRequest::Local { root } => {
            (ComicProviderConfig::Local { root }, None)
        }
        ComicProviderConfigureRequest::Komga {
            base_url,
            auth_mode,
            username,
            secret,
        } => (
            ComicProviderConfig::Komga {
                base_url,
                auth_mode,
                username,
            },
            secret,
        ),
        ComicProviderConfigureRequest::Kavita {
            base_url,
            auth_mode,
            username,
            secret,
        } => (
            ComicProviderConfig::Kavita {
                base_url,
                auth_mode,
                username,
            },
            secret,
        ),
    }
}

async fn read_or_store_credential(
    store: SecretStore,
    origin: String,
    incoming_secret: Option<String>,
) -> Result<Option<String>, ComicProviderCommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(secret) = incoming_secret {
            store
                .set(SecretKind::RuntimeConnectorToken, Some(&origin), &secret)
                .map_err(|_| secret_store_error())?;
        }
        store
            .get(SecretKind::RuntimeConnectorToken, Some(&origin))
            .map_err(|_| secret_store_error())?
            .ok_or_else(|| {
                command_error(
                    ProviderErrorKind::AuthRequired,
                    AUTH_ERROR,
                    false,
                    None,
                    Some("configure"),
                )
            })
            .map(Some)
    })
    .await
    .map_err(|_| secret_store_error())?
}

async fn persist_health(
    registry: &ComicProviderRegistry,
    database: &Database,
    provider_id: &str,
    operation: &str,
) {
    let Ok(snapshot) = registry.health(provider_id, operation) else {
        return;
    };
    let sqlite = database.sqlite_arc();
    let persisted = tauri::async_runtime::spawn_blocking(move || {
        ProviderHealthRepository::new(sqlite.as_ref()).upsert(&snapshot)
    })
    .await;
    if !matches!(persisted, Ok(Ok(()))) {
        tracing::warn!(
            provider_id,
            operation,
            "comic provider health snapshot persistence failed"
        );
    }
}

fn redact_provider_error(error: ComicProviderError) -> ComicProviderCommandError {
    match error {
        ComicProviderError::InvalidConfig(_) => command_error(
            ProviderErrorKind::PolicyBlocked,
            CONFIG_ERROR,
            false,
            None,
            Some("configure"),
        ),
        ComicProviderError::Security(_) => command_error(
            ProviderErrorKind::PolicyBlocked,
            SECURITY_ERROR,
            false,
            None,
            Some("configure"),
        ),
        ComicProviderError::CircuitOpen(_) => {
            command_error(ProviderErrorKind::Network, NETWORK_ERROR, true, None, None)
        }
        ComicProviderError::Provider(provider) => redact_domain_error(provider),
    }
}

fn redact_domain_error(provider: ProviderError) -> ComicProviderCommandError {
    let message = match provider.kind {
        ProviderErrorKind::AuthRequired => AUTH_ERROR,
        ProviderErrorKind::Network
        | ProviderErrorKind::Timeout
        | ProviderErrorKind::RateLimited => NETWORK_ERROR,
        ProviderErrorKind::PolicyBlocked => SECURITY_ERROR,
        _ => PROVIDER_ERROR,
    };
    command_error(
        provider.kind,
        message,
        provider.retryable,
        provider.provider_id.as_deref(),
        provider.operation.as_deref(),
    )
}

fn config_storage_error() -> ComicProviderCommandError {
    command_error(
        ProviderErrorKind::Unknown,
        CONFIG_STORAGE_ERROR,
        true,
        None,
        Some("configure"),
    )
}

fn secret_store_error() -> ComicProviderCommandError {
    command_error(
        ProviderErrorKind::Unknown,
        SECRET_STORE_ERROR,
        false,
        None,
        Some("configure"),
    )
}

fn command_error(
    kind: ProviderErrorKind,
    message: &str,
    retryable: bool,
    provider_id: Option<&str>,
    operation: Option<&str>,
) -> ComicProviderCommandError {
    ComicProviderCommandError {
        kind,
        message: message.to_string(),
        retryable,
        provider_id: provider_id.map(str::to_string),
        operation: operation.map(str::to_string),
    }
}

#[cfg(test)]
mod persisted_config_tests {
    use super::*;

    #[test]
    fn persisted_remote_config_roundtrips_without_credentials() {
        let config = ComicProviderConfig::Komga {
            base_url: "https://komga.example.test".to_string(),
            auth_mode: ComicProviderAuthMode::Bearer,
            username: None,
        };
        let (kind, value) = persisted_config(&config);
        let wire = value.to_string();
        assert_eq!(kind, "komga");
        assert!(!wire.contains("sentinel-secret"));
        assert!(!wire.to_ascii_lowercase().contains("token"));
        assert_eq!(decode_persisted_config(&kind, value).unwrap(), config);
    }

    #[test]
    fn persisted_local_config_roundtrips_the_root_only() {
        let config = ComicProviderConfig::Local {
            root: "C:/Comics".to_string(),
        };
        let (kind, value) = persisted_config(&config);
        assert_eq!(decode_persisted_config(&kind, value).unwrap(), config);
    }

    #[tokio::test]
    async fn persisted_local_provider_restores_after_registry_restart() {
        let root =
            std::env::temp_dir().join(format!("moeplay-comic-persist-{}", uuid::Uuid::new_v4()));
        let chapter = root.join("Series").join("Chapter 01");
        std::fs::create_dir_all(&chapter).unwrap();
        std::fs::write(chapter.join("001.jpg"), b"fixture").unwrap();
        let config = ComicProviderConfig::Local {
            root: root.to_string_lossy().into_owned(),
        };
        let registry = ComicProviderRegistry::new();
        let descriptor = registry.configure(config.clone(), None).unwrap();
        let database = Database::open_at(root.join("data")).unwrap();
        let (provider_kind, config_json) = persisted_config(&config);
        ProviderConfigRepository::new(database.sqlite())
            .upsert(&ProviderConfigUpsert {
                provider_id: descriptor.id.clone(),
                resource_kind: ResourceKind::Comic,
                provider_kind,
                config_version: 1,
                config: config_json,
                enabled: true,
            })
            .unwrap();
        let restarted = ComicProviderRegistry::new();
        restore_persisted_configs(&restarted, &database, &SecretStore::new())
            .await
            .unwrap();
        assert_eq!(restarted.list().unwrap()[0].id, descriptor.id);
        let _ = std::fs::remove_dir_all(root);
    }
}
