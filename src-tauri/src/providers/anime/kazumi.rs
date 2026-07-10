use crate::domain::{
    ProviderCapability, ProviderErrorKind, ProviderManifest, ProviderTrust, ResourceKind,
};

use super::{
    provider_error, AdapterFuture, AnimeDetail, AnimeEpisode, AnimeResolveRequest,
    AnimeResolveResponse, AnimeSearchItem, AnimeSearchQuery, AnimeSourceAdapter,
};

/// Compatibility-only marker for the existing Kazumi rule engine.
///
/// This deliberately does **not** import or duplicate `crate::anime` parsing,
/// XPath, rule, or request logic. An integration layer can later implement this
/// boundary by translating legacy command results into the Batch 2 DTOs.
#[derive(Debug, Clone)]
pub struct KazumiCompatibilityAdapter {
    provider_id: String,
    display_name: String,
}

impl KazumiCompatibilityAdapter {
    pub fn new(provider_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            display_name: display_name.into(),
        }
    }

    fn unavailable<T>(&self, operation: &'static str) -> super::ProviderResult<T> {
        Err(provider_error(
            self.provider_id.clone(),
            operation,
            ProviderErrorKind::Unsupported,
            "Kazumi compatibility bridge is not wired yet; use the legacy anime command path",
            false,
        ))
    }
}

impl AnimeSourceAdapter for KazumiCompatibilityAdapter {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest {
            id: self.provider_id.clone(),
            name: self.display_name.clone(),
            resource_kinds: vec![ResourceKind::Anime],
            capabilities: vec![
                ProviderCapability::Search,
                ProviderCapability::Detail,
                ProviderCapability::Children,
                ProviderCapability::Resolve,
            ],
            trust: ProviderTrust::UserConfigured,
            version: "compat-draft".to_string(),
            enabled: false,
            requires_auth: false,
            // Legacy rule hosts are not trusted at this new boundary until a
            // policy translation/allowlist integration is explicitly added.
            allowed_hosts: vec![],
        }
    }

    fn search<'a>(&'a self, _query: AnimeSearchQuery) -> AdapterFuture<'a, Vec<AnimeSearchItem>> {
        Box::pin(async move { self.unavailable("search") })
    }

    fn detail<'a>(&'a self, _item_id: &'a str) -> AdapterFuture<'a, AnimeDetail> {
        Box::pin(async move { self.unavailable("detail") })
    }

    fn episodes<'a>(&'a self, _series_id: &'a str) -> AdapterFuture<'a, Vec<AnimeEpisode>> {
        Box::pin(async move { self.unavailable("episodes") })
    }

    fn resolve<'a>(
        &'a self,
        _request: AnimeResolveRequest,
    ) -> AdapterFuture<'a, AnimeResolveResponse> {
        Box::pin(async move { self.unavailable("resolve") })
    }
}
