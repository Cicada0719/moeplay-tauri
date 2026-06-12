// 存档探测 + 云同步 + 备份管理
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveCandidateDir {
    pub path: String,
    pub category: String,
    pub score: i32,
    pub write_count: u32,
    pub last_write_time: Option<String>,
    pub file_count: u32,
    pub total_size_bytes: u64,
    pub matched_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSnapshot {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub created_at: String,
    pub file_size_bytes: u64,
    pub note: Option<String>,
    pub file_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    pub unchanged: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConflict {
    pub relative_path: String,
    pub local_path: String,
    pub remote_path: String,
    pub local_modified: Option<String>,
    pub remote_modified: Option<String>,
    pub local_size: u64,
    pub remote_size: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncConfig {
    pub enabled: bool,
    pub provider: CloudProvider,
    pub server_url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub sync_directory: String,
    pub auto_sync: bool,
    pub sync_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    None,
    WebDAV,
    AliyunOSS,
    LocalFolder,
}

const SAVE_KEYWORDS: &[&str] = &[
    "save",
    "savegame",
    "savedata",
    "sav",
    "存档",
    "セーブ",
    "userdata",
    "user",
    "profile",
    "progress",
    "record",
];

const EXCLUDE_KEYWORDS: &[&str] = &[
    "temp",
    "tmp",
    "cache",
    "log",
    "crash",
    "update",
    "backup",
    "uninstall",
    "redist",
    "directx",
    "vcredist",
    "__pycache__",
];

const SAVE_EXTENSIONS: &[&str] = &[
    "sav", "save", "dat", "json", "xml", "ini", "cfg", "conf", "bin", "slot", "savegame",
    "rpgsave", "rvdata", "rvdata2", "rxdata", "ksd", "ksd_", "asd", "global",
];

impl Default for CloudSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: CloudProvider::None,
            server_url: None,
            username: None,
            password: None,
            sync_directory: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("MoeGame")
                .join("CloudSync")
                .to_string_lossy()
                .to_string(),
            auto_sync: false,
            sync_interval_seconds: 3600,
        }
    }
}

/// 探测游戏存档目录。
///
/// 兼容旧调用：只返回路径；完整候选信息请使用 `detect_save_candidates`。
pub fn detect_save_dirs(game_dir: &Path, game_name: &str) -> Vec<PathBuf> {
    detect_save_candidates(game_dir, game_name, None, None)
        .into_iter()
        .map(|c| PathBuf::from(c.path))
        .collect()
}

/// 高级存档目录探测：结合引擎特定路径库、游戏目录、用户目录和名称匹配打分。
pub fn detect_save_candidates(
    game_dir: &Path,
    game_name: &str,
    developer: Option<&str>,
    engine: Option<&str>,
) -> Vec<SaveCandidateDir> {
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (path, category, base_score, rule) in
        candidate_paths(game_dir, game_name, developer, engine)
    {
        if !path.is_dir() {
            continue;
        }

        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        let key = canonical.to_string_lossy().to_lowercase();
        if !seen.insert(key) {
            continue;
        }

        let (file_count, total_size, last_write_time) = dir_stats(&canonical, true);
        if file_count == 0 {
            continue;
        }

        let mut score = base_score + path_score(&canonical, game_dir, game_name, developer);
        if contains_save_like_files(&canonical) {
            score += 15;
        }
        score = score.clamp(0, 100);

        candidates.push(SaveCandidateDir {
            path: canonical.to_string_lossy().to_string(),
            category,
            score,
            write_count: 0,
            last_write_time: last_write_time.map(format_datetime),
            file_count,
            total_size_bytes: total_size,
            matched_rule: rule,
        });
    }

    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.file_count.cmp(&a.file_count))
    });
    candidates.truncate(16);
    candidates
}

