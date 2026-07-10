use reqwest::{redirect::Policy, StatusCode};
use serde_json::Value;
use url::Url;

use crate::domain::{
    ProviderCapability, ProviderErrorKind, ProviderManifest, ProviderTrust, ResolvedTarget,
    ResourceKind,
};

use super::{
    provider_error, AdapterFuture, AnimeDetail, AnimeEpisode, AnimeEpisodeIdentity,
    AnimeResolveRequest, AnimeResolveResponse, AnimeSearchItem, AnimeSearchQuery,
    AnimeSourceAdapter, ProviderResult,
};

pub const JELLYFIN_PROVIDER_ID: &str = "jellyfin";

/// Self-hosted Jellyfin credentials. The token intentionally has no Serialize
/// implementation so it cannot be accidentally included in frontend DTOs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JellyfinConfig {
    pub base_url: String,
    pub access_token: String,
}

impl JellyfinConfig {
    pub fn new(
        base_url: impl Into<String>,
        access_token: impl Into<String>,
    ) -> ProviderResult<Self> {
        let base_url = base_url.into();
        let access_token = access_token.into();
        validate_jellyfin_base_url(&base_url)?;
        if access_token.trim().is_empty() {
            return Err(provider_error(
                JELLYFIN_PROVIDER_ID,
                "configure",
                ProviderErrorKind::AuthRequired,
                "Jellyfin access token is required",
                false,
            ));
        }
        Ok(Self {
            base_url,
            access_token,
        })
    }
}

pub struct JellyfinConnector {
    config: JellyfinConfig,
    base: Url,
    client: reqwest::Client,
}

impl JellyfinConnector {
    pub fn new(config: JellyfinConfig) -> ProviderResult<Self> {
        let base = normalized_base_url(&config.base_url)?;
        let client = reqwest::Client::builder()
            // Redirects are intentionally not followed: a self-hosted token may
            // never be sent to a different origin.
            .redirect(Policy::none())
            .build()
            .map_err(|error| {
                provider_error(
                    JELLYFIN_PROVIDER_ID,
                    "configure",
                    ProviderErrorKind::Unknown,
                    format!("failed to initialize Jellyfin client: {error}"),
                    false,
                )
            })?;
        Ok(Self {
            config,
            base,
            client,
        })
    }

    pub fn allowed_origin(&self) -> String {
        origin_string(&self.base)
    }

    /// Builds a URL below the configured base path. No endpoint input can
    /// replace the configured origin or invoke a different scheme.
    pub fn endpoint_url(&self, relative_path: &str) -> ProviderResult<Url> {
        if relative_path.starts_with("//") || Url::parse(relative_path).is_ok() {
            return Err(provider_error(
                JELLYFIN_PROVIDER_ID,
                "build_url",
                ProviderErrorKind::PolicyBlocked,
                "Jellyfin endpoint must be a relative API path",
                false,
            ));
        }
        self.base
            .join(relative_path.trim_start_matches('/'))
            .map_err(|error| {
                provider_error(
                    JELLYFIN_PROVIDER_ID,
                    "build_url",
                    ProviderErrorKind::PolicyBlocked,
                    format!("invalid Jellyfin API path: {error}"),
                    false,
                )
            })
    }

    /// Returns the token header only for the exact configured origin. This is
    /// also used before requests, preventing cross-origin authorization leaks.
    pub fn authorization_headers_for(&self, url: &Url) -> ProviderResult<Vec<(String, String)>> {
        if !same_origin(&self.base, url) {
            return Err(provider_error(
                JELLYFIN_PROVIDER_ID,
                "authorize",
                ProviderErrorKind::PolicyBlocked,
                "refusing to attach Jellyfin credentials to a different origin",
                false,
            ));
        }
        Ok(vec![(
            "X-Emby-Token".to_string(),
            self.config.access_token.clone(),
        )])
    }

