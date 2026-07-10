//! Batch 2 anime-provider boundary.
//!
//! The registry and command bridge are intentionally isolated from the legacy
//! anime page and player. Registration in the Tauri entrypoint is left to the
//! owning integration change.

mod adapter;
mod dto;
mod error;
mod jellyfin;
mod kazumi;
mod local_media;
mod orchestrator;
mod playback_proxy;
mod registry;
mod scan;

pub use adapter::{AdapterFuture, AnimeSourceAdapter};
pub use dto::{
    ensure_anime_playback_target, AnimeDetail, AnimeEpisode, AnimeEpisodeIdentity,
    AnimeResolveRequest, AnimeResolveResponse, AnimeSearchItem, AnimeSearchQuery,
    AnimeSearchResponse,
};
pub use error::{provider_error, ProviderResult};
pub use jellyfin::{validate_jellyfin_base_url, JellyfinConfig, JellyfinConnector};
pub use kazumi::KazumiCompatibilityAdapter;
pub use local_media::{LocalMediaAdapter, LocalMediaEpisode, LocalMediaSeries};
pub use orchestrator::{AnimeProviderOrchestrator, CircuitBreakerConfig};
pub use playback_proxy::protect_hls_target;
pub use registry::{
    AnimeLocalMediaEpisode, AnimeLocalMediaSeries, AnimeProviderConfig, AnimeProviderDescriptor,
    AnimeProviderKind, AnimeProviderRegistry,
};
pub use scan::{scan_local_media_directory, AnimeLocalMediaScanResult};
