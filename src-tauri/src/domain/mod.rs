use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Game,
    Anime,
    Comic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapability {
    Probe,
    Search,
    Detail,
    Children,
    Resolve,
    ProgressRead,
    ProgressWrite,
    Download,
    Verify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTrust {
    BuiltIn,
    PublicApi,
    UserConfigured,
    SelfHosted,
    CatalogOnly,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderManifest {
    pub id: String,
    pub name: String,
    pub resource_kinds: Vec<ResourceKind>,
    pub capabilities: Vec<ProviderCapability>,
    pub trust: ProviderTrust,
    pub version: String,
    pub enabled: bool,
    pub requires_auth: bool,
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorKind {
    Network,
    Timeout,
    RateLimited,
    AuthRequired,
    CaptchaRequired,
    ParseChanged,
    GeoBlocked,
    EmbedBlocked,
    UnsupportedDrm,
    PolicyBlocked,
    Cancelled,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderError {
    pub kind: ProviderErrorKind,
    pub message: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
    pub provider_id: Option<String>,
    pub operation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderHealthState {
    Healthy,
    Degraded,
    OpenCircuit,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderHealth {
    pub provider_id: String,
    pub operation: String,
    pub state: ProviderHealthState,
    pub success_count: u64,
    pub failure_count: u64,
    pub consecutive_failures: u32,
    pub latency_ms_ema: Option<f64>,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub circuit_open_until: Option<String>,
    pub last_error_kind: Option<ProviderErrorKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResolvedTarget {
    NativeHls {
        url: String,
        headers: Vec<(String, String)>,
    },
    NativeFile {
        path: String,
    },
    ImagePages {
        pages: Vec<String>,
        headers: Vec<(String, String)>,
    },
    Webview {
        url: String,
        allowed_hosts: Vec<String>,
    },
    External {
        url: String,
        reason: String,
    },
    Unsupported {
        reason: String,
        error_kind: ProviderErrorKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEventType {
    Started,
    Progressed,
    Completed,
    Rated,
    Favorited,
    Imported,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: String,
    pub resource_kind: ResourceKind,
    pub resource_id: String,
    pub event_type: ActivityEventType,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<u64>,
    pub provider_id: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressRecord {
    pub resource_kind: ResourceKind,
    pub resource_id: String,
    pub provider_id: Option<String>,
    pub position: Value,
    pub updated_at: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundJobStatus {
    Queued,
    Running,
    Paused,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundJob {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub status: BackgroundJobStatus,
    pub progress: f32,
    pub created_at: String,
    pub updated_at: String,
    pub error: Option<ProviderError>,
    pub metadata: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_error_uses_stable_wire_names() {
        let error = ProviderError {
            kind: ProviderErrorKind::RateLimited,
            message: "rate limited".into(),
            retryable: true,
            retry_after_ms: Some(1_000),
            provider_id: Some("fixture".into()),
            operation: Some("search".into()),
        };
        let value = serde_json::to_value(error).unwrap();
        assert_eq!(value["kind"], "rate_limited");
        assert_eq!(value["retryAfterMs"], 1_000);
    }

    #[test]
    fn resolved_target_is_explicitly_tagged() {
        let target = ResolvedTarget::Unsupported {
            reason: "DRM protected".into(),
            error_kind: ProviderErrorKind::UnsupportedDrm,
        };
        let value = serde_json::to_value(target).unwrap();
        assert_eq!(value["mode"], "unsupported");
        assert_eq!(value["errorKind"], "unsupported_drm");
    }
}
