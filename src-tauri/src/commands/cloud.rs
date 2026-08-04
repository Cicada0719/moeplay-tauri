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