    pub fn resolve_item_value(
        &self,
        episode: AnimeEpisodeIdentity,
        item: &Value,
    ) -> ProviderResult<AnimeResolveResponse> {
        if episode.provider_id != JELLYFIN_PROVIDER_ID {
            return Err(provider_error(
                JELLYFIN_PROVIDER_ID,
                "resolve",
                ProviderErrorKind::PolicyBlocked,
                "a Jellyfin connector cannot resolve an episode from another provider",
                false,
            ));
        }
        if item_requires_drm(item) {
            return Ok(terminal_resolution(
                episode,
                "DRM-protected Jellyfin media cannot be played by the native player",
                ProviderErrorKind::UnsupportedDrm,
            ));
        }
        if item_is_policy_blocked(item) {
            return Ok(terminal_resolution(
                episode,
                "Jellyfin policy does not permit playback for this item",
                ProviderErrorKind::PolicyBlocked,
            ));
        }

        let media_source_id = item
            .get("MediaSources")
            .and_then(Value::as_array)
            .and_then(|sources| {
                sources.iter().find(|source| {
                    !item_requires_drm(source)
                        && source
                            .get("SupportsDirectStream")
                            .and_then(Value::as_bool)
                            .unwrap_or(true)
                })
            })
            .and_then(|source| source.get("Id"))
            .and_then(Value::as_str)
            .or_else(|| item.get("Id").and_then(Value::as_str));

        let Some(media_source_id) = media_source_id else {
            return Ok(terminal_resolution(
                episode,
                "Jellyfin did not expose a playable media source",
                ProviderErrorKind::Unsupported,
            ));
        };

        let mut url = self.endpoint_url(&format!(
            "Videos/{}/master.m3u8",
            urlencoding::encode(&episode.episode_id)
        ))?;
        url.query_pairs_mut()
            .append_pair("static", "true")
            .append_pair("MediaSourceId", media_source_id);
        let headers = self.authorization_headers_for(&url)?;
        Ok(AnimeResolveResponse {
            episode,
            target: ResolvedTarget::NativeHls {
                url: url.into(),
                headers,
            },
        })
    }

    async fn get_json(&self, url: Url, operation: &str) -> ProviderResult<Value> {
        let headers = self.authorization_headers_for(&url)?;
        let mut request = self.client.get(url.clone());
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request.send().await.map_err(|error| {
            provider_error(
                JELLYFIN_PROVIDER_ID,
                operation,
                if error.is_timeout() {
                    ProviderErrorKind::Timeout
                } else {
                    ProviderErrorKind::Network
                },
                format!("Jellyfin request failed: {error}"),
                true,
            )
        })?;
        if response.status().is_redirection() {
            return Err(provider_error(
                JELLYFIN_PROVIDER_ID,
                operation,
                ProviderErrorKind::PolicyBlocked,
                "Jellyfin redirect rejected to prevent cross-origin credential forwarding",
                false,
            ));
        }
        if !response.status().is_success() {
            return Err(http_error(operation, response.status()));
        }
        response.json::<Value>().await.map_err(|error| {
            provider_error(
                JELLYFIN_PROVIDER_ID,
                operation,
                ProviderErrorKind::ParseChanged,
                format!("Jellyfin returned an invalid JSON response: {error}"),
                false,
            )
        })
    }

    fn item_artwork_url(&self, item: &Value) -> Option<String> {
        let id = item.get("Id")?.as_str()?;
        item.get("ImageTags")?.get("Primary")?.as_str()?;
        self.endpoint_url(&format!("Items/{}/Images/Primary", urlencoding::encode(id)))
            .ok()
            .map(Into::into)
    }

