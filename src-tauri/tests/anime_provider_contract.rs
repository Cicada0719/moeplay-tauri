#![allow(dead_code, unused_imports)]

pub mod domain {
    pub use moeplay_lib::domain::*;
}

#[path = "../src/providers/anime/mod.rs"]
mod anime_provider;

use std::sync::Arc;

use anime_provider::{
    AdapterFuture, AnimeEpisodeIdentity, AnimeProviderOrchestrator, AnimeResolveRequest,
    AnimeSearchItem, AnimeSearchQuery, AnimeSourceAdapter, CircuitBreakerConfig, JellyfinConfig,
    JellyfinConnector, LocalMediaAdapter, LocalMediaEpisode, LocalMediaSeries,
};
use domain::{
    ProviderCapability, ProviderErrorKind, ProviderManifest, ProviderTrust, ResolvedTarget,
    ResourceKind,
};
use serde_json::Value;

const JELLYFIN_PLAYABLE: &str = include_str!("fixtures/anime_provider/jellyfin_playable.json");
const JELLYFIN_DRM: &str = include_str!("fixtures/anime_provider/jellyfin_drm.json");
const JELLYFIN_POLICY_BLOCKED: &str =
    include_str!("fixtures/anime_provider/jellyfin_policy_blocked.json");

#[derive(Debug)]
struct FailingSource;

impl AnimeSourceAdapter for FailingSource {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest {
            id: "fixture_failure".to_string(),
            name: "Fixture failure".to_string(),
            resource_kinds: vec![ResourceKind::Anime],
            capabilities: vec![ProviderCapability::Search],
            trust: ProviderTrust::BuiltIn,
            version: "test".to_string(),
            enabled: true,
            requires_auth: false,
            allowed_hosts: vec![],
        }
    }

    fn search<'a>(&'a self, _query: AnimeSearchQuery) -> AdapterFuture<'a, Vec<AnimeSearchItem>> {
        Box::pin(async {
            Err(anime_provider::provider_error(
                "fixture_failure",
                "search",
                ProviderErrorKind::Network,
                "fixture source is down",
                true,
            ))
        })
    }

    fn detail<'a>(&'a self, _item_id: &'a str) -> AdapterFuture<'a, anime_provider::AnimeDetail> {
        Box::pin(async { unreachable!("search is the only operation used by this fixture") })
    }

    fn episodes<'a>(
        &'a self,
        _series_id: &'a str,
    ) -> AdapterFuture<'a, Vec<anime_provider::AnimeEpisode>> {
        Box::pin(async { unreachable!("search is the only operation used by this fixture") })
    }

    fn resolve<'a>(
        &'a self,
        _request: AnimeResolveRequest,
    ) -> AdapterFuture<'a, anime_provider::AnimeResolveResponse> {
        Box::pin(async { unreachable!("search is the only operation used by this fixture") })
    }
}

fn local_media() -> LocalMediaAdapter {
    LocalMediaAdapter::try_new(vec![LocalMediaSeries {
        id: "series-1".to_string(),
        title: "Fixture Show".to_string(),
        original_title: None,
        synopsis: Some("fixture".to_string()),
        artwork_url: None,
        genres: vec!["Animation".to_string()],
        episodes: vec![
            LocalMediaEpisode {
                id: "episode-a".to_string(),
                title: "Episode".to_string(),
                number: Some(1),
                path: "C:\\fixture\\episode-a.mkv".to_string(),
            },
            LocalMediaEpisode {
                id: "episode-b".to_string(),
                title: "Episode".to_string(),
                number: Some(2),
                path: "C:\\fixture\\episode-b.mkv".to_string(),
            },
        ],
    }])
    .unwrap()
}

#[tokio::test]
async fn one_source_failure_does_not_block_other_search_results_and_opens_only_its_circuit() {
    let orchestrator = AnimeProviderOrchestrator::new(
        vec![Arc::new(FailingSource), Arc::new(local_media())],
        CircuitBreakerConfig {
            consecutive_failure_threshold: 1,
            open_for: std::time::Duration::from_secs(60),
        },
    );

    let first = orchestrator
        .search(AnimeSearchQuery {
            query: "fixture".to_string(),
            limit: None,
        })
        .await;
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].provider_id, "local_media");
    assert_eq!(first.failures.len(), 1);
    assert_eq!(
        first.failures[0].provider_id.as_deref(),
        Some("fixture_failure")
    );

    let second = orchestrator
        .search(AnimeSearchQuery {
            query: "fixture".to_string(),
            limit: None,
        })
        .await;
    assert_eq!(
        second.items.len(),
        1,
        "healthy source must keep serving search"
    );
    assert_eq!(
        second.failures.len(),
        1,
        "open circuit is reported per source"
    );
    assert!(second.failures[0].retry_after_ms.is_some());
    assert!(second
        .provider_health
        .iter()
        .any(|health| health.provider_id == "fixture_failure"
            && health.state == domain::ProviderHealthState::OpenCircuit));
}

