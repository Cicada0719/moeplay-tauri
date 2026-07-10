use crate::domain::{ProviderError, ProviderErrorKind};

pub type ProviderResult<T> = Result<T, ProviderError>;

pub fn provider_error(
    provider_id: impl Into<String>,
    operation: impl Into<String>,
    kind: ProviderErrorKind,
    message: impl Into<String>,
    retryable: bool,
) -> ProviderError {
    ProviderError {
        kind,
        message: message.into(),
        retryable,
        retry_after_ms: None,
        provider_id: Some(provider_id.into()),
        operation: Some(operation.into()),
    }
}
