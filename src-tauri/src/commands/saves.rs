use crate::db::Database;
use crate::models::{Game, SaveBackup, SaveData, SaveInfo};
use crate::sync;
use std::{fs, path::PathBuf};
use tauri::State;

// ===== Save data =====

#[tauri::command]
pub fn update_save_data(
    db: State<'_, Database>,
    id: String,
    save_data: SaveData,
) -> Result<Game, String> {
    db.update_save_data(&id, save_data)
}

#[tauri::command]
pub fn set_save_dir(
    db: State<'_, Database>,
    id: String,
    save_dir: Option<String>,
) -> Result<Game, String> {
    db.set_save_dir(&id, save_dir)
}

#[tauri::command]
pub fn configure_auto_backup(
    db: State<'_, Database>,
    id: String,
    auto_backup: bool,
    interval_minutes: u32,
    max_backups: u32,
) -> Result<Game, String> {
    db.configure_auto_backup(&id, auto_backup, interval_minutes, max_backups)
}

#[tauri::command]
pub fn add_game_backup(
    db: State<'_, Database>,
    id: String,
    backup: SaveBackup,
) -> Result<Game, String> {
    db.add_backup(&id, backup)
}

#[tauri::command]
pub fn remove_game_backup(
    db: State<'_, Database>,
    id: String,
    backup_id: String,
) -> Result<Game, String> {
    db.remove_backup(&id, &backup_id)
}

#[tauri::command]
pub fn update_backup_note(
    db: State<'_, Database>,
    id: String,
    backup_id: String,
    note: Option<String>,
) -> Result<Game, String> {
    db.update_backup_note(&id, &backup_id, note)
}

#[tauri::command]
pub fn configure_cloud_sync(
    db: State<'_, Database>,
    id: String,
    cloud_sync: bool,
    cloud_provider: Option<String>,
) -> Result<Game, String> {
    db.configure_cloud_sync(&id, cloud_sync, cloud_provider)
}

// ===== Save files and snapshots =====

#[tauri::command]
pub fn get_game_saves(game_id: String, db: State<'_, Database>) -> Result<Vec<SaveInfo>, String> {
    let games = db.get_games();
    let game = games.iter().find(|g| g.id == game_id).ok_or("游戏不存在")?;

    let exe_path = PathBuf::from(&game.exe_path);
    let game_dir = exe_path.parent().ok_or("无法获取游戏目录")?;

    let save_dirs = vec![
        game_dir.join("Save"),
        game_dir.join("Saves"),
        game_dir.join("save"),
        game_dir.join("saves"),
        game_dir.join("UserData"),
        dirs::document_dir()
            .unwrap_or_default()
            .join("My Games")
            .join(&game.name),
        dirs::data_local_dir().unwrap_or_default().join(&game.name),
    ];

    let mut saves = vec![];
    for save_dir in save_dirs {
        if save_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&save_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let metadata = fs::metadata(&path).ok();
                        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                        let created = metadata
                            .and_then(|m| m.modified().ok())
                            .map(|t| {
                                chrono::DateTime::<chrono::Utc>::from(t)
                                    .format("%Y-%m-%d %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_default();
                        saves.push(SaveInfo {
                            name: path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_string(),
                            path: path.to_string_lossy().to_string(),
                            size,
                            created,
                        });
                    }
                }
            }
        }
    }

    Ok(saves)
}

