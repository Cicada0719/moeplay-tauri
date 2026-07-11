// 萌游 MoeGame · SQLite 存储层（M0 完整版）
//
// 设计：**索引投影列 + 热 data_json 列**。
//   - 把 Game 整体 serde 进 `data_json`（保真、零字段映射成本）；
//   - 同时投影若干常查字段为独立列（name/sort_name/game_type/favorite/hidden...）并建索引，
//     解决"单 JSON 文件撑不住 1000+ 库"的规模问题。
//   - 后续里程碑再把多值字段（标签/会话/存档）拆成独立表做关系查询。
//
// 本模块为 M0-3 命令切换做准备：方法签名与 `db::Database` 对齐，commands.rs 可无缝切换。

use crate::models::{
    AppDatabase, CompletionStatus, Game, GameAlias, GameMetadata, GamePlatform, PlaySession,
    PlayTracker, SaveBackup, SaveData, Settings, StoreLink, Tag,
};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// Current schema version. v6 adds bounded, redacted background-job event timelines.
pub const SCHEMA_VERSION: i64 = 6;

/// Staged module registration keeps the repository layer compiled and testable while
/// `lib.rs` remains untouched for the Batch integrator.
#[path = "repositories/mod.rs"]
pub mod repositories;

#[derive(Debug, Clone, Copy)]
struct SchemaMigration {
    version: i64,
    name: &'static str,
    checksum: &'static str,
}

const SCHEMA_MIGRATIONS: &[SchemaMigration] = &[
    SchemaMigration {
        version: 1,
        name: "legacy_base_schema",
        checksum: "sha256:ce4ab5de8ac3bb1e7e7dceec17d90b24c6f43d6ea14c1b5afcc606cf6fd6af24",
    },
    SchemaMigration {
        version: 2,
        name: "legacy_sqlite_v2_projections",
        checksum: "sha256:8e9eb519d229cabf79729db6b7782c64b4a581d07d29de4a4d63d3133e6d214f",
    },
    SchemaMigration {
        version: 3,
        name: "domain_activity_progress_health_jobs",
        checksum: "sha256:0316ec4a5a9c14970660c8a10355c16060e5147236e80f1f7ea59cf4b6e32262",
    },
    SchemaMigration {
        version: 4,
        name: "provider_config_non_secret_json",
        checksum: "sha256:dd9d6d953d8d0dfc426fa87addd35f21cf1f715c945ff59b9ebb4da8934fdad5",
    },
    SchemaMigration {
        version: 5,
        name: "validated_ai_task_results",
        checksum: "sha256:d564e622ca8b5c86ab3d3e8fc5746e576099e36474579a97f685aa04b6b7b422",
    },
    SchemaMigration {
        version: 6,
        name: "background_job_event_timelines",
        checksum: "sha256:fb584e57117039681ce5664ea411bc788cc8ce7c3b79821c7c7a20be007d1cc9",
    },
];

fn migration_by_version(version: i64) -> Option<&'static SchemaMigration> {
    SCHEMA_MIGRATIONS
        .iter()
        .find(|migration| migration.version == version)
}

fn validate_migration_ledger(conn: &Connection) -> Result<(), String> {
    let mut statement = conn
        .prepare("SELECT version,name,checksum FROM schema_migrations ORDER BY version")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (version, name, checksum) = row.map_err(|e| e.to_string())?;
        let expected = migration_by_version(version).ok_or_else(|| {
            format!("database migration ledger contains unsupported version {version}")
        })?;
        if name != expected.name || checksum != expected.checksum {
            return Err(format!(
                "database migration ledger checksum mismatch for version {version}"
            ));
        }
    }
    Ok(())
}

fn record_migration(conn: &Connection, migration: &SchemaMigration) -> Result<(), String> {
    conn.execute(
        "INSERT INTO schema_migrations(version,name,checksum,applied_at) VALUES(?1,?2,?3,?4) \
         ON CONFLICT(version) DO NOTHING",
        params![
            migration.version,
            migration.name,
            migration.checksum,
            chrono::Utc::now().to_rfc3339(),
        ],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn apply_v3_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS activity_events (
            id               TEXT PRIMARY KEY,
            resource_kind    TEXT NOT NULL,
            resource_id      TEXT NOT NULL,
            event_type       TEXT NOT NULL,
            started_at       TEXT NOT NULL,
            ended_at         TEXT,
            duration_seconds INTEGER CHECK(duration_seconds IS NULL OR duration_seconds >= 0),
            duration_quality TEXT NOT NULL DEFAULT 'none' CHECK(duration_quality IN ('exact','estimated','baseline','none')),
            session_id       TEXT,
            source_legacy_id TEXT,
            provider_id      TEXT,
            payload_json     TEXT NOT NULL CHECK(json_valid(payload_json))
         );
         CREATE INDEX IF NOT EXISTS idx_activity_events_started_at
            ON activity_events(started_at DESC, id DESC);
         CREATE INDEX IF NOT EXISTS idx_activity_events_type_started_at
            ON activity_events(resource_kind, event_type, started_at DESC, id DESC);
         CREATE INDEX IF NOT EXISTS idx_activity_events_resource_started_at
            ON activity_events(resource_kind, resource_id, started_at DESC, id DESC);

         CREATE TABLE IF NOT EXISTS progress_records (
            resource_kind TEXT NOT NULL,
            resource_id   TEXT NOT NULL,
            provider_id   TEXT NOT NULL DEFAULT '',
            position_json TEXT NOT NULL CHECK(json_valid(position_json)),
            updated_at    TEXT NOT NULL,
            completed     INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0,1)),
            PRIMARY KEY(resource_kind, resource_id, provider_id)
         );
         CREATE INDEX IF NOT EXISTS idx_progress_records_resource_updated
            ON progress_records(resource_kind, resource_id, updated_at DESC);
         CREATE INDEX IF NOT EXISTS idx_progress_records_updated_at
            ON progress_records(updated_at DESC);

         CREATE TABLE IF NOT EXISTS provider_health (
            provider_id          TEXT NOT NULL,
            operation            TEXT NOT NULL,
            state                TEXT NOT NULL,
            success_count        INTEGER NOT NULL DEFAULT 0 CHECK(success_count >= 0),
            failure_count        INTEGER NOT NULL DEFAULT 0 CHECK(failure_count >= 0),
            consecutive_failures INTEGER NOT NULL DEFAULT 0 CHECK(consecutive_failures >= 0),
            latency_ms_ema       REAL,
            last_success_at      TEXT,
            last_failure_at      TEXT,
            circuit_open_until   TEXT,
            last_error_kind      TEXT,
            PRIMARY KEY(provider_id, operation)
         );
         CREATE INDEX IF NOT EXISTS idx_provider_health_state
            ON provider_health(state, provider_id, operation);

         CREATE TABLE IF NOT EXISTS background_jobs (
            id            TEXT PRIMARY KEY,
            kind          TEXT NOT NULL,
            title         TEXT NOT NULL,
            status        TEXT NOT NULL,
            progress      REAL NOT NULL DEFAULT 0 CHECK(progress >= 0.0 AND progress <= 1.0),
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL,
            error_json    TEXT CHECK(error_json IS NULL OR json_valid(error_json)),
            metadata_json TEXT NOT NULL CHECK(json_valid(metadata_json))
         );
         CREATE INDEX IF NOT EXISTS idx_background_jobs_status_updated
            ON background_jobs(status, updated_at DESC, id DESC);
         CREATE INDEX IF NOT EXISTS idx_background_jobs_kind_status
            ON background_jobs(kind, status, updated_at DESC);",
    )
    .map_err(|e| e.to_string())
}

const V4_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS provider_configs (
    provider_id    TEXT NOT NULL CHECK(length(trim(provider_id)) > 0),
    resource_kind  TEXT NOT NULL CHECK(length(trim(resource_kind)) > 0),
    provider_kind  TEXT NOT NULL CHECK(length(trim(provider_kind)) > 0),
    config_version INTEGER NOT NULL CHECK(config_version > 0),
    config_json    TEXT NOT NULL CHECK(json_valid(config_json)),
    enabled        INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    PRIMARY KEY(provider_id, resource_kind)
 );
 CREATE INDEX IF NOT EXISTS idx_provider_configs_resource_enabled
    ON provider_configs(resource_kind, enabled, provider_id);
 CREATE INDEX IF NOT EXISTS idx_provider_configs_kind
    ON provider_configs(resource_kind, provider_kind, enabled);";

fn apply_v4_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(V4_SCHEMA_SQL).map_err(|e| e.to_string())
}

const V5_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS ai_task_results (
    task_id        TEXT PRIMARY KEY CHECK(length(trim(task_id)) > 0 AND length(task_id) <= 128),
    outcome_kind   TEXT NOT NULL CHECK(outcome_kind IN ('succeeded','failed')),
    outcome_json   TEXT NOT NULL CHECK(json_valid(outcome_json)),
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK(schema_version = 1),
    created_at     TEXT NOT NULL,
    expires_at     TEXT NOT NULL
 );
 CREATE INDEX IF NOT EXISTS idx_ai_task_results_expires
    ON ai_task_results(expires_at, task_id);";