#[tokio::test]
async fn selected_provider_search_does_not_dispatch_other_sources() {
    let orchestrator = AnimeProviderOrchestrator::new(
        vec![Arc::new(FailingSource), Arc::new(local_media())],
        CircuitBreakerConfig::default(),
    );
    let response = orchestrator
        .search_provider(
            "local_media",
            AnimeSearchQuery {
                query: "fixture".to_string(),
                limit: None,
            },
        )
        .await;

    assert_eq!(response.items.len(), 1);
    assert!(response.failures.is_empty());
    assert!(response
        .provider_health
        .iter()
        .all(|health| health.provider_id != "fixture_failure"));
}

#[tokio::test]
async fn episode_identity_is_provider_and_series_scoped_not_title_or_position() {
    let adapter = local_media();
    let episodes = adapter.episodes("series-1").await.unwrap();
    assert_eq!(
        episodes[0].title, episodes[1].title,
        "fixture labels intentionally collide"
    );
    assert_ne!(episodes[0].identity, episodes[1].identity);
    assert_eq!(episodes[1].identity.episode_id, "episode-b");

    let resolved = adapter
        .resolve(AnimeResolveRequest {
            episode: episodes[1].identity.clone(),
        })
        .await
        .unwrap();
    assert_eq!(resolved.episode.episode_id, "episode-b");
    assert_eq!(
        resolved.target,
        ResolvedTarget::NativeFile {
            path: "C:\\fixture\\episode-b.mkv".to_string()
        }
    );
}

#[test]
fn jellyfin_drm_and_policy_are_terminal_unsupported_targets() {
    let connector = JellyfinConnector::new(
        JellyfinConfig::new("https://media.example/jellyfin", "private-token").unwrap(),
    )
    .unwrap();
    let identity = |episode_id: &str| AnimeEpisodeIdentity {
        provider_id: "jellyfin".to_string(),
        series_id: "show-1".to_string(),
        episode_id: episode_id.to_string(),
    };

    let drm: Value = serde_json::from_str(JELLYFIN_DRM).unwrap();
    let drm_result = connector
        .resolve_item_value(identity("episode-drm"), &drm)
        .unwrap();
    assert!(matches!(
        drm_result.target,
        ResolvedTarget::Unsupported {
            error_kind: ProviderErrorKind::UnsupportedDrm,
            ..
        }
    ));

    let policy: Value = serde_json::from_str(JELLYFIN_POLICY_BLOCKED).unwrap();
    let policy_result = connector
        .resolve_item_value(identity("episode-policy"), &policy)
        .unwrap();
    assert!(matches!(
        policy_result.target,
        ResolvedTarget::Unsupported {
            error_kind: ProviderErrorKind::PolicyBlocked,
            ..
        }
    ));

    let playable: Value = serde_json::from_str(JELLYFIN_PLAYABLE).unwrap();
    let playable_result = connector
        .resolve_item_value(identity("episode-a"), &playable)
        .unwrap();
    match playable_result.target {
        ResolvedTarget::NativeHls { url, headers } => {
            assert!(url.starts_with("https://media.example/jellyfin/Videos/episode-a/master.m3u8"));
            assert_eq!(
                headers,
                vec![("X-Emby-Token".to_string(), "private-token".to_string())]
            );
        }
        target => panic!("expected native_hls, got {target:?}"),
    }
}

#[test]
fn jellyfin_url_and_authorization_boundaries_prevent_ssrf_and_cross_origin_auth() {
    assert!(JellyfinConfig::new("https://jellyfin.example/base", "token").is_ok());
    assert!(JellyfinConfig::new("http://localhost:8096", "token").is_ok());
    assert!(JellyfinConfig::new("http://127.0.0.1:8096", "token").is_err());
    assert!(JellyfinConfig::new("http://jellyfin.example", "token").is_err());
    assert!(JellyfinConfig::new("http://localhost.evil.example", "token").is_err());

    let connector = JellyfinConnector::new(
        JellyfinConfig::new("https://jellyfin.example:9443/base", "secret").unwrap(),
    )
    .unwrap();
    assert_eq!(connector.allowed_origin(), "https://jellyfin.example:9443");
    assert!(connector
        .endpoint_url("https://evil.example/collect")
        .is_err());
    assert!(connector.endpoint_url("//evil.example/collect").is_err());

    let same_origin = connector.endpoint_url("Items/1").unwrap();
    assert_eq!(
        connector
            .authorization_headers_for(&same_origin)
            .unwrap()
            .len(),
        1
    );
    let cross_origin = url::Url::parse("https://evil.example/Items/1").unwrap();
    assert!(connector.authorization_headers_for(&cross_origin).is_err());
    let port_changed = url::Url::parse("https://jellyfin.example/base/Items/1").unwrap();
    assert!(connector.authorization_headers_for(&port_changed).is_err());
}

#[test]
fn anime_resolution_boundary_rejects_non_playback_image_pages_targets() {
    let rejected = anime_provider::ensure_anime_playback_target(
        "fixture",
        ResolvedTarget::ImagePages {
            pages: vec!["https://images.example/page-1.jpg".to_string()],
            headers: vec![],
        },
    )
    .unwrap_err();
    assert_eq!(rejected.kind, ProviderErrorKind::Unsupported);

    assert!(anime_provider::ensure_anime_playback_target(
        "fixture",
        ResolvedTarget::External {
            url: "https://example.invalid/watch".to_string(),
            reason: "manual handoff".to_string(),
        },
    )
    .is_ok());
}
