use super::{
    provider_error, validate_jellyfin_base_url, AnimeDetail, AnimeEpisode,
    AnimeProviderOrchestrator, AnimeResolveRequest, AnimeResolveResponse, AnimeSearchQuery,
    AnimeSearchResponse, AnimeSourceAdapter, CircuitBreakerConfig, JellyfinConfig,
    JellyfinConnector, LocalMediaAdapter, LocalMediaEpisode, LocalMediaSeries, ProviderResult,
};
use crate::domain::{ProviderErrorKind, ProviderHealth, ProviderManifest};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use url::Url;

/// Supported sources are deliberately finite. A source is only available after
/// it has been explicitly configured through the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimeProviderKind {
    LocalMedia,
    Jellyfin,
}

/// Serializable local-media file entry. The registry converts this boundary
/// type into the adapter's internal media type only after path authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeLocalMediaEpisode {
    pub id: String,
    pub title: String,
    pub number: Option<u32>,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeLocalMediaSeries {
    pub id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub synopsis: Option<String>,
    pub artwork_url: Option<String>,
    pub genres: Vec<String>,
    pub episodes: Vec<AnimeLocalMediaEpisode>,
}

/// Non-secret source configuration. Jellyfin credentials are supplied out of
/// band and are never persisted in this value or returned by `list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimeProviderConfig {
    LocalMedia {
        library: Vec<AnimeLocalMediaSeries>,
        allowed_paths: Vec<String>,
    },
    Jellyfin {
        base_url: String,
    },
}

impl AnimeProviderConfig {
    pub fn kind(&self) -> AnimeProviderKind {
        match self {
            Self::LocalMedia { .. } => AnimeProviderKind::LocalMedia,
            Self::Jellyfin { .. } => AnimeProviderKind::Jellyfin,
        }
    }

    /// Canonical HTTP origin used to scope the RuntimeConnectorToken secret.
    pub fn origin(&self) -> ProviderResult<Option<String>> {
        match self {
            Self::LocalMedia { .. } => Ok(None),
            Self::Jellyfin { base_url } => Ok(Some(jellyfin_origin(base_url)?)),
        }
    }
}

/// Frontend-safe configured-source metadata. In particular, no credential,
/// authorization header, or local file list is exposed here.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeProviderDescriptor {
    pub id: String,
    pub kind: AnimeProviderKind,
    pub name: String,
    pub local_file_count: Option<usize>,
    pub allowed_paths: Option<Vec<String>>,
    pub base_url: Option<String>,
    pub origin: Option<String>,
    pub secret_configured: bool,
    pub manifest: ProviderManifest,
}

struct RegistryEntry {
    adapter: Arc<dyn AnimeSourceAdapter>,
    descriptor: AnimeProviderDescriptor,
}

struct RegistryState {
    entries: BTreeMap<String, RegistryEntry>,
    orchestrator: Arc<AnimeProviderOrchestrator>,
}

impl Default for RegistryState {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            orchestrator: Arc::new(AnimeProviderOrchestrator::new(
                Vec::new(),
                CircuitBreakerConfig::default(),
            )),
        }
    }
}

/// Thread-safe runtime registry for real Tauri Anime Provider commands.
///
/// Configuration rebuilds the orchestrator atomically. In-flight requests keep
/// their `Arc` snapshot, while future requests see the new source set.
pub struct AnimeProviderRegistry {
    state: RwLock<RegistryState>,
    circuit_breaker: CircuitBreakerConfig,
}

impl Default for AnimeProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimeProviderRegistry {
    pub fn new() -> Self {
        Self::with_circuit_breaker(CircuitBreakerConfig::default())
    }

    pub fn with_circuit_breaker(circuit_breaker: CircuitBreakerConfig) -> Self {
        Self {
            state: RwLock::new(RegistryState::default()),
            circuit_breaker,
        }
    }

