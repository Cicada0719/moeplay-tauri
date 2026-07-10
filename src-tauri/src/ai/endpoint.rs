use crate::ai::contracts::AiProviderKind;
use crate::ai::error::{AiError, AiErrorKind, AiResult};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use url::{Host, Url};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointScope {
    Loopback,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedEndpoint {
    pub url: String,
    pub origin: String,
    pub scope: EndpointScope,
}

impl ValidatedEndpoint {
    pub fn parse(raw: &str, provider_kind: AiProviderKind) -> AiResult<Self> {
        let mut url = Url::parse(raw).map_err(|_| {
            AiError::new(
                AiErrorKind::PolicyRejected,
                "AI endpoint is not a valid absolute URL",
                false,
            )
        })?;

        if !url.username().is_empty() || url.password().is_some() {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "AI endpoint URL credentials are forbidden",
                false,
            ));
        }
        if url.query().is_some() {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "AI endpoint query parameters are forbidden",
                false,
            ));
        }
        if url.fragment().is_some() {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "AI endpoint fragments are forbidden",
                false,
            ));
        }

        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "AI endpoint must use HTTP or HTTPS",
                false,
            ));
        }

        let host = url.host().ok_or_else(|| {
            AiError::new(
                AiErrorKind::PolicyRejected,
                "AI endpoint must include a host",
                false,
            )
        })?;
        let scope = if is_loopback_host(host) {
            EndpointScope::Loopback
        } else {
            EndpointScope::Remote
        };

        if scheme == "http" && (scope != EndpointScope::Loopback || !provider_kind.is_local()) {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "plaintext HTTP is allowed only for an explicitly local provider on loopback",
                false,
            ));
        }

        // Normalize an empty path without changing an explicitly configured API path.
        if url.path().is_empty() {
            url.set_path("/");
        }
        let origin = url.origin().ascii_serialization();

        Ok(Self {
            url: url.to_string(),
            origin,
            scope,
        })
    }

    /// Authorize a concrete adapter request against the endpoint selected during
    /// provider resolution. Redirects are disabled by the transport, so every
    /// outbound request must retain this exact origin.
    pub fn authorize_request_url(&self, request_url: &str) -> AiResult<()> {
        let parsed = Url::parse(request_url).map_err(|_| {
            AiError::new(
                AiErrorKind::PolicyRejected,
                "AI adapter produced an invalid request URL",
                false,
            )
        })?;
        if !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.fragment().is_some()
            || !matches!(parsed.scheme(), "http" | "https")
        {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "AI adapter request URL violated endpoint policy",
                false,
            ));
        }
        if parsed.origin().ascii_serialization() != self.origin {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "AI adapter request attempted to leave the bound endpoint origin",
                false,
            ));
        }
        Ok(())
    }
}

fn is_loopback_host(host: Host<&str>) -> bool {
    match host {
        Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(address) => IpAddr::V4(address).is_loopback(),
        Host::Ipv6(address) => IpAddr::V6(address).is_loopback(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointBinding {
    pub provider_id: String,
    pub bound_origin: String,
}

impl EndpointBinding {
    pub fn authorize(&self, endpoint: &ValidatedEndpoint) -> AiResult<()> {
        if self.bound_origin != endpoint.origin {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "provider endpoint origin changed; explicit secret re-binding is required",
                false,
            ));
        }
        Ok(())
    }

    pub fn authorize_provider(
        &self,
        provider_id: &str,
        endpoint: &ValidatedEndpoint,
    ) -> AiResult<()> {
        if self.provider_id != provider_id {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "provider identity did not match the endpoint binding",
                false,
            ));
        }
        self.authorize(endpoint)
    }
}
