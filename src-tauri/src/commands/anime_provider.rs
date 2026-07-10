use crate::db::Database;
use crate::db_sqlite::repositories::{
    ProviderConfigRepository, ProviderConfigUpsert, ProviderHealthRepository,
};
use crate::domain::{
    ProviderError, ProviderErrorKind, ProviderHealth, ResolvedTarget, ResourceKind,
};
use crate::providers::anime::{
    protect_hls_target, scan_local_media_directory, AnimeDetail, AnimeEpisode,
    AnimeLocalMediaScanResult, AnimeLocalMediaSeries, AnimeProviderConfig, AnimeProviderDescriptor,
    AnimeProviderRegistry, AnimeResolveRequest, AnimeResolveResponse, AnimeSearchQuery,
    AnimeSearchResponse,
};
use crate::secret_store::{SecretKind, SecretStore};
use serde::{Deserialize, Serialize};
use tauri::{State, WebviewUrl, WebviewWindowBuilder};

const CONFIG_ERROR: &str = "anime provider configuration is invalid";
const SECURITY_ERROR: &str = "anime provider configuration was rejected by the security policy";
const AUTH_ERROR: &str = "anime provider credential is missing or invalid";
const NETWORK_ERROR: &str = "anime provider network request failed";
const PROVIDER_ERROR: &str = "anime provider request failed";
const SECRET_STORE_ERROR: &str = "secure credential storage operation failed";
const CONFIG_STORAGE_ERROR: &str = "anime provider configuration storage operation failed";

/// A one-time Jellyfin token may be supplied during configuration. It is stored
/// under `RuntimeConnectorToken` by origin, immediately read back for adapter
/// construction, and never returned by a command or descriptor.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnimeProviderConfigureRequest {
    LocalMedia {
        library: Vec<AnimeLocalMediaSeries>,
        #[serde(rename = "allowedPaths")]
        allowed_paths: Vec<String>,
    },
    Jellyfin {
        #[serde(rename = "baseUrl")]
        base_url: String,
        token: Option<String>,
    },
}

/// Error DTO intentionally mirrors the provider-error frontend contract while
/// replacing connector and filesystem details with stable safe messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeProviderCommandError {
    pub kind: ProviderErrorKind,
    pub message: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
    pub provider_id: Option<String>,
    pub operation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeProviderSearchCommandResponse {
    pub items: Vec<crate::providers::anime::AnimeSearchItem>,
    pub failures: Vec<AnimeProviderCommandError>,
    pub provider_health: Vec<ProviderHealth>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeProviderFallbackOpenResponse {
    pub mode: String,
}

#[tauri::command]
pub async fn anime_provider_configure(
    registry: State<'_, AnimeProviderRegistry>,
    secrets: State<'_, SecretStore>,
    database: State<'_, Database>,
    request: AnimeProviderConfigureRequest,
) -> Result<AnimeProviderDescriptor, AnimeProviderCommandError> {
    let (config, incoming_token) = split_configure_request(request);
    let (provider_kind, persisted_config) = persisted_config(&config);
    let credential = match config.kind() {
        crate::providers::anime::AnimeProviderKind::LocalMedia => {
            if incoming_token.is_some() {
                return Err(command_error(
                    ProviderErrorKind::PolicyBlocked,
                    CONFIG_ERROR,
                    false,
                    None,
                    Some("configure"),
                    None,
                ));
            }
            None
        }
        crate::providers::anime::AnimeProviderKind::Jellyfin => {
            let origin = config
                .origin()
                .map_err(redact_provider_error)?
                .ok_or_else(|| {
                    command_error(
                        ProviderErrorKind::PolicyBlocked,
                        CONFIG_ERROR,
                        false,
                        None,
                        Some("configure"),
                        None,
                    )
                })?;
            Some(
                read_or_store_runtime_token(secrets.inner().clone(), origin, incoming_token)
                    .await?,
            )
        }
    };

    let descriptor = registry
        .configure(config, credential)
        .map_err(redact_provider_error)?;
    let upsert = ProviderConfigUpsert {
        provider_id: descriptor.id.clone(),
        resource_kind: ResourceKind::Anime,
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
pub async fn anime_provider_list(
    registry: State<'_, AnimeProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
) -> Result<Vec<AnimeProviderDescriptor>, AnimeProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    registry.list().map_err(redact_provider_error)
}

#[tauri::command]
pub async fn anime_provider_remove(
    registry: State<'_, AnimeProviderRegistry>,
    secrets: State<'_, SecretStore>,
    database: State<'_, Database>,
    provider_id: String,
) -> Result<bool, AnimeProviderCommandError> {
    let (removed_persisted, persisted_origin) =
        remove_persisted_config(database.inner(), provider_id.clone()).await?;
    let runtime_origin = registry
        .list()
        .map_err(redact_provider_error)?
        .into_iter()
        .find(|provider| provider.id == provider_id)
        .and_then(|provider| provider.origin);
    let removed_runtime = registry
        .remove(&provider_id)
        .map_err(redact_provider_error)?;
    if let Some(origin) = runtime_origin.or(persisted_origin) {
        let store = secrets.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            store.delete(SecretKind::RuntimeConnectorToken, Some(&origin))
        })
        .await
        .map_err(|_| secret_store_error())?
        .map_err(|_| secret_store_error())?;
    }
    Ok(removed_persisted || removed_runtime)
}

