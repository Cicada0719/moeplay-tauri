#[allow(dead_code)]
#[path = "../src/commands/secrets.rs"]
mod secret_commands;
#[path = "../src/secret_store.rs"]
mod secret_store;

use secret_store::{BackendError, SecretBackend, SecretKind, SecretStore, SecretStoreError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type CredentialKey = (String, String);

#[derive(Default)]
struct MemoryBackend {
    values: Mutex<HashMap<CredentialKey, String>>,
    last_key: Mutex<Option<CredentialKey>>,
    forced_error: Mutex<Option<BackendError>>,
}

impl MemoryBackend {
    fn force_error(&self, error: BackendError) {
        *self.forced_error.lock().expect("forced error lock") = Some(error);
    }

    fn take_forced_error(&self) -> Result<(), BackendError> {
        match self.forced_error.lock().expect("forced error lock").take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn remember(&self, service: &str, account: &str) {
        *self.last_key.lock().expect("last key lock") =
            Some((service.to_owned(), account.to_owned()));
    }

    fn last_key(&self) -> CredentialKey {
        self.last_key
            .lock()
            .expect("last key lock")
            .clone()
            .expect("a backend call")
    }
}

impl SecretBackend for MemoryBackend {
    fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), BackendError> {
        self.remember(service, account);
        self.take_forced_error()?;
        self.values
            .lock()
            .expect("values lock")
            .insert((service.to_owned(), account.to_owned()), secret.to_owned());
        Ok(())
    }

    fn get(&self, service: &str, account: &str) -> Result<String, BackendError> {
        self.remember(service, account);
        self.take_forced_error()?;
        self.values
            .lock()
            .expect("values lock")
            .get(&(service.to_owned(), account.to_owned()))
            .cloned()
            .ok_or(BackendError::Missing)
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), BackendError> {
        self.remember(service, account);
        self.take_forced_error()?;
        match self
            .values
            .lock()
            .expect("values lock")
            .remove(&(service.to_owned(), account.to_owned()))
        {
            Some(_) => Ok(()),
            None => Err(BackendError::Missing),
        }
    }
}

fn memory_store() -> (SecretStore, Arc<MemoryBackend>) {
    let backend = Arc::new(MemoryBackend::default());
    let store = SecretStore::with_backend(backend.clone());
    (store, backend)
}

#[test]
fn whitelist_accepts_only_fixed_secret_kinds() {
    let allowed = [
        ("ai_api_key", SecretKind::AiApiKey),
        ("steam_api_key", SecretKind::SteamApiKey),
        ("bangumi_token", SecretKind::BangumiToken),
        ("picacg_token", SecretKind::PicacgToken),
        ("runtime_connector_token", SecretKind::RuntimeConnectorToken),
    ];

    for (wire_name, expected) in allowed {
        let parsed: SecretKind =
            serde_json::from_str(&format!("\"{wire_name}\"")).expect("allowed secret kind");
        assert_eq!(parsed, expected);
    }

    assert!(serde_json::from_str::<SecretKind>("\"custom_service\"").is_err());
    assert!(serde_json::from_str::<SecretKind>("{\"service\":\"x\",\"account\":\"y\"}").is_err());
}

#[test]
fn set_get_status_delete_lifecycle_uses_memory_only() {
    let (store, _) = memory_store();
    let kind = SecretKind::BangumiToken;

    assert!(!store.status(kind, None).expect("initial status").configured);
    assert_eq!(store.get(kind, None).expect("initial get"), None);

    let set_status = store.set(kind, None, "token-value").expect("set secret");
    assert!(set_status.configured);
    assert_eq!(
        store.get(kind, None).expect("get secret"),
        Some("token-value".to_owned())
    );
    assert!(
        store
            .status(kind, None)
            .expect("configured status")
            .configured
    );

    let deleted = store.delete(kind, None).expect("delete secret");
    assert!(!deleted.configured);
    assert_eq!(store.get(kind, None).expect("get after delete"), None);

    let deleted_again = store.delete(kind, None).expect("idempotent delete");
    assert!(!deleted_again.configured);
}

#[test]
fn ai_secrets_require_origin_and_do_not_cross_origins() {
    let (store, _) = memory_store();
    let kind = SecretKind::AiApiKey;

    assert_eq!(
        store.set(kind, None, "secret").unwrap_err(),
        SecretStoreError::InvalidOrigin
    );

    store
        .set(
            kind,
            Some("https://api.example.com/v1/chat?model=x"),
            "origin-a",
        )
        .expect("set origin A");

    assert_eq!(
        store
            .get(kind, Some("https://api.example.com/another/path"))
            .expect("same origin get"),
        Some("origin-a".to_owned())
    );
    assert_eq!(
        store
            .get(kind, Some("https://other.example.com/v1/chat"))
            .expect("other origin get"),
        None
    );
}

#[test]
fn service_is_stable_and_accounts_are_derived_not_caller_supplied() {
    let (store, backend) = memory_store();

    store
        .set(SecretKind::SteamApiKey, None, "steam-key")
        .expect("set global secret");
    let global_key = backend.last_key();
    assert_eq!(global_key.0, "com.moeplay.app.secret-store.v1");
    assert_eq!(global_key.1, "v1:steam_api_key:global");

    store
        .set(
            SecretKind::SteamApiKey,
            Some("HTTPS://API.EXAMPLE.COM:443/path"),
            "scoped-key",
        )
        .expect("set scoped secret");
    let scoped_key = backend.last_key();
    assert_eq!(scoped_key.0, global_key.0);
    assert!(scoped_key.1.starts_with("v1:steam_api_key:origin:"));
    assert_ne!(scoped_key.1, global_key.1);

    store
        .status(
            SecretKind::SteamApiKey,
            Some("https://api.example.com/other"),
        )
        .expect("normalized scoped status");
    assert_eq!(backend.last_key(), scoped_key);
}

#[test]
fn invalid_origins_and_empty_secrets_are_rejected() {
    let (store, _) = memory_store();

    for invalid in [
        "not-a-url",
        "file:///tmp/key",
        "https://user:pass@example.com/v1",
        "",
    ] {
        assert_eq!(
            store
                .status(SecretKind::AiApiKey, Some(invalid))
                .unwrap_err(),
            SecretStoreError::InvalidOrigin
        );
    }

    assert_eq!(
        store
            .set(SecretKind::BangumiToken, None, " \t\n")
            .unwrap_err(),
        SecretStoreError::EmptySecret
    );
}

#[test]
fn command_safe_status_serialization_contains_no_secret_material() {
    let (store, _) = memory_store();
    let secret = "do-not-return-this-secret";
    let status = store
        .set(SecretKind::PicacgToken, None, secret)
        .expect("set secret");

    let json = serde_json::to_string(&status).expect("serialize status");
    assert_eq!(json, r#"{"kind":"picacg_token","configured":true}"#);
    assert!(!json.contains(secret));
}

#[test]
fn backend_errors_are_redacted() {
    let (store, backend) = memory_store();
    let secret = "sensitive-value";
    let origin = "https://private.example.com/v1";

    backend.force_error(BackendError::Failed);
    let error = store
        .set(SecretKind::AiApiKey, Some(origin), secret)
        .unwrap_err()
        .to_string();

    assert_eq!(error, "credential store operation failed");
    assert!(!error.contains(secret));
    assert!(!error.contains(origin));
    assert!(!error.contains("com.moeplay.app"));

    backend.force_error(BackendError::Unavailable);
    assert_eq!(
        store
            .status(SecretKind::BangumiToken, None)
            .unwrap_err()
            .to_string(),
        "credential store is unavailable"
    );
}
