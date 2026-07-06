// 萌游 MoeGame · 数据库（SQLite 后端）
//
// Database 现在是 SqliteDb 的薄包装，向后兼容 commands.rs 的全部调用。
// 存储从单 JSON 文件迁移到 SQLite（WAL + 事务 + 索引）。

use crate::db_sqlite::SqliteDb;
use crate::models::{
    AppDatabase, CompletionStatus, Game, GameAlias, GameMetadata, GamePlatform, PlaySession,
    PlayTracker, SaveBackup, SaveData, Settings, Tag,
};
use std::path::PathBuf;

pub struct Database {
    db: SqliteDb,
    /// 旧 JSON 路径（用于数据迁移和向后兼容）
    _json_path: PathBuf,
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    /// 初始化数据库：打开 SQLite，若不存在则尝试从旧 JSON 迁移。
    pub fn new() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("moeplay");

        std::fs::create_dir_all(&data_dir).ok();

        let json_path = data_dir.join("database.json");
        let sqlite_path = data_dir.join("moegame.db");

        // 尝试打开 SQLite（含迁移）。若迁移失败则备份旧库再重建，避免静默内存库
        let db = match SqliteDb::open(&sqlite_path) {
            Ok(db) => db,
            Err(e) => {
                tracing::error!(error = %e, "SQLite migration failed — backing up and recreating");
                let backup_path = data_dir.join("moegame.db.bak");
                if let Err(be) = std::fs::copy(&sqlite_path, &backup_path) {
                    tracing::warn!(error = %be, "DB backup copy failed");
                }
                let _ = std::fs::remove_file(&sqlite_path);
                SqliteDb::open(&sqlite_path).unwrap_or_else(|e2| {
                    tracing::error!(error = %e2, "Fresh DB creation failed — using in-memory (data will NOT persist)");
                    SqliteDb::open_in_memory().unwrap_or_else(|e3| {
                        tracing::error!(error = %e3, "In-memory DB also failed — trying temp fallback");
                        let fallback = data_dir.join("moegame-fallback.db");
                        let _ = std::fs::remove_file(&fallback);
                        SqliteDb::open(&fallback).expect("至少应能打开一个 SQLite 数据库")
                    })
                })
            }
        };

        // 如果 SQLite 为空且 JSON 文件存在，自动迁移
        if let Ok(count) = db.game_count() {
            if count == 0 && json_path.exists() {
                tracing::info!("SQLite is empty, attempting auto-migration from JSON...");
                if let Ok(content) = std::fs::read_to_string(&json_path) {
                    if let Ok(mut app_db) = serde_json::from_str::<AppDatabase>(&content) {
                        if let Err(e) = crate::migration::run_migrations(&mut app_db) {
                            tracing::error!(error = %e, "JSON pre-migration failed");
                        } else if let Err(e) = db.replace_data(&app_db) {
                            tracing::error!(error = %e, "Auto-migration failed");
                        } else {
                            tracing::info!(games = app_db.games.len(), "Auto-migration success");
                        }
                    }
                }
            }
        }