    /// Build an adapter first, then atomically replace the source having the
    /// same stable provider ID. `credential` is consumed by Jellyfin only and
    /// never copied into descriptor metadata.
    pub fn configure(
        &self,
        config: AnimeProviderConfig,
        credential: Option<String>,
    ) -> ProviderResult<AnimeProviderDescriptor> {
        let (adapter, descriptor) = self.build_entry(config, credential)?;
        let mut state = self
            .state
            .write()
            .map_err(|_| registry_error("configure"))?;
        state.entries.insert(
            descriptor.id.clone(),
            RegistryEntry {
                adapter,
                descriptor: descriptor.clone(),
            },
        );
        rebuild_orchestrator(&mut state, self.circuit_breaker);
        Ok(descriptor)
    }

    pub fn list(&self) -> ProviderResult<Vec<AnimeProviderDescriptor>> {
        let state = self.state.read().map_err(|_| registry_error("list"))?;
        Ok(state
            .entries
            .values()
            .map(|entry| entry.descriptor.clone())
            .collect())
    }

    /// Removing a source does not remove its RuntimeConnectorToken; credential
    /// lifecycle is intentionally an explicit SecretStore operation.
    pub fn remove(&self, provider_id: &str) -> ProviderResult<bool> {
        let mut state = self.state.write().map_err(|_| registry_error("remove"))?;
        let removed = state.entries.remove(provider_id).is_some();
        if removed {
            rebuild_orchestrator(&mut state, self.circuit_breaker);
        }
        Ok(removed)
    }

    pub fn health(&self) -> ProviderResult<Vec<ProviderHealth>> {
        Ok(self.orchestrator()?.health())
    }

    /// Clears all ephemeral circuit-breaker evidence for a configured source.
    /// Source Center also deletes the persisted health projection in the same
    /// command, keeping reset semantics truthful across app layers.
    pub fn reset_health(&self, provider_id: &str) -> ProviderResult<bool> {
        Ok(self.orchestrator()?.reset_provider_health(provider_id))
    }

    pub async fn search(&self, query: AnimeSearchQuery) -> AnimeSearchResponse {
        let orchestrator = match self.orchestrator() {
            Ok(orchestrator) => orchestrator,
            Err(failure) => {
                return AnimeSearchResponse {
                    items: Vec::new(),
                    failures: vec![failure],
                    provider_health: Vec::new(),
                };
            }
        };
        orchestrator.search(query).await
    }

    pub async fn search_provider(
        &self,
        provider_id: &str,
        query: AnimeSearchQuery,
    ) -> AnimeSearchResponse {
        match self.orchestrator() {
            Ok(orchestrator) => orchestrator.search_provider(provider_id, query).await,
            Err(failure) => AnimeSearchResponse {
                items: Vec::new(),
                failures: vec![failure],
                provider_health: Vec::new(),
            },
        }
    }

    pub async fn detail(&self, provider_id: &str, item_id: &str) -> ProviderResult<AnimeDetail> {
        self.orchestrator()?.detail(provider_id, item_id).await
    }

    pub async fn episodes(
        &self,
        provider_id: &str,
        series_id: &str,
    ) -> ProviderResult<Vec<AnimeEpisode>> {
        self.orchestrator()?.episodes(provider_id, series_id).await
    }

    pub async fn resolve(
        &self,
        request: AnimeResolveRequest,
    ) -> ProviderResult<AnimeResolveResponse> {
        self.orchestrator()?.resolve(request).await
    }

    fn orchestrator(&self) -> ProviderResult<Arc<AnimeProviderOrchestrator>> {
        let state = self
            .state
            .read()
            .map_err(|_| registry_error("select_provider"))?;
        Ok(Arc::clone(&state.orchestrator))
    }

