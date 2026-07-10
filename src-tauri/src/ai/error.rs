use crate::domain::{ProviderError, ProviderErrorKind};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiErrorKind {
    NotConfigured,
    Auth,
    RateLimited,
    BudgetExceeded,
    Timeout,
    InvalidOutput,
    ProviderUnavailable,
    Cancelled,
    PolicyRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("{kind:?}: {message}")]
#[serde(rename_all = "camelCase")]
pub struct AiError {
    pub kind: AiErrorKind,
    pub message: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
}

impl AiError {
    pub fn new(kind: AiErrorKind, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
            retry_after_ms: None,
        }
    }

    pub fn with_retry_after(mut self, retry_after_ms: u64) -> Self {
        self.retry_after_ms = Some(retry_after_ms);
        self
    }

    /// Reuses the repository's provider error vocabulary at integration boundaries.
    pub fn to_provider_error(
        &self,
        provider_id: impl Into<String>,
        operation: impl Into<String>,
    ) -> ProviderError {
        let kind = match self.kind {
            AiErrorKind::Auth => ProviderErrorKind::AuthRequired,
            AiErrorKind::RateLimited => ProviderErrorKind::RateLimited,
            AiErrorKind::Timeout => ProviderErrorKind::Timeout,
            AiErrorKind::InvalidOutput => ProviderErrorKind::ParseChanged,
            AiErrorKind::Cancelled => ProviderErrorKind::Cancelled,
            AiErrorKind::PolicyRejected | AiErrorKind::BudgetExceeded => {
                ProviderErrorKind::PolicyBlocked
            }
            AiErrorKind::NotConfigured => ProviderErrorKind::Unsupported,
            AiErrorKind::ProviderUnavailable => ProviderErrorKind::Network,
        };

        ProviderError {
            kind,
            message: self.message.clone(),
            retryable: self.retryable,
            retry_after_ms: self.retry_after_ms,
            provider_id: Some(provider_id.into()),
            operation: Some(operation.into()),
        }
    }
}

pub type AiResult<T> = Result<T, AiError>;