fn candidate_paths(
    game_dir: &Path,
    game_name: &str,
    developer: Option<&str>,
    engine: Option<&str>,
) -> Vec<(PathBuf, String, i32, String)> {
    let mut paths = Vec::new();

    for name in [
        "savedata",
        "save",
        "saves",
        "SaveData",
        "Save",
        "Saves",
        "UserData",
        "profile",
        "profiles",
        "www/save",
        "game/saves",
    ] {
        paths.push((
            game_dir.join(name),
            "GameDir".to_string(),
            45,
            format!("game_dir:{}", name),
        ));
    }

    let normalized_game = clean_name(game_name);
    let normalized_dev = developer.map(clean_name).unwrap_or_default();
    let mut name_variants = vec![game_name.to_string(), normalized_game.clone()];
    if let Some(dev) = developer {
        name_variants.push(dev.to_string());
    }
    if !normalized_dev.is_empty() {
        name_variants.push(normalized_dev.clone());
    }
    name_variants.retain(|s| !s.trim().is_empty());
    name_variants.sort();
    name_variants.dedup();

    if let Some(doc) = dirs::document_dir() {
        for variant in &name_variants {
            paths.push((
                doc.join("My Games").join(variant),
                "UserDir".to_string(),
                55,
                "documents_my_games".to_string(),
            ));
            paths.push((
                doc.join(variant),
                "UserDir".to_string(),
                35,
                "documents_name".to_string(),
            ));
        }
    }

    for root in [dirs::data_dir(), dirs::data_local_dir()]
        .into_iter()
        .flatten()
    {
        for variant in &name_variants {
            paths.push((
                root.join(variant),
                "UserDir".to_string(),
                50,
                "appdata_name".to_string(),
            ));
            paths.push((
                root.join("MoeGame").join(variant),
                "UserDir".to_string(),
                35,
                "moegame_appdata".to_string(),
            ));
        }
    }

    if let Some(home) = dirs::home_dir() {
        for variant in &name_variants {
            paths.push((
                home.join("Saved Games").join(variant),
                "UserDir".to_string(),
                50,
                "saved_games".to_string(),
            ));
        }
    }

    paths.extend(engine_candidate_paths(
        game_dir, game_name, developer, engine,
    ));
    paths
}

fn engine_candidate_paths(
    game_dir: &Path,
    game_name: &str,
    developer: Option<&str>,
    engine: Option<&str>,
) -> Vec<(PathBuf, String, i32, String)> {
    let mut paths = Vec::new();
    let engine = engine.unwrap_or_default().to_lowercase();
    let game = clean_name(game_name);
    let developer = developer.map(clean_name).unwrap_or_default();

    let add_local_named =
        |paths: &mut Vec<(PathBuf, String, i32, String)>, rule: &str, score: i32| {
            if let Some(local) = dirs::data_local_dir() {
                if !game.is_empty() {
                    paths.push((
                        local.join(&game),
                        "Engine".to_string(),
                        score,
                        rule.to_string(),
                    ));
                }
                if !developer.is_empty() && !game.is_empty() {
                    paths.push((
                        local.join(&developer).join(&game),
                        "Engine".to_string(),
                        score,
                        rule.to_string(),
                    ));
                }
            }
        };

    if engine.contains("ren") || engine.contains("renpy") || engine.contains("ren'py") {
        paths.push((
            game_dir.join("game").join("saves"),
            "Engine".to_string(),
            75,
            "renpy_game_saves".to_string(),
        ));
        if let Some(data) = dirs::data_dir() {
            paths.push((
                data.join("RenPy").join(&game),
                "Engine".to_string(),
                70,
                "renpy_appdata".to_string(),
            ));
        }
    }

    if engine.contains("rpg") || game_dir.join("www").join("save").exists() {
        paths.push((
            game_dir.join("www").join("save"),
            "Engine".to_string(),
            80,
            "rpg_maker_mv_mz".to_string(),
        ));
        paths.push((
            game_dir.join("save"),
            "Engine".to_string(),
            60,
            "rpg_maker_legacy".to_string(),
        ));
    }

    if engine.contains("tyrano") || game_dir.join("tyrano").exists() {
        paths.push((
            game_dir.join("data").join("save"),
            "Engine".to_string(),
            70,
            "tyranoscript_data_save".to_string(),
        ));
        add_local_named(&mut paths, "tyranoscript_localappdata", 60);
    }

    if engine.contains("unity") || game_dir.join("UnityPlayer.dll").exists() {
        add_local_named(&mut paths, "unity_persistent_data", 65);
        if let Some(local_low) =
            dirs::data_local_dir().and_then(|p| p.parent().map(|p| p.join("LocalLow")))
        {
            if !developer.is_empty() && !game.is_empty() {
                paths.push((
                    local_low.join(&developer).join(&game),
                    "Engine".to_string(),
                    75,
                    "unity_locallow".to_string(),
                ));
            }
        }
    }

    if engine.contains("kirikiri") || engine.contains("krkr") || game_dir.join("data.xp3").exists()
    {
        paths.push((
            game_dir.join("savedata"),
            "Engine".to_string(),
            80,
            "kirikiri_savedata".to_string(),
        ));
        paths.push((
            game_dir.join("SaveData"),
            "Engine".to_string(),
            80,
            "kirikiri_SaveData".to_string(),
        ));
    }

    if engine.contains("nscripter")
        || engine.contains("onscripter")
        || game_dir.join("nscript.dat").exists()
    {
        paths.push((
            game_dir.join("save"),
            "Engine".to_string(),
            70,
            "nscripter_save".to_string(),
        ));
    }

    paths
}