#[tauri::command]
pub async fn anime_provider_search(
    registry: State<'_, AnimeProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    query: AnimeSearchQuery,
    provider_id: Option<String>,
) -> Result<AnimeProviderSearchCommandResponse, AnimeProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let response = match provider_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(provider_id) => registry.search_provider(provider_id, query).await,
        None => registry.search(query).await,
    };
    persist_health(registry.inner(), database.inner()).await;
    Ok(sanitize_search_response(response))
}

#[tauri::command]
pub async fn anime_provider_detail(
    registry: State<'_, AnimeProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    provider_id: String,
    item_id: String,
) -> Result<AnimeDetail, AnimeProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.detail(&provider_id, &item_id).await;
    persist_health(registry.inner(), database.inner()).await;
    result.map_err(redact_provider_error)
}

#[tauri::command]
pub async fn anime_provider_episodes(
    registry: State<'_, AnimeProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    provider_id: String,
    series_id: String,
) -> Result<Vec<AnimeEpisode>, AnimeProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.episodes(&provider_id, &series_id).await;
    persist_health(registry.inner(), database.inner()).await;
    result.map_err(redact_provider_error)
}

#[tauri::command]
pub async fn anime_provider_resolve(
    registry: State<'_, AnimeProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    request: AnimeResolveRequest,
) -> Result<AnimeResolveResponse, AnimeProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.resolve(request).await;
    persist_health(registry.inner(), database.inner()).await;
    let mut response = result.map_err(redact_provider_error)?;
    if let ResolvedTarget::NativeHls { url, headers } = response.target {
        response.target = ResolvedTarget::NativeHls {
            url: protect_hls_target(url, headers).map_err(redact_provider_error)?,
            headers: Vec::new(),
        };
    }
    Ok(response)
}

#[tauri::command]
pub fn anime_provider_pick_local_directory(
) -> Result<Option<AnimeLocalMediaScanResult>, AnimeProviderCommandError> {
    let Some(directory) = rfd::FileDialog::new()
        .set_title("选择番剧媒体目录")
        .pick_folder()
    else {
        return Ok(None);
    };
    scan_local_media_directory(&directory)
        .map(Some)
        .map_err(redact_provider_error)
}

#[tauri::command]
pub async fn anime_provider_open_fallback(
    app: tauri::AppHandle,
    registry: State<'_, AnimeProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
    request: AnimeResolveRequest,
) -> Result<AnimeProviderFallbackOpenResponse, AnimeProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let response = registry
        .resolve(request)
        .await
        .map_err(redact_provider_error)?;
    persist_health(registry.inner(), database.inner()).await;
    match response.target {
        ResolvedTarget::NativeFile { path } => {
            open::that(path).map_err(|_| fallback_error("native_file"))?;
            Ok(AnimeProviderFallbackOpenResponse {
                mode: "native_file".to_string(),
            })
        }
        ResolvedTarget::Webview { url, allowed_hosts } => {
            let parsed = validate_fallback_url(&url, Some(&allowed_hosts))?;
            let label = format!("anime-provider-{}", uuid::Uuid::new_v4().simple());
            WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed))
                .title("番剧播放")
                .inner_size(1100.0, 760.0)
                .min_inner_size(720.0, 520.0)
                .resizable(true)
                .center()
                .build()
                .map_err(|_| fallback_error("webview"))?;
            Ok(AnimeProviderFallbackOpenResponse {
                mode: "webview".to_string(),
            })
        }
        ResolvedTarget::External { url, .. } => {
            let parsed = validate_fallback_url(&url, None)?;
            open::that(parsed.as_str()).map_err(|_| fallback_error("external"))?;
            Ok(AnimeProviderFallbackOpenResponse {
                mode: "external".to_string(),
            })
        }
        _ => Err(fallback_error("unsupported")),
    }
}