    fn build_entry(
        &self,
        config: AnimeProviderConfig,
        credential: Option<String>,
    ) -> ProviderResult<(Arc<dyn AnimeSourceAdapter>, AnimeProviderDescriptor)> {
        match config {
            AnimeProviderConfig::LocalMedia {
                library,
                allowed_paths,
            } => {
                if credential.is_some() {
                    return Err(config_error(
                        "local media configuration must not include a credential",
                    ));
                }
                let (library, canonical_allowed_paths) =
                    validate_local_library(library, allowed_paths)?;
                let local_file_count = library.iter().map(|series| series.episodes.len()).sum();
                let adapter: Arc<dyn AnimeSourceAdapter> =
                    Arc::new(LocalMediaAdapter::try_new(library)?);
                let manifest = adapter.manifest();
                let descriptor = AnimeProviderDescriptor {
                    id: manifest.id.clone(),
                    kind: AnimeProviderKind::LocalMedia,
                    name: manifest.name.clone(),
                    local_file_count: Some(local_file_count),
                    allowed_paths: Some(canonical_allowed_paths),
                    base_url: None,
                    origin: None,
                    secret_configured: false,
                    manifest,
                };
                Ok((adapter, descriptor))
            }
            AnimeProviderConfig::Jellyfin { base_url } => {
                let access_token = credential.ok_or_else(|| {
                    provider_error(
                        "jellyfin",
                        "configure",
                        ProviderErrorKind::AuthRequired,
                        "Jellyfin credential is required",
                        false,
                    )
                })?;
                let origin = jellyfin_origin(&base_url)?;
                let connector =
                    JellyfinConnector::new(JellyfinConfig::new(base_url.clone(), access_token)?)?;
                let adapter: Arc<dyn AnimeSourceAdapter> = Arc::new(connector);
                let manifest = adapter.manifest();
                let descriptor = AnimeProviderDescriptor {
                    id: manifest.id.clone(),
                    kind: AnimeProviderKind::Jellyfin,
                    name: manifest.name.clone(),
                    local_file_count: None,
                    allowed_paths: None,
                    base_url: Some(base_url),
                    origin: Some(origin),
                    secret_configured: true,
                    manifest,
                };
                Ok((adapter, descriptor))
            }
        }
    }
}

fn rebuild_orchestrator(state: &mut RegistryState, circuit_breaker: CircuitBreakerConfig) {
    let adapters = state
        .entries
        .values()
        .map(|entry| Arc::clone(&entry.adapter))
        .collect();
    state.orchestrator = Arc::new(AnimeProviderOrchestrator::new(adapters, circuit_breaker));
}

fn jellyfin_origin(base_url: &str) -> ProviderResult<String> {
    validate_jellyfin_base_url(base_url)?;
    let url = Url::parse(base_url).map_err(|_| config_error("invalid Jellyfin base URL"))?;
    let origin = url.origin().ascii_serialization();
    if origin == "null" {
        return Err(config_error("invalid Jellyfin base URL"));
    }
    Ok(origin)
}

/// Canonicalize all allowed roots/files before constructing the local adapter.
/// This makes the adapter's exact-file lookup a second authorization check and
/// prevents `..` or symlink escapes from reaching the native player.
fn validate_local_library(
    library: Vec<AnimeLocalMediaSeries>,
    allowed_paths: Vec<String>,
) -> ProviderResult<(Vec<LocalMediaSeries>, Vec<String>)> {
    if allowed_paths.is_empty() {
        return Err(config_error("at least one allowed local path is required"));
    }

    let canonical_allowed_paths = allowed_paths
        .into_iter()
        .map(|value| canonicalize_path(&value, "allowed local path"))
        .collect::<ProviderResult<Vec<_>>>()?;

    let mut converted = Vec::with_capacity(library.len());
    for series in library {
        let mut episodes = Vec::with_capacity(series.episodes.len());
        for episode in series.episodes {
            let path = canonicalize_path(&episode.path, "local media file")?;
            if !is_path_allowed(&path, &canonical_allowed_paths) {
                return Err(config_error(
                    "local media file is outside the allowed paths",
                ));
            }
            if !path.is_file() {
                return Err(config_error("local media entry must be a file"));
            }
            episodes.push(LocalMediaEpisode {
                id: episode.id,
                title: episode.title,
                number: episode.number,
                path: path.to_string_lossy().into_owned(),
            });
        }
        converted.push(LocalMediaSeries {
            id: series.id,
            title: series.title,
            original_title: series.original_title,
            synopsis: series.synopsis,
            artwork_url: series.artwork_url,
            genres: series.genres,
            episodes,
        });
    }

    Ok((
        converted,
        canonical_allowed_paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    ))
}

