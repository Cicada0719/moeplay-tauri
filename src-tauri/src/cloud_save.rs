// 萌游 MoeGame · 云存档兼容层（M6 → task-05 兼容化改造）
//
// 历史记录 WebDAV 同步（FR-09）已重构至 `src-tauri/src/sync/` 模块
// （`sync/{mod,webdav,merge,keyring_store}.rs`），凭据一律走 keyring，
// 不在此保留任何明文凭据代码路径（spec task-05 §2 / §4.6 / DoD）。
//
// 本文件仅保留**本地文件夹备份**能力：`backup_to_local` 被
// `commands/cloud.rs::backup_snapshot_local` 兼容转发调用，无网络/凭据依赖。
// 旧的 WebDAV 快照上传/下载实现（`upload_snapshot_webdav` /
// `download_snapshot_webdav`）为无调用方的死代码且携带明文密码参数，已删除。

use std::fs;
use std::path::{Path, PathBuf};

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