#[tauri::command]
pub async fn anime_provider_health(
    registry: State<'_, AnimeProviderRegistry>,
    database: State<'_, Database>,
    secrets: State<'_, SecretStore>,
) -> Result<Vec<ProviderHealth>, AnimeProviderCommandError> {
    restore_persisted_configs(registry.inner(), database.inner(), secrets.inner()).await?;
    let result = registry.health().map_err(redact_provider_error)?;
    persist_health(registry.inner(), database.inner()).await;
    Ok(result)
}

fn persisted_config(config: &AnimeProviderConfig) -> (String, serde_json::Value) {
    match config {
        AnimeProviderConfig::LocalMedia {
            library,
            allowed_paths,
        } => (
            "local_media".to_string(),
            serde_json::json!({
                "library": library,
                "allowedPaths": allowed_paths,
            }),
        ),
        AnimeProviderConfig::Jellyfin { base_url } => (
            "jellyfin".to_string(),
            serde_json::json!({ "baseUrl": base_url }),
        ),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedLocalMediaConfig {
    library: Vec<AnimeLocalMediaSeries>,
    allowed_paths: Vec<String>,
}

fn decode_persisted_config(
    provider_kind: &str,
    config: serde_json::Value,
) -> Result<AnimeProviderConfig, AnimeProviderCommandError> {
    match provider_kind {
        "local_media" => {
            let local: PersistedLocalMediaConfig =
                serde_json::from_value(config).map_err(|_| config_storage_error())?;
            Ok(AnimeProviderConfig::LocalMedia {
                library: local.library,
                allowed_paths: local.allowed_paths,
            })
        }
        "jellyfin" => {
            let base_url = config
                .get("baseUrl")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(config_storage_error)?;
            Ok(AnimeProviderConfig::Jellyfin {
                base_url: base_url.to_string(),
            })
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

async fn remove_persisted_config(
    database: &Database,
    provider_id: String,
) -> Result<(bool, Option<String>), AnimeProviderCommandError> {
    let sqlite = database.sqlite_arc();
    tauri::async_runtime::spawn_blocking(move || {
        let repository = ProviderConfigRepository::new(sqlite.as_ref());
        let record = repository
            .get(&provider_id, ResourceKind::Anime)
            .map_err(|_| ())?;
        let origin = record
            .as_ref()
            .and_then(|record| {
                decode_persisted_config(&record.provider_kind, record.config.clone()).ok()
            })
            .and_then(|config| config.origin().ok().flatten());
        let removed = repository
            .delete(&provider_id, ResourceKind::Anime)
            .map_err(|_| ())?;
        Ok::<_, ()>((removed, origin))
    })
    .await
    .map_err(|_| config_storage_error())?
    .map_err(|_| config_storage_error())
}

async fn restore_persisted_configs(
    registry: &AnimeProviderRegistry,
    database: &Database,
    secrets: &SecretStore,
) -> Result<(), AnimeProviderCommandError> {
    let configured = registry
        .list()
        .map_err(redact_provider_error)?
        .into_iter()
        .map(|provider| provider.id)
        .collect::<std::collections::BTreeSet<_>>();
    let sqlite = database.sqlite_arc();
    let records = tauri::async_runtime::spawn_blocking(move || {
        ProviderConfigRepository::new(sqlite.as_ref())
            .list_enabled_for_resource_kind(ResourceKind::Anime)
    })
    .await
    .map_err(|_| config_storage_error())?
    .map_err(|_| config_storage_error())?;

    for record in records {
        if configured.contains(&record.provider_id) {
            continue;
        }
        if record.config_version != 1 {
            tracing::warn!(provider_id = %record.provider_id, "unsupported persisted anime provider config version");
            continue;
        }
        let Ok(config) = decode_persisted_config(&record.provider_kind, record.config) else {
            tracing::warn!(provider_id = %record.provider_id, "persisted anime provider config is invalid");
            continue;
        };
        let credential = match config.kind() {
            crate::providers::anime::AnimeProviderKind::LocalMedia => None,
            crate::providers::anime::AnimeProviderKind::Jellyfin => {
                let Some(origin) = config.origin().map_err(redact_provider_error)? else {
                    continue;
                };
                let store = secrets.clone();
                let token = tauri::async_runtime::spawn_blocking(move || {
                    store.get(SecretKind::RuntimeConnectorToken, Some(&origin))
                })
                .await
                .map_err(|_| secret_store_error())?
                .map_err(|_| secret_store_error())?;
                let Some(token) = token else {
                    tracing::warn!(provider_id = %record.provider_id, "persisted anime provider credential is unavailable");
                    continue;
                };
                Some(token)
            }
        };
        match registry.configure(config, credential) {
            Ok(descriptor) if descriptor.id == record.provider_id => {}
            Ok(descriptor) => {
                let _ = registry.remove(&descriptor.id);
                tracing::warn!(provider_id = %record.provider_id, "persisted anime provider id mismatch");
            }
            Err(_) => {
                tracing::warn!(provider_id = %record.provider_id, "persisted anime provider could not be restored")
            }
        }
    }
    Ok(())
}

fn validate_fallback_url(
    value: &str,
    allowed_hosts: Option<&[String]>,
) -> Result<url::Url, AnimeProviderCommandError> {
    let parsed = url::Url::parse(value).map_err(|_| fallback_error("url"))?;
    let localhost_http = parsed.scheme() == "http"
        && parsed
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("localhost"));
    if parsed.scheme() != "https" && !localhost_http {
        return Err(fallback_error("url"));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() || parsed.fragment().is_some() {
        return Err(fallback_error("url"));
    }
    if let Some(allowed_hosts) = allowed_hosts {
        let host = parsed.host_str().unwrap_or_default();
        let origin = parsed.origin().ascii_serialization();
        let allowed = allowed_hosts.iter().any(|allowed| {
            allowed.eq_ignore_ascii_case(host) || allowed.eq_ignore_ascii_case(&origin)
        });
        if !allowed {
            return Err(fallback_error("host"));
        }
    }
    Ok(parsed)
}

fn fallback_error(operation: &str) -> AnimeProviderCommandError {
    command_error(
        ProviderErrorKind::PolicyBlocked,
        SECURITY_ERROR,
        false,
        None,
        Some(operation),
        None,
    )
}

fn split_configure_request(
    request: AnimeProviderConfigureRequest,
) -> (AnimeProviderConfig, Option<String>) {
    match request {
        AnimeProviderConfigureRequest::LocalMedia {
            library,
            allowed_paths,
        } => (
            AnimeProviderConfig::LocalMedia {
                library,
                allowed_paths,
            },
            None,
        ),
        AnimeProviderConfigureRequest::Jellyfin { base_url, token } => {
            (AnimeProviderConfig::Jellyfin { base_url }, token)
        }
    }
}

async fn read_or_store_runtime_token(
    store: SecretStore,
    origin: String,
    incoming_token: Option<String>,
) -> Result<String, AnimeProviderCommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(token) = incoming_token {
            store
                .set(SecretKind::RuntimeConnectorToken, Some(&origin), &token)
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
                    None,
                )
            })
    })
    .await
    .map_err(|_| secret_store_error())?
}

