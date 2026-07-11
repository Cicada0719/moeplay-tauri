use crate::db::Database;
use crate::models::Settings;
use crate::secret_store::{SecretKind, SecretStore};
use tauri::State;

const SECRET_MIGRATION_FAILED: &str = "failed to migrate legacy settings credentials";

#[tauri::command]
pub fn get_settings(db: State<'_, Database>, store: State<'_, SecretStore>) -> Settings {
    get_settings_impl(db.inner(), store.inner())
}

#[tauri::command]
pub fn update_settings(
    db: State<'_, Database>,
    store: State<'_, SecretStore>,
    settings: Settings,
) -> Result<Settings, String> {
    update_settings_impl(db.inner(), store.inner(), settings)
}

#[tauri::command]
pub fn add_watch_dir(
    db: State<'_, Database>,
    store: State<'_, SecretStore>,
    dir: String,
) -> Result<Settings, String> {
    let mut settings = load_settings_with_secret_migration(db.inner(), store.inner())?;
    if !settings.watch_dirs.contains(&dir) {
        settings.watch_dirs.push(dir);
    }
    update_settings_impl(db.inner(), store.inner(), settings)
}

#[tauri::command]
pub fn remove_watch_dir(
    db: State<'_, Database>,
    store: State<'_, SecretStore>,
    dir: String,
) -> Result<Settings, String> {
    let mut settings = load_settings_with_secret_migration(db.inner(), store.inner())?;
    settings.watch_dirs.retain(|d| d != &dir);
    update_settings_impl(db.inner(), store.inner(), settings)
}

pub(crate) fn load_settings_with_secret_migration(
    db: &Database,
    store: &SecretStore,
) -> Result<Settings, String> {
    migrate_legacy_secrets(db, store, true)
}

fn get_settings_impl(db: &Database, store: &SecretStore) -> Settings {
    match migrate_legacy_secrets(db, store, false) {
        Ok(settings) => settings,
        Err(_) => db.get_settings().redacted(),
    }
}

fn update_settings_impl(
    db: &Database,
    store: &SecretStore,
    mut settings: Settings,
) -> Result<Settings, String> {
    // Migrate only credentials already persisted by an older version. Ordinary
    // settings IPC is never a credential write path, even if a stale or
    // malicious client still submits the legacy fields.
    load_settings_with_secret_migration(db, store)?;
    settings.redact_secrets();
    settings.normalize_appearance();
    db.update_settings(settings)
}

fn migrate_legacy_secrets(
    db: &Database,
    store: &SecretStore,
    fail_on_error: bool,
) -> Result<Settings, String> {
    let legacy_json = db.sqlite().get_setting("app_settings")?;
    let legacy_object = legacy_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|value| value.as_object().cloned());
    let had_legacy_ai = legacy_object
        .as_ref()
        .is_some_and(|object| object.contains_key("ai_api_key"));
    let had_legacy_steam = legacy_object
        .as_ref()
        .is_some_and(|object| object.contains_key("steam_api_key"));

    let mut persisted = db.get_settings();
    let legacy_ai_key = std::mem::take(&mut persisted.ai_api_key);
    let legacy_steam_key = persisted.steam_api_key.take();
    let mut migration_failed = false;

    if !legacy_ai_key.trim().is_empty()
        && !store_and_verify_secret(
            store,
            SecretKind::AiApiKey,
            Some(persisted.ai_api_url.as_str()),
            legacy_ai_key.trim(),
        )
    {
        migration_failed = true;
    }

    if let Some(legacy_steam_key) = legacy_steam_key.as_deref() {
        if !legacy_steam_key.trim().is_empty()
            && !store_and_verify_secret(
                store,
                SecretKind::SteamApiKey,
                None,
                legacy_steam_key.trim(),
            )
        {
            migration_failed = true;
        }
    }

    if migration_failed {
        // Keep the original DB JSON intact so the migration remains retryable.
        // The returned DTO is still redacted and the legacy fields never
        // serialize through current models.
        if fail_on_error {
            return Err(SECRET_MIGRATION_FAILED.to_string());
        }
        return Ok(persisted.redacted());
    }

    if had_legacy_ai || had_legacy_steam {
        // Both keyring writes were read back successfully (or the old values
        // were empty placeholders), so rewriting settings removes plaintext.
        persisted = db.update_settings(persisted.redacted())?;
    }

    Ok(persisted.redacted())
}