fn apply_v5_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(V5_SCHEMA_SQL).map_err(|e| e.to_string())
}

const V6_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS background_job_events (
    job_id     TEXT NOT NULL REFERENCES background_jobs(id) ON DELETE CASCADE,
    sequence   INTEGER NOT NULL CHECK(sequence > 0),
    level      TEXT NOT NULL CHECK(length(trim(level)) > 0),
    code       TEXT NOT NULL CHECK(length(trim(code)) > 0),
    message    TEXT NOT NULL,
    progress   REAL CHECK(progress IS NULL OR (progress >= 0.0 AND progress <= 1.0)),
    created_at TEXT NOT NULL,
    PRIMARY KEY(job_id, sequence)
 );
 CREATE INDEX IF NOT EXISTS idx_background_job_events_job_sequence_desc
    ON background_job_events(job_id, sequence DESC);
 DELETE FROM background_job_events
  WHERE job_id IN (
    SELECT id FROM background_jobs
     WHERE status IN ('succeeded', 'failed', 'cancelled')
       AND julianday(updated_at) < julianday('now', '-30 days')
  );";

fn apply_v6_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(V6_SCHEMA_SQL).map_err(|e| e.to_string())
}

/// 若 `table` 里不存在 `column`，则 ALTER TABLE 添加之。幂等。
fn column_exists(conn: &rusqlite::Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?;
    let exists = rows.try_fold(false, |found, name| {
        name.map(|name| found || name == column)
            .map_err(|e| e.to_string())
    })?;
    Ok(exists)
}

fn ensure_column(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
    col_type: &str,
) -> Result<(), String> {
    let exists = column_exists(conn, table, column)?;
    if !exists {
        conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {col_type};"
        ))
        .map_err(|e| e.to_string())?;
        tracing::info!(table, column, "Schema migration: added missing column");
    }
    Ok(())
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            params![table],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

fn column_notnull(conn: &rusqlite::Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(3)?)))
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (name, notnull) = row.map_err(|e| e.to_string())?;
        if name == column {
            return Ok(notnull != 0);
        }
    }
    Ok(false)
}

fn migrate_games_table(conn: &rusqlite::Connection) -> Result<(), String> {
    if !table_exists(conn, "games")? {
        return Ok(());
    }

    let has_legacy_required_install_path = column_exists(conn, "games", "install_path")?
        && column_notnull(conn, "games", "install_path")?;

    if has_legacy_required_install_path {
        ensure_column(conn, "games", "sort_name", "TEXT")?;
        ensure_column(conn, "games", "game_type", "TEXT")?;
        ensure_column(conn, "games", "favorite", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "games", "hidden", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "games", "created_at", "TEXT")?;
        ensure_column(conn, "games", "updated_at", "TEXT")?;
        ensure_column(conn, "games", "data_json", "TEXT")?;

        conn.execute_batch(
            "DROP TABLE IF EXISTS games_migration;
             CREATE TABLE games_migration (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                sort_name  TEXT,
                game_type  TEXT,
                favorite   INTEGER NOT NULL DEFAULT 0,
                hidden     INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                data_json  TEXT NOT NULL
             );
             INSERT OR REPLACE INTO games_migration
                (id,name,sort_name,game_type,favorite,hidden,created_at,updated_at,data_json)
             SELECT
                id,
                name,
                sort_name,
                game_type,
                COALESCE(favorite, 0),
                COALESCE(hidden, 0),
                created_at,
                updated_at,
                data_json
             FROM games
             WHERE id IS NOT NULL AND name IS NOT NULL AND data_json IS NOT NULL;
             DROP TABLE games;
             ALTER TABLE games_migration RENAME TO games;",
        )
        .map_err(|e| e.to_string())?;
        tracing::info!("Schema migration: rebuilt legacy games table");
    }

    ensure_column(conn, "games", "sort_name", "TEXT")?;
    ensure_column(conn, "games", "game_type", "TEXT")?;
    ensure_column(conn, "games", "favorite", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "games", "hidden", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "games", "created_at", "TEXT")?;
    ensure_column(conn, "games", "updated_at", "TEXT")?;
    ensure_column(conn, "games", "data_json", "TEXT")?;
    Ok(())
}

