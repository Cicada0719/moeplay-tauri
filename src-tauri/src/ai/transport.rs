use crate::ai::endpoint::{EndpointBinding, ValidatedEndpoint};
use crate::ai::error::{AiError, AiErrorKind, AiResult};
use crate::ai::provider::{AdapterHttpRequest, AdapterHttpResponse, CredentialRequirement};
use crate::secret_store::{SecretKind, SecretStore};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method, Request};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

const MAX_TRANSPORT_RESPONSE_BYTES: usize = 1_048_576;
const CANCELLATION_POLL_INTERVAL_MS: u64 = 20;

/// Internal credential source consumed only by the reqwest transport. Command
/// DTOs and provider adapters never carry credential material.
pub trait CredentialSource: Send + Sync {
    fn bearer_configured(&self, origin: &str) -> AiResult<bool>;
    fn bearer_secret(&self, origin: &str) -> AiResult<Option<String>>;
}

pub struct SecretStoreCredentialSource<'a> {
    store: &'a SecretStore,
}

impl<'a> SecretStoreCredentialSource<'a> {
    pub fn new(store: &'a SecretStore) -> Self {
        Self { store }
    }
}

impl CredentialSource for SecretStoreCredentialSource<'_> {
    fn bearer_configured(&self, origin: &str) -> AiResult<bool> {
        self.store
            .status(SecretKind::AiApiKey, Some(origin))
            .map(|status| status.configured)
            .map_err(|_| credential_store_error())
    }

    fn bearer_secret(&self, origin: &str) -> AiResult<Option<String>> {
        self.store
            .get(SecretKind::AiApiKey, Some(origin))
            .map_err(|_| credential_store_error())
    }
}

pub trait CancellationProbe: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct NeverCancelled;

impl CancellationProbe for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

pub type TransportFuture<'a> =
    Pin<Box<dyn Future<Output = AiResult<AdapterHttpResponse>> + Send + 'a>>;

pub trait AiHttpTransport: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn send<'a>(
        &'a self,
        provider_id: &'a str,
        endpoint: &'a ValidatedEndpoint,
        binding: &'a EndpointBinding,
        request: AdapterHttpRequest,
        credentials: &'a dyn CredentialSource,
        cancellation: &'a dyn CancellationProbe,
        timeout: Duration,
    ) -> TransportFuture<'a>;
}

#[derive(Debug, Clone)]
pub struct ReqwestAiTransport {
    client: Client,
}

impl ReqwestAiTransport {
    pub fn try_new() -> AiResult<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(30))
            .user_agent("MoePlay-AI-v2")
            .build()
            .map_err(|_| {
                AiError::new(
                    AiErrorKind::ProviderUnavailable,
                    "AI HTTP transport could not be initialized",
                    true,
                )
            })?;
        Ok(Self { client })
    }

    fn build_request(
        &self,
        provider_id: &str,
        endpoint: &ValidatedEndpoint,
        binding: &EndpointBinding,
        request: AdapterHttpRequest,
        credentials: &dyn CredentialSource,
    ) -> AiResult<Request> {
        binding.authorize_provider(provider_id, endpoint)?;
        endpoint.authorize_request_url(&request.url)?;

        let method = Method::from_bytes(request.method.as_bytes()).map_err(|_| {
            AiError::new(
                AiErrorKind::PolicyRejected,
                "AI adapter produced an invalid HTTP method",
                false,
            )
        })?;
        if method != Method::POST {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "AI structured generation permits POST requests only",
                false,
            ));
        }

        let headers = safe_headers(&request.headers)?;
        let mut builder = self
            .client
            .request(method, &request.url)
            .headers(headers)
            .json(&request.body);

        if request.credential == CredentialRequirement::BearerSecret {
            let secret = credentials
                .bearer_secret(&endpoint.origin)?
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    AiError::new(
                        AiErrorKind::NotConfigured,
                        "AI provider credential is not configured for the bound endpoint origin",
                        false,
                    )
                })?;
            // This is the only production boundary where credential material is
            // attached to an HTTP request. It is never copied into an adapter DTO.
            builder = builder.bearer_auth(secret);
        }

        builder.build().map_err(|_| {
            AiError::new(
                AiErrorKind::PolicyRejected,
                "AI HTTP request could not be constructed",
                false,
            )
        })
    }

    async fn execute_request(&self, request: Request) -> AiResult<AdapterHttpResponse> {
        let response = self.client.execute(request).await.map_err(|error| {
            if error.is_timeout() {
                AiError::new(AiErrorKind::Timeout, "AI provider request timed out", true)
            } else {
                AiError::new(
                    AiErrorKind::ProviderUnavailable,
                    "AI provider request failed",
                    true,
                )
            }
        })?;
        let status = response.status().as_u16();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_TRANSPORT_RESPONSE_BYTES as u64)
        {
            return Err(AiError::new(
                AiErrorKind::InvalidOutput,
                "AI provider response exceeded the transport size limit",
                false,
            ));
        }

        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| {
                AiError::new(
                    AiErrorKind::ProviderUnavailable,
                    "AI provider response stream failed",
                    true,
                )
            })?;
            if body.len().saturating_add(chunk.len()) > MAX_TRANSPORT_RESPONSE_BYTES {
                return Err(AiError::new(
                    AiErrorKind::InvalidOutput,
                    "AI provider response exceeded the transport size limit",
                    false,
                ));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(AdapterHttpResponse { status, body })
    }
}