/// 扫描存档文件
pub fn scan_saves(save_dir: &Path) -> Vec<SaveInfo> {
    let mut saves = Vec::new();
    if let Ok(entries) = fs::read_dir(save_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path) {
                    saves.push(SaveInfo {
                        name: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        path: path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        created: metadata
                            .modified()
                            .ok()
                            .map(|time| {
                                DateTime::<Utc>::from(time)
                                    .format("%Y-%m-%d %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_default(),
                    });
                }
            } else if path.is_dir() {
                // 递归扫描子目录
                saves.extend(scan_saves(&path));
            }
        }
    }
    saves
}

/// 创建备份
pub fn create_backup(
    source_dir: &std::path::Path,
    backup_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_name = format!("backup_{}.zip", timestamp);
    let backup_path = backup_dir.join(&backup_name);

    std::fs::create_dir_all(backup_dir).map_err(|e| e.to_string())?;

    // 创建 zip 文件
    let file = std::fs::File::create(&backup_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    add_dir_to_zip(&mut zip, source_dir, source_dir, &options)?;
    zip.finish().map_err(|e| e.to_string())?;

    Ok(backup_path)
}

/// 恢复备份
pub fn restore_backup(
    backup_path: &std::path::Path,
    target_dir: &std::path::Path,
) -> Result<(), String> {
    let file = std::fs::File::open(backup_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = target_dir.join(file.mangled_name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = std::fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// 列表备份文件
pub fn list_backups(backup_dir: &std::path::Path) -> Vec<SaveInfo> {
    let mut backups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "zip") {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    backups.push(SaveInfo {
                        name: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        path: path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        created: "".to_string(),
                    });
                }
            }
        }
    }
    // 按名称排序（最新的在前）
    backups.sort_by(|a, b| b.name.cmp(&a.name));
    backups
}

