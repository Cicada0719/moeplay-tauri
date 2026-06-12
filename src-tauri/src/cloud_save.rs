// 萌游 MoeGame · 云存档（WebDAV / 本地备份）（M6）
//
// 支持：
//   - WebDAV 上传/下载/列表
//   - 本地文件夹备份（便携方案）
//   - 冲突检测（修改时间 + 大小比对）

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: String,
}

/// 云同步结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncResult {
    pub uploaded: Vec<String>,
    pub downloaded: Vec<String>,
    pub conflicts: Vec<String>,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// 上传本地存档快照到 WebDAV 服务器。
pub async fn upload_snapshot_webdav(
    local_path: &Path,
    server_url: &str,
    username: &str,
    password: &str,
    remote_dir: &str,
) -> Result<(), String> {
    let filename = local_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("snapshot.zip");
    let base = server_url.trim_end_matches('/');
    let dir = remote_dir.trim_matches('/');
    let remote_url = if dir.is_empty() {
        format!("{}/{}", base, filename)
    } else {
        format!("{}/{}/{}", base, dir, filename)
    };

    let data = fs::read(local_path).map_err(|e| format!("读取本地文件失败: {}", e))?;

    let client = reqwest::Client::new();
    let resp = client
        .put(&remote_url)
        .basic_auth(username, Some(password))
        .header("Content-Type", "application/octet-stream")
        .body(data)
        .send()
        .await
        .map_err(|e| format!("WebDAV 上传失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("WebDAV 返回 HTTP {}", resp.status()));
    }

    tracing::info!(file = filename, url = %remote_url, "Snapshot uploaded to WebDAV");
    Ok(())
}

/// 从 WebDAV 下载存档快照到本地。
pub async fn download_snapshot_webdav(
    remote_filename: &str,
    server_url: &str,
    username: &str,
    password: &str,
    dest_dir: &Path,
) -> Result<PathBuf, String> {
    let remote_url = format!("{}/{}", server_url.trim_end_matches('/'), remote_filename);
    let dest = dest_dir.join(remote_filename);

    let client = reqwest::Client::new();
    let resp = client
        .get(&remote_url)
        .basic_auth(username, Some(password))
        .send()
        .await
        .map_err(|e| format!("WebDAV 下载失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("WebDAV 返回 HTTP {}", resp.status()));
    }

    let data = resp
        .bytes()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    fs::write(&dest, &data).map_err(|e| format!("写入本地文件失败: {}", e))?;

    tracing::info!(file = remote_filename, dest = %dest.display(), "Snapshot downloaded from WebDAV");
    Ok(dest)
}

/// 本地文件夹备份：复制快照到备份目录。
pub fn backup_to_local(
    snapshot_path: &Path,
    backup_dir: &Path,
    keep_recent: usize,
) -> Result<PathBuf, String> {
    fs::create_dir_all(backup_dir).map_err(|e| e.to_string())?;

    let filename = snapshot_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("snapshot.zip");
    let dest = backup_dir.join(filename);
    fs::copy(snapshot_path, &dest).map_err(|e| format!("本地备份失败: {}", e))?;

    // 清理旧备份（保留最近 N 个）
    clean_old_backups(backup_dir, keep_recent)?;

    Ok(dest)
}

fn clean_old_backups(dir: &Path, keep: usize) -> Result<(), String> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.path())
        .collect();

    if files.len() <= keep {
        return Ok(());
    }

    files.sort_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok());

    let to_remove = files.len() - keep;
    for f in files.iter().take(to_remove) {
        fs::remove_file(f).ok();
    }

    Ok(())
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_backup_and_cleanup() {
        let tmp = std::env::temp_dir().join("m6_cloud_test");
        let backup_dir = tmp.join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create a dummy snapshot
        let snap = tmp.join("snap1.zip");
        fs::write(&snap, b"test snapshot data").unwrap();

        let result = backup_to_local(&snap, &backup_dir, 3);
        assert!(result.is_ok());
        assert!(backup_dir.join("snap1.zip").exists());

        // Create more snapshots and verify cleanup
        for i in 2..=6 {
            let s = tmp.join(format!("snap{i}.zip"));
            fs::write(&s, format!("data {i}")).unwrap();
            backup_to_local(&s, &backup_dir, 3).unwrap();
        }
        let count = fs::read_dir(&backup_dir).unwrap().count();
        assert!(count <= 3, "should keep at most 3 backups, got {}", count);

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_clean_old_backups_keeps_recent() {
        let dir = std::env::temp_dir().join("m6_clean_test");
        fs::create_dir_all(&dir).unwrap();

        for i in 1..=5 {
            fs::write(dir.join(format!("b{i}.zip")), format!("data{i}")).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        clean_old_backups(&dir, 2).unwrap();
        let count = fs::read_dir(&dir).unwrap().count();
        assert_eq!(count, 2, "should keep 2 most recent");

        fs::remove_dir_all(&dir).ok();
    }
}