    fn item_to_search(&self, item: &Value) -> Option<AnimeSearchItem> {
        let item_id = item.get("Id")?.as_str()?.to_string();
        Some(AnimeSearchItem {
            provider_id: JELLYFIN_PROVIDER_ID.to_string(),
            item_id,
            title: item_title(item)?,
            original_title: item
                .get("OriginalTitle")
                .and_then(Value::as_str)
                .map(str::to_string),
            synopsis: item
                .get("Overview")
                .and_then(Value::as_str)
                .map(str::to_string),
            artwork_url: self.item_artwork_url(item),
        })
    }
}

impl AnimeSourceAdapter for JellyfinConnector {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest {
            id: JELLYFIN_PROVIDER_ID.to_string(),
            name: "Jellyfin".to_string(),
            resource_kinds: vec![ResourceKind::Anime],
            capabilities: vec![
                ProviderCapability::Probe,
                ProviderCapability::Search,
                ProviderCapability::Detail,
                ProviderCapability::Children,
                ProviderCapability::Resolve,
            ],
            trust: ProviderTrust::SelfHosted,
            version: "batch2".to_string(),
            enabled: true,
            requires_auth: true,
            allowed_hosts: vec![self.allowed_origin()],
        }
    }

    fn search<'a>(&'a self, query: AnimeSearchQuery) -> AdapterFuture<'a, Vec<AnimeSearchItem>> {
        Box::pin(async move {
            let mut url = self.endpoint_url("Items")?;
            {
                let mut params = url.query_pairs_mut();
                params.append_pair("SearchTerm", query.query.trim());
                params.append_pair("IncludeItemTypes", "Series");
                params.append_pair("Recursive", "true");
                params.append_pair("Fields", "Overview,PrimaryImageAspectRatio");
                params.append_pair("Limit", &query.limit.unwrap_or(50).to_string());
            }
            let json = self.get_json(url, "search").await?;
            Ok(json
                .get("Items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| self.item_to_search(item))
                .collect())
        })
    }

    fn detail<'a>(&'a self, item_id: &'a str) -> AdapterFuture<'a, AnimeDetail> {
        Box::pin(async move {
            let url = self.endpoint_url(&format!(
                "Items/{}?Fields=Overview,Genres,PrimaryImageAspectRatio",
                urlencoding::encode(item_id)
            ))?;
            let item = self.get_json(url, "detail").await?;
            Ok(AnimeDetail {
                provider_id: JELLYFIN_PROVIDER_ID.to_string(),
                item_id: item
                    .get("Id")
                    .and_then(Value::as_str)
                    .unwrap_or(item_id)
                    .to_string(),
                title: item_title(&item).ok_or_else(|| {
                    provider_error(
                        JELLYFIN_PROVIDER_ID,
                        "detail",
                        ProviderErrorKind::ParseChanged,
                        "Jellyfin item has no title",
                        false,
                    )
                })?,
                original_title: item
                    .get("OriginalTitle")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                synopsis: item
                    .get("Overview")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                artwork_url: self.item_artwork_url(&item),
                genres: item
                    .get("Genres")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            })
        })
    }

    fn episodes<'a>(&'a self, series_id: &'a str) -> AdapterFuture<'a, Vec<AnimeEpisode>> {
        Box::pin(async move {
            let url = self.endpoint_url(&format!(
                "Shows/{}/Episodes?Fields=PrimaryImageAspectRatio",
                urlencoding::encode(series_id)
            ))?;
            let json = self.get_json(url, "episodes").await?;
            Ok(json
                .get("Items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| {
                    let episode_id = item.get("Id")?.as_str()?.to_string();
                    Some(AnimeEpisode {
                        identity: AnimeEpisodeIdentity {
                            provider_id: JELLYFIN_PROVIDER_ID.to_string(),
                            series_id: series_id.to_string(),
                            episode_id,
                        },
                        title: item_title(item)?,
                        number: item
                            .get("IndexNumber")
                            .and_then(Value::as_u64)
                            .and_then(|number| u32::try_from(number).ok()),
                        artwork_url: self.item_artwork_url(item),
                    })
                })
                .collect())
        })
    }

    fn resolve<'a>(
        &'a self,
        request: AnimeResolveRequest,
    ) -> AdapterFuture<'a, AnimeResolveResponse> {
        Box::pin(async move {
            let url = self.endpoint_url(&format!(
                "Items/{}?Fields=MediaSources,UserData,CanPlay,IsProtected",
                urlencoding::encode(&request.episode.episode_id)
            ))?;
            let item = self.get_json(url, "resolve").await?;
            self.resolve_item_value(request.episode, &item)
        })
    }
}