impl AiHttpTransport for ReqwestAiTransport {
    #[allow(clippy::too_many_arguments)]
    fn send<'a>(
        &'a self,
        provider_id: &'a str,
        endpoint: &'a ValidatedEndpoint,
        binding: &'a EndpointBinding,
        request: AdapterHttpRequest,
        credentials: &'a dyn CredentialSource,
        cancellation: &'a dyn CancellationProbe,
        timeout: Duration,
    ) -> TransportFuture<'a> {
        Box::pin(async move {
            ensure_active(cancellation)?;
            let request =
                self.build_request(provider_id, endpoint, binding, request, credentials)?;
            let operation = tokio::time::timeout(timeout, self.execute_request(request));
            tokio::pin!(operation);

            tokio::select! {
                result = &mut operation => match result {
                    Ok(result) => result,
                    Err(_) => Err(AiError::new(
                        AiErrorKind::Timeout,
                        "AI provider request timed out",
                        true,
                    )),
                },
                _ = wait_for_cancellation(cancellation) => Err(AiError::new(
                    AiErrorKind::Cancelled,
                    "AI task was cancelled",
                    false,
                )),
            }
        })
    }
}

fn safe_headers(headers: &std::collections::BTreeMap<String, String>) -> AiResult<HeaderMap> {
    let mut safe = HeaderMap::new();
    for (name, value) in headers {
        let lowered = name.to_ascii_lowercase();
        if matches!(
            lowered.as_str(),
            "authorization"
                | "proxy-authorization"
                | "x-api-key"
                | "api-key"
                | "cookie"
                | "set-cookie"
        ) {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "AI adapter attempted to supply a sensitive transport header",
                false,
            ));
        }
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            AiError::new(
                AiErrorKind::PolicyRejected,
                "AI adapter produced an invalid HTTP header name",
                false,
            )
        })?;
        let value = HeaderValue::from_str(value).map_err(|_| {
            AiError::new(
                AiErrorKind::PolicyRejected,
                "AI adapter produced an invalid HTTP header value",
                false,
            )
        })?;
        safe.insert(name, value);
    }
    Ok(safe)
}

fn ensure_active(cancellation: &dyn CancellationProbe) -> AiResult<()> {
    if cancellation.is_cancelled() {
        Err(AiError::new(
            AiErrorKind::Cancelled,
            "AI task was cancelled",
            false,
        ))
    } else {
        Ok(())
    }
}

async fn wait_for_cancellation(cancellation: &dyn CancellationProbe) {
    loop {
        if cancellation.is_cancelled() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(CANCELLATION_POLL_INTERVAL_MS)).await;
    }
}