        Self {
            db,
            _json_path: json_path,
        }
    }

    // ========================================================================
    // 游戏查询
    // ========================================================================

    pub fn get_games(&self) -> Vec<Game> {
        self.db.get_games().unwrap_or_default()
    }

    pub fn get_game(&self, id: &str) -> Result<Game, String> {
        self.db.get_game(id)
    }

    pub fn search_games(&self, query: &str) -> Vec<Game> {
        self.db.search_games(query).unwrap_or_default()
    }

    pub fn export_data(&self) -> AppDatabase {
        self.db.export_data().unwrap_or_default()
    }

    pub fn replace_data(&self, data: AppDatabase) -> Result<AppDatabase, String> {
        self.db.replace_data(&data).map(|_| data)
    }

    // ========================================================================
    // 游戏增删改
    // ========================================================================

    pub fn add_game(&self, game: Game) -> Result<Game, String> {
        self.db.add_game(game)
    }

    pub fn update_game(&self, game: Game) -> Result<Game, String> {
        self.db.update_game(game)
    }

    pub fn delete_game(&self, id: &str) -> Result<(), String> {
        self.db.delete_game(id)
    }

    // ========================================================================
    // 快捷布尔切换
    // ========================================================================

    pub fn toggle_favorite(&self, id: &str) -> Result<Game, String> {
        self.db.toggle_favorite(id)
    }

    pub fn toggle_hidden(&self, id: &str) -> Result<Game, String> {
        self.db.toggle_hidden(id)
    }

    // ========================================================================
    // 基本信息更新
    // ========================================================================

    pub fn update_game_name(&self, id: &str, name: String) -> Result<Game, String> {
        self.db.update_game_name(id, name)
    }

    pub fn update_game_description(
        &self,
        id: &str,
        description: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_game_description(id, description)
    }

    pub fn update_game_cover(&self, id: &str, cover: Option<String>) -> Result<Game, String> {
        self.db.update_game_cover(id, cover)
    }

    pub fn update_game_background(
        &self,
        id: &str,
        background: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_game_background(id, background)
    }

    pub fn update_game_icon(&self, id: &str, icon: Option<String>) -> Result<Game, String> {
        self.db.update_game_icon(id, icon)
    }

    pub fn update_game_type(&self, id: &str, game_type: Option<String>) -> Result<Game, String> {
        self.db.update_game_type(id, game_type)
    }

    pub fn update_install_dir(
        &self,
        id: &str,
        install_dir: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_install_dir(id, install_dir)
    }

    pub fn update_exe_path(&self, id: &str, exe_path: String) -> Result<Game, String> {
        self.db.update_exe_path(id, exe_path)
    }

    pub fn update_game_created_at(&self, id: &str, created_at: String) -> Result<Game, String> {
        self.db.update_game_created_at(id, created_at)
    }

    pub fn update_add_date(&self, id: &str, add_date: Option<String>) -> Result<Game, String> {
        self.db.update_add_date(id, add_date)
    }

    // ========================================================================
    // 简单标签
    // ========================================================================

    pub fn add_simple_tag(&self, id: &str, tag: String) -> Result<Game, String> {
        self.db.add_simple_tag(id, tag)
    }

    pub fn remove_simple_tag(&self, id: &str, tag: &str) -> Result<Game, String> {
        self.db.remove_simple_tag(id, tag)
    }

    pub fn set_simple_tags(&self, id: &str, tags: Vec<String>) -> Result<Game, String> {
        self.db.set_simple_tags(id, tags)
    }

    // ========================================================================
    // 增强标签
    // ========================================================================

    pub fn add_tag_entry(&self, id: &str, tag: Tag) -> Result<Game, String> {
        self.db.add_tag_entry(id, tag)
    }

    pub fn remove_tag_entry(&self, id: &str, tag_name: &str) -> Result<Game, String> {
        self.db.remove_tag_entry(id, tag_name)
    }

    pub fn update_tag_entry(&self, id: &str, tag_name: &str, tag: Tag) -> Result<Game, String> {
        self.db.update_tag_entry(id, tag_name, tag)
    }

    pub fn set_tag_entries(&self, id: &str, tags: Vec<Tag>) -> Result<Game, String> {
        self.db.set_tag_entries(id, tags)
    }

    // ========================================================================
    // 别名
    // ========================================================================

    pub fn add_alias(&self, id: &str, alias: GameAlias) -> Result<Game, String> {
        self.db.add_alias(id, alias)
    }

    pub fn remove_alias(&self, id: &str, alias_name: &str) -> Result<Game, String> {
        self.db.remove_alias(id, alias_name)
    }

    pub fn set_primary_alias(&self, id: &str, alias_name: &str) -> Result<Game, String> {
        self.db.set_primary_alias(id, alias_name)
    }

    pub fn set_aliases(&self, id: &str, aliases: Vec<GameAlias>) -> Result<Game, String> {
        self.db.set_aliases(id, aliases)
    }

    // ========================================================================
    // 元数据
    // ========================================================================

    pub fn update_game_metadata(&self, id: &str, metadata: GameMetadata) -> Result<Game, String> {
        self.db.update_game_metadata(id, metadata)
    }

    pub fn update_developer(&self, id: &str, developer: Option<String>) -> Result<Game, String> {
        self.db.update_developer(id, developer)
    }

    pub fn update_publisher(&self, id: &str, publisher: Option<String>) -> Result<Game, String> {
        self.db.update_publisher(id, publisher)
    }

    pub fn update_platform(
        &self,
        id: &str,
        platform: Option<GamePlatform>,
    ) -> Result<Game, String> {
        self.db.update_platform(id, platform)
    }

    pub fn update_engine(&self, id: &str, engine: Option<String>) -> Result<Game, String> {
        self.db.update_engine(id, engine)
    }

    pub fn update_game_version(&self, id: &str, version: Option<String>) -> Result<Game, String> {
        self.db.update_game_version(id, version)
    }

    pub fn update_original_name(
        &self,
        id: &str,
        original_name: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_original_name(id, original_name)
    }

    pub fn update_homepage(&self, id: &str, homepage: Option<String>) -> Result<Game, String> {
        self.db.update_homepage(id, homepage)
    }

    pub fn update_developer_homepage(
        &self,
        id: &str,
        developer_homepage: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_developer_homepage(id, developer_homepage)
    }

    pub fn update_age_rating(&self, id: &str, age_rating: Option<String>) -> Result<Game, String> {
        self.db.update_age_rating(id, age_rating)
    }

    pub fn update_series(&self, id: &str, series: Option<String>) -> Result<Game, String> {
        self.db.update_series(id, series)
    }

    pub fn update_release_date(
        &self,
        id: &str,
        release_date: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_release_date(id, release_date)
    }

    pub fn update_release_year(&self, id: &str, release_year: Option<u32>) -> Result<Game, String> {
        self.db.update_release_year(id, release_year)
    }

    pub fn update_estimated_hours(
        &self,
        id: &str,
        estimated_hours: Option<f64>,
    ) -> Result<Game, String> {
        self.db.update_estimated_hours(id, estimated_hours)
    }

    pub fn update_vndb_rating(&self, id: &str, vndb_rating: Option<f64>) -> Result<Game, String> {
        self.db.update_vndb_rating(id, vndb_rating)
    }

    pub fn update_bangumi_rating(
        &self,
        id: &str,
        bangumi_rating: Option<f64>,
    ) -> Result<Game, String> {
        self.db.update_bangumi_rating(id, bangumi_rating)
    }

    pub fn update_vndb_id(&self, id: &str, vndb_id: Option<String>) -> Result<Game, String> {
        self.db.update_vndb_id(id, vndb_id)
    }

    pub fn update_bangumi_id(&self, id: &str, bangumi_id: Option<String>) -> Result<Game, String> {
        self.db.update_bangumi_id(id, bangumi_id)
    }

    pub fn set_genres(&self, id: &str, genres: Vec<String>) -> Result<Game, String> {
        self.db.set_genres(id, genres)
    }

    pub fn set_languages(&self, id: &str, languages: Vec<String>) -> Result<Game, String> {
        self.db.set_languages(id, languages)
    }

    pub fn set_voice_languages(
        &self,
        id: &str,
        voice_languages: Vec<String>,
    ) -> Result<Game, String> {
        self.db.set_voice_languages(id, voice_languages)
    }

    // ========================================================================
    // 游玩追踪
    // ========================================================================

    pub fn update_play_tracker(&self, id: &str, tracker: PlayTracker) -> Result<Game, String> {
        self.db.update_play_tracker(id, tracker)
    }

    pub fn start_session(&self, id: &str) -> Result<String, String> {
        self.db.start_session(id)
    }

    /// 别名：`start_play_session` → `start_session`
    pub fn start_play_session(&self, id: &str) -> Result<String, String> {
        self.db.start_session(id)
    }

    pub fn end_session(
        &self,
        id: &str,
        session_id: &str,
        duration_seconds: u64,
    ) -> Result<bool, String> {
        self.db.end_session(id, session_id, duration_seconds)
    }

    /// 别名：`end_play_session` → `end_session`（命令用，返回 Game）
    pub fn end_play_session(
        &self,
        id: &str,
        session_id: &str,
        duration_seconds: u64,
    ) -> Result<Game, String> {
        self.db.end_session(id, session_id, duration_seconds)?;
        self.db.get_game(id)
    }

    pub fn update_completion_status(
        &self,
        id: &str,
        status: CompletionStatus,
    ) -> Result<Game, String> {
        self.db.update_completion_status(id, status)
    }

    pub fn update_user_rating(&self, id: &str, rating: Option<f64>) -> Result<Game, String> {
        self.db.update_user_rating(id, rating)
    }

    pub fn update_review(&self, id: &str, review: Option<String>) -> Result<Game, String> {
        self.db.update_review(id, review)
    }

    pub fn update_achievements(&self, id: &str, total: u32, unlocked: u32) -> Result<Game, String> {
        self.db.update_achievements(id, total, unlocked)
    }

    pub fn mark_finished(&self, id: &str, finished: bool) -> Result<Game, String> {
        self.db.mark_finished(id, finished)
    }

    pub fn get_play_sessions(&self, id: &str) -> Result<Vec<PlaySession>, String> {
        self.db.get_play_sessions(id)
    }

    pub fn update_play_session(
        &self,
        id: &str,
        session_id: &str,
        session: PlaySession,
    ) -> Result<Game, String> {
        self.db.update_play_session(id, session_id, session)
    }

    /// 别名：`update_session` → `update_play_session`
    pub fn update_session(
        &self,
        id: &str,
        session_id: &str,
        session: PlaySession,
    ) -> Result<Game, String> {
        self.db.update_play_session(id, session_id, session)
    }

    pub fn remove_play_session(&self, id: &str, session_id: &str) -> Result<Game, String> {
        self.db.remove_play_session(id, session_id)
    }

    /// 别名：`remove_session` → `remove_play_session`
    pub fn remove_session(&self, id: &str, session_id: &str) -> Result<Game, String> {
        self.db.remove_play_session(id, session_id)
    }

    pub fn set_sessions(&self, id: &str, sessions: Vec<PlaySession>) -> Result<Game, String> {
        self.db.set_sessions(id, sessions)
    }

    pub fn update_total_seconds(&self, id: &str, total_seconds: u64) -> Result<Game, String> {
        self.db.update_total_seconds(id, total_seconds)
    }

    pub fn update_first_played(
        &self,
        id: &str,
        first_played: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_first_played(id, first_played)
    }

    pub fn update_last_played(
        &self,
        id: &str,
        last_played: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_last_played(id, last_played)
    }

    pub fn update_completion_count(&self, id: &str, count: u32) -> Result<Game, String> {
        self.db.update_completion_count(id, count)
    }

    pub fn add_play_time(&self, id: &str, seconds: u64) -> Result<(), String> {
        self.db.add_play_time(id, seconds)
    }

    // ========================================================================
    // 截图
    // ========================================================================

    pub fn add_screenshot(&self, id: &str, path: String) -> Result<Game, String> {
        self.db.add_screenshot(id, path)
    }

    pub fn remove_screenshot(&self, id: &str, index: usize) -> Result<Game, String> {
        self.db.remove_screenshot(id, index)
    }

    pub fn remove_screenshot_by_path(&self, id: &str, path: &str) -> Result<Game, String> {
        self.db.remove_screenshot_by_path(id, path)
    }

    pub fn set_screenshots(&self, id: &str, screenshots: Vec<String>) -> Result<Game, String> {
        self.db.set_screenshots(id, screenshots)
    }

    // ========================================================================
    // 存档数据管理
    // ========================================================================

    pub fn update_save_data(&self, id: &str, save_data: SaveData) -> Result<Game, String> {
        self.db.update_save_data(id, save_data)
    }

    pub fn set_save_dir(&self, id: &str, save_dir: Option<String>) -> Result<Game, String> {
        self.db.set_save_dir(id, save_dir)
    }

    pub fn configure_auto_backup(
        &self,
        id: &str,
        auto_backup: bool,
        interval_minutes: u32,
        max_backups: u32,
    ) -> Result<Game, String> {
        self.db
            .configure_auto_backup(id, auto_backup, interval_minutes, max_backups)
    }

    pub fn add_backup(&self, id: &str, backup: SaveBackup) -> Result<Game, String> {
        self.db.add_backup(id, backup)
    }

    pub fn remove_backup(&self, id: &str, backup_id: &str) -> Result<Game, String> {
        self.db.remove_backup(id, backup_id)
    }

    pub fn update_backup_note(
        &self,
        id: &str,
        backup_id: &str,
        note: Option<String>,
    ) -> Result<Game, String> {
        self.db.update_backup_note(id, backup_id, note)
    }

    pub fn configure_cloud_sync(
        &self,
        id: &str,
        cloud_sync: bool,
        cloud_provider: Option<String>,
    ) -> Result<Game, String> {
        self.db.configure_cloud_sync(id, cloud_sync, cloud_provider)
    }

    // ========================================================================
    // 刮削结果应用
    // ========================================================================

    #[allow(clippy::too_many_arguments)]
    pub fn apply_scrape_result_ext(
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
        self.db.apply_scrape_result(
            id,
            title,
            description,
            cover,
            background,
            tags,
            rating,
            release_year,
            source,
            source_id,
            developer,
            publisher,
            genres,
            languages,
            engine,
            age_rating,
            series,
            release_date,
            voice_languages,
            aliases,
            screenshots,
            homepage,
        )
    }

    // ========================================================================
    // 设置
    // ========================================================================

    pub fn get_settings(&self) -> Settings {
        self.db.get_settings().unwrap_or_default()
    }

    pub fn update_settings(&self, settings: Settings) -> Result<Settings, String> {
        self.db.update_settings(&settings).map(|_| settings)
    }

    // ========================================================================
    // 数据库信息
    // ========================================================================

    pub fn schema_version(&self) -> u32 {
        self.db.schema_version().unwrap_or(0) as u32
    }

    pub fn game_count(&self) -> usize {
        self.db.game_count().unwrap_or(0) as usize
    }

    // ========================================================================
    // C# 迁移桥（M1）
    // ========================================================================

    /// 获取内部 SqliteDb 引用（供迁移模块使用）。
    pub fn sqlite(&self) -> &SqliteDb {
        &self.db
    }
}