pub fn validate_jellyfin_base_url(value: &str) -> ProviderResult<()> {
    normalized_base_url(value).map(|_| ())
}

fn normalized_base_url(value: &str) -> ProviderResult<Url> {
    let mut url = Url::parse(value).map_err(|error| {
        provider_error(
            JELLYFIN_PROVIDER_ID,
            "configure",
            ProviderErrorKind::PolicyBlocked,
            format!("Jellyfin base URL is invalid: {error}"),
            false,
        )
    })?;
    let is_https = url.scheme() == "https";
    let is_explicit_localhost_http = url.scheme() == "http" && url.host_str() == Some("localhost");
    if !is_https && !is_explicit_localhost_http {
        return Err(provider_error(
            JELLYFIN_PROVIDER_ID,
            "configure",
            ProviderErrorKind::PolicyBlocked,
            "Jellyfin must use HTTPS, except explicit http://localhost for a local self-hosted server",
            false,
        ));
    }
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(provider_error(
            JELLYFIN_PROVIDER_ID,
            "configure",
            ProviderErrorKind::PolicyBlocked,
            "Jellyfin base URL must contain a host and no credentials, query, or fragment",
            false,
        ));
    }
    let path = url.path().trim_end_matches('/');
    url.set_path(&format!("{path}/"));
    Ok(url)
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn origin_string(url: &Url) -> String {
    format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default())
        + &url
            .port()
            .map(|port| format!(":{port}"))
            .unwrap_or_default()
}

fn item_title(item: &Value) -> Option<String> {
    item.get("Name")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .map(str::to_string)
}

fn item_requires_drm(item: &Value) -> bool {
    ["IsProtected", "IsDrm", "RequiresDrm"]
        .iter()
        .any(|key| item.get(*key).and_then(Value::as_bool) == Some(true))
        || item
            .get("ProtectionType")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.eq_ignore_ascii_case("none") && !value.is_empty())
}

fn item_is_policy_blocked(item: &Value) -> bool {
    item.get("CanPlay").and_then(Value::as_bool) == Some(false)
        || item
            .get("UserData")
            .and_then(|data| data.get("CanPlay"))
            .and_then(Value::as_bool)
            == Some(false)
        || item
            .get("PlayAccess")
            .and_then(Value::as_str)
            .is_some_and(|access| matches!(access, "Denied" | "Blocked"))
}

fn terminal_resolution(
    episode: AnimeEpisodeIdentity,
    reason: &str,
    error_kind: ProviderErrorKind,
) -> AnimeResolveResponse {
    AnimeResolveResponse {
        episode,
        target: ResolvedTarget::Unsupported {
            reason: reason.to_string(),
            error_kind,
        },
    }
}

fn http_error(operation: &str, status: StatusCode) -> crate::domain::ProviderError {
    let (kind, retryable) = match status {
        StatusCode::UNAUTHORIZED => (ProviderErrorKind::AuthRequired, false),
        StatusCode::TOO_MANY_REQUESTS => (ProviderErrorKind::RateLimited, true),
        status if status.is_server_error() => (ProviderErrorKind::Network, true),
        _ => (ProviderErrorKind::Unsupported, false),
    };
    provider_error(
        JELLYFIN_PROVIDER_ID,
        operation,
        kind,
        format!("Jellyfin returned HTTP {status}"),
        retryable,
    )
}
