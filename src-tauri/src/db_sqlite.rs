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

/// 当前 schema 版本。每次破坏性改表都 +1，并在 `migrate` 里加升级分支。
pub const SCHEMA_VERSION: i64 = 2;

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

    /// 幂等迁移：建表 → 补列 → 建索引，三步走保证旧库可升级。
    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Step 1: 基础表（不含索引，避免旧表缺列时 CREATE INDEX 失败）
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
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

        // Step 2: 补齐 v1 新增列（旧库升级时 CREATE TABLE 会跳过，需手动 ALTER）
        migrate_games_table(&conn)?;
        migrate_settings_table(&conn)?;

        // Step 3: 索引（此时列已保证存在）
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_games_sort ON games(sort_name);
             CREATE INDEX IF NOT EXISTS idx_games_type ON games(game_type);",
        )
        .map_err(|e| e.to_string())?;

        let have: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if have == 0 {
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute(
                "UPDATE schema_version SET version=?1 WHERE version < ?1",
                params![SCHEMA_VERSION],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 当前 schema 版本。
    pub fn schema_version(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
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
        let all = self.list_games()?;
        let query_lower = query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|game| {
                if game.name.to_lowercase().contains(&query_lower) {
                    return true;
                }
                if let Some(ref desc) = game.description {
                    if desc.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                if game
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_lower))
                {
                    return true;
                }
                if game
                    .tag_entries
                    .iter()
                    .any(|t| t.name.to_lowercase().contains(&query_lower))
                {
                    return true;
                }
                if game
                    .aliases
                    .iter()
                    .any(|a| a.name.to_lowercase().contains(&query_lower))
                {
                    return true;
                }
                if let Some(ref dev) = game.metadata.developer {
                    if dev.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                if let Some(ref pub_) = game.metadata.publisher {
                    if pub_.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                if let Some(ref orig) = game.metadata.original_name {
                    if orig.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                if game.exe_path.to_lowercase().contains(&query_lower) {
                    return true;
                }
                false
            })
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
}
