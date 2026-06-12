//! 数据迁移模块
//!
//! JSON 文件数据库的 schema 升级系统。每次 Game 模型新增字段
//! 或调整结构时，编写对应的迁移函数，按版本号顺序执行。
//!
//! ## 使用方式
//!
//! 1. 递增 `CURRENT_SCHEMA_VERSION`
//! 2. 在 `get_migrations()` 中添加新的 Migration 条目
//! 3. 在 `Database::new()` 中调用 `run_migrations()`

use crate::models::AppDatabase;
use serde::{Deserialize, Serialize};

/// 当前数据库 schema 版本号
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// 一次版本迁移
#[derive(Clone)]
pub struct Migration {
    /// 迁移目标版本号（执行后 schema_version 将变为此值）
    pub version: u32,
    /// 迁移说明（用于日志/调试）
    pub description: &'static str,
    /// 迁移逻辑：原地修改 AppDatabase
    pub apply: fn(&mut AppDatabase) -> Result<(), String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInfo {
    pub version: u32,
    pub description: String,
    pub applied: bool,
}

pub fn migration_status(current_version: u32) -> Vec<MigrationInfo> {
    get_migrations()
        .into_iter()
        .map(|migration| MigrationInfo {
            version: migration.version,
            description: migration.description.to_string(),
            applied: migration.version <= current_version,
        })
        .collect()
}

/// 获取所有迁移（按版本号升序排列）
pub fn get_migrations() -> Vec<Migration> {
    vec![
        // ====================================================================
        // v0 → v1: 将旧版扁平字段迁移到结构化子模型
        // ====================================================================
        Migration {
            version: 1,
            description: "Migrate legacy flat fields into GameMetadata / PlayTracker",
            apply: |data| {
                for game in &mut data.games {
                    // release_year → metadata.release_year
                    if game.metadata.release_year.is_none() {
                        game.metadata.release_year = game.release_year;
                    }

                    // rating → play_tracker.user_rating
                    if game.play_tracker.user_rating.is_none() {
                        game.play_tracker.user_rating = game.rating;
                    }

                    // last_played → play_tracker.last_played
                    if game.play_tracker.last_played.is_none() {
                        game.play_tracker.last_played = game.last_played.take();
                    }

                    // vndb_id → metadata.vndb_id
                    if game.metadata.vndb_id.is_none() {
                        game.metadata.vndb_id = game.vndb_id.take();
                    }

                    // bangumi_id → metadata.bangumi_id
                    if game.metadata.bangumi_id.is_none() {
                        game.metadata.bangumi_id = game.bangumi_id.take();
                    }

                    // play_time_seconds → play_tracker.total_seconds
                    if game.play_tracker.total_seconds == 0 && game.play_time_seconds > 0 {
                        game.play_tracker.total_seconds = game.play_time_seconds;
                    }
                }
                Ok(())
            },
        },
    ]
}

/// 按序执行所有未执行的迁移，返回最终版本号
pub fn run_migrations(data: &mut AppDatabase) -> Result<u32, String> {
    let start_version = data.schema_version;
    let migrations = get_migrations();

    for migration in &migrations {
        if migration.version > data.schema_version {
            (migration.apply)(data)?;
            data.schema_version = migration.version;
        }
    }

    if data.schema_version > start_version {
        println!(
            "[migration] Database upgraded: v{} → v{}",
            start_version, data.schema_version
        );
    }

    Ok(data.schema_version)
}
