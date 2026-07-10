use super::{
    validate_base_url, AuthConfig, ChapterDto, ComicProviderError, ComicResult, ComicSourceAdapter,
    KavitaConnector, KomgaConnector, LocalComicAdapter, ProbeDto, ResolveRequest, SearchRequest,
    SeriesDetailDto, SeriesDto,
};
use crate::domain::{ProviderHealth, ProviderManifest, ResolvedTarget};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComicProviderKind {
    Local,
    Komga,
    Kavita,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComicProviderAuthMode {
    None,
    Basic,
    Bearer,
    ApiKey,
}

impl ComicProviderAuthMode {
    pub fn requires_secret(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Non-sensitive provider configuration. Credentials are deliberately supplied
/// separately and are never retained in registry metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComicProviderConfig {
    Local {
        root: String,
    },
    Komga {
        base_url: String,
        auth_mode: ComicProviderAuthMode,
        username: Option<String>,
    },
    Kavita {
        base_url: String,
        auth_mode: ComicProviderAuthMode,
        username: Option<String>,
    },
}

impl ComicProviderConfig {
    pub fn kind(&self) -> ComicProviderKind {
        match self {
            Self::Local { .. } => ComicProviderKind::Local,
            Self::Komga { .. } => ComicProviderKind::Komga,
            Self::Kavita { .. } => ComicProviderKind::Kavita,
        }
    }

    pub fn auth_mode(&self) -> ComicProviderAuthMode {
        match self {
            Self::Local { .. } => ComicProviderAuthMode::None,
            Self::Komga { auth_mode, .. } | Self::Kavita { auth_mode, .. } => *auth_mode,
        }
    }

    pub fn validate(&self) -> ComicResult<()> {
        match self {
            Self::Local { .. } => Ok(()),
            Self::Komga {
                auth_mode,
                username,
                ..
            } => {
                self.origin()?;
                if matches!(auth_mode, ComicProviderAuthMode::ApiKey) {
                    return Err(ComicProviderError::InvalidConfig(
                        "Komga supports none, basic, or bearer authentication".to_string(),
                    ));
                }
                validate_username(*auth_mode, username.as_deref())
            }
            Self::Kavita {
                auth_mode,
                username,
                ..
            } => {
                self.origin()?;
                if !matches!(auth_mode, ComicProviderAuthMode::ApiKey) {
                    return Err(ComicProviderError::InvalidConfig(
                        "Kavita requires api_key authentication".to_string(),
                    ));
                }
                validate_username(*auth_mode, username.as_deref())
            }
        }
    }

    /// Canonical origin used as the SecretStore RuntimeConnectorToken scope.
    pub fn origin(&self) -> ComicResult<Option<String>> {
        match self {
            Self::Local { .. } => Ok(None),
            Self::Komga { base_url, .. } | Self::Kavita { base_url, .. } => {
                let url = validate_base_url(base_url)?;
                Ok(Some(url.origin().ascii_serialization()))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComicProviderDescriptor {
    pub id: String,
    pub kind: ComicProviderKind,
    pub name: String,
    pub local_root: Option<String>,
    pub base_url: Option<String>,
    pub origin: Option<String>,
    pub username: Option<String>,
    pub auth_mode: ComicProviderAuthMode,
    pub secret_configured: bool,
    pub manifest: ProviderManifest,
}

type ConfiguredComicProvider = (
    Arc<dyn ComicSourceAdapter>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    ComicProviderAuthMode,
);

struct RegistryEntry {
    adapter: Arc<dyn ComicSourceAdapter>,
    descriptor: ComicProviderDescriptor,
}

/// Managed, thread-safe registry shared by all comic provider commands.
#[derive(Default)]
pub struct ComicProviderRegistry {
    entries: RwLock<BTreeMap<String, RegistryEntry>>,
}

impl ComicProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build and atomically install an adapter. `credential` is consumed into
    /// the adapter's HTTP client and is never copied into listable metadata.
    pub fn configure(
        &self,
        config: ComicProviderConfig,
        credential: Option<String>,
    ) -> ComicResult<ComicProviderDescriptor> {
        config.validate()?;
        let kind = config.kind();
        let (adapter, local_root, base_url, origin, username, auth_mode): ConfiguredComicProvider =
            match config {
                ComicProviderConfig::Local { root } => {
                    if credential.is_some() {
                        return Err(ComicProviderError::InvalidConfig(
                            "local providers do not accept credentials".to_string(),
                        ));
                    }
                    let adapter = LocalComicAdapter::new(&root)?;
                    let canonical_root = adapter.root().to_string_lossy().into_owned();
                    (
                        Arc::new(adapter),
                        Some(canonical_root),
                        None,
                        None,
                        None,
                        ComicProviderAuthMode::None,
                    )
                }
                ComicProviderConfig::Komga {
                    base_url,
                    auth_mode,
                    username,
                } => {
                    let normalized =
                        normalize_remote_config(&base_url, auth_mode, username, credential)?;
                    let adapter =
                        KomgaConnector::new(normalized.base_url.clone(), normalized.auth)?;
                    (
                        Arc::new(adapter),
                        None,
                        Some(normalized.base_url),
                        Some(normalized.origin),
                        normalized.username,
                        auth_mode,
                    )
                }
                ComicProviderConfig::Kavita {
                    base_url,
                    auth_mode,
                    username,
                } => {
                    let normalized =
                        normalize_remote_config(&base_url, auth_mode, username, credential)?;
                    let NormalizedRemoteConfig {
                        base_url,
                        origin,
                        username,
                        auth,
                    } = normalized;
                    let api_key = match auth {
                        AuthConfig::ApiKey(api_key) => api_key,
                        _ => {
                            return Err(ComicProviderError::InvalidConfig(
                                "Kavita requires api_key authentication".to_string(),
                            ))
                        }
                    };
                    let adapter = KavitaConnector::new(base_url.clone(), api_key)?;
                    (
                        Arc::new(adapter),
                        None,
                        Some(base_url),
                        Some(origin),
                        username,
                        auth_mode,
                    )
                }
            };

        let manifest = adapter.manifest();
        let descriptor = ComicProviderDescriptor {
            id: manifest.id.clone(),
            kind,
            name: manifest.name.clone(),
            local_root,
            base_url,
            origin,
            username,
            auth_mode,
            secret_configured: auth_mode.requires_secret(),
            manifest,
        };
        let mut entries = self.entries.write().map_err(|_| registry_unavailable())?;
        entries.insert(
            descriptor.id.clone(),
            RegistryEntry {
                adapter,
                descriptor: descriptor.clone(),
            },
        );
        Ok(descriptor)
    }

    pub fn remove(&self, provider_id: &str) -> ComicResult<bool> {
        let mut entries = self.entries.write().map_err(|_| registry_unavailable())?;
        Ok(entries.remove(provider_id).is_some())
    }

    pub fn list(&self) -> ComicResult<Vec<ComicProviderDescriptor>> {
        let entries = self.entries.read().map_err(|_| registry_unavailable())?;
        Ok(entries
            .values()
            .map(|entry| entry.descriptor.clone())
            .collect())
    }

    pub fn health(&self, provider_id: &str, operation: &str) -> ComicResult<ProviderHealth> {
        Ok(self.adapter(provider_id, operation)?.health(operation))
    }

    pub async fn probe(&self, provider_id: &str) -> ComicResult<ProbeDto> {
        self.adapter(provider_id, "probe")?.probe().await
    }

    pub async fn search(
        &self,
        provider_id: &str,
        request: SearchRequest,
    ) -> ComicResult<Vec<SeriesDto>> {
        self.adapter(provider_id, "search")?.search(request).await
    }

    pub async fn detail(
        &self,
        provider_id: &str,
        series_id: String,
    ) -> ComicResult<SeriesDetailDto> {
        self.adapter(provider_id, "detail")?.detail(series_id).await
    }

    pub async fn chapters(
        &self,
        provider_id: &str,
        series_id: String,
    ) -> ComicResult<Vec<ChapterDto>> {
        self.adapter(provider_id, "chapters")?
            .chapters(series_id)
            .await
    }

    pub async fn resolve(
        &self,
        provider_id: &str,
        request: ResolveRequest,
    ) -> ComicResult<ResolvedTarget> {
        self.adapter(provider_id, "resolve")?.resolve(request).await
    }

    fn adapter(
        &self,
        provider_id: &str,
        operation: &str,
    ) -> ComicResult<Arc<dyn ComicSourceAdapter>> {
        let entries = self.entries.read().map_err(|_| registry_unavailable())?;
        entries
            .get(provider_id)
            .map(|entry| Arc::clone(&entry.adapter))
            .ok_or_else(|| {
                super::provider_error(
                    provider_id,
                    operation,
                    crate::domain::ProviderErrorKind::Unsupported,
                    "comic provider is not configured",
                    false,
                )
            })
    }
}

struct NormalizedRemoteConfig {
    base_url: String,
    origin: String,
    username: Option<String>,
    auth: AuthConfig,
}

fn validate_username(auth_mode: ComicProviderAuthMode, username: Option<&str>) -> ComicResult<()> {
    let has_username = username
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    if matches!(auth_mode, ComicProviderAuthMode::Basic) && !has_username {
        return Err(ComicProviderError::InvalidConfig(
            "basic authentication requires a username".to_string(),
        ));
    }
    if !matches!(auth_mode, ComicProviderAuthMode::Basic) && has_username {
        return Err(ComicProviderError::InvalidConfig(
            "username is supported only for basic authentication".to_string(),
        ));
    }
    Ok(())
}

fn normalize_remote_config(
    base_url: &str,
    auth_mode: ComicProviderAuthMode,
    username: Option<String>,
    credential: Option<String>,
) -> ComicResult<NormalizedRemoteConfig> {
    let mut url = validate_base_url(base_url)?;
    let username = username.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    let credential = credential.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(value)
    });

    let auth = match auth_mode {
        ComicProviderAuthMode::None => {
            if credential.is_some() {
                return Err(ComicProviderError::InvalidConfig(
                    "auth mode none does not accept a credential".to_string(),
                ));
            }
            AuthConfig::None
        }
        ComicProviderAuthMode::Basic => {
            let username = username.clone().ok_or_else(|| {
                ComicProviderError::InvalidConfig(
                    "basic authentication requires a username".to_string(),
                )
            })?;
            let password = credential.ok_or_else(missing_credential)?;
            AuthConfig::Basic { username, password }
        }
        ComicProviderAuthMode::Bearer => {
            AuthConfig::Bearer(credential.ok_or_else(missing_credential)?)
        }
        ComicProviderAuthMode::ApiKey => {
            AuthConfig::ApiKey(credential.ok_or_else(missing_credential)?)
        }
    };

    if !matches!(auth_mode, ComicProviderAuthMode::Basic) && username.is_some() {
        return Err(ComicProviderError::InvalidConfig(
            "username is supported only for basic authentication".to_string(),
        ));
    }

    // Normalize an empty root path while preserving an explicitly configured
    // reverse-proxy prefix.
    if url.path().is_empty() {
        url.set_path("/");
    }
    let origin = url.origin().ascii_serialization();
    Ok(NormalizedRemoteConfig {
        base_url: url.to_string().trim_end_matches('/').to_string(),
        origin,
        username,
        auth,
    })
}

fn missing_credential() -> ComicProviderError {
    ComicProviderError::InvalidConfig(
        "selected authentication mode requires a stored credential".to_string(),
    )
}

fn registry_unavailable() -> ComicProviderError {
    ComicProviderError::InvalidConfig("comic provider registry is unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "moeplay-comic-registry-{name}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[tokio::test]
    async fn local_registry_end_to_end() {
        let root = temp_root("local");
        fs::create_dir_all(root.join("pages")).unwrap();
        fs::write(root.join("pages/002.jpg"), b"two").unwrap();
        fs::write(root.join("pages/001.jpg"), b"one").unwrap();
        fs::write(
            root.join(".moeplay-comic.json"),
            r#"{"version":1,"providerId":"local-fixture","libraryName":"Fixture","series":[{"id":"series","title":"Local Series","path":".","chapters":[{"id":"chapter","title":"Chapter 1","path":"pages"}]}]}"#,
        )
        .unwrap();

        let registry = ComicProviderRegistry::new();
        let configured = registry
            .configure(
                ComicProviderConfig::Local {
                    root: root.to_string_lossy().into_owned(),
                },
                None,
            )
            .unwrap();
        assert_eq!(configured.id, "local-fixture");
        assert_eq!(registry.list().unwrap().len(), 1);

        let found = registry
            .search(
                "local-fixture",
                SearchRequest {
                    query: "Local".into(),
                    ..SearchRequest::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(found[0].title, "Local Series");
        let chapters = registry
            .chapters("local-fixture", "series".into())
            .await
            .unwrap();
        assert_eq!(chapters[0].identity.chapter_id, "chapter");
        let target = registry
            .resolve(
                "local-fixture",
                ResolveRequest {
                    series_id: "series".into(),
                    chapter_id: "chapter".into(),
                },
            )
            .await
            .unwrap();
        match target {
            ResolvedTarget::ImagePages { pages, headers } => {
                assert_eq!(pages.len(), 2);
                assert!(pages[0].ends_with("001.jpg"));
                assert!(headers.is_empty());
            }
            other => panic!("unexpected target: {other:?}"),
        }
        assert!(registry.remove("local-fixture").unwrap());
        assert!(registry.list().unwrap().is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn remote_descriptor_never_serializes_credentials() {
        let sentinel = "SENTINEL_REMOTE_SECRET_MUST_NOT_LEAK";
        let registry = ComicProviderRegistry::new();
        let descriptor = registry
            .configure(
                ComicProviderConfig::Komga {
                    base_url: "https://example.com/komga".into(),
                    auth_mode: ComicProviderAuthMode::Basic,
                    username: Some("reader".into()),
                },
                Some(sentinel.into()),
            )
            .unwrap();
        let serialized = serde_json::to_string(&(descriptor, registry.list().unwrap())).unwrap();
        assert!(!serialized.contains(sentinel));
        assert!(serialized.contains("reader"));
        assert!(serialized.contains("secretConfigured"));
    }

    #[test]
    fn remote_config_validates_ssrf_before_building_an_adapter() {
        let registry = ComicProviderRegistry::new();
        assert!(registry
            .configure(
                ComicProviderConfig::Kavita {
                    base_url: "http://192.168.1.20:5000".into(),
                    auth_mode: ComicProviderAuthMode::ApiKey,
                    username: None,
                },
                Some("secret".into()),
            )
            .is_err());
        assert!(registry.list().unwrap().is_empty());
    }
}
