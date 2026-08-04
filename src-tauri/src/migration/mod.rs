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

// ============================================================================
// 历史数据 v1 → v2 自动迁移框架（FR-08）
//
// 迁移来源为 `<app_data_dir>/history.json`（spec §4.1 的 v1 契约），
// 目标为 `moeplay.db` 的 v2 schema（见 `db_sqlite::SCHEMA_V2_SQL`）。
// 核心设计：备份 → 分批事务写入 → 条数校验 → 标记 completed；
// 任意失败自动回滚（按 `migration_staging` 删除本次写入），
// 崩溃后从 `migration_state.last_offset` 断点续迁且幂等无重复。
// ============================================================================

use crate::db_sqlite::{get_or_create_device_id, DbError, HistoryDb};
use crate::domain::history::HistoryRecord;
use crate::migration::v1_to_v2::{map_v1_to_v2, upsert_idempotent, UpsertOutcome, V1HistoryEntry};
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

pub mod commands;
pub mod v1_to_v2;
#[cfg(test)]
mod tests;

/// v1 历史 JSON 文件名。
pub const V1_HISTORY_FILE: &str = "history.json";
/// 迁移完成后 v1 文件重命名后缀。
pub const V1_HISTORY_MIGRATED_FILE: &str = "history.json.migrated";
/// 备份目录名。
pub const BACKUP_DIR: &str = "backup";
/// 备份文件名前缀。
pub const BACKUP_PREFIX: &str = "history_v1_backup_";
/// 每批写入条数。
pub const BATCH_SIZE: usize = 500;
/// v2 schema 版本号（与 `PRAGMA user_version` 对齐）。
pub const V2_SCHEMA_VERSION: i64 = 2;

/// 迁移状态（对前端 wire format 为 camelCase）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MigrationStatus {
    NotNeeded,
    Pending,
    InProgress,
    Completed,
    Failed(String),
    RolledBack,
}

/// 迁移进度/结果报告（`migration://progress` 事件 payload）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub status: MigrationStatus,
    pub total: i64,
    pub migrated: i64,
    pub backup_path: Option<String>,
}

/// 迁移错误。
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("database error: {0}")]
    Db(#[from] DbError),
    #[error("invalid v1 history file {path}: {message}")]
    InvalidV1 { path: PathBuf, message: String },
    #[error("backup not found: {0}")]
    BackupNotFound(PathBuf),
    #[error("checksum mismatch: {0}")]
    ChecksumMismatch(String),
    #[error("migration failed: {0}")]
    Migration(String),
    #[error("rollback failed: {0}")]
    Rollback(String),
}

impl From<rusqlite::Error> for MigrationError {
    fn from(error: rusqlite::Error) -> Self {
        MigrationError::Db(DbError::Sqlite(error))
    }
}

impl From<std::io::Error> for MigrationError {
    fn from(error: std::io::Error) -> Self {
        MigrationError::Db(DbError::Io(error))
    }
}

impl From<String> for MigrationError {
    fn from(message: String) -> Self {
        MigrationError::Migration(message)
    }
}

/// 进度回调（每批提交后触发一次）。
pub type ProgressSink = Arc<dyn Fn(&MigrationReport) + Send + Sync>;
/// 测试用批处理钩子：每批提交后调用；返回 `Err` 模拟写失败（触发回滚），
/// `panic!` 模拟崩溃（断点续迁）。
pub type BatchHook = Arc<dyn Fn(u32) -> Result<(), MigrationError> + Send + Sync>;

/// 迁移器。持有 `HistoryDb` + app_data_dir + 稳定的 `device_id`。
#[derive(Clone)]
pub struct Migrator {
    db: HistoryDb,
    app_data_dir: PathBuf,
    device_id: String,
    progress_sink: ProgressSink,
    batch_hook: Option<BatchHook>,
}

impl Migrator {
    pub fn new(db: HistoryDb, app_data_dir: PathBuf) -> Self {
        let device_id = get_or_create_device_id(&app_data_dir)
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
        Self {
            db,
            app_data_dir,
            device_id,
            progress_sink: Arc::new(|_| {}),
            batch_hook: None,
        }
    }