async fn persist_health(registry: &AnimeProviderRegistry, database: &Database) {
    let Ok(snapshots) = registry.health() else {
        return;
    };
    if snapshots.is_empty() {
        return;
    }
    let sqlite = database.sqlite_arc();
    let persisted = tauri::async_runtime::spawn_blocking(move || {
        let repository = ProviderHealthRepository::new(sqlite.as_ref());
        for snapshot in snapshots {
            repository.upsert(&snapshot)?;
        }
        Ok::<(), String>(())
    })
    .await;
    if !matches!(persisted, Ok(Ok(()))) {
        tracing::warn!("anime provider health snapshot persistence failed");
    }
}

fn sanitize_search_response(response: AnimeSearchResponse) -> AnimeProviderSearchCommandResponse {
    AnimeProviderSearchCommandResponse {
        items: response.items,
        failures: response
            .failures
            .into_iter()
            .map(redact_provider_error)
            .collect(),
        provider_health: response.provider_health,
    }
}

fn redact_provider_error(error: ProviderError) -> AnimeProviderCommandError {
    let message = match error.kind {
        ProviderErrorKind::AuthRequired => AUTH_ERROR,
        ProviderErrorKind::Network
        | ProviderErrorKind::Timeout
        | ProviderErrorKind::RateLimited => NETWORK_ERROR,
        ProviderErrorKind::PolicyBlocked => SECURITY_ERROR,
        ProviderErrorKind::Cancelled => "anime provider request was cancelled",
        _ => PROVIDER_ERROR,
    };
    command_error(
        error.kind,
        message,
        error.retryable,
        error.retry_after_ms,
        error.operation.as_deref(),
        error.provider_id.as_deref(),
    )
}

