use crate::domain::{ProviderError, ProviderErrorKind, ProviderHealth, ResolvedTarget};
use serde::{Deserialize, Serialize};

use super::error::{provider_error, ProviderResult};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeEpisodeIdentity {
    /// Provider-owned stable ID. This is never an array position or a display label.
    pub provider_id: String,
    /// Provider-owned parent series ID.
    pub series_id: String,
    /// Provider-owned episode/item ID.
    pub episode_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSearchQuery {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSearchItem {
    pub provider_id: String,
    pub item_id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub synopsis: Option<String>,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeDetail {
    pub provider_id: String,
    pub item_id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub synopsis: Option<String>,
    pub artwork_url: Option<String>,
    pub genres: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeEpisode {
    pub identity: AnimeEpisodeIdentity,
    pub title: String,
    pub number: Option<u32>,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeResolveRequest {
    pub episode: AnimeEpisodeIdentity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeResolveResponse {
    pub episode: AnimeEpisodeIdentity,
    /// Anime adapters may return only native_hls, native_file, webview, external,
    /// or unsupported. `ensure_anime_playback_target` enforces this at the boundary.
    pub target: ResolvedTarget,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSearchResponse {
    pub items: Vec<AnimeSearchItem>,
    /// Per-provider errors are informational. A broken source must not discard
    /// successful search results from another source.
    pub failures: Vec<ProviderError>,
    pub provider_health: Vec<ProviderHealth>,
}

pub fn ensure_anime_playback_target(
    provider_id: &str,
    target: ResolvedTarget,
) -> ProviderResult<ResolvedTarget> {
    match target {
        ResolvedTarget::NativeHls { .. }
        | ResolvedTarget::NativeFile { .. }
        | ResolvedTarget::Webview { .. }
        | ResolvedTarget::External { .. }
        | ResolvedTarget::Unsupported { .. } => Ok(target),
        ResolvedTarget::ImagePages { .. } => Err(provider_error(
            provider_id,
            "resolve",
            ProviderErrorKind::Unsupported,
            "anime providers may not return image_pages playback targets",
            false,
        )),
    }
}