fn canonicalize_path(value: &str, label: &str) -> ProviderResult<PathBuf> {
    let value = value.trim();
    if value.is_empty() || !Path::new(value).is_absolute() {
        return Err(config_error(&format!("{label} must be an absolute path")));
    }
    std::fs::canonicalize(value).map_err(|_| config_error(&format!("{label} is unavailable")))
}

fn is_path_allowed(path: &Path, allowed_paths: &[PathBuf]) -> bool {
    allowed_paths
        .iter()
        .any(|allowed| path == allowed || path.starts_with(allowed))
}

fn config_error(message: &str) -> crate::domain::ProviderError {
    provider_error(
        "anime_provider",
        "configure",
        ProviderErrorKind::PolicyBlocked,
        message,
        false,
    )
}

fn registry_error(operation: &str) -> crate::domain::ProviderError {
    provider_error(
        "anime_provider",
        operation,
        ProviderErrorKind::Unknown,
        "anime provider registry is unavailable",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn local_config(root: &Path, media: &Path) -> AnimeProviderConfig {
        AnimeProviderConfig::LocalMedia {
            allowed_paths: vec![root.to_string_lossy().into_owned()],
            library: vec![AnimeLocalMediaSeries {
                id: "series-1".to_string(),
                title: "Fixture Show".to_string(),
                original_title: None,
                synopsis: Some("fixture".to_string()),
                artwork_url: None,
                genres: vec!["Animation".to_string()],
                episodes: vec![AnimeLocalMediaEpisode {
                    id: "episode-1".to_string(),
                    title: "Episode 1".to_string(),
                    number: Some(1),
                    path: media.to_string_lossy().into_owned(),
                }],
            }],
        }
    }

    #[tokio::test]
    async fn configured_local_media_runs_search_detail_episodes_and_resolve_end_to_end() {
        let root =
            std::env::temp_dir().join(format!("moeplay-anime-provider-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let media = root.join("episode.mkv");
        fs::write(&media, b"fixture").unwrap();

        let registry = AnimeProviderRegistry::new();
        registry
            .configure(local_config(&root, &media), None)
            .unwrap();
        let response = registry
            .search(AnimeSearchQuery {
                query: "fixture".to_string(),
                limit: None,
            })
            .await;
        assert_eq!(response.items.len(), 1);
        let item = &response.items[0];
        let detail = registry
            .detail(&item.provider_id, &item.item_id)
            .await
            .unwrap();
        assert_eq!(detail.title, "Fixture Show");
        let episodes = registry
            .episodes(&item.provider_id, &item.item_id)
            .await
            .unwrap();
        let resolution = registry
            .resolve(AnimeResolveRequest {
                episode: episodes[0].identity.clone(),
            })
            .await
            .unwrap();
        assert!(matches!(
            resolution.target,
            crate::domain::ResolvedTarget::NativeFile { .. }
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn jellyfin_descriptor_never_contains_the_runtime_connector_token() {
        let registry = AnimeProviderRegistry::new();
        let descriptor = registry
            .configure(
                AnimeProviderConfig::Jellyfin {
                    base_url: "https://jellyfin.example.test/library".to_string(),
                },
                Some("super-secret-token".to_string()),
            )
            .unwrap();
        let json = serde_json::to_string(&descriptor).unwrap();
        assert!(!json.contains("super-secret-token"));
        assert_eq!(
            descriptor.origin.as_deref(),
            Some("https://jellyfin.example.test")
        );
    }
}