    /// 设置进度回调（默认 no-op）。
    pub fn set_progress_sink(&mut self, sink: Option<ProgressSink>) {
        if let Some(sink) = sink {
            self.progress_sink = sink;
        }
    }

    /// 注入批处理钩子（测试用）。
    pub fn set_batch_hook(&mut self, hook: Option<BatchHook>) {
        self.batch_hook = hook;
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn db(&self) -> &HistoryDb {
        &self.db
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    /// 启动时调用：判断是否需要迁移。
    ///
    /// - `migration_state` 无 completed 记录且 v1 JSON 存在 → `Pending`
    /// - 上次中断（`in_progress`）→ `InProgress`（断点续迁）
    /// - 上次失败（`failed`/`rolled_back`）→ `Pending`（从头重试）
    /// - 已完成或无 v1 数据 → `NotNeeded`
    pub fn check(&self) -> Result<MigrationStatus, MigrationError> {
        let conn = self.db.conn();
        let guard = lock_conn(&conn)?;
        match load_state(&guard)? {
            Some(row) => match row.status.as_str() {
                "completed" => Ok(MigrationStatus::NotNeeded),
                "in_progress" => Ok(MigrationStatus::InProgress),
                "failed" | "rolled_back" => Ok(MigrationStatus::Pending),
                _ => Ok(MigrationStatus::Pending),
            },
            None => {
                if self.app_data_dir.join(V1_HISTORY_FILE).exists() {
                    Ok(MigrationStatus::Pending)
                } else {
                    Ok(MigrationStatus::NotNeeded)
                }
            }
        }
    }

    /// 执行迁移（同步阻塞；调用方需在 `spawn_blocking` 中运行）。
    ///
    /// 内部流程：备份 → 分批事务写入 → 校验条数 → 标记 completed。
    /// 任何步骤失败 → 自动回滚（清空本次写入 + `status='rolled_back'`）并返回错误。
    pub fn run(&self) -> Result<MigrationReport, MigrationError> {
        let source = self.app_data_dir.join(V1_HISTORY_FILE);
        self.run_with_source(&source)
    }

    /// “从备份恢复”入口（R4 应对）：将备份 JSON 作为 v1 数据源重新迁移。
    pub fn restore_from_backup(&self, backup_path: &Path) -> Result<MigrationReport, MigrationError> {
        if !backup_path.exists() {
            return Err(MigrationError::BackupNotFound(backup_path.to_path_buf()));
        }
        self.reset_state_for_restore(backup_path)?;
        self.run_with_source(backup_path)
    }

    /// 列出备份目录下的 v1 备份文件。
    pub fn list_backups(&self) -> Result<Vec<PathBuf>, MigrationError> {
        let dir = self.app_data_dir.join(BACKUP_DIR);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(BACKUP_PREFIX)
            {
                out.push(entry.path());
            }
        }
        out.sort();
        Ok(out)
    }

    /// 迁移主体（`run` 与 `restore_from_backup` 共用）。
    fn run_with_source(&self, source: &Path) -> Result<MigrationReport, MigrationError> {
        if !source.exists() {
            return Ok(MigrationReport {
                status: MigrationStatus::NotNeeded,
                total: 0,
                migrated: 0,
                backup_path: None,
            });
        }

        let conn = self.db.conn();

        // ---- 1. 加载状态 / 断点 ----
        let prior = {
            let guard = lock_conn(&conn)?;
            load_state(&guard)?
        };
        if let Some(row) = &prior {
            if row.status == "completed" {
                return Ok(MigrationReport {
                    status: MigrationStatus::NotNeeded,
                    total: row.total_count,
                    migrated: row.migrated_count,
                    backup_path: row.backup_path.clone(),
                });
            }
        }
        let resume = prior
            .as_ref()
            .is_some_and(|row| row.status == "in_progress");
        let resume_offset = if resume {
            prior.as_ref().map(|row| row.last_offset).unwrap_or(0)
        } else {
            0
        };
        let mut migrated_count = if resume {
            prior.as_ref().map(|row| row.migrated_count).unwrap_or(0)
        } else {
            0
        };
        let started_at = now_ms();

        // ---- 2. 标记 in_progress ----
        {
            let guard = lock_conn(&conn)?;
            save_state(
                &guard,
                &MigrationStateRow {
                    status: "in_progress".to_string(),
                    total_count: prior.as_ref().map(|row| row.total_count).unwrap_or(0),
                    migrated_count,
                    last_offset: resume_offset,
                    backup_path: prior.as_ref().and_then(|row| row.backup_path.clone()),
                    error_message: None,
                    started_at: Some(started_at),
                    finished_at: None,
                },
            )?;
        }

        // ---- 3. 备份（断点/恢复场景复用已有 backup_path）----
        let backup_path = if let Some(existing) = prior.as_ref().and_then(|row| row.backup_path.clone())
        {
            PathBuf::from(existing)
        } else {
            let backup = create_backup(source, &self.app_data_dir)?;
            let backup_str = backup.to_string_lossy().to_string();
            let guard = lock_conn(&conn)?;
            let mut row = load_state(&guard)?.unwrap_or_else(|| MigrationStateRow {
                status: "in_progress".to_string(),
                total_count: 0,
                migrated_count: 0,
                last_offset: 0,
                backup_path: None,
                error_message: None,
                started_at: None,
                finished_at: None,
            });
            row.status = "in_progress".to_string();
            row.backup_path = Some(backup_str.clone());
            save_state(&guard, &row)?;
            tracing::info!(backup = %backup_str, "history v1 backup created and checksum-verified");
            backup
        };
        let backup_str = backup_path.to_string_lossy().to_string();

        // ---- 4+ 解析 → 分批写入 → 校验（任一失败自动回滚）----
        let outcome = (|| -> Result<MigrationReport, MigrationError> {
            let entries = read_v1_entries(source)?;
            let mut valid: Vec<HistoryRecord> = Vec::with_capacity(entries.len());
            let mut skipped = 0usize;
            for entry in &entries {
                match map_v1_to_v2(entry, &self.device_id) {
                    Ok(record) => valid.push(record),
                    Err(error) => {
                        skipped += 1;
                        tracing::warn!(error = %error, "skipping invalid v1 history entry");
                    }
                }
            }
            let total = valid.len() as i64;

            // 无有效记录：直接完成（备份已产生，源文件重命名）。
            if total == 0 {
                let guard = lock_conn(&conn)?;
                save_state(
                    &guard,
                    &MigrationStateRow {
                        status: "completed".to_string(),
                        total_count: 0,
                        migrated_count: 0,
                        last_offset: 0,
                        backup_path: Some(backup_str.clone()),
                        error_message: None,
                        started_at: Some(started_at),
                        finished_at: Some(now_ms()),
                    },
                )?;
                guard.execute("DELETE FROM migration_staging", [])?;
                drop(guard);
                crate::db::rename_v1_history_after_migration(&self.app_data_dir)?;
                return Ok(MigrationReport {
                    status: MigrationStatus::Completed,
                    total: 0,
                    migrated: 0,
                    backup_path: Some(backup_str.clone()),
                });
            }

            let expected_unique = count_unique_merge_keys(&valid) as i64;
            let mut offset = usize::try_from(resume_offset.max(0))
                .unwrap_or(usize::MAX)
                .min(valid.len());
            let mut batch_index = 0u32;

            while offset < valid.len() {
                let end = (offset + BATCH_SIZE).min(valid.len());
                let batch = &valid[offset..end];
                {
                    let mut guard = lock_conn(&conn)?;
                    let tx = guard.transaction()?;
                    for record in batch {
                        match upsert_idempotent(&tx, record) {
                            Ok(UpsertOutcome::Inserted(id)) | Ok(UpsertOutcome::Replaced(id)) => {
                                tx.execute(
                                    "INSERT OR IGNORE INTO migration_staging (id) VALUES (?1)",
                                    rusqlite::params![id],
                                )?;
                            }
                            Ok(UpsertOutcome::Skipped) => {}
                            Err(error) => return Err(MigrationError::Db(error)),
                        }
                    }
                    migrated_count += batch.len() as i64;
                    // 状态更新与数据写入在同一事务内：崩溃一致性。
                    save_state(
                        &tx,
                        &MigrationStateRow {
                            status: "in_progress".to_string(),
                            total_count: total,
                            migrated_count,
                            last_offset: end as i64,
                            backup_path: Some(backup_str.clone()),
                            error_message: None,
                            started_at: Some(started_at),
                            finished_at: None,
                        },
                    )?;
                    tx.commit()?;
                }
                batch_index += 1;
                let report = MigrationReport {
                    status: MigrationStatus::InProgress,
                    total,
                    migrated: migrated_count,
                    backup_path: Some(backup_str.clone()),
                };
                (self.progress_sink)(&report);
                tracing::info!(
                    batch = batch_index,
                    migrated = migrated_count,
                    total,
                    "history migration batch committed"
                );
                if let Some(hook) = &self.batch_hook {
                    if let Err(error) = hook(batch_index) {
                        return Err(error);
                    }
                }
                offset = end;
            }

            // ---- 5. 条数校验 ----
            let guard = lock_conn(&conn)?;
            let actual: i64 = guard.query_row(
                "SELECT COUNT(*) FROM history WHERE device_id = ?1",
                rusqlite::params![self.device_id],
                |row| row.get(0),
            )?;
            if actual != expected_unique {
                let message = format!(
                    "history row count mismatch: expected {expected_unique}, found {actual}"
                );
                return Err(MigrationError::Migration(message));
            }
            guard.execute("DELETE FROM migration_staging", [])?;
            save_state(
                &guard,
                &MigrationStateRow {
                    status: "completed".to_string(),
                    total_count: total,
                    migrated_count,
                    last_offset: valid.len() as i64,
                    backup_path: Some(backup_str.clone()),
                    error_message: None,
                    started_at: Some(started_at),
                    finished_at: Some(now_ms()),
                },
            )?;
            drop(guard);

            tracing::info!(
                total,
                migrated = migrated_count,
                skipped,
                "history migration completed"
            );
            crate::db::rename_v1_history_after_migration(&self.app_data_dir)?;

            Ok(MigrationReport {
                status: MigrationStatus::Completed,
                total,
                migrated: migrated_count,
                backup_path: Some(backup_str.clone()),
            })
        })();

        match outcome {
            Ok(report) => Ok(report),
            Err(error) => Err(self.fail_and_rollback(&error.to_string())),
        }
    }

    /// 失败自动回滚：删除 staging 表登记的本次写入 + 清空 staging + 标记
    /// `rolled_back`。返回携带原错误信息的 `MigrationError`。
    fn fail_and_rollback(&self, message: &str) -> MigrationError {
        match self.do_rollback(message) {
            Ok(()) => MigrationError::Migration(message.to_string()),
            Err(rollback_error) => {
                tracing::error!(
                    message,
                    error = %rollback_error,
                    "migration failed and rollback also failed"
                );
                MigrationError::Rollback(format!("{message}; rollback error: {rollback_error}"))
            }
        }
    }

    fn do_rollback(&self, message: &str) -> Result<(), MigrationError> {
        tracing::error!(message, "history migration failed, rolling back");
        let conn = self.db.conn();
        let mut guard = lock_conn(&conn)?;
        let tx = guard.transaction()?;
        tx.execute(
            "DELETE FROM history \
             WHERE device_id = ?1 AND id IN (SELECT id FROM migration_staging)",
            rusqlite::params![self.device_id],
        )?;
        tx.execute("DELETE FROM migration_staging", [])?;
        save_state(
            &tx,
            &MigrationStateRow {
                status: "rolled_back".to_string(),
                total_count: 0,
                migrated_count: 0,
                last_offset: 0,
                backup_path: None,
                error_message: Some(message.to_string()),
                started_at: None,
                finished_at: Some(now_ms()),
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    fn reset_state_for_restore(&self, backup_path: &Path) -> Result<(), MigrationError> {
        let conn = self.db.conn();
        let guard = lock_conn(&conn)?;
        guard.execute("DELETE FROM migration_staging", [])?;
        save_state(
            &guard,
            &MigrationStateRow {
                status: "in_progress".to_string(),
                total_count: 0,
                migrated_count: 0,
                last_offset: 0,
                backup_path: Some(backup_path.to_string_lossy().to_string()),
                error_message: None,
                started_at: Some(now_ms()),
                finished_at: None,
            },
        )?;
        Ok(())
    }
}

/// `migration_state` 单行（id=1）的内存表示。
#[derive(Debug, Clone)]
struct MigrationStateRow {
    status: String,
    total_count: i64,
    migrated_count: i64,
    last_offset: i64,
    backup_path: Option<String>,
    error_message: Option<String>,
    started_at: Option<i64>,
    finished_at: Option<i64>,
}

fn lock_conn(
    conn: &Arc<Mutex<rusqlite::Connection>>,
) -> Result<MutexGuard<'_, rusqlite::Connection>, MigrationError> {
    conn.lock()
        .map_err(|_| MigrationError::Migration("history db lock poisoned".to_string()))
}

fn load_state(conn: &Connection) -> Result<Option<MigrationStateRow>, MigrationError> {
    let mut stmt = conn.prepare(
        "SELECT status, total_count, migrated_count, last_offset, backup_path, error_message, \
                started_at, finished_at \
         FROM migration_state WHERE id = 1",
    )?;
    let mut rows = stmt.query_map([], |row| {
        Ok(MigrationStateRow {
            status: row.get(0)?,
            total_count: row.get(1)?,
            migrated_count: row.get(2)?,
            last_offset: row.get(3)?,
            backup_path: row.get(4)?,
            error_message: row.get(5)?,
            started_at: row.get(6)?,
            finished_at: row.get(7)?,
        })
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

fn save_state(conn: &Connection, row: &MigrationStateRow) -> Result<(), MigrationError> {
    conn.execute(
        "INSERT INTO migration_state
            (id, from_version, to_version, status, total_count, migrated_count, last_offset,
             backup_path, error_message, started_at, finished_at)
         VALUES (1, 1, 2, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
            from_version=excluded.from_version, to_version=excluded.to_version,
            status=excluded.status, total_count=excluded.total_count,
            migrated_count=excluded.migrated_count, last_offset=excluded.last_offset,
            backup_path=excluded.backup_path, error_message=excluded.error_message,
            started_at=excluded.started_at, finished_at=excluded.finished_at",
        rusqlite::params![
            row.status,
            row.total_count,
            row.migrated_count,
            row.last_offset,
            row.backup_path,
            row.error_message,
            row.started_at,
            row.finished_at,
        ],
    )?;
    Ok(())
}

/// 读取 v1 条目（`run` 用 `history.json`，`restore_from_backup` 用备份文件）。
pub(crate) fn read_v1_entries(source: &Path) -> Result<Vec<V1HistoryEntry>, MigrationError> {
    let content = std::fs::read_to_string(source).map_err(|error| MigrationError::InvalidV1 {
        path: source.to_path_buf(),
        message: format!("read: {error}"),
    })?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content).map_err(|error| MigrationError::InvalidV1 {
        path: source.to_path_buf(),
        message: format!("parse: {error}"),
    })
}

fn create_backup(source: &Path, app_data_dir: &Path) -> Result<PathBuf, MigrationError> {
    let backup_dir = app_data_dir.join(BACKUP_DIR);
    std::fs::create_dir_all(&backup_dir)?;
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let mut path = backup_dir.join(format!("{BACKUP_PREFIX}{timestamp}.json"));
    let mut n = 1;
    while path.exists() {
        path = backup_dir.join(format!("{BACKUP_PREFIX}{timestamp}_{n}.json"));
        n += 1;
    }
    std::fs::copy(source, &path)?;
    let source_hash = sha256_file(source)?;
    let backup_hash = sha256_file(&path)?;
    if source_hash != backup_hash {
        let _ = std::fs::remove_file(&path);
        return Err(MigrationError::ChecksumMismatch(format!(
            "backup {} does not match source {}",
            path.display(),
            source.display()
        )));
    }
    Ok(path)
}

fn sha256_file(path: &Path) -> Result<String, MigrationError> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn count_unique_merge_keys(records: &[HistoryRecord]) -> usize {
    let mut seen = HashSet::new();
    for record in records {
        seen.insert((
            record.content_id.clone(),
            record.source_id.clone(),
            record.chapter_id.clone(),
        ));
    }
    seen.len()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