fn config_storage_error() -> AnimeProviderCommandError {
    command_error(
        ProviderErrorKind::Unknown,
        CONFIG_STORAGE_ERROR,
        true,
        None,
        Some("configure"),
        None,
    )
}

fn secret_store_error() -> AnimeProviderCommandError {
    command_error(
        ProviderErrorKind::Unknown,
        SECRET_STORE_ERROR,
        false,
        None,
        Some("configure"),
        None,
    )
}

fn command_error(
    kind: ProviderErrorKind,
    message: &str,
    retryable: bool,
    retry_after_ms: Option<u64>,
    operation: Option<&str>,
    provider_id: Option<&str>,
) -> AnimeProviderCommandError {
    AnimeProviderCommandError {
        kind,
        message: message.to_string(),
        retryable,
        retry_after_ms,
        provider_id: provider_id.map(str::to_string),
        operation: operation.map(str::to_string),
    }
}

#[cfg(test)]
mod persisted_config_tests {
    use super::*;

    #[test]
    fn persisted_jellyfin_config_never_contains_a_token() {
        let config = AnimeProviderConfig::Jellyfin {
            base_url: "https://jellyfin.example.test".to_string(),
        };
        let (kind, value) = persisted_config(&config);
        assert_eq!(kind, "jellyfin");
        assert!(!value.to_string().to_ascii_lowercase().contains("token"));
        assert_eq!(decode_persisted_config(&kind, value).unwrap(), config);
    }

    #[tokio::test]
    async fn persisted_local_provider_restores_after_registry_restart() {
        let root =
            std::env::temp_dir().join(format!("moeplay-anime-persist-{}", uuid::Uuid::new_v4()));
        let media = root.join("Series");
        std::fs::create_dir_all(&media).unwrap();
        std::fs::write(media.join("Episode 01.mp4"), b"fixture").unwrap();
        let scan = scan_local_media_directory(&root).unwrap();
        let config = AnimeProviderConfig::LocalMedia {
            library: scan.library,
            allowed_paths: scan.allowed_paths,
        };
        let registry = AnimeProviderRegistry::default();
        let descriptor = registry.configure(config.clone(), None).unwrap();
        let data_dir = root.join("data");
        let database = Database::open_at(&data_dir).unwrap();
        let (provider_kind, config_json) = persisted_config(&config);
        ProviderConfigRepository::new(database.sqlite())
            .upsert(&ProviderConfigUpsert {
                provider_id: descriptor.id.clone(),
                resource_kind: ResourceKind::Anime,
                provider_kind,
                config_version: 1,
                config: config_json,
                enabled: true,
            })
            .unwrap();
        let restarted = AnimeProviderRegistry::default();
        restore_persisted_configs(&restarted, &database, &SecretStore::new())
            .await
            .unwrap();
        assert_eq!(restarted.list().unwrap()[0].id, descriptor.id);
        let _ = std::fs::remove_dir_all(root);
    }
}
