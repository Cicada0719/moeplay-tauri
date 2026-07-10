use crate::secret_store::{SecretKind, SecretStatus, SecretStore};
use tauri::State;

const COMMAND_FAILURE: &str = "secret store command failed";

#[tauri::command]
pub async fn secret_status(
    store: State<'_, SecretStore>,
    kind: SecretKind,
    origin: Option<String>,
) -> Result<SecretStatus, String> {
    let store = store.inner().clone();
    run_blocking(move || store.status(kind, origin.as_deref())).await
}

#[tauri::command]
pub async fn secret_set(
    store: State<'_, SecretStore>,
    kind: SecretKind,
    origin: Option<String>,
    secret: String,
) -> Result<SecretStatus, String> {
    let store = store.inner().clone();
    run_blocking(move || store.set(kind, origin.as_deref(), &secret)).await
}

#[tauri::command]
pub async fn secret_delete(
    store: State<'_, SecretStore>,
    kind: SecretKind,
    origin: Option<String>,
) -> Result<SecretStatus, String> {
    let store = store.inner().clone();
    run_blocking(move || store.delete(kind, origin.as_deref())).await
}

async fn run_blocking<F>(operation: F) -> Result<SecretStatus, String>
where
    F: FnOnce() -> Result<SecretStatus, crate::secret_store::SecretStoreError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|_| COMMAND_FAILURE.to_string())?
        .map_err(|error| error.to_string())
}
