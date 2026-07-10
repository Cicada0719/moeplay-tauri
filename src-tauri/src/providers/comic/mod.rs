//! Batch2 comic provider architecture.
//!
//! This module is intentionally not wired into `lib.rs` yet. The integration
//! request is listed in the Batch2 hand-off notes and the contract tests load
//! this module directly so the existing comic commands/store remain untouched.

mod dto;
mod error;
mod health;
mod http;
mod kavita;
mod komga;
mod local;
mod registry;
mod util;

pub use dto::*;
pub use error::*;
pub use health::*;
pub use http::{validate_base_url, AuthConfig, ComicHttpClient, ComicHttpConfig};
pub use kavita::KavitaConnector;
pub use komga::KomgaConnector;
pub use local::LocalComicAdapter;
pub use registry::*;

use crate::domain::{ProviderHealth, ProviderManifest, ResolvedTarget};
use std::future::Future;
use std::pin::Pin;

pub type ComicFuture<'a, T> = Pin<Box<dyn Future<Output = ComicResult<T>> + Send + 'a>>;

/// Unified read-only contract used by remote self-hosted sources and local
/// filesystem sources.
pub trait ComicSourceAdapter: Send + Sync {
    fn manifest(&self) -> ProviderManifest;
    fn health(&self, operation: &str) -> ProviderHealth;
    fn probe(&self) -> ComicFuture<'_, ProbeDto>;
    fn libraries(&self) -> ComicFuture<'_, Vec<LibraryDto>>;
    fn search(&self, request: SearchRequest) -> ComicFuture<'_, Vec<SeriesDto>>;
    fn detail(&self, series_id: String) -> ComicFuture<'_, SeriesDetailDto>;
    fn chapters(&self, series_id: String) -> ComicFuture<'_, Vec<ChapterDto>>;
    fn resolve(&self, request: ResolveRequest) -> ComicFuture<'_, ResolvedTarget>;
}
