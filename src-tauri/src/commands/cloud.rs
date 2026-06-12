use std::path::PathBuf;

#[tauri::command]
pub fn backup_snapshot_local(snapshot_path: String, backup_dir: String) -> Result<String, String> {
    let dest = crate::cloud_save::backup_to_local(
        &PathBuf::from(&snapshot_path),
        &PathBuf::from(&backup_dir),
        5,
    )?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn test_webdav_connection(
    server_url: String,
    username: String,
    password: String,
) -> Result<bool, String> {
    let test_url = format!("{}/.moegame_test", server_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    match client
        .head(&test_url)
        .basic_auth(&username, Some(&password))
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
    {
        Ok(resp) => {
            Ok(resp.status().is_success() || resp.status() == reqwest::StatusCode::NOT_FOUND)
        }
        Err(e) => Err(format!("WebDAV connection failed: {}", e)),
    }
}
