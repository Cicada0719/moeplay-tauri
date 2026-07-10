use std::{future::Future, pin::Pin};

use crate::domain::ProviderManifest;

use super::{
    AnimeDetail, AnimeEpisode, AnimeResolveRequest, AnimeResolveResponse, AnimeSearchItem,
    AnimeSearchQuery, ProviderResult,
};

pub type AdapterFuture<'a, T> = Pin<Box<dyn Future<Output = ProviderResult<T>> + Send + 'a>>;

/// Uniform provider contract. Every identifier returned from an adapter remains
/// provider-scoped; callers must never infer an episode identity from list order.
pub trait AnimeSourceAdapter: Send + Sync {
    fn manifest(&self) -> ProviderManifest;

    fn search<'a>(&'a self, query: AnimeSearchQuery) -> AdapterFuture<'a, Vec<AnimeSearchItem>>;

    fn detail<'a>(&'a self, item_id: &'a str) -> AdapterFuture<'a, AnimeDetail>;

    fn episodes<'a>(&'a self, series_id: &'a str) -> AdapterFuture<'a, Vec<AnimeEpisode>>;

    fn resolve<'a>(
        &'a self,
        request: AnimeResolveRequest,
    ) -> AdapterFuture<'a, AnimeResolveResponse>;
}
