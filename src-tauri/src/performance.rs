//! 性能监控

use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub timestamp: u64,
    pub game_count: usize,
    pub database_size_bytes: u64,
    pub cache_size_bytes: u64,
    pub target_dir_size_bytes: u64,
}

pub fn snapshot(db: &Database) -> PerformanceSnapshot {
    PerformanceSnapshot {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        game_count: db.game_count(),
        database_size_bytes: data_file_size("database.json"),
        cache_size_bytes: dir_size(dirs::cache_dir().unwrap_or_default().join("moeplay")),
        target_dir_size_bytes: dir_size(std::path::PathBuf::from("target")),
    }
}

fn data_file_size(name: &str) -> u64 {
    dirs::data_dir()
        .unwrap_or_default()
        .join("moeplay")
        .join(name)
        .metadata()
        .map(|m| m.len())
        .unwrap_or(0)
}

fn dir_size(path: std::path::PathBuf) -> u64 {
    if !path.is_dir() {
        return 0;
    }
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(path);
            } else {
                total += path.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}