fn migrate_settings_table(conn: &rusqlite::Connection) -> Result<(), String> {
    ensure_column(conn, "settings", "value_json", "TEXT")?;

    let has_legacy_value = column_exists(conn, "settings", "value")?;
    if has_legacy_value {
        conn.execute(
            "UPDATE settings
             SET value_json=value
             WHERE (value_json IS NULL OR value_json='') AND value IS NOT NULL",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    let default_settings =
        serde_json::to_string(&Settings::default()).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE settings
         SET value_json=?1
         WHERE key='app_settings' AND (value_json IS NULL OR value_json='')",
        params![default_settings],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE settings SET value_json='null' WHERE value_json IS NULL",
        [],
    )
    .map_err(|e| e.to_string())?;

    if has_legacy_value {
        conn.execute_batch(
            "DROP TABLE IF EXISTS settings_migration;
             CREATE TABLE settings_migration (
                key        TEXT PRIMARY KEY,
                value_json TEXT NOT NULL
             );
             INSERT OR REPLACE INTO settings_migration(key,value_json)
             SELECT key,value_json FROM settings WHERE key IS NOT NULL;
             DROP TABLE settings;
             ALTER TABLE settings_migration RENAME TO settings;",
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// SQLite 存储。内部用 `Mutex<Connection>` 保证写串行（rusqlite Connection 非 Sync）。
/// API 面与 `db::Database` 对齐，供 commands.rs 无缝切换。
pub struct SqliteDb {
    conn: Mutex<Connection>,
}

impl SqliteDb {
    // ========================================================================
    // 生命周期
    // ========================================================================

    /// 打开（或创建）磁盘库，启用 WAL + 外键，并跑迁移。
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// 内存库（单元测试用）。
    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// 写入/更新单个游戏（幂等 upsert）。
    pub fn upsert_game(&self, game: &Game) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        upsert_with(&conn, game)
    }

    /// Idempotent, transactional migration. Connection PRAGMAs live outside the
    /// transaction; every schema/data change and the version bump commit together.
    fn migrate(&self) -> Result<(), String> {
        self.migrate_with_hook(|| Ok(()))
    }

    fn migrate_with_hook<F>(&self, after_v4_schema: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;

        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS games (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                sort_name  TEXT,
                game_type  TEXT,
                favorite   INTEGER NOT NULL DEFAULT 0,
                hidden     INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                data_json  TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS settings (
                key        TEXT PRIMARY KEY,
                value_json TEXT NOT NULL
             );",
        )
        .map_err(|e| e.to_string())?;

        // Preserve the existing v1/v2 repair path inside this transaction.
        migrate_games_table(&tx)?;
        migrate_settings_table(&tx)?;
        tx.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_games_sort ON games(sort_name);
             CREATE INDEX IF NOT EXISTS idx_games_type ON games(game_type);
             CREATE TABLE IF NOT EXISTS schema_migrations (
                version    INTEGER PRIMARY KEY,
                name       TEXT NOT NULL,
                checksum   TEXT NOT NULL,
                applied_at TEXT NOT NULL
             );",
        )
        .map_err(|e| e.to_string())?;

        let legacy_version: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if legacy_version > SCHEMA_VERSION {
            return Err(format!(
                "database schema version {legacy_version} is newer than supported version {SCHEMA_VERSION}"
            ));
        }

        validate_migration_ledger(&tx)?;
        // Pre-ledger v1/v2 databases are now structurally repaired, so ledger rows
        // are backfilled atomically before v3 is applied.
        record_migration(&tx, migration_by_version(1).expect("v1 migration exists"))?;
        record_migration(&tx, migration_by_version(2).expect("v2 migration exists"))?;

        apply_v3_schema(&tx)?;
        record_migration(&tx, migration_by_version(3).expect("v3 migration exists"))?;

        apply_v4_schema(&tx)?;
        // Test-only failure injection exercises the same transaction used in production.
        after_v4_schema()?;
        record_migration(&tx, migration_by_version(4).expect("v4 migration exists"))?;

        apply_v5_schema(&tx)?;
        record_migration(&tx, migration_by_version(5).expect("v5 migration exists"))?;

        apply_v6_schema(&tx)?;
        record_migration(&tx, migration_by_version(6).expect("v6 migration exists"))?;

        let have: i64 = tx
            .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if have == 0 {
            tx.execute(
                "INSERT INTO schema_version(version) VALUES(?1)",
                params![SCHEMA_VERSION],
            )
            .map_err(|e| e.to_string())?;
        } else {
            tx.execute(
                "UPDATE schema_version SET version=?1",
                params![SCHEMA_VERSION],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())
    }

    pub(crate) fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        operation(&conn)
    }

    pub(crate) fn with_connection_mut<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        operation(&mut conn)
    }

    /// 当前 schema 版本。
    pub fn schema_version(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())
    }

    // ========================================================================
    // 内部 helper：读-改-写（持锁，原子）
    // ========================================================================

    /// 原子地读取、修改、写回一个游戏。不存在的 id 返回 `Err("游戏不存在")`。
    fn with_game_mut<F, R>(&self, id: &str, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut Game) -> Result<R, String>,
    {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let json: String = conn
            .query_row(
                "SELECT data_json FROM games WHERE id=?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(|_| "游戏不存在".to_string())?;
        let mut game: Game = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        game.normalize_for_persistence();
        let result = f(&mut game)?;
        game.touch_updated();
        upsert_with(&conn, &game)?;
        Ok(result)
    }

    // ========================================================================
    // 游戏查询
    // ========================================================================

    /// 获取所有游戏（按排序名）。
    pub fn get_games(&self) -> Result<Vec<Game>, String> {
        self.list_games()
    }

    /// 获取单个游戏（兼容 db::Database 签名：不存在返回 Err）。
    pub fn get_game(&self, id: &str) -> Result<Game, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let json: String = conn
            .query_row(
                "SELECT data_json FROM games WHERE id=?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(|_| "游戏不存在".to_string())?;
        let mut game: Game = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        game.normalize_for_persistence();
        Ok(game)
    }

    /// 获取单个游戏（Option 版）。
    pub fn get_game_opt(&self, id: &str) -> Result<Option<Game>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let row: rusqlite::Result<String> = conn.query_row(
            "SELECT data_json FROM games WHERE id=?1",
            params![id],
            |r| r.get(0),
        );
        match row {
            Ok(j) => {
                let mut game: Game = serde_json::from_str(&j).map_err(|e| e.to_string())?;
                game.normalize_for_persistence();
                Ok(Some(game))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// 列出全部游戏（按排序名）。
    pub fn list_games(&self) -> Result<Vec<Game>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT data_json FROM games ORDER BY sort_name")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        collect_games(rows)
    }

    /// 全文搜索（名称/描述/标签/别名/开发商/发行商/原版标题/exe路径）。
    /// 复用 db::Database 的搜索逻辑：加载全部→内存过滤。
    pub fn search_games(&self, query: &str) -> Result<Vec<Game>, String> {
        let query = query.trim();
        if query.is_empty() {
            return self.list_games();
        }
        let query_lower = query.to_lowercase();

        // ASCII queries use SQLite to narrow the candidate set before JSON
        // deserialization. Non-ASCII case folding stays in Rust so CJK and
        // Unicode behavior remains identical to the legacy implementation.
        let candidates = if query.is_ascii() {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            let escaped = query
                .replace('\\', r"\\")
                .replace('%', r"\%")
                .replace('_', r"\_");
            let pattern = format!("%{escaped}%");
            let mut statement = conn
                .prepare(
                    r"SELECT data_json FROM games
                      WHERE name LIKE ?1 ESCAPE '\' OR sort_name LIKE ?1 ESCAPE '\'
                         OR data_json LIKE ?1 ESCAPE '\'
                      ORDER BY sort_name",
                )
                .map_err(|e| e.to_string())?;
            let rows = statement
                .query_map(params![pattern], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            collect_games(rows)?
        } else {
            self.list_games()?
        };

        Ok(candidates
            .into_iter()
            .filter(|game| game_matches_search(game, &query_lower))
            .collect())
    }

    /// 名称前缀模糊搜索（SQL 索引级，快）。
    pub fn search_games_fast(&self, query: &str) -> Result<Vec<Game>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let like = format!("%{}%", query.to_lowercase());
        let mut stmt = conn
            .prepare(
                "SELECT data_json FROM games WHERE sort_name LIKE ?1 OR name LIKE ?1 ORDER BY sort_name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![like], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        collect_games(rows)
    }

    /// 游戏总数。
    pub fn game_count(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT COUNT(*) FROM games", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    /// 导出完整数据库快照（含设置）。
    pub fn export_data(&self) -> Result<AppDatabase, String> {
        let games = self.list_games()?;
        let settings = self.get_settings()?;
        Ok(AppDatabase {
            schema_version: self.schema_version()? as u32,
            games,
            settings,
        })
    }

    /// 替换完整数据库（全量导入）。
    pub fn replace_data(&self, data: &AppDatabase) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM games", [])
            .map_err(|e| e.to_string())?;
        for g in &data.games {
            upsert_with(&tx, g)?;
        }
        // 保存设置
        let settings_json = serde_json::to_string(&data.settings).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO settings(key,value_json) VALUES('app_settings',?1)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            params![settings_json],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    // ========================================================================
    // 游戏增删改
    // ========================================================================

    /// 添加游戏（自动去重——按 exe_path）。
    pub fn add_game(&self, game: Game) -> Result<Game, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        // 查重
        let dup: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE id IN (SELECT id FROM games) AND data_json LIKE ?1",
                params![format!("%{}%", &game.exe_path.replace(['%', '_'], ""))],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        // 简单去重：加载所有，按 exe_path 比对
        if dup {
            // 改用全量检查
            let mut stmt = conn
                .prepare("SELECT data_json FROM games")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for r in rows {
                let j = r.map_err(|e| e.to_string())?;
                if let Ok(g) = serde_json::from_str::<Game>(&j) {
                    if g.exe_path == game.exe_path {
                        return Err("该游戏已存在".to_string());
                    }
                }
            }
        }

        let game = normalize_game_for_persistence(&game);
        let result = game.clone();
        upsert_with(&conn, &game)?;
        Ok(result)
    }

    /// 全量更新游戏（整对象替换）。
    pub fn update_game(&self, game: Game) -> Result<Game, String> {
        let game = normalize_game_for_persistence(&game);
        let result = game.clone();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        // 确认存在
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE id=?1",
                params![game.id],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .map_err(|e| e.to_string())?;
        if !exists {
            return Err("游戏不存在".to_string());
        }
        upsert_with(&conn, &game)?;
        Ok(result)
    }

    /// 删除游戏。
    pub fn delete_game(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let affected = conn
            .execute("DELETE FROM games WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
        if affected == 0 {
            return Err("游戏不存在".to_string());
        }
        Ok(())
    }

    /// 批量导入（**迁移桥的核心写入路径**）：单事务、幂等 upsert。
    pub fn import_games(&self, games: &[Game]) -> Result<usize, String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let mut n = 0usize;
        for g in games {
            let g = normalize_game_for_persistence(g);
            upsert_with(&tx, &g)?;
            n += 1;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(n)
    }

    // ========================================================================
    // 快捷布尔切换
    // ========================================================================

    pub fn toggle_favorite(&self, id: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.favorite = !game.favorite;
            Ok(game.clone())
        })
    }

    pub fn toggle_hidden(&self, id: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.hidden = !game.hidden;
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 基本信息更新
    // ========================================================================

    pub fn update_game_name(&self, id: &str, name: String) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.name = name;
            Ok(game.clone())
        })
    }

    pub fn update_game_description(
        &self,
        id: &str,
        description: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.description = description;
            Ok(game.clone())
        })
    }

    pub fn update_game_cover(&self, id: &str, cover: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.cover = cover;
            game.sync_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_game_background(
        &self,
        id: &str,
        background: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.background = background;
            game.sync_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_game_icon(&self, id: &str, icon: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.icon = icon;
            Ok(game.clone())
        })
    }

    pub fn update_game_type(&self, id: &str, game_type: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.game_type = game_type;
            Ok(game.clone())
        })
    }

    pub fn update_install_dir(
        &self,
        id: &str,
        install_dir: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.install_dir = install_dir;
            Ok(game.clone())
        })
    }

    pub fn update_exe_path(&self, id: &str, exe_path: String) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.exe_path = exe_path;
            Ok(game.clone())
        })
    }

    pub fn update_game_created_at(&self, id: &str, created_at: String) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.created_at = created_at;
            Ok(game.clone())
        })
    }

    pub fn update_add_date(&self, id: &str, add_date: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.add_date = add_date;
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 简单标签
    // ========================================================================

    pub fn add_simple_tag(&self, id: &str, tag: String) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if !game.tags.contains(&tag) {
                game.tags.push(tag);
            }
            Ok(game.clone())
        })
    }

    pub fn remove_simple_tag(&self, id: &str, tag: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.tags.retain(|t| t != tag);
            Ok(game.clone())
        })
    }

    pub fn set_simple_tags(&self, id: &str, tags: Vec<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.tags = tags;
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 增强标签
    // ========================================================================

    pub fn add_tag_entry(&self, id: &str, tag: Tag) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if !game.tag_entries.iter().any(|t| t.name == tag.name) {
                game.tag_entries.push(tag);
            }
            Ok(game.clone())
        })
    }

    pub fn remove_tag_entry(&self, id: &str, tag_name: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.tag_entries.retain(|t| t.name != tag_name);
            Ok(game.clone())
        })
    }

    pub fn update_tag_entry(&self, id: &str, tag_name: &str, tag: Tag) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if let Some(entry) = game.tag_entries.iter_mut().find(|t| t.name == tag_name) {
                *entry = tag;
            }
            Ok(game.clone())
        })
    }

    pub fn set_tag_entries(&self, id: &str, tags: Vec<Tag>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.tag_entries = tags;
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 别名
    // ========================================================================

    pub fn add_alias(&self, id: &str, alias: GameAlias) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if !game.aliases.iter().any(|a| a.name == alias.name) {
                game.aliases.push(alias);
            }
            Ok(game.clone())
        })
    }

    pub fn remove_alias(&self, id: &str, alias_name: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.aliases.retain(|a| a.name != alias_name);
            Ok(game.clone())
        })
    }

    pub fn set_primary_alias(&self, id: &str, alias_name: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            for alias in &mut game.aliases {
                alias.is_primary = alias.name == alias_name;
            }
            Ok(game.clone())
        })
    }

    pub fn set_aliases(&self, id: &str, aliases: Vec<GameAlias>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.aliases = aliases;
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 元数据
    // ========================================================================

    pub fn update_game_metadata(&self, id: &str, metadata: GameMetadata) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata = metadata;
            game.sync_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_developer(&self, id: &str, developer: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.developer = developer;
            Ok(game.clone())
        })
    }

    pub fn update_publisher(&self, id: &str, publisher: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.publisher = publisher;
            Ok(game.clone())
        })
    }

    pub fn update_platform(
        &self,
        id: &str,
        platform: Option<GamePlatform>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.platform = platform;
            Ok(game.clone())
        })
    }

    pub fn update_engine(&self, id: &str, engine: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.engine = engine;
            Ok(game.clone())
        })
    }

    pub fn update_game_version(&self, id: &str, version: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.version = version;
            Ok(game.clone())
        })
    }

    pub fn update_original_name(
        &self,
        id: &str,
        original_name: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.original_name = original_name;
            Ok(game.clone())
        })
    }

    pub fn update_homepage(&self, id: &str, homepage: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.homepage = homepage;
            Ok(game.clone())
        })
    }

    pub fn update_developer_homepage(
        &self,
        id: &str,
        developer_homepage: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.developer_homepage = developer_homepage;
            Ok(game.clone())
        })
    }

    pub fn update_age_rating(&self, id: &str, age_rating: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.age_rating = age_rating;
            Ok(game.clone())
        })
    }

    pub fn update_series(&self, id: &str, series: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.series = series;
            Ok(game.clone())
        })
    }

    pub fn update_release_date(
        &self,
        id: &str,
        release_date: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.release_date = release_date;
            Ok(game.clone())
        })
    }

    pub fn update_release_year(&self, id: &str, release_year: Option<u32>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.release_year = release_year;
            game.sync_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_estimated_hours(
        &self,
        id: &str,
        estimated_hours: Option<f64>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.estimated_hours = estimated_hours;
            Ok(game.clone())
        })
    }

    pub fn update_vndb_rating(&self, id: &str, vndb_rating: Option<f64>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.vndb_rating = vndb_rating;
            Ok(game.clone())
        })
    }

    pub fn update_bangumi_rating(
        &self,
        id: &str,
        bangumi_rating: Option<f64>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.bangumi_rating = bangumi_rating;
            Ok(game.clone())
        })
    }

    pub fn update_vndb_id(&self, id: &str, vndb_id: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.vndb_id = vndb_id;
            game.sync_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_bangumi_id(&self, id: &str, bangumi_id: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.bangumi_id = bangumi_id;
            game.sync_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn set_genres(&self, id: &str, genres: Vec<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.genres = genres;
            Ok(game.clone())
        })
    }

    pub fn set_languages(&self, id: &str, languages: Vec<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.languages = languages;
            Ok(game.clone())
        })
    }

    pub fn set_voice_languages(
        &self,
        id: &str,
        voice_languages: Vec<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.metadata.voice_languages = voice_languages;
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 游玩追踪
    // ========================================================================

    pub fn update_play_tracker(&self, id: &str, tracker: PlayTracker) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker = tracker;
            game.sync_tracker_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn start_session(&self, id: &str) -> Result<String, String> {
        self.with_game_mut(id, |game| Ok(game.start_session()))
    }

    pub fn end_session(
        &self,
        id: &str,
        session_id: &str,
        duration_seconds: u64,
    ) -> Result<bool, String> {
        self.with_game_mut(
            id,
            |game| Ok(game.end_session(session_id, duration_seconds)),
        )
    }

    pub fn update_completion_status(
        &self,
        id: &str,
        status: CompletionStatus,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.completion_status = status;
            Ok(game.clone())
        })
    }

    pub fn update_user_rating(&self, id: &str, rating: Option<f64>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.user_rating = rating;
            game.sync_tracker_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_review(&self, id: &str, review: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.review = review;
            Ok(game.clone())
        })
    }

    pub fn update_achievements(&self, id: &str, total: u32, unlocked: u32) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.achievements_total = total;
            game.play_tracker.achievements_unlocked = unlocked;
            Ok(game.clone())
        })
    }

    pub fn mark_finished(&self, id: &str, finished: bool) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.finished = finished;
            if finished {
                game.play_tracker.completion_count += 1;
                game.play_tracker.completion_status = CompletionStatus::Completed;
            }
            Ok(game.clone())
        })
    }

    pub fn get_play_sessions(&self, id: &str) -> Result<Vec<PlaySession>, String> {
        let game = self.get_game(id)?;
        Ok(game.play_tracker.sessions)
    }

    pub fn update_play_session(
        &self,
        id: &str,
        session_id: &str,
        session: PlaySession,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if let Some(s) = game
                .play_tracker
                .sessions
                .iter_mut()
                .find(|s| s.id == session_id)
            {
                *s = session;
            }
            game.play_tracker.total_seconds = game
                .play_tracker
                .sessions
                .iter()
                .map(|s| s.duration_seconds)
                .sum();
            game.sync_tracker_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn remove_play_session(&self, id: &str, session_id: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.sessions.retain(|s| s.id != session_id);
            game.play_tracker.total_seconds = game
                .play_tracker
                .sessions
                .iter()
                .map(|s| s.duration_seconds)
                .sum();
            game.sync_tracker_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn set_sessions(&self, id: &str, sessions: Vec<PlaySession>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.sessions = sessions;
            game.play_tracker.total_seconds = game
                .play_tracker
                .sessions
                .iter()
                .map(|s| s.duration_seconds)
                .sum();
            game.sync_tracker_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_total_seconds(&self, id: &str, total_seconds: u64) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.total_seconds = total_seconds;
            game.sync_tracker_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_first_played(
        &self,
        id: &str,
        first_played: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.first_played = first_played;
            Ok(game.clone())
        })
    }

    pub fn update_last_played(
        &self,
        id: &str,
        last_played: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.last_played = last_played;
            game.sync_tracker_to_legacy();
            Ok(game.clone())
        })
    }

    pub fn update_completion_count(&self, id: &str, count: u32) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.play_tracker.completion_count = count;
            Ok(game.clone())
        })
    }

    pub fn add_play_time(&self, id: &str, seconds: u64) -> Result<(), String> {
        self.with_game_mut(id, |game| {
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
            game.play_tracker.total_seconds += seconds;
            game.play_tracker.last_played = Some(now);
            game.sync_tracker_to_legacy();
            Ok(())
        })?;
        Ok(())
    }

    // ========================================================================
    // 截图
    // ========================================================================

    pub fn add_screenshot(&self, id: &str, path: String) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if !game.screenshots.contains(&path) {
                game.screenshots.push(path);
            }
            Ok(game.clone())
        })
    }

    pub fn remove_screenshot(&self, id: &str, index: usize) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if index < game.screenshots.len() {
                game.screenshots.remove(index);
                Ok(game.clone())
            } else {
                Err("截图索引越界".to_string())
            }
        })
    }

    pub fn remove_screenshot_by_path(&self, id: &str, path: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.screenshots.retain(|s| s != path);
            Ok(game.clone())
        })
    }

    pub fn set_screenshots(&self, id: &str, screenshots: Vec<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.screenshots = screenshots;
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 存档数据管理
    // ========================================================================

    pub fn update_save_data(&self, id: &str, save_data: SaveData) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.save_data = save_data;
            Ok(game.clone())
        })
    }

    pub fn set_save_dir(&self, id: &str, save_dir: Option<String>) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.save_data.save_dir = save_dir;
            Ok(game.clone())
        })
    }

    pub fn configure_auto_backup(
        &self,
        id: &str,
        auto_backup: bool,
        interval_minutes: u32,
        max_backups: u32,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.save_data.auto_backup = auto_backup;
            game.save_data.backup_interval_minutes = interval_minutes;
            game.save_data.max_backups = max_backups;
            Ok(game.clone())
        })
    }

    pub fn add_backup(&self, id: &str, backup: SaveBackup) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.save_data.backups.push(backup);
            Ok(game.clone())
        })
    }

    pub fn remove_backup(&self, id: &str, backup_id: &str) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.save_data.backups.retain(|b| b.id != backup_id);
            Ok(game.clone())
        })
    }

    pub fn update_backup_note(
        &self,
        id: &str,
        backup_id: &str,
        note: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if let Some(backup) = game
                .save_data
                .backups
                .iter_mut()
                .find(|b| b.id == backup_id)
            {
                backup.note = note;
            }
            Ok(game.clone())
        })
    }

    pub fn configure_cloud_sync(
        &self,
        id: &str,
        cloud_sync: bool,
        cloud_provider: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            game.save_data.cloud_sync = cloud_sync;
            game.save_data.cloud_provider = cloud_provider;
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 刮削结果应用（扩展版）
    // ========================================================================

    #[allow(clippy::too_many_arguments)]
    pub fn apply_scrape_result(
        &self,
        id: &str,
        title: Option<String>,
        description: Option<String>,
        cover: Option<String>,
        background: Option<String>,
        tags: Option<Vec<String>>,
        rating: Option<f64>,
        release_year: Option<u32>,
        source: Option<&str>,
        source_id: Option<String>,
        developer: Option<String>,
        publisher: Option<String>,
        genres: Option<Vec<String>>,
        languages: Option<Vec<String>>,
        engine: Option<String>,
        age_rating: Option<String>,
        series: Option<String>,
        release_date: Option<String>,
        voice_languages: Option<Vec<String>>,
        aliases: Option<Vec<String>>,
        screenshots: Option<Vec<String>>,
        homepage: Option<String>,
    ) -> Result<Game, String> {
        self.with_game_mut(id, |game| {
            if let Some(t) = title {
                game.name = t;
            }
            if let Some(d) = description {
                game.description = Some(d);
            }
            if let Some(c) = cover {
                game.metadata.cover = Some(c);
            }
            if let Some(b) = background {
                game.metadata.background = Some(b);
            }
            if let Some(t) = tags {
                game.tags = t;
            }
            if let Some(r) = rating {
                match source {
                    Some("vndb") => game.metadata.vndb_rating = Some(r),
                    Some("bangumi") => game.metadata.bangumi_rating = Some(r),
                    _ => game.play_tracker.user_rating = Some(r),
                }
            }
            if let Some(y) = release_year {
                game.metadata.release_year = Some(y);
            }
            if let Some(s) = source {
                if let Some(ref sid) = source_id {
                    if s == "vndb" {
                        game.metadata.vndb_id = Some(sid.clone());
                    } else if s == "bangumi" {
                        game.metadata.bangumi_id = Some(sid.clone());
                    }
                    match s {
                        "dlsite" => push_store_link(
                            &mut game.metadata.stores,
                            "DLsite",
                            format!("https://www.dlsite.com/maniax/work/=/product_id/{sid}.html"),
                        ),
                        "steam" => push_store_link(
                            &mut game.metadata.stores,
                            "Steam",
                            format!("https://store.steampowered.com/app/{sid}/"),
                        ),
                        "erogamescape" => push_store_link(
                            &mut game.metadata.stores,
                            "ErogameScape",
                            format!(
                                "https://erogamescape.dyndns.org/~ap2/ero/toukei_kaiseki/game.php?game={sid}"
                            ),
                        ),
                        "touchgal" | "kungal" => {
                            game.metadata.homepage =
                                Some(format!("https://www.kungal.com/galgame/{sid}"));
                        }
                        "pcgw" => {
                            let pcgw_title = sid.replace(' ', "_");
                            game.metadata.homepage =
                                Some(format!("https://www.pcgamingwiki.com/wiki/{pcgw_title}"));
                        }
                        "ymgal" => {
                            game.metadata.homepage =
                                Some(format!("https://www.ymgal.games/game/{sid}"));
                        }
                        _ => {}
                    }
                }
            }
            if let Some(dev) = developer {
                game.metadata.developer = Some(dev);
            }
            if let Some(pub_) = publisher {
                game.metadata.publisher = Some(pub_);
            }
            if let Some(g) = genres {
                game.metadata.genres = g;
            }
            if let Some(l) = languages {
                game.metadata.languages = l;
            }
            if let Some(e) = engine {
                game.metadata.engine = Some(e);
            }
            if let Some(a) = age_rating {
                game.metadata.age_rating = Some(a);
            }
            if let Some(s) = series {
                game.metadata.series = Some(s);
            }
            if let Some(rd) = release_date {
                game.metadata.release_date = Some(rd);
            }
            if let Some(vl) = voice_languages {
                game.metadata.voice_languages = vl;
            }
            if let Some(a) = aliases {
                for alias_name in a {
                    if !game
                        .aliases
                        .iter()
                        .any(|existing| existing.name == alias_name)
                    {
                        game.aliases.push(GameAlias {
                            name: alias_name,
                            language: None,
                            source: source.map(|s| s.to_string()),
                            is_primary: false,
                        });
                    }
                }
            }
            if let Some(ss) = screenshots {
                game.screenshots = ss;
            }
            if let Some(hp) = homepage {
                game.metadata.homepage = Some(hp);
            }
            game.sync_to_legacy();
            game.sync_tracker_to_legacy();
            Ok(game.clone())
        })
    }

    // ========================================================================
    // 设置
    // ========================================================================

    /// 获取完整设置（从 KV 表中反序列化，若无则返回默认值）。
    pub fn get_settings(&self) -> Result<Settings, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let row: rusqlite::Result<String> = conn.query_row(
            "SELECT value_json FROM settings WHERE key='app_settings'",
            [],
            |r| r.get(0),
        );
        match row {
            Ok(j) => serde_json::from_str(&j).map_err(|e| e.to_string()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Settings::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// 保存完整设置。
    pub fn update_settings(&self, settings: &Settings) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO settings(key,value_json) VALUES('app_settings',?1)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            params![json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ---- KV 设置（向后兼容） ----

    /// 读设置（JSON 字符串）。
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let row: rusqlite::Result<String> = conn.query_row(
            "SELECT value_json FROM settings WHERE key=?1",
            params![key],
            |r| r.get(0),
        );
        match row {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// 写设置（JSON 字符串）。
    pub fn set_setting(&self, key: &str, value_json: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO settings(key,value_json) VALUES(?1,?2)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            params![key, value_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ============================================================================
// 内部函数
// ============================================================================

/// 在给定连接/事务上 upsert 一个游戏（投影列 + data_json）。
fn upsert_with(conn: &Connection, game: &Game) -> Result<(), String> {
    let game = normalize_game_for_persistence(game);
    let json = serde_json::to_string(&game).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO games (id,name,sort_name,game_type,favorite,hidden,created_at,updated_at,data_json)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, sort_name=excluded.sort_name, game_type=excluded.game_type,
            favorite=excluded.favorite, hidden=excluded.hidden, updated_at=excluded.updated_at,
            data_json=excluded.data_json",
        params![
            game.id,
            game.name,
            game.name.to_lowercase(),
            game.game_type,
            game.favorite as i64,
            game.hidden as i64,
            game.created_at,
            game.updated_at,
            json
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn normalize_game_for_persistence(game: &Game) -> Game {
    let mut game = game.clone();
    game.normalize_for_persistence();
    game
}

fn push_store_link(stores: &mut Vec<StoreLink>, name: &str, url: String) {
    if stores
        .iter()
        .any(|store| store.name == name && store.url == url)
    {
        return;
    }
    stores.push(StoreLink {
        name: name.to_string(),
        url,
        price: None,
        currency: None,
    });
}

/// 把 `query_map` 的行（data_json 字符串）收成 `Vec<Game>`，跳过反序列化失败的脏行。
fn game_matches_search(game: &Game, query_lower: &str) -> bool {
    game.name.to_lowercase().contains(query_lower)
        || game
            .description
            .as_deref()
            .is_some_and(|value| value.to_lowercase().contains(query_lower))
        || game
            .tags
            .iter()
            .any(|value| value.to_lowercase().contains(query_lower))
        || game
            .tag_entries
            .iter()
            .any(|value| value.name.to_lowercase().contains(query_lower))
        || game
            .aliases
            .iter()
            .any(|value| value.name.to_lowercase().contains(query_lower))
        || game
            .metadata
            .developer
            .as_deref()
            .is_some_and(|value| value.to_lowercase().contains(query_lower))
        || game
            .metadata
            .publisher
            .as_deref()
            .is_some_and(|value| value.to_lowercase().contains(query_lower))
        || game
            .metadata
            .original_name
            .as_deref()
            .is_some_and(|value| value.to_lowercase().contains(query_lower))
        || game.exe_path.to_lowercase().contains(query_lower)
}

fn collect_games(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<String>>,
) -> Result<Vec<Game>, String> {
    let mut out = Vec::new();
    for r in rows {
        let j = r.map_err(|e| e.to_string())?;
        if let Ok(mut g) = serde_json::from_str::<Game>(&j) {
            g.normalize_for_persistence();
            out.push(g);
        }
    }
    Ok(out)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn mock(id: &str, name: &str) -> Game {
        serde_json::from_str(&format!(
            r#"{{"id":"{id}","name":"{name}","exe_path":"x_{id}.exe","install_dir":null,
                 "game_type":null,"created_at":"2026-01-01","updated_at":"2026-01-01",
                 "description":null,"cover":null,"background":null,"icon":null}}"#
        ))
        .expect("mock game should deserialize")
    }

    #[test]
    fn schema_and_settings() {
        let db = SqliteDb::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        db.set_setting("theme", "\"sakura\"").unwrap();
        assert_eq!(
            db.get_setting("theme").unwrap().as_deref(),
            Some("\"sakura\"")
        );
        assert_eq!(db.get_setting("missing").unwrap(), None);
    }

    #[test]
    fn v6_schema_contains_ledger_domain_tables_and_indexes() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.with_connection(|conn| {
            let ledger = conn
                .prepare("SELECT version, name, checksum FROM schema_migrations ORDER BY version")
                .map_err(|e| e.to_string())?
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            assert_eq!(ledger.len(), SCHEMA_MIGRATIONS.len());
            for (actual, expected) in ledger.iter().zip(SCHEMA_MIGRATIONS) {
                assert_eq!(actual.0, expected.version);
                assert_eq!(actual.1, expected.name);
                assert_eq!(actual.2, expected.checksum);
            }

            for table in [
                "activity_events",
                "progress_records",
                "provider_health",
                "background_jobs",
                "provider_configs",
                "ai_task_results",
                "background_job_events",
            ] {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                        params![table],
                        |row| row.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                assert!(exists, "missing schema table {table}");
            }

            for index in [
                "idx_activity_events_started_at",
                "idx_activity_events_type_started_at",
                "idx_activity_events_resource_started_at",
                "idx_progress_records_resource_updated",
                "idx_progress_records_updated_at",
                "idx_provider_health_state",
                "idx_background_jobs_status_updated",
                "idx_background_jobs_kind_status",
                "idx_provider_configs_resource_enabled",
                "idx_provider_configs_kind",
                "idx_ai_task_results_expires",
                "idx_background_job_events_job_sequence_desc",
            ] {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name=?1)",
                        params![index],
                        |row| row.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                assert!(exists, "missing schema index {index}");
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn v6_job_event_schema_enforces_constraints_cascade_and_retention() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO background_jobs(id,kind,title,status,progress,created_at,updated_at,metadata_json)
                 VALUES('active','import','Import','running',0.0,'2026-07-01T00:00:00Z','2026-07-01T00:00:00Z','{}')",
                [],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO background_job_events(job_id,sequence,level,code,message,progress,created_at)
                 VALUES('active',1,'info','started','safe',0.0,'2026-07-01T00:00:00Z')",
                [],
            )
            .map_err(|e| e.to_string())?;
            assert!(conn
                .execute(
                    "INSERT INTO background_job_events(job_id,sequence,level,code,message,progress,created_at)
                     VALUES('active',1,'info','duplicate','safe',0.0,'2026-07-01T00:00:00Z')",
                    [],
                )
                .is_err());
            assert!(conn
                .execute(
                    "INSERT INTO background_job_events(job_id,sequence,level,code,message,progress,created_at)
                     VALUES('active',2,'info','bad_progress','safe',1.1,'2026-07-01T00:00:00Z')",
                    [],
                )
                .is_err());
            conn.execute("DELETE FROM background_jobs WHERE id='active'", [])
                .map_err(|e| e.to_string())?;
            let cascaded: i64 = conn
                .query_row("SELECT COUNT(*) FROM background_job_events WHERE job_id='active'", [], |row| row.get(0))
                .map_err(|e| e.to_string())?;
            assert_eq!(cascaded, 0);

            conn.execute(
                "INSERT INTO background_jobs(id,kind,title,status,progress,created_at,updated_at,metadata_json)
                 VALUES('old-terminal','import','Import','succeeded',1.0,'2020-01-01T00:00:00Z','2020-01-01T00:00:00Z','{}')",
                [],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO background_job_events(job_id,sequence,level,code,message,progress,created_at)
                 VALUES('old-terminal',1,'info','complete','safe',1.0,'2020-01-01T00:00:00Z')",
                [],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();
        // Migrations run on every open and also execute the fixed 30-day
        // terminal-event cleanup, so this exercises the migration path itself.
        db.migrate().unwrap();
        db.with_connection(|conn| {
            let retained: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM background_job_events WHERE job_id='old-terminal'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            assert_eq!(retained, 0);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn v2_upgrade_is_idempotent_and_preserves_legacy_rows() {
        let path = std::env::temp_dir().join(format!(
            "moeplay_v2_upgrade_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE schema_version (version INTEGER NOT NULL);
                 INSERT INTO schema_version(version) VALUES (2);
                 CREATE TABLE games (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    sort_name TEXT,
                    game_type TEXT,
                    favorite INTEGER NOT NULL DEFAULT 0,
                    hidden INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT,
                    updated_at TEXT,
                    data_json TEXT NOT NULL
                 );
                 INSERT INTO games(id,name,sort_name,data_json)
                 VALUES('legacy','Legacy Game','legacy','{}');
                 CREATE TABLE settings (key TEXT PRIMARY KEY, value_json TEXT NOT NULL);
                 INSERT INTO settings(key,value_json) VALUES('custom','\"kept\"');",
            )
            .unwrap();
        }

        {
            let db = SqliteDb::open(&path).unwrap();
            assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
            assert_eq!(db.game_count().unwrap(), 1);
            db.with_connection(|conn| {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                        row.get(0)
                    })
                    .map_err(|e| e.to_string())?;
                assert_eq!(count, SCHEMA_MIGRATIONS.len() as i64);
                Ok(())
            })
            .unwrap();
        }
        {
            let db = SqliteDb::open(&path).unwrap();
            assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
            assert_eq!(db.game_count().unwrap(), 1);
            db.with_connection(|conn| {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                        row.get(0)
                    })
                    .map_err(|e| e.to_string())?;
                assert_eq!(count, SCHEMA_MIGRATIONS.len() as i64);
                Ok(())
            })
            .unwrap();
        }

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    #[test]
    fn migration_ledger_checksum_mismatch_fails_closed() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.with_connection(|conn| {
            conn.execute(
                "UPDATE schema_migrations SET checksum='sha256:tampered' WHERE version=4",
                [],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
        .unwrap();
        let error = db.migrate().unwrap_err();
        assert!(error.contains("checksum mismatch for version 4"));
    }

    #[test]
    fn migration_failure_rolls_back_schema_and_ledger() {
        let db = SqliteDb {
            conn: Mutex::new(Connection::open_in_memory().unwrap()),
        };
        let error = db
            .migrate_with_hook(|| Err("injected v4 failure".to_string()))
            .unwrap_err();
        assert_eq!(error, "injected v4 failure");
        db.with_connection(|conn| {
            for table in [
                "schema_version",
                "games",
                "settings",
                "schema_migrations",
                "activity_events",
                "progress_records",
                "provider_health",
                "background_jobs",
                "provider_configs",
                "ai_task_results",
                "background_job_events",
            ] {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                        params![table],
                        |row| row.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                assert!(!exists, "failed migration left table {table}");
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn failed_v4_upgrade_preserves_existing_v3_schema_and_ledger() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.with_connection(|conn| {
            conn.execute_batch(
                "DROP TABLE provider_configs;
                 DROP TABLE ai_task_results;
                 DELETE FROM schema_migrations WHERE version>=4;
                 UPDATE schema_version SET version=3;
                 INSERT INTO provider_health(provider_id,operation,state)
                 VALUES('preserved','search','healthy');",
            )
            .map_err(|e| e.to_string())
        })
        .unwrap();

        let error = db
            .migrate_with_hook(|| Err("injected v4 upgrade failure".to_string()))
            .unwrap_err();
        assert_eq!(error, "injected v4 upgrade failure");
        assert_eq!(db.schema_version().unwrap(), 3);
        db.with_connection(|conn| {
            let ledger_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row.get(0))
                .map_err(|e| e.to_string())?;
            assert_eq!(ledger_count, 3);
            let provider_configs_exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='provider_configs')",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            assert!(!provider_configs_exists);
            let ai_results_exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='ai_task_results')",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            assert!(!ai_results_exists);
            let preserved: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM provider_health WHERE provider_id='preserved'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            assert_eq!(preserved, 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn crud_and_search() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.upsert_game(&mock("g1", "Steins Gate")).unwrap();
        db.upsert_game(&mock("g2", "Clannad")).unwrap();
        assert_eq!(db.game_count().unwrap(), 2);
        assert_eq!(db.get_game("g1").unwrap().name, "Steins Gate");
        assert_eq!(db.search_games_fast("clan").unwrap().len(), 1);
        db.delete_game("g1").unwrap();
        assert_eq!(db.game_count().unwrap(), 1);
    }

    #[test]
    fn whole_game_writes_normalize_legacy_fields() {
        let db = SqliteDb::open_in_memory().unwrap();
        let mut game = mock("legacy", "Legacy Only");
        game.cover = Some("legacy-cover.jpg".to_string());
        game.background = Some("legacy-bg.jpg".to_string());
        game.release_year = Some(2001);
        game.rating = Some(8.5);
        game.last_played = Some("2026-06-01 12:00".to_string());
        game.vndb_id = Some("v123".to_string());
        game.bangumi_id = Some("bgm456".to_string());
        game.play_time_seconds = 3600;

        let added = db.add_game(game).unwrap();
        assert_eq!(added.metadata.cover.as_deref(), Some("legacy-cover.jpg"));
        assert_eq!(added.metadata.background.as_deref(), Some("legacy-bg.jpg"));
        assert_eq!(added.metadata.release_year, Some(2001));
        assert_eq!(added.play_tracker.user_rating, Some(8.5));
        assert_eq!(
            added.play_tracker.last_played.as_deref(),
            Some("2026-06-01 12:00")
        );
        assert_eq!(added.play_tracker.total_seconds, 3600);

        let mut update = added.clone();
        update.metadata.release_year = Some(2002);
        update.release_year = Some(1999);
        update.play_tracker.user_rating = Some(9.0);
        update.rating = Some(1.0);
        update.play_tracker.total_seconds = 7200;
        update.play_time_seconds = 60;

        let updated = db.update_game(update).unwrap();
        assert_eq!(updated.metadata.release_year, Some(2002));
        assert_eq!(updated.release_year, Some(2002));
        assert_eq!(updated.play_tracker.user_rating, Some(9.0));
        assert_eq!(updated.rating, Some(9.0));
        assert_eq!(updated.play_tracker.total_seconds, 7200);
        assert_eq!(updated.play_time_seconds, 7200);
    }

    #[test]
    fn scrape_apply_writes_canonical_then_projects_legacy() {
        let db = SqliteDb::open_in_memory().unwrap();
        let mut game = mock("scrape", "Scrape Target");
        game.cover = Some("stale-cover.jpg".to_string());
        game.rating = Some(2.0);
        game.metadata.cover = Some("old-canonical-cover.jpg".to_string());
        game.play_tracker.user_rating = Some(4.0);
        db.upsert_game(&game).unwrap();

        let updated = db
            .apply_scrape_result(
                "scrape",
                Some("Scraped Target".to_string()),
                None,
                Some("scraped-cover.jpg".to_string()),
                Some("scraped-bg.jpg".to_string()),
                None,
                Some(8.5),
                Some(2024),
                Some("vndb"),
                Some("v12345".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();

        assert_eq!(updated.name, "Scraped Target");
        assert_eq!(updated.metadata.cover.as_deref(), Some("scraped-cover.jpg"));
        assert_eq!(updated.cover.as_deref(), Some("scraped-cover.jpg"));
        assert_eq!(
            updated.metadata.background.as_deref(),
            Some("scraped-bg.jpg")
        );
        assert_eq!(updated.background.as_deref(), Some("scraped-bg.jpg"));
        assert_eq!(updated.metadata.vndb_rating, Some(8.5));
        assert_eq!(updated.play_tracker.user_rating, Some(4.0));
        assert_eq!(updated.rating, Some(4.0));
        assert_eq!(updated.metadata.release_year, Some(2024));
        assert_eq!(updated.release_year, Some(2024));
        assert_eq!(updated.metadata.vndb_id.as_deref(), Some("v12345"));
        assert_eq!(updated.vndb_id.as_deref(), Some("v12345"));
    }

    #[test]
    fn scrape_apply_keeps_source_store_links_once() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.add_game(mock("dlsite", "DLsite Game")).unwrap();

        for _ in 0..2 {
            db.apply_scrape_result(
                "dlsite",
                Some("DLsite Game".to_string()),
                None,
                None,
                None,
                Some(vec![]),
                None,
                None,
                Some("dlsite"),
                Some("RJ123456".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        }

        let game = db.get_game("dlsite").unwrap();
        let dlsite_links: Vec<_> = game
            .metadata
            .stores
            .iter()
            .filter(|store| store.name == "DLsite")
            .collect();
        assert_eq!(dlsite_links.len(), 1);
        assert_eq!(
            dlsite_links[0].url,
            "https://www.dlsite.com/maniax/work/=/product_id/RJ123456.html"
        );
    }

    #[test]
    fn bulk_import_keeps_distinct_identity() {
        let db = SqliteDb::open_in_memory().unwrap();
        let games: Vec<Game> = (0..50)
            .map(|i| mock(&format!("g{i}"), &format!("Game {i}")))
            .collect();
        assert_eq!(db.import_games(&games).unwrap(), 50);
        assert_eq!(db.game_count().unwrap(), 50);
        for i in 0..50 {
            let g = db.get_game(&format!("g{i}")).unwrap();
            assert_eq!(g.name, format!("Game {i}"));
        }
        assert_eq!(db.import_games(&games).unwrap(), 50);
        assert_eq!(db.game_count().unwrap(), 50);
    }

    #[test]
    fn toggle_favorite_and_hidden() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.upsert_game(&mock("g1", "Test")).unwrap();
        let g = db.toggle_favorite("g1").unwrap();
        assert!(g.favorite);
        let g = db.toggle_hidden("g1").unwrap();
        assert!(g.hidden);
        let g = db.toggle_favorite("g1").unwrap();
        assert!(!g.favorite);
    }

    #[test]
    fn with_game_mut_atomic() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.upsert_game(&mock("g1", "Original")).unwrap();
        db.update_game_name("g1", "Updated".to_string()).unwrap();
        assert_eq!(db.get_game("g1").unwrap().name, "Updated");
    }

    #[test]
    fn full_settings_roundtrip() {
        let db = SqliteDb::open_in_memory().unwrap();
        let s = db.get_settings().unwrap();
        assert_eq!(s.theme, "dark");
        let mut s2 = s.clone();
        s2.theme = "sakura".to_string();
        db.update_settings(&s2).unwrap();
        assert_eq!(db.get_settings().unwrap().theme, "sakura");
    }

    #[test]
    fn migrates_legacy_settings_value_column() {
        let path = std::env::temp_dir().join(format!(
            "moeplay_legacy_settings_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .unwrap();

            let mut legacy = Settings::default();
            legacy.theme = "legacy".to_string();
            let legacy_json = serde_json::to_string(&legacy).unwrap();
            conn.execute(
                "INSERT INTO settings(key,value) VALUES('app_settings',?1)",
                params![legacy_json],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO settings(key,value) VALUES('custom',?1)",
                params!["\"kept\""],
            )
            .unwrap();
        }

        {
            let db = SqliteDb::open(&path).unwrap();
            assert_eq!(db.get_settings().unwrap().theme, "legacy");
            assert_eq!(
                db.get_setting("custom").unwrap().as_deref(),
                Some("\"kept\"")
            );

            let mut updated = db.get_settings().unwrap();
            updated.theme = "sakura".to_string();
            db.update_settings(&updated).unwrap();
            assert_eq!(db.get_settings().unwrap().theme, "sakura");
        }

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    }

    #[test]
    fn migrates_legacy_games_projection_columns() {
        let path = std::env::temp_dir().join(format!(
            "moeplay_legacy_games_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE schema_version (version INTEGER NOT NULL);
                 INSERT INTO schema_version (version) VALUES (1);
                 CREATE TABLE games (
                    id        TEXT PRIMARY KEY,
                    name      TEXT NOT NULL,
                    install_path TEXT NOT NULL,
                    sort_name TEXT,
                    game_type TEXT
                 );
                 CREATE TABLE settings (
                    key        TEXT PRIMARY KEY,
                    value_json TEXT NOT NULL
                 );",
            )
            .unwrap();
        }

        let db = SqliteDb::open(&path).unwrap();
        db.upsert_game(&mock("g1", "Game One")).unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(db.game_count().unwrap(), 1);
        assert_eq!(db.get_game("g1").unwrap().name, "Game One");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    }

    #[test]
    fn export_import_roundtrip() {
        let db = SqliteDb::open_in_memory().unwrap();
        db.upsert_game(&mock("g1", "Game One")).unwrap();
        db.upsert_game(&mock("g2", "Game Two")).unwrap();
        let exported = db.export_data().unwrap();
        assert_eq!(exported.games.len(), 2);

        let db2 = SqliteDb::open_in_memory().unwrap();
        db2.replace_data(&exported).unwrap();
        assert_eq!(db2.game_count().unwrap(), 2);
        assert_eq!(db2.get_game("g1").unwrap().name, "Game One");
    }

    #[test]
    fn search_finds_tags_and_aliases() {
        let db = SqliteDb::open_in_memory().unwrap();
        let mut g = mock("g1", "Visual Novel");
        g.tags = vec!["yuri".to_string(), "drama".to_string()];
        g.aliases = vec![GameAlias {
            name: "純愛".to_string(),
            language: Some("ja".to_string()),
            source: Some("vndb".to_string()),
            is_primary: false,
        }];
        db.upsert_game(&g).unwrap();
        assert_eq!(db.search_games("yuri").unwrap().len(), 1);
        assert_eq!(db.search_games("drama").unwrap().len(), 1);
        assert_eq!(db.search_games("notfound").unwrap().len(), 0);
    }

    #[test]
    fn concurrent_identity_isolation() {
        // 模拟并发处理不同游戏：每个的修改只影响自己
        let db = SqliteDb::open_in_memory().unwrap();
        let games: Vec<Game> = (0..20)
            .map(|i| mock(&format!("g{i}"), &format!("Game {i}")))
            .collect();
        db.import_games(&games).unwrap();

        // 逐个修改——每个改后断言其他不变
        for i in 0..20 {
            db.update_game_name(&format!("g{i}"), format!("Modified {i}"))
                .unwrap();
            // 检查前一个仍然保持其修改
            if i > 0 {
                assert_eq!(
                    db.get_game(&format!("g{}", i - 1)).unwrap().name,
                    format!("Modified {}", i - 1)
                );
            }
            // 检查后一个仍为原名
            if i < 19 {
                assert_eq!(
                    db.get_game(&format!("g{}", i + 1)).unwrap().name,
                    format!("Game {}", i + 1)
                );
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_scrape_result_keeps_distinct_identity() {
        use std::sync::Arc;
        use tokio::sync::Semaphore;
        use tokio::time::{timeout, Duration};

        let db = Arc::new(SqliteDb::open_in_memory().unwrap());
        let games: Vec<Game> = (0..24)
            .map(|i| mock(&format!("g{i}"), &format!("Game {i}")))
            .collect();
        db.import_games(&games).unwrap();

        let gate = Arc::new(Semaphore::new(4));
        let mut handles = Vec::new();
        for i in 0..24 {
            let db = Arc::clone(&db);
            let gate = Arc::clone(&gate);
            handles.push(tokio::spawn(async move {
                let _permit = gate.acquire_owned().await.unwrap();
                db.apply_scrape_result(
                    &format!("g{i}"),
                    Some(format!("Scraped Game {i}")),
                    Some(format!("Description {i}")),
                    Some(format!("cover-{i}.jpg")),
                    Some(format!("background-{i}.jpg")),
                    Some(vec![format!("tag-{i}")]),
                    Some(7.0 + (i as f64 / 10.0)),
                    Some(2000 + i),
                    Some("vndb"),
                    Some(format!("v{i:04}")),
                    Some(format!("Developer {i}")),
                    Some(format!("Publisher {i}")),
                    Some(vec![format!("genre-{i}")]),
                    Some(vec!["zh-CN".to_string(), format!("lang-{i}")]),
                    Some(format!("Engine {i}")),
                    Some(format!("R{i}")),
                    Some(format!("Series {i}")),
                    Some(format!("2026-{:02}-01", (i % 12) + 1)),
                    Some(vec![format!("voice-{i}")]),
                    Some(vec![format!("Alias {i}")]),
                    Some(vec![format!("shot-{i}.jpg")]),
                    Some(format!("https://example.test/game/{i}")),
                )
                .unwrap();
            }));
        }

        timeout(Duration::from_secs(5), async {
            for handle in handles {
                handle.await.unwrap();
            }
        })
        .await
        .expect("bounded concurrent scrape updates should finish");

        for i in 0..24 {
            let game = db.get_game(&format!("g{i}")).unwrap();
            assert_eq!(game.id, format!("g{i}"));
            assert_eq!(game.name, format!("Scraped Game {i}"));
            assert_eq!(game.exe_path, format!("x_g{i}.exe"));
            assert_eq!(game.tags, vec![format!("tag-{i}")]);
            assert_eq!(game.metadata.vndb_id, Some(format!("v{i:04}")));
            assert_eq!(game.metadata.developer, Some(format!("Developer {i}")));
            assert_eq!(game.metadata.cover, Some(format!("cover-{i}.jpg")));
            assert_eq!(game.screenshots, vec![format!("shot-{i}.jpg")]);
            assert_eq!(
                game.aliases.first().map(|alias| alias.name.clone()),
                Some(format!("Alias {i}"))
            );
        }
    }

    #[test]
    fn benchmark_1000_games() {
        let db = SqliteDb::open_in_memory().unwrap();
        let start = std::time::Instant::now();
        let games: Vec<Game> = (0..1000)
            .map(|i| mock(&format!("g{:04}", i), &format!("Game {}", i)))
            .collect();
        assert_eq!(db.import_games(&games).unwrap(), 1000);
        let import_ms = start.elapsed().as_millis();
        assert!(import_ms < 5000, "Import 1000 games took {}ms", import_ms);

        let t0 = std::time::Instant::now();
        let all = db.list_games().unwrap();
        assert_eq!(all.len(), 1000);
        assert!(t0.elapsed().as_millis() < 500, "List too slow");

        let t0 = std::time::Instant::now();
        let found = db.search_games("Game 500").unwrap();
        assert!(!found.is_empty());
        assert!(t0.elapsed().as_millis() < 200, "Search too slow");
    }
    #[test]
    fn benchmark_5000_games_release_candidate_budget() {
        let db = SqliteDb::open_in_memory().unwrap();
        let games: Vec<Game> = (0..5_000)
            .map(|index| {
                mock(
                    &format!("perf-{index:05}"),
                    &format!("Performance Game {index}"),
                )
            })
            .collect();

        let started = std::time::Instant::now();
        assert_eq!(db.import_games(&games).unwrap(), 5_000);
        let import_ms = started.elapsed().as_millis();
        assert!(import_ms < 15_000, "Import 5000 games took {import_ms}ms");

        let started = std::time::Instant::now();
        let all = db.list_games().unwrap();
        let list_ms = started.elapsed().as_millis();
        assert_eq!(all.len(), 5_000);
        assert!(list_ms < 2_000, "List 5000 games took {list_ms}ms");

        let started = std::time::Instant::now();
        let found = db.search_games("Performance Game 4321").unwrap();
        let search_ms = started.elapsed().as_millis();
        assert_eq!(
            found.first().map(|game| game.id.as_str()),
            Some("perf-04321")
        );
        assert!(search_ms < 750, "Search 5000 games took {search_ms}ms");
    }

    fn percentile_ms(samples: &mut [u128], percentile: usize) -> u128 {
        samples.sort_unstable();
        let index = ((samples.len() - 1) * percentile).div_ceil(100);
        samples[index.min(samples.len() - 1)]
    }

    #[test]
    #[ignore = "nightly Windows performance gate"]
    fn benchmark_10000_games_nightly() {
        let path = std::env::temp_dir().join(format!(
            "moeplay_10k_nightly_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);
        let games: Vec<Game> = (0..10_000)
            .map(|index| {
                mock(
                    &format!("nightly-{index:05}"),
                    &format!("Nightly Performance Game {index}"),
                )
            })
            .collect();

        let (list_ms, search_p95, update_p95, update_p99) = {
            let db = SqliteDb::open(&path).unwrap();
            assert_eq!(db.import_games(&games).unwrap(), 10_000);

            let started = std::time::Instant::now();
            let all = db.list_games().unwrap();
            let list_ms = started.elapsed().as_millis();
            assert_eq!(all.len(), 10_000);
            assert!(list_ms <= 4_000, "List 10000 games took {list_ms}ms");

            let mut search_samples = Vec::new();
            for index in (0..10_000).step_by(499).take(20) {
                let started = std::time::Instant::now();
                let found = db
                    .search_games(&format!("Nightly Performance Game {index}"))
                    .unwrap();
                search_samples.push(started.elapsed().as_millis());
                let expected_id = format!("nightly-{index:05}");
                assert!(found.iter().any(|game| game.id == expected_id));
            }
            let search_p95 = percentile_ms(&mut search_samples, 95);
            assert!(
                search_p95 <= 150,
                "Search 10000 games P95 took {search_p95}ms"
            );

            let mut update_samples = Vec::new();
            for index in 0..100 {
                let started = std::time::Instant::now();
                db.update_game_name("nightly-05000", format!("Nightly Updated Game {index}"))
                    .unwrap();
                update_samples.push(started.elapsed().as_millis());
            }
            let update_p95 = percentile_ms(&mut update_samples.clone(), 95);
            let update_p99 = percentile_ms(&mut update_samples, 99);
            assert!(update_p95 <= 50, "Update P95 took {update_p95}ms");
            assert!(update_p99 <= 150, "Update P99 took {update_p99}ms");

            // Recreate the metadata shape of a structurally repaired v2 database
            // so reopening measures the full v3-v5 migration on a 10k library.
            db.with_connection(|conn| {
                conn.execute_batch(
                    "DROP TABLE activity_events;
                     DROP TABLE progress_records;
                     DROP TABLE provider_health;
                     DROP TABLE background_jobs;
                     DROP TABLE provider_configs;
                     DROP TABLE ai_task_results;
                     DROP TABLE background_job_events;
                     DELETE FROM schema_migrations WHERE version>=3;
                     UPDATE schema_version SET version=2;",
                )
                .map_err(|error| error.to_string())
            })
            .unwrap();
            (list_ms, search_p95, update_p95, update_p99)
        };

        let migration_started = std::time::Instant::now();
        let migrated = SqliteDb::open(&path).unwrap();
        let migration_ms = migration_started.elapsed().as_millis();
        assert_eq!(migrated.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(migrated.game_count().unwrap(), 10_000);
        assert!(
            migration_ms <= 30_000,
            "v2 to v5 migration for 10000 games took {migration_ms}ms"
        );
        drop(migrated);

        let mut open_samples = Vec::new();
        for _ in 0..20 {
            let started = std::time::Instant::now();
            let db = SqliteDb::open(&path).unwrap();
            assert_eq!(db.game_count().unwrap(), 10_000);
            open_samples.push(started.elapsed().as_millis());
        }
        let open_p95 = percentile_ms(&mut open_samples, 95);
        assert!(open_p95 <= 250, "DB open P95 took {open_p95}ms");

        println!(
            "10k-nightly list_ms={list_ms} search_p95_ms={search_p95} update_p95_ms={update_p95} update_p99_ms={update_p99} migration_ms={migration_ms} open_p95_ms={open_p95}"
        );
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }
}
