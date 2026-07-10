use crate::domain::{ProviderError, ProviderErrorKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComicProviderError {
    #[error("invalid comic provider configuration: {0}")]
    InvalidConfig(String),
    #[error("comic provider security policy blocked the request: {0}")]
    Security(String),
    #[error("comic provider circuit is open: {0}")]
    CircuitOpen(String),
    #[error("provider error: {0:?}")]
    Provider(ProviderError),
}

pub type ComicResult<T> = Result<T, ComicProviderError>;

impl ComicProviderError {
    pub fn provider_error(&self) -> ProviderError {
        match self {
            Self::Provider(error) => error.clone(),
            Self::InvalidConfig(message) => ProviderError {
                kind: ProviderErrorKind::PolicyBlocked,
                message: message.clone(),
                retryable: false,
                retry_after_ms: None,
                provider_id: None,
                operation: None,
            },
            Self::Security(message) => ProviderError {
                kind: ProviderErrorKind::PolicyBlocked,
                message: message.clone(),
                retryable: false,
                retry_after_ms: None,
                provider_id: None,
                operation: None,
            },
            Self::CircuitOpen(message) => ProviderError {
                kind: ProviderErrorKind::Network,
                message: message.clone(),
                retryable: true,
                retry_after_ms: Some(30_000),
                provider_id: None,
                operation: None,
            },
        }
    }
}

pub fn provider_error(
    provider_id: &str,
    operation: &str,
    kind: ProviderErrorKind,
    message: impl Into<String>,
    retryable: bool,
) -> ComicProviderError {
    ComicProviderError::Provider(ProviderError {
        kind,
        message: message.into(),
        retryable,
        retry_after_ms: None,
        provider_id: Some(provider_id.to_string()),
        operation: Some(operation.to_string()),
    })
}