fn credential_store_error() -> AiError {
    AiError::new(
        AiErrorKind::ProviderUnavailable,
        "AI credential store is unavailable",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::{BackendError, SecretBackend};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct MemoryBackend {
        values: Mutex<HashMap<(String, String), String>>,
    }

    impl SecretBackend for MemoryBackend {
        fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), BackendError> {
            self.values.lock().unwrap().insert(
                (service.to_string(), account.to_string()),
                secret.to_string(),
            );
            Ok(())
        }

        fn get(&self, service: &str, account: &str) -> Result<String, BackendError> {
            self.values
                .lock()
                .unwrap()
                .get(&(service.to_string(), account.to_string()))
                .cloned()
                .ok_or(BackendError::Missing)
        }

        fn delete(&self, service: &str, account: &str) -> Result<(), BackendError> {
            self.values
                .lock()
                .unwrap()
                .remove(&(service.to_string(), account.to_string()))
                .map(|_| ())
                .ok_or(BackendError::Missing)
        }
    }

    struct AtomicCancellation(Arc<AtomicBool>);

    impl CancellationProbe for AtomicCancellation {
        fn is_cancelled(&self) -> bool {
            self.0.load(Ordering::Acquire)
        }
    }

    #[test]
    fn secret_store_value_is_added_only_to_the_built_reqwest_request() {
        let sentinel = "transport-boundary-secret";
        let origin = "https://api.example.test";
        let store = SecretStore::with_backend(Arc::new(MemoryBackend::default()));
        store
            .set(SecretKind::AiApiKey, Some(origin), sentinel)
            .unwrap();
        let credentials = SecretStoreCredentialSource::new(&store);
        let transport = ReqwestAiTransport::try_new().unwrap();
        let endpoint = ValidatedEndpoint::parse(
            "https://api.example.test/v1",
            crate::ai::contracts::AiProviderKind::OpenAiCompatible,
        )
        .unwrap();
        let binding = EndpointBinding {
            provider_id: "provider".to_string(),
            bound_origin: endpoint.origin.clone(),
        };
        let adapter_request = AdapterHttpRequest {
            method: "POST".to_string(),
            url: "https://api.example.test/v1/chat/completions".to_string(),
            headers: std::collections::BTreeMap::from([(
                "content-type".to_string(),
                "application/json".to_string(),
            )]),
            body: serde_json::json!({"model": "fixture"}),
            credential: CredentialRequirement::BearerSecret,
        };
        assert!(!serde_json::to_string(&adapter_request)
            .unwrap()
            .contains(sentinel));

        let request = transport
            .build_request(
                "provider",
                &endpoint,
                &binding,
                adapter_request,
                &credentials,
            )
            .unwrap();
        assert_eq!(
            request.headers().get("authorization").unwrap(),
            &HeaderValue::from_str(&format!("Bearer {sentinel}")).unwrap()
        );
    }

    #[tokio::test]
    async fn cancellation_drops_an_in_flight_reqwest_operation() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0_u8; 2048];
            let _ = socket.read(&mut buffer).await;
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .await;
        });

        let endpoint = ValidatedEndpoint::parse(
            &format!("http://{address}"),
            crate::ai::contracts::AiProviderKind::Ollama,
        )
        .unwrap();
        let binding = EndpointBinding {
            provider_id: "ollama".to_string(),
            bound_origin: endpoint.origin.clone(),
        };
        let request = AdapterHttpRequest {
            method: "POST".to_string(),
            url: format!("http://{address}/api/chat"),
            headers: std::collections::BTreeMap::new(),
            body: serde_json::json!({}),
            credential: CredentialRequirement::None,
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        let probe = AtomicCancellation(Arc::clone(&cancelled));
        let cancel_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(60)).await;
            cancelled.store(true, Ordering::Release);
        });
        let started = std::time::Instant::now();
        let error = ReqwestAiTransport::try_new()
            .unwrap()
            .send(
                "ollama",
                &endpoint,
                &binding,
                request,
                &NoCredentials,
                &probe,
                Duration::from_secs(5),
            )
            .await
            .unwrap_err();
        assert_eq!(error.kind, AiErrorKind::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(1));
        cancel_task.await.unwrap();
        server.abort();
    }

    struct NoCredentials;

    impl CredentialSource for NoCredentials {
        fn bearer_configured(&self, _origin: &str) -> AiResult<bool> {
            Ok(false)
        }

        fn bearer_secret(&self, _origin: &str) -> AiResult<Option<String>> {
            Ok(None)
        }
    }
}