/// 清理旧备份（保留最新 N 个）
pub fn prune_old_backups(backup_dir: &std::path::Path, keep_count: usize) -> Result<(), String> {
    let mut backups = list_backups(backup_dir);
    if backups.len() <= keep_count {
        return Ok(());
    }
    // 保留最新的 keep_count 个
    backups.sort_by(|a, b| b.name.cmp(&a.name));
    for backup in backups.iter().skip(keep_count) {
        std::fs::remove_file(&backup.path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 获取游戏快照目录。
pub fn snapshot_dir(game_id: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("moeplay")
        .join("saves")
        .join("snapshots")
        .join(game_id)
}

/// 创建目录级存档快照。
pub fn create_snapshot(
    game_id: &str,
    source_dir: &Path,
    note: Option<&str>,
) -> Result<SaveSnapshot, String> {
    if !source_dir.is_dir() {
        return Err("存档目录不存在".to_string());
    }

    let dir = snapshot_dir(game_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let safe_note = note.map(sanitize_file_name).filter(|n| !n.is_empty());
    let file_name = match safe_note {
        Some(note) => format!("{}_{}.zip", timestamp, note),
        None => format!("{}.zip", timestamp),
    };
    let path = dir.join(file_name);

    let file = fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    add_dir_to_zip(&mut zip, source_dir, source_dir, &options)?;
    zip.finish().map_err(|e| e.to_string())?;

    snapshot_from_path(&path)
}

/// 列出游戏快照。
pub fn list_snapshots(game_id: &str) -> Vec<SaveSnapshot> {
    let mut snapshots = Vec::new();
    let dir = snapshot_dir(game_id);

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("zip") {
                if let Ok(snapshot) = snapshot_from_path(&path) {
                    snapshots.push(snapshot);
                }
            }
        }
    }

    snapshots.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    snapshots
}

/// 恢复快照。`create_safety` 为 true 时，会先对当前目录创建安全检查点。
pub fn restore_snapshot(
    game_id: &str,
    snapshot_path: &Path,
    target_dir: &Path,
    create_safety: bool,
) -> Result<(), String> {
    if !snapshot_path.is_file() {
        return Err("快照文件不存在".to_string());
    }

    if create_safety
        && target_dir.is_dir()
        && fs::read_dir(target_dir)
            .map(|mut r| r.next().is_some())
            .unwrap_or(false)
    {
        let note = format!("safety_checkpoint_{}", Utc::now().format("%Y%m%d_%H%M%S"));
        let _ = create_snapshot(game_id, target_dir, Some(&note));
    }

    if target_dir.exists() {
        clear_directory(target_dir)?;
    } else {
        fs::create_dir_all(target_dir).map_err(|e| e.to_string())?;
    }

    restore_backup(snapshot_path, target_dir)
}

/// 删除快照文件。
pub fn delete_snapshot(snapshot_path: &Path) -> Result<(), String> {
    if !snapshot_path.is_file() {
        return Err("快照文件不存在".to_string());
    }
    fs::remove_file(snapshot_path).map_err(|e| e.to_string())
}

/// 比较快照与当前存档目录。
pub fn compare_snapshot(snapshot_path: &Path, current_dir: &Path) -> Result<SnapshotDiff, String> {
    if !snapshot_path.is_file() {
        return Err("快照文件不存在".to_string());
    }
    if !current_dir.is_dir() {
        return Err("当前存档目录不存在".to_string());
    }

    let snapshot_files = zip_fingerprints(snapshot_path)?;
    let current_files = dir_fingerprints(current_dir)?;

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged = 0;

    for (path, current) in &current_files {
        match snapshot_files.get(path) {
            None => added.push(path.clone()),
            Some(snapshot) if snapshot != current => changed.push(path.clone()),
            Some(_) => unchanged += 1,
        }
    }

    for path in snapshot_files.keys() {
        if !current_files.contains_key(path) {
            removed.push(path.clone());
        }
    }

    added.sort();
    removed.sort();
    changed.sort();

    Ok(SnapshotDiff {
        added,
        removed,
        changed,
        unchanged,
    })
}

/// 检测本地目录和云目录之间的文件冲突。
pub fn detect_conflicts(local_dir: &Path, remote_dir: &Path) -> Vec<SaveConflict> {
    let local = dir_file_metadata(local_dir).unwrap_or_default();
    let remote = dir_file_metadata(remote_dir).unwrap_or_default();
    let mut conflicts = Vec::new();

    for (rel, local_meta) in &local {
        let Some(remote_meta) = remote.get(rel) else {
            continue;
        };
        if local_meta.size == remote_meta.size && local_meta.modified == remote_meta.modified {
            continue;
        }

        conflicts.push(SaveConflict {
            relative_path: rel.clone(),
            local_path: local_dir.join(rel).to_string_lossy().to_string(),
            remote_path: remote_dir.join(rel).to_string_lossy().to_string(),
            local_modified: local_meta.modified.map(format_datetime),
            remote_modified: remote_meta.modified.map(format_datetime),
            local_size: local_meta.size,
            remote_size: remote_meta.size,
            reason: if local_meta.modified > remote_meta.modified {
                "local_newer".to_string()
            } else if remote_meta.modified > local_meta.modified {
                "remote_newer".to_string()
            } else {
                "same_time_different_size".to_string()
            },
        });
    }

    conflicts.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    conflicts
}

/// 同步某游戏的全部本地快照到配置的云后端。
pub async fn sync_snapshots_to_cloud(
    game_id: &str,
    config: &CloudSyncConfig,
) -> Result<u32, String> {
    if !config.enabled {
        return Ok(0);
    }

    let snapshots = list_snapshots(game_id);
    let mut synced = 0;

    match config.provider {
        CloudProvider::LocalFolder => {
            let remote_dir = PathBuf::from(&config.sync_directory)
                .join("MoeGameSaves")
                .join(game_id);
            fs::create_dir_all(&remote_dir).map_err(|e| e.to_string())?;

            for snapshot in snapshots {
                let src = PathBuf::from(&snapshot.file_path);
                let dst = remote_dir.join(&snapshot.file_name);
                if should_copy(&src, &dst) {
                    fs::copy(src, dst).map_err(|e| e.to_string())?;
                    synced += 1;
                }
            }
        }
        CloudProvider::WebDAV => {
            for snapshot in snapshots {
                sync_to_cloud(config, Path::new(&snapshot.file_path)).await?;
                synced += 1;
            }
        }
        CloudProvider::AliyunOSS => return Err("阿里云 OSS 尚未实现".to_string()),
        CloudProvider::None => {}
    }

    Ok(synced)
}

/// 从本地云目录恢复最新快照。
pub fn restore_latest_snapshot_from_local_cloud(
    game_id: &str,
    cloud_dir: &Path,
    target_dir: &Path,
) -> Result<Option<SaveSnapshot>, String> {
    let remote_dir = cloud_dir.join("MoeGameSaves").join(game_id);
    if !remote_dir.is_dir() {
        return Ok(None);
    }

    let latest = fs::read_dir(remote_dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("zip"))
        .max_by_key(|path| fs::metadata(path).and_then(|m| m.modified()).ok());

    if let Some(path) = latest {
        restore_snapshot(game_id, &path, target_dir, true)?;
        snapshot_from_path(&path).map(Some)
    } else {
        Ok(None)
    }
}

/// 云同步 - 上传
pub async fn sync_to_cloud(
    config: &CloudSyncConfig,
    local_path: &std::path::Path,
) -> Result<(), String> {
    match config.provider {
        CloudProvider::WebDAV => {
            let server_url = config
                .server_url
                .as_ref()
                .ok_or("WebDAV URL not configured")?;
            let client = reqwest::Client::new();
            let content = std::fs::read(local_path).map_err(|e| e.to_string())?;
            let remote_path = format!(
                "{}/{}",
                server_url,
                local_path.file_name().unwrap_or_default().to_string_lossy()
            );

            client
                .put(&remote_path)
                .body(content)
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        CloudProvider::LocalFolder => {
            let target = PathBuf::from(&config.sync_directory)
                .join(local_path.file_name().unwrap_or_default());
            std::fs::copy(local_path, &target).map_err(|e| e.to_string())?;
        }
        CloudProvider::AliyunOSS => {
            // TODO: 阿里云 OSS 集成
            return Err("阿里云 OSS 尚未实现".to_string());
        }
        CloudProvider::None => {}
    }
    Ok(())
}

/// 云同步 - 下载
pub async fn sync_from_cloud(
    config: &CloudSyncConfig,
    remote_name: &str,
    local_path: &std::path::Path,
) -> Result<(), String> {
    match config.provider {
        CloudProvider::WebDAV => {
            let server_url = config
                .server_url
                .as_ref()
                .ok_or("WebDAV URL not configured")?;
            let client = reqwest::Client::new();
            let remote_path = format!("{}/{}", server_url, remote_name);

            let response = client
                .get(&remote_path)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let content = response.bytes().await.map_err(|e| e.to_string())?;
            std::fs::write(local_path, &content).map_err(|e| e.to_string())?;
        }
        CloudProvider::LocalFolder => {
            let source = PathBuf::from(&config.sync_directory).join(remote_name);
            std::fs::copy(&source, local_path).map_err(|e| e.to_string())?;
        }
        CloudProvider::AliyunOSS => {
            return Err("阿里云 OSS 尚未实现".to_string());
        }
        CloudProvider::None => {}
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileFingerprint {
    size: u64,
    hash: u64,
}

#[derive(Debug, Clone, Copy)]
struct FileMeta {
    size: u64,
    modified: Option<DateTime<Utc>>,
}

fn snapshot_from_path(path: &Path) -> Result<SaveSnapshot, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("snapshot.zip")
        .to_string();
    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
    let created_at = parse_snapshot_time(stem)
        .or_else(|| metadata.modified().ok().map(DateTime::<Utc>::from))
        .unwrap_or_else(Utc::now);

    Ok(SaveSnapshot {
        id: stem.to_string(),
        file_path: path.to_string_lossy().to_string(),
        file_name,
        created_at: created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        file_size_bytes: metadata.len(),
        note: parse_snapshot_note(stem),
        file_count: zip_entry_count(path).unwrap_or(0),
    })
}

fn parse_snapshot_note(stem: &str) -> Option<String> {
    let mut parts = stem.splitn(3, '_');
    let _date = parts.next()?;
    let _time = parts.next()?;
    parts
        .next()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

fn parse_snapshot_time(stem: &str) -> Option<DateTime<Utc>> {
    let mut parts = stem.split('_');
    let date = parts.next()?;
    let time = parts.next()?;
    let naive =
        chrono::NaiveDateTime::parse_from_str(&format!("{}{}", date, time), "%Y%m%d%H%M%S").ok()?;
    Some(naive.and_utc())
}

fn zip_entry_count(path: &Path) -> Result<u32, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut count = 0;
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        if !file.is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

fn clear_directory(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn path_score(path: &Path, game_dir: &Path, game_name: &str, developer: Option<&str>) -> i32 {
    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(clean_name)
        .unwrap_or_default();
    let path_lower = path.to_string_lossy().to_lowercase();
    let game_name = clean_name(game_name);
    let developer = developer.map(clean_name).unwrap_or_default();
    let mut score = 0;

    if SAVE_KEYWORDS
        .iter()
        .any(|kw| dir_name.contains(kw) || path_lower.contains(kw))
    {
        score += 35;
    }
    if EXCLUDE_KEYWORDS
        .iter()
        .any(|kw| dir_name.contains(kw) || path_lower.contains(kw))
    {
        score -= 50;
    }
    if game_name.len() >= 2 && (dir_name.contains(&game_name) || path_lower.contains(&game_name)) {
        score += 25;
    }
    if developer.len() >= 2 && (dir_name.contains(&developer) || path_lower.contains(&developer)) {
        score += 15;
    }
    if path.starts_with(game_dir) {
        score += 10;
    }
    if is_user_dir(path) {
        score += 15;
    }

    score
}

fn is_user_dir(path: &Path) -> bool {
    [
        dirs::data_dir(),
        dirs::data_local_dir(),
        dirs::document_dir(),
        dirs::home_dir(),
    ]
    .into_iter()
    .flatten()
    .any(|root| path.starts_with(root))
}

fn dir_stats(path: &Path, recursive: bool) -> (u32, u64, Option<DateTime<Utc>>) {
    let mut file_count = 0;
    let mut total_size = 0;
    let mut last_write = None;
    collect_dir_stats(
        path,
        recursive,
        &mut file_count,
        &mut total_size,
        &mut last_write,
    );
    (file_count, total_size, last_write)
}

fn collect_dir_stats(
    path: &Path,
    recursive: bool,
    file_count: &mut u32,
    total_size: &mut u64,
    last_write: &mut Option<DateTime<Utc>>,
) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && recursive {
            collect_dir_stats(&path, recursive, file_count, total_size, last_write);
        } else if path.is_file() {
            if let Ok(metadata) = fs::metadata(&path) {
                *file_count += 1;
                *total_size += metadata.len();
                if let Ok(modified) = metadata.modified() {
                    let modified = DateTime::<Utc>::from(modified);
                    if last_write.map(|current| modified > current).unwrap_or(true) {
                        *last_write = Some(modified);
                    }
                }
            }
        }
    }
}

fn contains_save_like_files(path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };

    for entry in entries.flatten().take(128) {
        let path = entry.path();
        if path.is_dir() {
            if contains_save_like_files(&path) {
                return true;
            }
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if SAVE_EXTENSIONS.contains(&ext.as_str()) {
            return true;
        }
    }

    false
}

fn clean_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .trim()
        .chars()
        .take(40)
        .collect()
}

fn sanitize_file_name(name: &str) -> String {
    let invalid = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    name.chars()
        .filter(|c| !invalid.contains(c) && !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(50)
        .collect()
}

fn format_datetime(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn should_copy(src: &Path, dst: &Path) -> bool {
    if !dst.exists() {
        return true;
    }

    let Ok(src_meta) = fs::metadata(src) else {
        return false;
    };
    let Ok(dst_meta) = fs::metadata(dst) else {
        return true;
    };

    if src_meta.len() != dst_meta.len() {
        return true;
    }

    match (src_meta.modified(), dst_meta.modified()) {
        (Ok(src_time), Ok(dst_time)) => src_time > dst_time,
        _ => false,
    }
}

fn zip_fingerprints(path: &Path) -> Result<HashMap<String, FileFingerprint>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut map = HashMap::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().replace('\\', "/");
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        map.insert(
            name,
            FileFingerprint {
                size: bytes.len() as u64,
                hash: hash_bytes(&bytes),
            },
        );
    }

    Ok(map)
}

fn dir_fingerprints(root: &Path) -> Result<HashMap<String, FileFingerprint>, String> {
    let mut map = HashMap::new();
    collect_dir_fingerprints(root, root, &mut map)?;
    Ok(map)
}

fn collect_dir_fingerprints(
    root: &Path,
    current: &Path,
    map: &mut HashMap<String, FileFingerprint>,
) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect_dir_fingerprints(root, &path, map)?;
            continue;
        }

        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        let rel = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        map.insert(
            rel,
            FileFingerprint {
                size: bytes.len() as u64,
                hash: hash_bytes(&bytes),
            },
        );
    }
    Ok(())
}

fn dir_file_metadata(root: &Path) -> Result<HashMap<String, FileMeta>, String> {
    let mut map = HashMap::new();
    collect_dir_file_metadata(root, root, &mut map)?;
    Ok(map)
}

fn collect_dir_file_metadata(
    root: &Path,
    current: &Path,
    map: &mut HashMap<String, FileMeta>,
) -> Result<(), String> {
    if !current.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect_dir_file_metadata(root, &path, map)?;
            continue;
        }

        let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
        let rel = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        map.insert(
            rel,
            FileMeta {
                size: meta.len(),
                modified: meta.modified().ok().map(DateTime::<Utc>::from),
            },
        );
    }
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    root: &Path,
    current: &Path,
    options: &zip::write::SimpleFileOptions,
) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = path.strip_prefix(root).map_err(|e| e.to_string())?;

        if path.is_dir() {
            zip.add_directory(name.to_string_lossy().to_string(), *options)
                .map_err(|e| e.to_string())?;
            add_dir_to_zip(zip, root, &path, options)?;
        } else {
            zip.start_file(name.to_string_lossy().to_string(), *options)
                .map_err(|e| e.to_string())?;
            let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, zip).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
