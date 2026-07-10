//! Secure credential storage backed by the operating system keyring.
//!
//! Callers select a fixed [`SecretKind`] and, when appropriate, an endpoint
//! origin. The OS keyring service and account names are always derived here;
//! arbitrary service/account access is intentionally not exposed.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use thiserror::Error;
use url::Url;

/// Stable OS credential-store service name. Do not change without a migration.
const SERVICE_NAME: &str = "com.moeplay.app.secret-store.v1";
const ACCOUNT_VERSION: &str = "v1";
const MAX_ORIGIN_LENGTH: usize = 2_048;

/// The only secret categories that MoePlay is allowed to persist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    AiApiKey,
    SteamApiKey,
    BangumiToken,
    PicacgToken,
    RuntimeConnectorToken,
}

impl SecretKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::AiApiKey => "ai_api_key",
            Self::SteamApiKey => "steam_api_key",
            Self::BangumiToken => "bangumi_token",
            Self::PicacgToken => "picacg_token",
            Self::RuntimeConnectorToken => "runtime_connector_token",
        }
    }

    fn requires_origin(self) -> bool {
        matches!(self, Self::AiApiKey)
    }
}

/// Public command result. Secret material is deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SecretStatus {
    pub kind: SecretKind,
    pub configured: bool,
}

/// Errors safe to cross the command boundary.
///
/// No keyring error, secret, origin, service, or account value is included in
/// the display text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SecretStoreError {
    #[error("secret value must not be empty")]
    EmptySecret,
    #[error("a valid http(s) origin is required for this secret")]
    InvalidOrigin,
    #[error("credential store is unavailable")]
    StoreUnavailable,
    #[error("credential store operation failed")]
    OperationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackendError {
    Missing,
    Unavailable,
    Failed,
}

/// Narrow injection seam used by tests and by the OS implementation below.
/// It remains crate-private so application code cannot bypass the whitelist.
pub(crate) trait SecretBackend: Send + Sync {
    fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), BackendError>;
    fn get(&self, service: &str, account: &str) -> Result<String, BackendError>;
    fn delete(&self, service: &str, account: &str) -> Result<(), BackendError>;
}

#[derive(Default)]
struct OsKeyringBackend;

impl OsKeyringBackend {
    fn entry(service: &str, account: &str) -> Result<keyring::Entry, BackendError> {
        keyring::Entry::new(service, account).map_err(map_keyring_error)
    }
}

impl SecretBackend for OsKeyringBackend {
    fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), BackendError> {
        Self::entry(service, account)?
            .set_password(secret)
            .map_err(map_keyring_error)
    }

    fn get(&self, service: &str, account: &str) -> Result<String, BackendError> {
        Self::entry(service, account)?
            .get_password()
            .map_err(map_keyring_error)
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), BackendError> {
        Self::entry(service, account)?
            .delete_credential()
            .map_err(map_keyring_error)
    }
}

fn map_keyring_error(error: keyring::Error) -> BackendError {
    match error {
        keyring::Error::NoEntry => BackendError::Missing,
        keyring::Error::NoDefaultStore | keyring::Error::NoStorageAccess(_) => {
            BackendError::Unavailable
        }
        _ => BackendError::Failed,
    }
}

/// Thread-safe application state for OS-backed secret storage.
#[derive(Clone)]
pub struct SecretStore {
    backend: Arc<dyn SecretBackend>,
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore {
    /// Construct a store backed by the current operating system credential manager.
    pub fn new() -> Self {
        Self {
            backend: Arc::new(OsKeyringBackend),
        }
    }

    /// Inject a backend without exposing arbitrary service/account access.
    #[cfg(test)]
    pub(crate) fn with_backend(backend: Arc<dyn SecretBackend>) -> Self {
        Self { backend }
    }

    /// Persist a secret and return only its configured status.
    pub(crate) fn set(
        &self,
        kind: SecretKind,
        origin: Option<&str>,
        secret: &str,
    ) -> Result<SecretStatus, SecretStoreError> {
        if secret.trim().is_empty() {
            return Err(SecretStoreError::EmptySecret);
        }

        let account = derive_account(kind, origin)?;
        self.backend
            .set(SERVICE_NAME, &account, secret)
            .map_err(map_backend_error)?;

        Ok(SecretStatus {
            kind,
            configured: true,
        })
    }

    /// Internal-only secret retrieval. Commands must never return this value.
    pub(crate) fn get(
        &self,
        kind: SecretKind,
        origin: Option<&str>,
    ) -> Result<Option<String>, SecretStoreError> {
        let account = derive_account(kind, origin)?;
        match self.backend.get(SERVICE_NAME, &account) {
            Ok(secret) => Ok(Some(secret)),
            Err(BackendError::Missing) => Ok(None),
            Err(error) => Err(map_backend_error(error)),
        }
    }

    /// Check whether a secret exists without exposing it.
    pub fn status(
        &self,
        kind: SecretKind,
        origin: Option<&str>,
    ) -> Result<SecretStatus, SecretStoreError> {
        Ok(SecretStatus {
            kind,
            configured: self.get(kind, origin)?.is_some(),
        })
    }

    /// Delete a secret. Deleting an already absent secret is idempotent.
    pub fn delete(
        &self,
        kind: SecretKind,
        origin: Option<&str>,
    ) -> Result<SecretStatus, SecretStoreError> {
        let account = derive_account(kind, origin)?;
        match self.backend.delete(SERVICE_NAME, &account) {
            Ok(()) | Err(BackendError::Missing) => Ok(SecretStatus {
                kind,
                configured: false,
            }),
            Err(error) => Err(map_backend_error(error)),
        }
    }
}

fn map_backend_error(error: BackendError) -> SecretStoreError {
    match error {
        BackendError::Unavailable => SecretStoreError::StoreUnavailable,
        BackendError::Missing | BackendError::Failed => SecretStoreError::OperationFailed,
    }
}

fn derive_account(kind: SecretKind, origin: Option<&str>) -> Result<String, SecretStoreError> {
    match origin {
        Some(origin) => {
            let normalized = normalize_origin(origin)?;
            let digest = Sha256::digest(normalized.as_bytes());
            Ok(format!(
                "{ACCOUNT_VERSION}:{}:origin:{}",
                kind.as_str(),
                hex::encode(digest)
            ))
        }
        None if kind.requires_origin() => Err(SecretStoreError::InvalidOrigin),
        None => Ok(format!("{ACCOUNT_VERSION}:{}:global", kind.as_str())),
    }
}

fn normalize_origin(input: &str) -> Result<String, SecretStoreError> {
    let input = input.trim();
    if input.is_empty() || input.len() > MAX_ORIGIN_LENGTH {
        return Err(SecretStoreError::InvalidOrigin);
    }

    let parsed = Url::parse(input).map_err(|_| SecretStoreError::InvalidOrigin)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(SecretStoreError::InvalidOrigin);
    }

    let origin = parsed.origin().ascii_serialization();
    if origin == "null" {
        return Err(SecretStoreError::InvalidOrigin);
    }
    Ok(origin)
}
