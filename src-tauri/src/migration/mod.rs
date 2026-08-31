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
#[cfg(test)]
mod tests;
pub mod v1_to_v2;

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
/// 迁移进度事件名（wire 契约，前端订阅方以此为准）。
///
/// 事件名统一为 `migration://progress`：spec §4.2 步骤 8 明文定义为
/// `migration://progress`，步骤 7 的示例文本写作 `migration-progress`（连字符）属笔误。
/// 实现以步骤 8 为准；spec 属"禁止修改清单"已还原，
/// 口径差异以本注释 + PR 说明表达，不再改动 spec 文件。
pub const MIGRATION_PROGRESS_EVENT: &str = "migration://progress";

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
    /// 迁移互斥锁：`run` / `restore_from_backup` 串行执行。
    ///
    /// setup 钩子的 `spawn_blocking` 迁移与 `migration_run` / `migration_restore_backup`
    /// 命令克隆共享同一个 `Arc`，两个 run 并发时后到的一方阻塞等待，避免交错写库 /
    /// 更新 `migration_state` / 推送进度（并发迁移回归契约）。
    run_lock: Arc<Mutex<()>>,
}

impl Migrator {
    pub fn new(db: HistoryDb, app_data_dir: PathBuf) -> Result<Self, MigrationError> {
        // device_id 必须稳定（spec §3.4）：获取失败直接报错，绝不静默换新 uuid，
        // 否则同步合并会因设备标识漂移而错乱。
        let device_id = get_or_create_device_id(&app_data_dir).map_err(MigrationError::Db)?;
        Ok(Self {
            db,
            app_data_dir,
            device_id,
            progress_sink: Arc::new(|_| {}),
            batch_hook: None,
            run_lock: Arc::new(Mutex::new(())),
        })
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
    /// 状态机（spec §3.3 / §4.2 步骤 5）：
    /// - `migration_state` 无 completed 记录且 v1 JSON 存在 → `Pending`
    /// - 上次中断（`in_progress`）→ `InProgress`（断点续迁）
    /// - 上次失败（`failed`/`rolled_back`）→ `Pending`（从头重试）
    /// - 已完成或无 v1 数据 → `NotNeeded`
    ///
    /// 阻塞语义：`check()` 会获取数据库连接锁（可能等待后台迁移写库结束），
    /// 适合启动流程与 `migration_status` 这类"专查状态"的入口；`history_*`
    /// 命令入口必须用 [`Self::try_check`] 的非阻塞变体。
    pub fn check(&self) -> Result<MigrationStatus, MigrationError> {
        let conn = self.db.conn();
        let guard = lock_conn(&conn)?;
        status_from_state(&guard, &self.app_data_dir)
    }

    /// 非阻塞状态检查：`history_*` 命令入口专用。
    ///
    /// 若后台迁移正持有数据库连接锁（分批写库中），返回 `Ok(None)` 表示
    /// "落盘状态此刻不可读"，调用方不得等待，应按"迁移进行中"返回
    /// `MIGRATION_PENDING`。仅当成功拿到锁时返回 `Ok(Some(status))`。
    pub(crate) fn try_check(&self) -> Result<Option<MigrationStatus>, MigrationError> {
        let conn = self.db.conn();
        let guard = match conn.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(MigrationError::Migration(
                    "history db lock poisoned".to_string(),
                ));
            }
        };
        Ok(Some(status_from_state(&guard, &self.app_data_dir)?))
    }

    /// 执行迁移（同步阻塞；调用方需在 `spawn_blocking` 中运行）。
    ///
    /// 内部流程：备份 → 分批事务写入 → 校验条数 → 标记 completed。
    /// 任何步骤失败 → 自动回滚（清空本次写入 + `status='rolled_back'`）并返回错误。
    ///
    /// 并发安全：持有 `run_lock`，与 setup 的 `spawn_blocking` 迁移、
    /// `migration_run` / `migration_restore_backup` 命令互斥（同一 `Migrator`
    /// 的克隆共享同一把锁）。
    pub fn run(&self) -> Result<MigrationReport, MigrationError> {
        let _guard = self
            .run_lock
            .lock()
            .map_err(|_| MigrationError::Migration("migration run lock poisoned".to_string()))?;
        let source = self.app_data_dir.join(V1_HISTORY_FILE);
        self.run_with_source(&source)
    }

    /// “从备份恢复”入口（R4 应对）：将备份 JSON 作为 v1 数据源重新迁移。
    ///
    /// 与 `run` 共享 `run_lock`，避免与并发迁移交错执行。
    ///
    /// 恢复成功后会把备份内容**写回 v1 路径**（`<app_data_dir>/history.json`，
    /// 恢复成功后：迁移状态虽已 `completed`，但用户能直接看到
    /// v1 数据文件"回来了"，避免误以为恢复失败。
    pub fn restore_from_backup(
        &self,
        backup_path: &Path,
    ) -> Result<MigrationReport, MigrationError> {
        if !backup_path.exists() {
            return Err(MigrationError::BackupNotFound(backup_path.to_path_buf()));
        }
        let _guard = self
            .run_lock
            .lock()
            .map_err(|_| MigrationError::Migration("migration run lock poisoned".to_string()))?;
        self.reset_state_for_restore(backup_path)?;
        let report = self.run_with_source(backup_path)?;
        self.write_backup_to_v1_path(backup_path)?;
        Ok(report)
    }

    /// 把备份文件内容复制回 v1 路径 `history.json`（恢复成功的可见性保证）。
    ///
    /// 注意：`run_with_source` 成功路径会先把残留的旧 `history.json` 重命名为
    /// `.migrated`，这里再写入的就是本次恢复的备份内容，二者不会互相覆盖。
    fn write_backup_to_v1_path(&self, backup_path: &Path) -> Result<(), MigrationError> {
        let v1_path = self.app_data_dir.join(V1_HISTORY_FILE);
        std::fs::copy(backup_path, &v1_path)?;
        tracing::info!(
            v1 = %v1_path.display(),
            backup = %backup_path.display(),
            "backup content restored to v1 history.json"
        );
        Ok(())
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
            // 边界：迁移中断（in_progress）/ 失败（rolled_back 等）
            // 后 v1 源文件被外部删除或移动。此时续迁或重试都不可能完成，若不清理陈旧
            // 状态，`check()` 会因落盘 `migration_state` 恒返回 InProgress / Pending，
            // 门控永久卡死、`history_*` 命令永远 `MIGRATION_PENDING` 且无恢复入口。
            // 这里把非终态收敛为 `completed(total=0)` → `check()` 返回 `NotNeeded`，
            // 门控放行；同时保留既有 `backup_path`，用户仍可经 `list_backups` +
            // `restore_from_backup` 从备份恢复。
            let conn = self.db.conn();
            let guard = lock_conn(&conn)?;
            let preserved_backup = match load_state(&guard)? {
                Some(row) if row.status != "completed" => {
                    tracing::warn!(
                        v1 = %source.display(),
                        status = row.status,
                        "v1 source missing with non-terminal migration state; abandoning migration to unblock gate"
                    );
                    save_state(
                        &guard,
                        &MigrationStateRow {
                            status: "completed".to_string(),
                            total_count: 0,
                            migrated_count: 0,
                            last_offset: 0,
                            backup_path: row.backup_path.clone(),
                            error_message: None,
                            started_at: row.started_at,
                            finished_at: Some(now_ms()),
                        },
                    )?;
                    // 清空残留 staging：已提交批次的历史行保留为迁移成果，但不再与
                    // 任何未来迁移/回滚关联，避免 restore/重试时按陈旧 staging 误回滚。
                    guard.execute("DELETE FROM migration_staging", [])?;
                    row.backup_path
                }
                _ => None,
            };
            return Ok(MigrationReport {
                status: MigrationStatus::NotNeeded,
                total: 0,
                migrated: 0,
                backup_path: preserved_backup,
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

        // 回滚生命周期（spec §5）：fresh start / 失败重试时清空 staging 残留，保证失败回滚
        // 只作用于本次迁移写入；断点续迁（in_progress）则保留既有 staging——它精确对应已提交
        // 批次（状态更新与批次数据在同一事务内提交），回滚时据此还原被覆盖原行 / 删除本次写入行。
        if !resume {
            let guard = lock_conn(&conn)?;
            guard.execute("DELETE FROM migration_staging", [])?;
        }

        // ---- 2. 条数校验基线（spec §4.2.d“含本批次前已有数据需换算”）----
        // 基线 = 迁移开始前已存在的 history 行数（含其他来源/设备或手动 upsert 的记录）。
        // 非续迁场景 staging 刚清空，基线即当前行数；断点续迁时 staging 保留了此前批次
        // INSERT 的 id，减去后得到真正的迁移前基线。最终校验只统计“本次迁移新写入的行”
        // （staging `kind='inserted'`）+ 基线，避免把迁移前已存在的记录误判为本次写入
        // （迁移条数回归契约）。
        let baseline = {
            let guard = lock_conn(&conn)?;
            let current: i64 =
                guard.query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))?;
            let staged_inserted: i64 = guard.query_row(
                "SELECT COUNT(*) FROM migration_staging WHERE kind = 'inserted'",
                [],
                |row| row.get(0),
            )?;
            current - staged_inserted
        };

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
        let backup_path = if let Some(existing) =
            prior.as_ref().and_then(|row| row.backup_path.clone())
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

            // 无有效记录：直接完成（备份已产生，源文件重命名）。状态写回与 staging 清空
            // 在同一事务内，避免崩溃后残留 staging 与实际状态不一致。
            if total == 0 {
                let mut guard = lock_conn(&conn)?;
                let tx = guard.transaction()?;
                save_state(
                    &tx,
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
                tx.execute("DELETE FROM migration_staging", [])?;
                tx.commit()?;
                drop(guard);
                crate::db::rename_v1_history_after_migration(&self.app_data_dir)?;
                let report = MigrationReport {
                    status: MigrationStatus::Completed,
                    total: 0,
                    migrated: 0,
                    backup_path: Some(backup_str.clone()),
                };
                // 最后一个“批次”完成后也要推送 Completed，前端据此离开 InProgress。
                (self.progress_sink)(&report);
                return Ok(report);
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
                            Ok(UpsertOutcome::Inserted(id)) => {
                                tx.execute(
                                    "INSERT OR IGNORE INTO migration_staging (id, kind) \
                                     VALUES (?1, 'inserted')",
                                    rusqlite::params![id],
                                )?;
                            }
                            Ok(UpsertOutcome::Replaced(id, old)) => {
                                // 记录被覆盖前的完整快照：回滚时据此还原原行。
                                let snapshot = serde_json::to_string(&old)
                                    .map_err(|error| MigrationError::Db(DbError::Serde(error)))?;
                                // ON CONFLICT DO NOTHING：保留首次登记的 origin，跨批次 / 断点
                                // 续迁再次覆盖同一 id 时不改写 origin，保证回滚彻底：
                                // - 若该 id 更早批次登记为 'inserted'（迁移创建的行），再次被覆盖
                                //   仍保持 'inserted' → 回滚时 DELETE 而非还原；
                                // - 若登记为 'replaced'，保留最初的 pre-migration 快照，避免被
                                //   中间态快照覆盖后回滚只能还原到中间值。
                                tx.execute(
                                    "INSERT INTO migration_staging (id, kind, snapshot_json) \
                                     VALUES (?1, 'replaced', ?2) \
                                     ON CONFLICT(id) DO NOTHING",
                                    rusqlite::params![id, snapshot],
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
                    hook(batch_index)?;
                }
                offset = end;
            }

            // ---- 5. 条数校验 + 完成标记（同一事务：校验失败回滚、崩溃后状态与 staging 一致）----
            let mut guard = lock_conn(&conn)?;
            let tx = guard.transaction()?;
            // 只统计本次迁移写入的新行（staging `kind='inserted'`），加上迁移前基线，
            // 与最终 history 行数比对。迁移前已存在的其他来源/设备记录计入基线，不被误算。
            let inserted_staged: i64 = tx.query_row(
                "SELECT COUNT(*) FROM migration_staging WHERE kind = 'inserted'",
                [],
                |row| row.get(0),
            )?;
            let final_count: i64 =
                tx.query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))?;
            if final_count != baseline + inserted_staged {
                let message = format!(
                    "history row count mismatch: baseline {baseline} + this-migration inserts {inserted_staged}, found {final_count}"
                );
                return Err(MigrationError::Migration(message));
            }
            // 防呆：staging 登记的写入行数不得超过 v1 去重后的 merge key 数，
            // 否则说明同一 merge key 被写了多行（重复记录）。
            let staged_total: i64 =
                tx.query_row("SELECT COUNT(*) FROM migration_staging", [], |row| {
                    row.get(0)
                })?;
            if staged_total > expected_unique {
                let message = format!(
                    "migration staged {staged_total} rows, exceeding {expected_unique} unique merge keys"
                );
                return Err(MigrationError::Migration(message));
            }
            tx.execute("DELETE FROM migration_staging", [])?;
            save_state(
                &tx,
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
            tx.commit()?;
            drop(guard);

            tracing::info!(
                total,
                migrated = migrated_count,
                skipped,
                "history migration completed"
            );
            crate::db::rename_v1_history_after_migration(&self.app_data_dir)?;

            let report = MigrationReport {
                status: MigrationStatus::Completed,
                total,
                migrated: migrated_count,
                backup_path: Some(backup_str.clone()),
            };
            // 最后一个批次提交后必须也触发 progress_sink 并发出 Completed 事件，
            // 前端进度页据此从 InProgress 收敛到终态（spec §4.2 步骤 7）。
            (self.progress_sink)(&report);
            Ok(report)
        })();

        match outcome {
            Ok(report) => Ok(report),
            // 回滚时保留本次备份路径：备份文件不因失败而"孤儿化"，下次重试直接复用
            // 该备份（rolled_back 后 check() 返回 Pending，
            // 重试不应再生成第二份备份）。
            Err(error) => Err(self.fail_and_rollback(&error.to_string(), backup_str.clone())),
        }
    }

    /// 失败自动回滚：删除 staging 表登记的本次写入 + 清空 staging + 标记
    /// `rolled_back`。返回携带原错误信息的 `MigrationError`。
    ///
    /// `backup_path` 为本次迁移实际使用的备份路径（`run_with_source` 已确认存在），
    /// 回滚后仍写回 `migration_state`，避免备份文件与状态脱节成为孤儿。
    fn fail_and_rollback(&self, message: &str, backup_path: String) -> MigrationError {
        match self.do_rollback(message, backup_path) {
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

    fn do_rollback(&self, message: &str, backup_path: String) -> Result<(), MigrationError> {
        tracing::error!(message, "history migration failed, rolling back");
        let conn = self.db.conn();
        let mut guard = lock_conn(&conn)?;
        let tx = guard.transaction()?;

        // 按 staging 表还原：本次 INSERT 的行删除；本次 UPDATE（Replaced）覆盖的
        // 原行按快照还原。
        //
        // 同批次内“先 INSERT 后又被 UPDATE”的链：staging 登记时已通过
        // `ON CONFLICT(id) DO NOTHING` 保留首次 origin，这里再加一道保险——
        // 即便同 id 同时登记了 'inserted' 与 'replaced'，也以 'inserted' 为准：
        // 该行由迁移创建，回滚必须 DELETE，而不是用中间态快照还原成残留行。
        let mut inserted_ids: HashSet<String> = HashSet::new();
        let mut replaced: Vec<(String, HistoryRecord)> = Vec::new();
        {
            let mut stmt = tx.prepare("SELECT id, kind, snapshot_json FROM migration_staging")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            for row in rows {
                let (id, kind, snapshot_json) = row?;
                match kind.as_str() {
                    "inserted" => {
                        inserted_ids.insert(id);
                    }
                    "replaced" => {
                        let json = snapshot_json.ok_or_else(|| {
                            MigrationError::Rollback(format!(
                                "staging row {id} is missing its replaced snapshot"
                            ))
                        })?;
                        let old: HistoryRecord = serde_json::from_str(&json)
                            .map_err(|error| MigrationError::Db(DbError::Serde(error)))?;
                        replaced.push((id, old));
                    }
                    other => {
                        return Err(MigrationError::Rollback(format!(
                            "unknown migration_staging kind: {other}"
                        )));
                    }
                }
            }
        }
        // 只还原未被登记为 'inserted' 的快照：insert-then-replace 链的行由迁移创建，
        // 必须删除而非还原，否则会残留中间态快照对应的多余行。
        replaced.retain(|(id, _)| !inserted_ids.contains(id));

        for id in &inserted_ids {
            tx.execute("DELETE FROM history WHERE id = ?1", rusqlite::params![id])?;
        }
        for (id, old) in &replaced {
            tx.execute(
                "UPDATE history SET
                    content_id=?1, content_type=?2, title=?3, cover=?4, source_id=?5,
                    chapter_id=?6, chapter_title=?7, page_index=?8, position_sec=?9,
                    scroll_pct=?10, updated_at=?11, device_id=?12, deleted=?13
                 WHERE id=?14",
                rusqlite::params![
                    old.content_id,
                    old.content_type.as_str(),
                    old.title,
                    old.cover,
                    old.source_id,
                    old.chapter_id,
                    old.chapter_title,
                    old.page_index,
                    old.position_sec,
                    old.scroll_pct,
                    old.updated_at,
                    old.device_id,
                    i64::from(old.deleted),
                    id,
                ],
            )?;
        }

        tx.execute("DELETE FROM migration_staging", [])?;
        save_state(
            &tx,
            &MigrationStateRow {
                status: "rolled_back".to_string(),
                total_count: 0,
                migrated_count: 0,
                last_offset: 0,
                // 保留本次备份路径：check() 据此返回 Pending（重试），重试的 run()
                // 复用该备份而不再生成第二份。
                backup_path: Some(backup_path),
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

/// 由 `migration_state` 单行 + v1 文件存在性推导迁移状态（spec §3.3 状态机）。
///
/// `check()` / `try_check()` 共用，保证两种入口的判定口径完全一致。
fn status_from_state(
    conn: &Connection,
    app_data_dir: &Path,
) -> Result<MigrationStatus, MigrationError> {
    match load_state(conn)? {
        Some(row) => match row.status.as_str() {
            "completed" => Ok(MigrationStatus::NotNeeded),
            "in_progress" => Ok(MigrationStatus::InProgress),
            // 失败/回滚 → Pending：下次 run() 从头重试（spec §3.3.c）。
            "failed" | "rolled_back" => Ok(MigrationStatus::Pending),
            _ => Ok(MigrationStatus::Pending),
        },
        None => {
            if app_data_dir.join(V1_HISTORY_FILE).exists() {
                Ok(MigrationStatus::Pending)
            } else {
                Ok(MigrationStatus::NotNeeded)
            }
        }
    }
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