fn store_and_verify_secret(
    store: &SecretStore,
    kind: SecretKind,
    origin: Option<&str>,
    secret: &str,
) -> bool {
    store.set(kind, origin, secret).is_ok()
        && matches!(store.get(kind, origin), Ok(Some(stored)) if stored == secret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::{BackendError, SecretBackend};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    type CredentialKey = (String, String);

    #[derive(Default)]
    struct MemoryBackend {
        values: Mutex<HashMap<CredentialKey, String>>,
        fail_set: Mutex<bool>,
    }

    impl SecretBackend for MemoryBackend {
        fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), BackendError> {
            if *self.fail_set.lock().unwrap() {
                return Err(BackendError::Failed);
            }
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

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "moeplay_settings_secret_test_{}",
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_context() -> (TestDir, Database, SecretStore, Arc<MemoryBackend>) {
        let dir = TestDir::new();
        let db = Database::open_at(dir.path()).unwrap();
        let backend = Arc::new(MemoryBackend::default());
        let store = SecretStore::with_backend(backend.clone());
        (dir, db, store, backend)
    }

    fn write_legacy_settings(db: &Database, ai_key: Option<&str>, steam_key: Option<&str>) {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        if let Some(ai_key) = ai_key {
            object.insert("ai_api_key".to_string(), serde_json::json!(ai_key));
        }
        if let Some(steam_key) = steam_key {
            object.insert("steam_api_key".to_string(), serde_json::json!(steam_key));
        }
        db.sqlite()
            .set_setting("app_settings", &serde_json::to_string(&value).unwrap())
            .unwrap();
    }

    #[test]
    fn sentinel_credentials_migrate_and_never_cross_settings_or_export_dtos() {
        let (_dir, db, store, _backend) = test_context();
        let ai_sentinel = "SENTINEL_AI_KEY_MUST_NOT_ESCAPE";
        let steam_sentinel = "SENTINEL_STEAM_KEY_MUST_NOT_ESCAPE";
        write_legacy_settings(&db, Some(ai_sentinel), Some(steam_sentinel));

        let returned = get_settings_impl(&db, &store);
        assert!(returned.ai_api_key.is_empty());
        assert!(returned.steam_api_key.is_none());
        assert_eq!(
            store
                .get(SecretKind::AiApiKey, Some(returned.ai_api_url.as_str()))
                .unwrap()
                .as_deref(),
            Some(ai_sentinel)
        );
        assert_eq!(
            store.get(SecretKind::SteamApiKey, None).unwrap().as_deref(),
            Some(steam_sentinel)
        );

        let ipc_json = serde_json::to_string(&returned).unwrap();
        let export_json = serde_json::to_string(&db.export_data()).unwrap();
        let stored_json = db.sqlite().get_setting("app_settings").unwrap().unwrap();
        for json in [&ipc_json, &export_json, &stored_json] {
            assert!(!json.contains(ai_sentinel));
            assert!(!json.contains(steam_sentinel));
        }
    }

    #[test]
    fn migration_failure_is_redacted_and_update_refuses_to_drop_retryable_secret() {
        let (_dir, db, store, backend) = test_context();
        let sentinel = "SENTINEL_RETRYABLE_AI_KEY";
        write_legacy_settings(&db, Some(sentinel), None);
        *backend.fail_set.lock().unwrap() = true;

        let returned = get_settings_impl(&db, &store);
        assert!(returned.ai_api_key.is_empty());
        assert!(!serde_json::to_string(&returned).unwrap().contains(sentinel));
        assert_eq!(db.get_settings().ai_api_key, sentinel);

        let error = update_settings_impl(&db, &store, returned).unwrap_err();
        assert_eq!(error, SECRET_MIGRATION_FAILED);
        assert_eq!(db.get_settings().ai_api_key, sentinel);
    }

    #[test]
    fn empty_update_does_not_delete_existing_secret() {
        let (_dir, db, store, _backend) = test_context();
        store
            .set(
                SecretKind::AiApiKey,
                Some(Settings::default().ai_api_url.as_str()),
                "existing-secret",
            )
            .unwrap();

        let updated = update_settings_impl(&db, &store, Settings::default()).unwrap();
        assert!(updated.ai_api_key.is_empty());
        assert_eq!(
            store
                .get(SecretKind::AiApiKey, Some(updated.ai_api_url.as_str()))
                .unwrap()
                .as_deref(),
            Some("existing-secret")
        );
    }

    #[test]
    fn settings_update_ignores_legacy_secret_fields_and_never_serializes_them() {
        let (_dir, db, store, _backend) = test_context();
        let sentinel = "SENTINEL_SETTINGS_UPDATE_MUST_NOT_PERSIST";
        let mut submitted = Settings::default();
        submitted.ai_api_key = sentinel.to_string();
        submitted.steam_api_key = Some(sentinel.to_string());

        let updated = update_settings_impl(&db, &store, submitted).unwrap();
        assert!(updated.ai_api_key.is_empty());
        assert!(updated.steam_api_key.is_none());
        assert_eq!(
            store
                .get(SecretKind::AiApiKey, Some(updated.ai_api_url.as_str()))
                .unwrap(),
            None
        );
        assert_eq!(store.get(SecretKind::SteamApiKey, None).unwrap(), None);

        let stored_json = db.sqlite().get_setting("app_settings").unwrap().unwrap();
        let ipc_json = serde_json::to_string(&updated).unwrap();
        for json in [stored_json, ipc_json] {
            assert!(!json.contains(sentinel));
            assert!(!json.contains("ai_api_key"));
            assert!(!json.contains("steam_api_key"));
        }
    }
}
