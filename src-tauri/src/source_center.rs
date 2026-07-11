use crate::db_sqlite::SourcePreferenceRecord;
use crate::domain::{ProviderCapability, ProviderErrorKind, ProviderHealth, ProviderHealthState};
use crate::providers::anime::{AnimeProviderDescriptor, AnimeProviderKind, AnimeProviderRegistry};
use crate::providers::comic::{ComicProviderDescriptor, ComicProviderKind, ComicProviderRegistry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The media namespace is intentionally explicit. Preferences never cross media
/// types, even if a provider ID happens to be reused by a future runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMediaType {
    Anime,
    Comic,
    ExternalRuntime,
}

impl SourceMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Anime => "anime",
            Self::Comic => "comic",
            Self::ExternalRuntime => "external_runtime",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAuthState {
    NotRequired,
    Configured,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRuntimeState {
    Available,
    Unavailable,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReference {
    pub provider_id: String,
    pub media_type: SourceMediaType,
}

impl SourceReference {
    pub fn normalized(&self) -> Result<Self, String> {
        let provider_id = self.provider_id.trim();
        if provider_id.is_empty()
            || provider_id.len() > 200
            || !provider_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
        {
            return Err("source provider_id is invalid".to_string());
        }
        Ok(Self {
            provider_id: provider_id.to_owned(),
            media_type: self.media_type,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFailureSummary {
    pub code: ProviderErrorKind,
    /// A stable redacted explanation. Raw upstream messages never leave the
    /// provider layer through Source Center.
    pub message: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthSummary {
    pub state: ProviderHealthState,
    pub latency_ms: Option<f64>,
    pub last_checked_at: Option<String>,
    pub last_error_kind: Option<ProviderErrorKind>,
    pub last_failure: Option<SourceFailureSummary>,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDescriptor {
    pub provider_id: String,
    pub media_type: SourceMediaType,
    pub kind: String,
    pub name: String,
    pub capabilities: Vec<ProviderCapability>,
    pub enabled: bool,
    pub priority: i32,
    pub health: SourceHealthSummary,
    pub latency: Option<f64>,
    pub last_checked_at: Option<String>,
    pub auth_state: SourceAuthState,
    pub runtime_state: SourceRuntimeState,
}

impl SourceDescriptor {
    pub fn source_reference(&self) -> SourceReference {
        SourceReference {
            provider_id: self.provider_id.clone(),
            media_type: self.media_type,
        }
    }
}

/// Builds the frontend-safe Source Center projection from configured runtime
/// registries. Credentials, paths, origins and configuration blobs are never
/// copied into this projection.
pub fn list_configured_sources(
    anime: &AnimeProviderRegistry,
    comic: &ComicProviderRegistry,
    preferences: &[SourcePreferenceRecord],
    health_records: &[ProviderHealth],
) -> Result<Vec<SourceDescriptor>, String> {
    let preferences = preferences
        .iter()
        .map(|value| {
            (
                (value.provider_id.clone(), value.media_type.clone()),
                value.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut descriptors = Vec::new();
    for descriptor in anime
        .list()
        .map_err(|_| "configured anime sources are unavailable".to_string())?
    {
        let preference_key = (descriptor.id.clone(), "anime".to_string());
        descriptors.push(anime_descriptor(
            descriptor,
            preferences.get(&preference_key),
            health_records,
        ));
    }
    for descriptor in comic
        .list()
        .map_err(|_| "configured comic sources are unavailable".to_string())?
    {
        let preference_key = (descriptor.id.clone(), "comic".to_string());
        descriptors.push(comic_descriptor(
            descriptor,
            preferences.get(&preference_key),
            health_records,
        ));
    }

    descriptors.sort_by(|left, right| {
        left.media_type
            .cmp(&right.media_type)
            .then_with(|| right.enabled.cmp(&left.enabled))
            .then_with(|| right.priority.cmp(&left.priority))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });
    Ok(descriptors)
}

pub fn health_summary_for(provider_id: &str, records: &[ProviderHealth]) -> SourceHealthSummary {
    let matching = records
        .iter()
        .filter(|record| record.provider_id == provider_id)
        .collect::<Vec<_>>();
    let state = matching
        .iter()
        .map(|record| record.state)
        .max_by_key(|state| match state {
            ProviderHealthState::OpenCircuit => 4,
            ProviderHealthState::Degraded => 3,
            ProviderHealthState::Disabled => 2,
            ProviderHealthState::Healthy => 1,
            ProviderHealthState::Unknown => 0,
        })
        .unwrap_or(ProviderHealthState::Unknown);
    let latency_ms = matching
        .iter()
        .filter_map(|record| record.latency_ms_ema)
        .max_by(f64::total_cmp);
    let last_checked_at = matching
        .iter()
        .flat_map(|record| {
            [
                record.last_success_at.as_ref(),
                record.last_failure_at.as_ref(),
            ]
        })
        .flatten()
        .max()
        .cloned();
    let latest_failure = matching
        .iter()
        .filter_map(|record| record.last_failure_at.as_ref().map(|when| (when, record)))
        .max_by(|left, right| left.0.cmp(right.0))
        .map(|(_, record)| record);
    let last_error_kind = latest_failure.and_then(|record| record.last_error_kind);
    let last_failure = latest_failure.and_then(|record| {
        let code = record.last_error_kind?;
        let occurred_at = record.last_failure_at.clone()?;
        Some(SourceFailureSummary {
            code,
            message: "source operation failed".to_string(),
            occurred_at,
        })
    });
    SourceHealthSummary {
        state,
        latency_ms,
        last_checked_at,
        last_error_kind,
        last_failure,
        consecutive_failures: matching
            .iter()
            .map(|record| record.consecutive_failures)
            .max()
            .unwrap_or(0),
    }
}

fn anime_descriptor(
    descriptor: AnimeProviderDescriptor,
    preference: Option<&SourcePreferenceRecord>,
    health_records: &[ProviderHealth],
) -> SourceDescriptor {
    let enabled = preference
        .map(|value| value.enabled)
        .unwrap_or(descriptor.manifest.enabled);
    let priority = preference.map(|value| value.priority).unwrap_or(0);
    let health = health_summary_for(&descriptor.id, health_records);
    SourceDescriptor {
        provider_id: descriptor.id,
        media_type: SourceMediaType::Anime,
        kind: anime_kind(descriptor.kind).to_string(),
        name: descriptor.name,
        capabilities: descriptor.manifest.capabilities,
        enabled,
        priority,
        latency: health.latency_ms,
        last_checked_at: health.last_checked_at.clone(),
        health,
        auth_state: auth_state(
            descriptor.manifest.requires_auth,
            descriptor.secret_configured,
        ),
        runtime_state: SourceRuntimeState::Available,
    }
}

fn comic_descriptor(
    descriptor: ComicProviderDescriptor,
    preference: Option<&SourcePreferenceRecord>,
    health_records: &[ProviderHealth],
) -> SourceDescriptor {
    let enabled = preference
        .map(|value| value.enabled)
        .unwrap_or(descriptor.manifest.enabled);
    let priority = preference.map(|value| value.priority).unwrap_or(0);
    let health = health_summary_for(&descriptor.id, health_records);
    SourceDescriptor {
        provider_id: descriptor.id,
        media_type: SourceMediaType::Comic,
        kind: comic_kind(descriptor.kind).to_string(),
        name: descriptor.name,
        capabilities: descriptor.manifest.capabilities,
        enabled,
        priority,
        latency: health.latency_ms,
        last_checked_at: health.last_checked_at.clone(),
        health,
        auth_state: auth_state(
            descriptor.manifest.requires_auth,
            descriptor.secret_configured,
        ),
        runtime_state: SourceRuntimeState::Available,
    }
}

fn auth_state(requires_auth: bool, secret_configured: bool) -> SourceAuthState {
    if !requires_auth {
        SourceAuthState::NotRequired
    } else if secret_configured {
        SourceAuthState::Configured
    } else {
        SourceAuthState::Missing
    }
}

fn anime_kind(kind: AnimeProviderKind) -> &'static str {
    match kind {
        AnimeProviderKind::LocalMedia => "local_media",
        AnimeProviderKind::Jellyfin => "jellyfin",
    }
}

fn comic_kind(kind: ComicProviderKind) -> &'static str {
    match kind {
        ComicProviderKind::Local => "local",
        ComicProviderKind::Komga => "komga",
        ComicProviderKind::Kavita => "kavita",
    }
}
