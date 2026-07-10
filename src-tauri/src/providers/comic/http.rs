use super::{provider_error, ComicProviderError, ComicResult};
use crate::domain::ProviderErrorKind;
use reqwest::{header, Client, Method, StatusCode};
use serde_json::Value;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::Url;

#[derive(Debug, Clone)]
pub enum AuthConfig {
    None,
    Basic { username: String, password: String },
    Bearer(String),
    ApiKey(String),
}

#[derive(Debug, Clone)]
pub struct ComicHttpConfig {
    pub base_url: String,
    pub auth: AuthConfig,
}

pub struct ComicHttpClient {
    client: Client,
    base_url: Url,
    auth: AuthConfig,
}

impl ComicHttpClient {
    pub fn new(config: ComicHttpConfig) -> ComicResult<Self> {
        let base_url = validate_base_url(&config.base_url)?;
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|error| {
                ComicProviderError::InvalidConfig(format!("cannot build HTTP client: {error}"))
            })?;
        Ok(Self {
            client,
            base_url,
            auth: config.auth,
        })
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub fn endpoint(&self, relative_path: &str) -> ComicResult<Url> {
        if relative_path.is_empty() || relative_path.contains("..") || relative_path.contains("\\")
        {
            return Err(ComicProviderError::Security(
                "endpoint path is not fixed or safe".to_string(),
            ));
        }
        let mut url = self.base_url.clone();
        let prefix = url.path().trim_end_matches('/');
        let suffix = relative_path.trim_start_matches('/');
        url.set_path(&format!("{prefix}/{suffix}"));
        url.set_query(None);
        url.set_fragment(None);
        Ok(url)
    }

    pub fn same_origin(&self, url: &Url) -> bool {
        self.base_url.scheme() == url.scheme()
            && self.base_url.host_str() == url.host_str()
            && self.base_url.port_or_known_default() == url.port_or_known_default()
    }

    pub fn auth_headers_for(&self, url: &Url) -> Vec<(String, String)> {
        if !self.same_origin(url) {
            return Vec::new();
        }
        match &self.auth {
            AuthConfig::None => Vec::new(),
            AuthConfig::Basic { username, password } => {
                let token = base64_basic(username, password);
                vec![("authorization".into(), format!("Basic {token}"))]
            }
            AuthConfig::Bearer(token) => vec![("authorization".into(), format!("Bearer {token}"))],
            AuthConfig::ApiKey(token) => vec![("x-api-key".into(), token.clone())],
        }
    }

    pub async fn get_json(
        &self,
        provider_id: &str,
        operation: &str,
        relative_path: &str,
        query: &[(&str, String)],
    ) -> ComicResult<Value> {
        self.request_json(
            provider_id,
            operation,
            Method::GET,
            relative_path,
            query,
            None,
        )
        .await
    }

    pub async fn post_json(
        &self,
        provider_id: &str,
        operation: &str,
        relative_path: &str,
        query: &[(&str, String)],
        body: Value,
    ) -> ComicResult<Value> {
        self.request_json(
            provider_id,
            operation,
            Method::POST,
            relative_path,
            query,
            Some(body),
        )
        .await
    }

    async fn request_json(
        &self,
        provider_id: &str,
        operation: &str,
        method: Method,
        relative_path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> ComicResult<Value> {
        let mut url = self.endpoint(relative_path)?;
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(key, value)| (*key, value.as_str())));
        }
        let mut request = self.client.request(method, url.clone());
        for (key, value) in self.auth_headers_for(&url) {
            let header_name = header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| ComicProviderError::InvalidConfig(e.to_string()))?;
            let header_value = header::HeaderValue::from_str(&value)
                .map_err(|e| ComicProviderError::InvalidConfig(e.to_string()))?;
            request = request.header(header_name, header_value);
        }
        request = request.header(header::ACCEPT, "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.map_err(|error| {
            let kind = if error.is_timeout() {
                ProviderErrorKind::Timeout
            } else {
                ProviderErrorKind::Network
            };
            provider_error(provider_id, operation, kind, error.to_string(), true)
        })?;
        let status = response.status();
        if !status.is_success() {
            let kind = match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderErrorKind::AuthRequired,
                StatusCode::TOO_MANY_REQUESTS => ProviderErrorKind::RateLimited,
                _ if status.is_server_error() => ProviderErrorKind::Network,
                _ => ProviderErrorKind::ParseChanged,
            };
            let retryable = matches!(
                kind,
                ProviderErrorKind::Network
                    | ProviderErrorKind::RateLimited
                    | ProviderErrorKind::Timeout
            );
            return Err(provider_error(
                provider_id,
                operation,
                kind,
                format!("HTTP {status}"),
                retryable,
            ));
        }
        response.json::<Value>().await.map_err(|error| {
            provider_error(
                provider_id,
                operation,
                ProviderErrorKind::ParseChanged,
                error.to_string(),
                false,
            )
        })
    }
}

pub fn validate_base_url(raw: &str) -> ComicResult<Url> {
    let url =
        Url::parse(raw).map_err(|error| ComicProviderError::InvalidConfig(error.to_string()))?;
    if url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ComicProviderError::InvalidConfig(
            "base URL must not contain credentials, query, or fragment".to_string(),
        ));
    }
    let host = url.host_str().ok_or_else(|| {
        ComicProviderError::InvalidConfig("base URL must contain a host".to_string())
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ComicProviderError::InvalidConfig(
            "only HTTP(S) base URLs are supported".to_string(),
        ));
    }
    let loopback = is_loopback_host(host);
    if url.scheme() == "http" && !loopback {
        return Err(ComicProviderError::Security(
            "HTTP is allowed only for localhost/loopback".to_string(),
        ));
    }
    if is_blocked_host(host) && !(url.scheme() == "http" && loopback) {
        return Err(ComicProviderError::Security(
            "base URL resolves to a private, loopback, link-local, multicast, or local-only host"
                .to_string(),
        ));
    }
    Ok(url)
}

fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

fn is_blocked_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.ends_with(".local")
    {
        return true;
    }
    let Ok(ip) = host.parse::<IpAddr>() else {
        return false;
    };
    match ip {
        IpAddr::V4(ip) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip == Ipv4Addr::new(100, 64, 0, 0)
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip.is_unique_local()
                || is_ipv6_link_local(ip)
        }
    }
}

fn is_ipv6_link_local(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

fn base64_basic(username: &str, password: &str) -> String {
    // Basic auth is intentionally implemented without another dependency.
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = format!("{username}:{password}").into_bytes();
    let mut output = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let a = bytes[index];
        let b = bytes.get(index + 1).copied().unwrap_or(0);
        let c = bytes.get(index + 2).copied().unwrap_or(0);
        output.push(TABLE[(a >> 2) as usize] as char);
        output.push(TABLE[((a & 0x03) << 4 | b >> 4) as usize] as char);
        output.push(if index + 1 < bytes.len() {
            TABLE[((b & 0x0f) << 2 | c >> 6) as usize] as char
        } else {
            '='
        });
        output.push(if index + 2 < bytes.len() {
            TABLE[(c & 0x3f) as usize] as char
        } else {
            '='
        });
        index += 3;
    }
    output
}
