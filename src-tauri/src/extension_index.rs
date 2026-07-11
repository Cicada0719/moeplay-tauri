//! Metadata-only remote extension directory cache.
//!
//! The cache stores only the allow-listed DTO below. It never downloads,
//! installs, parses, or executes extension packages.

use chrono::{DateTime, Duration, Utc};
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, fs, io::Write, path::PathBuf, time::Duration as StdDuration};
use url::{Host, Url};

pub const EXTENSION_INDEX_TTL_HOURS: i64 = 24;
const MAX_INDEX_BYTES: usize = 2 * 1024 * 1024;
const MAX_INDEX_ENTRIES: usize = 1_000;

/// Safe metadata projection. Unknown fields such as package URLs and hashes are
/// discarded during deserialization and never reach the disk cache or DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionIndexEntry {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub languages: Vec<String>,
    pub media_types: Vec<String>,
    pub nsfw: bool,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionIndexCacheState {
    Fresh,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionIndexSnapshot {
    pub endpoint: String,
    pub entries: Vec<ExtensionIndexEntry>,
    pub fetched_at: String,
    pub expires_at: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub cache_state: ExtensionIndexCacheState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionIndexRefreshState {
    FreshCache,
    Refreshed,
    NotModified,
    OfflineSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionIndexRefresh {
    pub snapshot: ExtensionIndexSnapshot,
    pub state: ExtensionIndexRefreshState,
    pub warning_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionIndexErrorKind {
    PolicyBlocked,
    Timeout,
    Network,
    InvalidMetadata,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionIndexError {
    pub kind: ExtensionIndexErrorKind,
    pub code: &'static str,
    pub retryable: bool,
}

impl ExtensionIndexError {
    fn new(kind: ExtensionIndexErrorKind, code: &'static str, retryable: bool) -> Self {
        Self {
            kind,
            code,
            retryable,
        }
    }
    fn policy() -> Self {
        Self::new(
            ExtensionIndexErrorKind::PolicyBlocked,
            "extension_index_policy_blocked",
            false,
        )
    }
    fn invalid() -> Self {
        Self::new(
            ExtensionIndexErrorKind::InvalidMetadata,
            "extension_index_invalid_metadata",
            true,
        )
    }
    fn storage() -> Self {
        Self::new(
            ExtensionIndexErrorKind::Storage,
            "extension_index_cache_failed",
            true,
        )
    }
    fn request(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::new(
                ExtensionIndexErrorKind::Timeout,
                "extension_index_timeout",
                true,
            )
        } else {
            Self::new(
                ExtensionIndexErrorKind::Network,
                "extension_index_network_failed",
                true,
            )
        }
    }
}
impl fmt::Display for ExtensionIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl std::error::Error for ExtensionIndexError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSnapshot {
    endpoint: String,
    entries: Vec<ExtensionIndexEntry>,
    fetched_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    etag: Option<String>,
    last_modified: Option<String>,
}
impl StoredSnapshot {
    fn dto(&self) -> ExtensionIndexSnapshot {
        ExtensionIndexSnapshot {
            endpoint: self.endpoint.clone(),
            entries: self.entries.clone(),
            fetched_at: self.fetched_at.to_rfc3339(),
            expires_at: self.expires_at.to_rfc3339(),
            etag: self.etag.clone(),
            last_modified: self.last_modified.clone(),
            cache_state: if self.expires_at > Utc::now() {
                ExtensionIndexCacheState::Fresh
            } else {
                ExtensionIndexCacheState::Stale
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WireIndex {
    Array(Vec<WireEntry>),
    Envelope { extensions: Vec<WireEntry> },
}
impl WireIndex {
    fn entries(self) -> Vec<WireEntry> {
        match self {
            Self::Array(x) | Self::Envelope { extensions: x } => x,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireEntry {
    id: String,
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    languages: Vec<String>,
    #[serde(default)]
    media_types: Vec<String>,
    #[serde(default)]
    nsfw: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    homepage_url: Option<String>,
}

#[derive(Clone)]
pub struct ExtensionIndexService {
    cache_dir: PathBuf,
    client: reqwest::Client,
}
impl Default for ExtensionIndexService {
    fn default() -> Self {
        Self::new(
            dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("moeplay")
                .join("extension-index"),
        )
    }
}
impl ExtensionIndexService {
    pub fn new(cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("valid extension-index client");
        Self { cache_dir, client }
    }
    pub fn validate_endpoint(&self, endpoint: &str) -> Result<String, ExtensionIndexError> {
        validate_endpoint(endpoint)
    }
    pub fn endpoint_fingerprint(&self, endpoint: &str) -> Result<String, ExtensionIndexError> {
        Ok(fingerprint(&self.validate_endpoint(endpoint)?))
    }
    pub fn get_snapshot(
        &self,
        endpoint: &str,
    ) -> Result<Option<ExtensionIndexSnapshot>, ExtensionIndexError> {
        let endpoint = self.validate_endpoint(endpoint)?;
        Ok(self.load(&endpoint)?.map(|item| item.dto()))
    }
    pub async fn refresh(
        &self,
        endpoint: &str,
        force: bool,
    ) -> Result<ExtensionIndexRefresh, ExtensionIndexError> {
        let endpoint = self.validate_endpoint(endpoint)?;
        let cached = self.load(&endpoint)?;
        if !force
            && cached
                .as_ref()
                .is_some_and(|item| item.expires_at > Utc::now())
        {
            return Ok(ExtensionIndexRefresh {
                snapshot: cached.expect("fresh cache").dto(),
                state: ExtensionIndexRefreshState::FreshCache,
                warning_code: None,
            });
        }
        let result = self.fetch(&endpoint, cached.as_ref()).await;
        match result {
            Ok(Fetch::NotModified) if cached.is_some() => {
                let mut stored = cached.expect("checked");
                stored.fetched_at = Utc::now();
                stored.expires_at = stored.fetched_at + Duration::hours(EXTENSION_INDEX_TTL_HOURS);
                self.save(&endpoint, &stored)?;
                Ok(ExtensionIndexRefresh {
                    snapshot: stored.dto(),
                    state: ExtensionIndexRefreshState::NotModified,
                    warning_code: None,
                })
            }
            Ok(Fetch::Entries {
                entries,
                etag,
                last_modified,
            }) => {
                let fetched_at = Utc::now();
                let stored = StoredSnapshot {
                    endpoint: endpoint.clone(),
                    entries,
                    fetched_at,
                    expires_at: fetched_at + Duration::hours(EXTENSION_INDEX_TTL_HOURS),
                    etag,
                    last_modified,
                };
                self.save(&endpoint, &stored)?;
                Ok(ExtensionIndexRefresh {
                    snapshot: stored.dto(),
                    state: ExtensionIndexRefreshState::Refreshed,
                    warning_code: None,
                })
            }
            Ok(Fetch::NotModified) => Err(ExtensionIndexError::invalid()),
            Err(error) if cached.is_some() && error.retryable => Ok(ExtensionIndexRefresh {
                snapshot: cached.expect("checked").dto(),
                state: ExtensionIndexRefreshState::OfflineSnapshot,
                warning_code: Some(error.code.to_string()),
            }),
            Err(error) => Err(error),
        }
    }
    async fn fetch(
        &self,
        endpoint: &str,
        cached: Option<&StoredSnapshot>,
    ) -> Result<Fetch, ExtensionIndexError> {
        let mut request = self
            .client
            .get(endpoint)
            .header("Accept", "application/json");
        if let Some(value) = cached.and_then(|x| x.etag.as_deref()) {
            request = request.header(IF_NONE_MATCH, value);
        }
        if let Some(value) = cached.and_then(|x| x.last_modified.as_deref()) {
            request = request.header(IF_MODIFIED_SINCE, value);
        }
        let response = request.send().await.map_err(ExtensionIndexError::request)?;
        if response.status().is_redirection() {
            return Err(ExtensionIndexError::policy());
        }
        if response.status().as_u16() == 304 {
            return Ok(Fetch::NotModified);
        }
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|x| x as usize > MAX_INDEX_BYTES)
        {
            return Err(ExtensionIndexError::invalid());
        }
        let etag = header(response.headers().get(ETAG));
        let last_modified = header(response.headers().get(LAST_MODIFIED));
        let bytes = response
            .bytes()
            .await
            .map_err(ExtensionIndexError::request)?;
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(ExtensionIndexError::invalid());
        }
        Ok(Fetch::Entries {
            entries: parse_entries(&bytes)?,
            etag,
            last_modified,
        })
    }
    fn cache_path(&self, endpoint: &str) -> PathBuf {
        self.cache_dir
            .join(format!("{}.json", fingerprint(endpoint)))
    }
    fn load(&self, endpoint: &str) -> Result<Option<StoredSnapshot>, ExtensionIndexError> {
        let path = self.cache_path(endpoint);
        if !path.exists() {
            return Ok(None);
        }
        let value: StoredSnapshot =
            serde_json::from_slice(&fs::read(path).map_err(|_| ExtensionIndexError::storage())?)
                .map_err(|_| ExtensionIndexError::storage())?;
        if value.endpoint != endpoint {
            return Err(ExtensionIndexError::storage());
        }
        Ok(Some(value))
    }
    fn save(&self, endpoint: &str, value: &StoredSnapshot) -> Result<(), ExtensionIndexError> {
        fs::create_dir_all(&self.cache_dir).map_err(|_| ExtensionIndexError::storage())?;
        let path = self.cache_path(endpoint);
        let temporary = path.with_extension("tmp");
        let mut file = fs::File::create(&temporary).map_err(|_| ExtensionIndexError::storage())?;
        file.write_all(&serde_json::to_vec(value).map_err(|_| ExtensionIndexError::storage())?)
            .and_then(|_| file.sync_all())
            .map_err(|_| ExtensionIndexError::storage())?;
        fs::rename(temporary, path).map_err(|_| ExtensionIndexError::storage())
    }
}

enum Fetch {
    NotModified,
    Entries {
        entries: Vec<ExtensionIndexEntry>,
        etag: Option<String>,
        last_modified: Option<String>,
    },
}

fn validate_endpoint(raw: &str) -> Result<String, ExtensionIndexError> {
    if raw.len() > 2048 || raw.chars().any(char::is_control) {
        return Err(ExtensionIndexError::policy());
    }
    let mut url = Url::parse(raw.trim()).map_err(|_| ExtensionIndexError::policy())?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ExtensionIndexError::policy());
    }
    let host = url.host().ok_or_else(ExtensionIndexError::policy)?;
    if url.scheme() != "https" && !(url.scheme() == "http" && is_loopback(&host)) {
        return Err(ExtensionIndexError::policy());
    }
    if url.path().is_empty() {
        url.set_path("/");
    }
    Ok(url.to_string())
}
fn is_loopback(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(x) => x.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(x) => x.is_loopback(),
        Host::Ipv6(x) => x.is_loopback(),
    }
}
fn fingerprint(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    hex::encode(hash.finalize())
}
fn header(value: Option<&reqwest::header::HeaderValue>) -> Option<String> {
    value
        .and_then(|x| x.to_str().ok())
        .filter(|x| x.len() <= 512 && !x.chars().any(char::is_control))
        .map(ToOwned::to_owned)
}
fn safe_text(value: Option<String>, max: usize) -> Result<Option<String>, ExtensionIndexError> {
    match value {
        None => Ok(None),
        Some(x) => {
            let x = x.trim().to_string();
            if x.is_empty() {
                Ok(None)
            } else if x.len() > max || x.chars().any(char::is_control) {
                Err(ExtensionIndexError::invalid())
            } else {
                Ok(Some(x))
            }
        }
    }
}
fn parse_entries(bytes: &[u8]) -> Result<Vec<ExtensionIndexEntry>, ExtensionIndexError> {
    let entries = serde_json::from_slice::<WireIndex>(bytes)
        .map_err(|_| ExtensionIndexError::invalid())?
        .entries();
    if entries.len() > MAX_INDEX_ENTRIES {
        return Err(ExtensionIndexError::invalid());
    }
    entries
        .into_iter()
        .map(|item| {
            let mut languages = item.languages;
            if let Some(language) = item.language {
                languages.push(language);
            }
            let id = safe_text(Some(item.id), 160)?.ok_or_else(ExtensionIndexError::invalid)?;
            let name = safe_text(Some(item.name), 200)?.ok_or_else(ExtensionIndexError::invalid)?;
            let homepage_url = safe_text(item.homepage_url, 1024)?;
            if let Some(ref homepage) = homepage_url {
                let url = Url::parse(homepage).map_err(|_| ExtensionIndexError::invalid())?;
                if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some()
                {
                    return Err(ExtensionIndexError::invalid());
                }
            }
            Ok(ExtensionIndexEntry {
                id,
                name,
                version: safe_text(item.version, 80)?,
                languages: normalize_list(languages, 24),
                media_types: normalize_list(item.media_types, 12),
                nsfw: item.nsfw,
                description: safe_text(item.description, 1024)?,
                homepage_url,
            })
        })
        .collect()
}
fn normalize_list(values: Vec<String>, max: usize) -> Vec<String> {
    let mut out = values
        .into_iter()
        .filter_map(|x| safe_text(Some(x), 32).ok().flatten())
        .map(|x| x.to_ascii_lowercase())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out.truncate(max);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_unsafe_endpoints() {
        let service = ExtensionIndexService::new(std::env::temp_dir());
        for value in [
            "http://example.test/a",
            "https://u:p@example.test/a",
            "https://example.test/a?token=x",
            "file:///a",
        ] {
            assert_eq!(
                service.validate_endpoint(value).unwrap_err().kind,
                ExtensionIndexErrorKind::PolicyBlocked
            );
        }
        assert!(service
            .validate_endpoint("http://127.0.0.1:8080/index.json")
            .is_ok());
    }
    #[test]
    fn persists_metadata_projection_only() {
        let entries = parse_entries(br#"{"extensions":[{"id":"x","name":"X","downloadUrl":"https://example.test/x.apk","language":"EN"}]}"#).unwrap();
        assert_eq!(entries[0].languages, vec!["en"]);
        let encoded = serde_json::to_string(&entries).unwrap();
        assert!(!encoded.contains("downloadUrl") && !encoded.contains(".apk"));
    }
}
