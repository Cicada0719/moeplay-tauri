use std::collections::HashSet;

use crate::domain::{
    ProviderCapability, ProviderErrorKind, ProviderManifest, ProviderTrust, ResolvedTarget,
    ResourceKind,
};

use super::{
    provider_error, AdapterFuture, AnimeDetail, AnimeEpisode, AnimeEpisodeIdentity,
    AnimeResolveRequest, AnimeResolveResponse, AnimeSearchItem, AnimeSearchQuery,
    AnimeSourceAdapter, ProviderResult,
};

pub const LOCAL_MEDIA_PROVIDER_ID: &str = "local_media";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMediaEpisode {
    pub id: String,
    pub title: String,
    pub number: Option<u32>,
    /// An app-owned, user-selected local file path. It is returned only after
    /// matching the configured library entry exactly.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMediaSeries {
    pub id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub synopsis: Option<String>,
    pub artwork_url: Option<String>,
    pub genres: Vec<String>,
    pub episodes: Vec<LocalMediaEpisode>,
}

#[derive(Debug, Clone)]
pub struct LocalMediaAdapter {
    library: Vec<LocalMediaSeries>,
}

impl LocalMediaAdapter {
    pub fn try_new(library: Vec<LocalMediaSeries>) -> ProviderResult<Self> {
        let mut series_ids = HashSet::new();
        for series in &library {
            if series.id.trim().is_empty() || !series_ids.insert(series.id.as_str()) {
                return Err(provider_error(
                    LOCAL_MEDIA_PROVIDER_ID,
                    "configure",
                    ProviderErrorKind::ParseChanged,
                    "local media series IDs must be non-empty and unique",
                    false,
                ));
            }
            let mut episode_ids = HashSet::new();
            for episode in &series.episodes {
                if episode.id.trim().is_empty()
                    || episode.path.trim().is_empty()
                    || !episode_ids.insert(episode.id.as_str())
                {
                    return Err(provider_error(
                        LOCAL_MEDIA_PROVIDER_ID,
                        "configure",
                        ProviderErrorKind::ParseChanged,
                        "local media episode IDs and paths must be non-empty and unique per series",
                        false,
                    ));
                }
            }
        }
        Ok(Self { library })
    }

    fn series(&self, id: &str) -> ProviderResult<&LocalMediaSeries> {
        self.library
            .iter()
            .find(|series| series.id == id)
            .ok_or_else(|| {
                provider_error(
                    LOCAL_MEDIA_PROVIDER_ID,
                    "detail",
                    ProviderErrorKind::Unsupported,
                    "local media item was not found in the configured library",
                    false,
                )
            })
    }
}

impl AnimeSourceAdapter for LocalMediaAdapter {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest {
            id: LOCAL_MEDIA_PROVIDER_ID.to_string(),
            name: "Local media".to_string(),
            resource_kinds: vec![ResourceKind::Anime],
            capabilities: vec![
                ProviderCapability::Search,
                ProviderCapability::Detail,
                ProviderCapability::Children,
                ProviderCapability::Resolve,
            ],
            trust: ProviderTrust::BuiltIn,
            version: "batch2".to_string(),
            enabled: true,
            requires_auth: false,
            allowed_hosts: vec![],
        }
    }

    fn search<'a>(&'a self, query: AnimeSearchQuery) -> AdapterFuture<'a, Vec<AnimeSearchItem>> {
        Box::pin(async move {
            let needle = query.query.trim().to_lowercase();
            let limit = query.limit.unwrap_or(50);
            Ok(self
                .library
                .iter()
                .filter(|series| {
                    needle.is_empty()
                        || series.title.to_lowercase().contains(&needle)
                        || series
                            .original_title
                            .as_deref()
                            .is_some_and(|title| title.to_lowercase().contains(&needle))
                })
                .take(limit)
                .map(|series| AnimeSearchItem {
                    provider_id: LOCAL_MEDIA_PROVIDER_ID.to_string(),
                    item_id: series.id.clone(),
                    title: series.title.clone(),
                    original_title: series.original_title.clone(),
                    synopsis: series.synopsis.clone(),
                    artwork_url: series.artwork_url.clone(),
                })
                .collect())
        })
    }

    fn detail<'a>(&'a self, item_id: &'a str) -> AdapterFuture<'a, AnimeDetail> {
        Box::pin(async move {
            let series = self.series(item_id)?;
            Ok(AnimeDetail {
                provider_id: LOCAL_MEDIA_PROVIDER_ID.to_string(),
                item_id: series.id.clone(),
                title: series.title.clone(),
                original_title: series.original_title.clone(),
                synopsis: series.synopsis.clone(),
                artwork_url: series.artwork_url.clone(),
                genres: series.genres.clone(),
            })
        })
    }

    fn episodes<'a>(&'a self, series_id: &'a str) -> AdapterFuture<'a, Vec<AnimeEpisode>> {
        Box::pin(async move {
            let series = self.series(series_id)?;
            Ok(series
                .episodes
                .iter()
                .map(|episode| AnimeEpisode {
                    identity: AnimeEpisodeIdentity {
                        provider_id: LOCAL_MEDIA_PROVIDER_ID.to_string(),
                        series_id: series.id.clone(),
                        episode_id: episode.id.clone(),
                    },
                    title: episode.title.clone(),
                    number: episode.number,
                    artwork_url: series.artwork_url.clone(),
                })
                .collect())
        })
    }

    fn resolve<'a>(
        &'a self,
        request: AnimeResolveRequest,
    ) -> AdapterFuture<'a, AnimeResolveResponse> {
        Box::pin(async move {
            if request.episode.provider_id != LOCAL_MEDIA_PROVIDER_ID {
                return Err(provider_error(
                    LOCAL_MEDIA_PROVIDER_ID,
                    "resolve",
                    ProviderErrorKind::PolicyBlocked,
                    "a local media adapter cannot resolve an episode from another provider",
                    false,
                ));
            }
            let series = self.series(&request.episode.series_id)?;
            let episode = series
                .episodes
                .iter()
                .find(|episode| episode.id == request.episode.episode_id)
                .ok_or_else(|| {
                    provider_error(
                        LOCAL_MEDIA_PROVIDER_ID,
                        "resolve",
                        ProviderErrorKind::Unsupported,
                        "local media episode was not found in the configured library",
                        false,
                    )
                })?;

            Ok(AnimeResolveResponse {
                episode: request.episode,
                target: ResolvedTarget::NativeFile {
                    path: episode.path.clone(),
                },
            })
        })
    }
}