#[tauri::command]
pub fn backup_save(save_path: String) -> Result<String, String> {
    let src = PathBuf::from(&save_path);
    if !src.exists() {
        return Err("存档文件不存在".to_string());
    }

    let backup_dir = dirs::data_dir()
        .unwrap_or_default()
        .join("moeplay")
        .join("saves");

    fs::create_dir_all(&backup_dir).ok();

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let file_name = src.file_name().and_then(|s| s.to_str()).unwrap_or("save");
    let backup_name = format!("{}_{}", timestamp, file_name);
    let backup_path = backup_dir.join(&backup_name);

    fs::copy(&src, &backup_path).map_err(|e| format!("备份失败: {}", e))?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn restore_save(backup_path: String, target_path: String) -> Result<(), String> {
    let src = PathBuf::from(&backup_path);
    let dst = PathBuf::from(&target_path);

    if !src.exists() {
        return Err("备份文件不存在".to_string());
    }

    fs::copy(&src, &dst).map_err(|e| format!("恢复失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn detect_save_candidates(
    db: State<'_, Database>,
    game_id: String,
) -> Result<Vec<sync::SaveCandidateDir>, String> {
    let game = db.get_game(&game_id)?;
    let game_dir = resolve_game_dir(&game)?;
    Ok(sync::detect_save_candidates(
        &game_dir,
        &game.name,
        game.metadata.developer.as_deref(),
        game.metadata.engine.as_deref(),
    ))
}

#[tauri::command]
pub fn scan_save_dir(save_dir: String) -> Result<Vec<sync::SaveInfo>, String> {
    let path = PathBuf::from(save_dir);
    if !path.is_dir() {
        return Err("存档目录不存在".to_string());
    }
    Ok(sync::scan_saves(&path))
}

#[tauri::command]
pub fn create_save_snapshot(
    db: State<'_, Database>,
    game_id: String,
    save_dir: Option<String>,
    note: Option<String>,
) -> Result<sync::SaveSnapshot, String> {
    let game = db.get_game(&game_id)?;
    let save_dir = resolve_game_save_dir(&game, save_dir)?;
    sync::create_snapshot(&game_id, &save_dir, note.as_deref())
}

#[tauri::command]
pub fn list_save_snapshots(game_id: String) -> Result<Vec<sync::SaveSnapshot>, String> {
    Ok(sync::list_snapshots(&game_id))
}

#[tauri::command]
pub fn restore_save_snapshot(
    db: State<'_, Database>,
    game_id: String,
    snapshot_path: String,
    save_dir: Option<String>,
    create_safety: Option<bool>,
) -> Result<(), String> {
    let game = db.get_game(&game_id)?;
    let save_dir = resolve_game_save_dir(&game, save_dir)?;
    sync::restore_snapshot(
        &game_id,
        &PathBuf::from(snapshot_path),
        &save_dir,
        create_safety.unwrap_or(true),
    )
}

#[tauri::command]
pub fn delete_save_snapshot(snapshot_path: String) -> Result<(), String> {
    sync::delete_snapshot(&PathBuf::from(snapshot_path))
}

#[tauri::command]
pub fn compare_save_snapshot(
    snapshot_path: String,
    save_dir: String,
) -> Result<sync::SnapshotDiff, String> {
    sync::compare_snapshot(&PathBuf::from(snapshot_path), &PathBuf::from(save_dir))
}

#[tauri::command]
pub fn detect_save_conflicts(
    local_dir: String,
    remote_dir: String,
) -> Result<Vec<sync::SaveConflict>, String> {
    let local = PathBuf::from(local_dir);
    let remote = PathBuf::from(remote_dir);
    if !local.is_dir() {
        return Err("本地目录不存在".to_string());
    }
    if !remote.is_dir() {
        return Err("远端目录不存在".to_string());
    }
    Ok(sync::detect_conflicts(&local, &remote))
}

#[tauri::command]
pub async fn sync_save_snapshots_to_cloud(
    game_id: String,
    config: sync::CloudSyncConfig,
) -> Result<u32, String> {
    sync::sync_snapshots_to_cloud(&game_id, &config).await
}

#[tauri::command]
pub fn restore_latest_save_snapshot_from_cloud(
    db: State<'_, Database>,
    game_id: String,
    cloud_dir: String,
    save_dir: Option<String>,
) -> Result<Option<sync::SaveSnapshot>, String> {
    let game = db.get_game(&game_id)?;
    let save_dir = resolve_game_save_dir(&game, save_dir)?;
    sync::restore_latest_snapshot_from_local_cloud(&game_id, &PathBuf::from(cloud_dir), &save_dir)
}

pub(crate) fn resolve_game_dir(game: &Game) -> Result<PathBuf, String> {
    if let Some(ref install_dir) = game.install_dir {
        let path = PathBuf::from(install_dir);
        if path.is_dir() {
            return Ok(path);
        }
    }

    let exe_path = PathBuf::from(&game.exe_path);
    exe_path
        .parent()
        .map(|p| p.to_path_buf())
        .filter(|p| p.is_dir())
        .ok_or_else(|| "无法获取游戏目录".to_string())
}

fn resolve_game_save_dir(game: &Game, custom_save_dir: Option<String>) -> Result<PathBuf, String> {
    if let Some(path) = custom_save_dir {
        let path = PathBuf::from(path);
        if path.is_dir() {
            return Ok(path);
        }
        return Err("指定存档目录不存在".to_string());
    }

    if let Some(ref configured) = game.save_data.save_dir {
        let path = PathBuf::from(configured);
        if path.is_dir() {
            return Ok(path);
        }
    }

    let game_dir = resolve_game_dir(game)?;
    sync::detect_save_candidates(
        &game_dir,
        &game.name,
        game.metadata.developer.as_deref(),
        game.metadata.engine.as_deref(),
    )
    .into_iter()
    .next()
    .map(|candidate| PathBuf::from(candidate.path))
    .ok_or_else(|| "未检测到存档目录".to_string())
}
