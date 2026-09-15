//! 历史记录 WebDAV 同步（FR-09 / spec task-05）。
//!
//! 结构：
//! - `mod.rs` —— 核心类型（`SyncMode`/`WebDavConfig`/`SyncResult`/`SyncError`/
//!   `SyncStatus`）、`SyncState`、`run_sync` 编排与 §3.6 全部 Tauri 命令；
//! - `merge.rs` —— last-write-wins 合并 + 墓碑清理（纯函数）；
//! - `webdav.rs` —— 轻量 WebDAV 客户端（MKCOL/GET/PUT/PROPFIND，Basic 认证）；
//! - `keyring_store.rs` —— 凭据 keyring 存取；
//! - `saves.rs` —— 旧存档云同步模块（迁移自原 `sync.rs`，`pub use saves::*` 保持
//!   `crate::sync::*` 既有 API 面不变）。
//!
//! 关键不变量（spec §2 / §4.2）：
//! - 同步全局单实例（`tokio::sync::Mutex` try_lock，并发返回 `Busy`）；
//! - 任何远端解析/认证/网络失败都不得写入或清空本地数据——远端拉取 → 内存合并 →
//!   PUT 远端全部成功后才单事务落本地；
//! - 凭据只存 keyring，配置表/日志绝不出现密码。

pub mod keyring_store;
pub mod merge;
pub mod saves;
pub mod webdav;

pub use merge::*;
pub use saves::*;

use crate::db_sqlite::{DbError, HistoryDb};
#[cfg(test)]
use crate::domain::history::ContentType;
use crate::domain::history::HistoryRecord;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::str::FromStr;
use std::sync::Mutex;
use tauri::State;
use thiserror::Error;

/// 同步模式（spec §3.2）。默认 `Manual`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    #[default]
    Manual,
    Auto,
}

/// WebDAV 配置（**不含密码**；密码只在 keyring，见 `keyring_store`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebDavConfig {
    /// 如 `https://dav.jianguoyun.com/dav`
    pub base_url: String,
    pub username: String,
    pub mode: SyncMode,
    /// 远端同步目录，默认 `moeplay-sync`
    pub remote_dir: String,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            username: String::new(),
            mode: SyncMode::Manual,
            remote_dir: "moeplay-sync".to_string(),
        }
    }
}

/// 同步结果（spec §3.2）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// 上传 n 条（本地有而远端缺失/过旧）
    pub uploaded: u32,
    /// 下载 m 条（远端新增/更新到本地的条数）
    pub downloaded: u32,
    /// 同键冲突合并 k 条
    pub conflicts: u32,
    pub tombstones_purged: u32,
    /// Unix 秒
    pub synced_at: i64,
}

/// 同步错误（spec §3.2）。序列化为 `{ "kind", "message" }` 供前端直接展示
/// 文案（认证失败文案必须明确）。
#[derive(Debug, Clone, Error)]
pub enum SyncError {
    #[error("认证失败，请检查用户名与授权密码")]
    Auth,
    #[error("网络连接失败: {0}")]
    Network(String),
    #[error("远端数据格式非法")]
    Parse,
    #[error("服务端错误: {0}")]
    Server(String),
    #[error("同步正在进行中")]
    Busy,
    #[error("未配置 WebDAV")]
    NotConfigured,
}

impl SyncError {
    pub fn kind(&self) -> &'static str {
        match self {
            SyncError::Auth => "auth",
            SyncError::Network(_) => "network",
            SyncError::Parse => "parse",
            SyncError::Server(_) => "server",
            SyncError::Busy => "busy",
            SyncError::NotConfigured => "not_configured",
        }
    }
}

impl Serialize for SyncError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SyncError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

/// 同步状态快照（spec §3.6）。
#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub configured: bool,
    pub last_result: Option<SyncResult>,
    pub syncing: bool,
}

/// 全局同步状态：互斥锁 + 内存配置 + 上次结果 + 任务 4 的 HistoryDb 克隆。
pub struct SyncState {
    /// 同步全局互斥（自动/手动同步不得并发）。
    lock: tokio::sync::Mutex<()>,
    config: Mutex<Option<WebDavConfig>>,
    last_result: Mutex<Option<SyncResult>>,
    history: Option<HistoryDb>,
}

impl SyncState {
    /// 从任务 4 数据库加载持久化配置并构造状态。
    pub fn new(history: Option<HistoryDb>) -> Self {
        let config = history
            .as_ref()
            .and_then(|db| load_config_from_db(db).ok().flatten());
        Self {
            lock: tokio::sync::Mutex::new(()),
            config: Mutex::new(config),
            last_result: Mutex::new(None),
            history,
        }
    }

    /// 覆盖内存配置（命令与测试用；持久化由调用方负责）。
    pub fn set_config(&self, config: Option<WebDavConfig>) -> Result<(), SyncError> {
        let mut guard = self
            .config
            .lock()
            .map_err(|_| SyncError::Server("同步配置锁损坏".to_string()))?;
        *guard = config;
        Ok(())
    }

    /// 读取内存配置（启动时已从数据库加载）。
    pub fn config(&self) -> Result<Option<WebDavConfig>, SyncError> {
        self.config
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| SyncError::Server("同步配置锁损坏".to_string()))
    }
}

// ============================================================================
// 配置持久化（复用任务 4 的 SQLite：`sync_config` 单行表，密码绝不进该表）
// ============================================================================

const CONFIG_TABLE_SQL: &str =
    "CREATE TABLE IF NOT EXISTS sync_config (key TEXT PRIMARY KEY, value_json TEXT NOT NULL)";
const CONFIG_KEY: &str = "webdav_config";

pub(crate) fn save_config_to_db(db: &HistoryDb, cfg: &WebDavConfig) -> Result<(), SyncError> {
    let conn = db.conn();
    let guard = conn
        .lock()
        .map_err(|_| SyncError::Server("历史数据库锁损坏".to_string()))?;
    guard
        .execute_batch(CONFIG_TABLE_SQL)
        .map_err(map_sqlite_error)?;
    let json = serde_json::to_string(cfg)
        .map_err(|e| SyncError::Server(format!("配置序列化失败: {e}")))?;
    guard
        .execute(
            "INSERT INTO sync_config(key,value_json) VALUES(?1,?2) \
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            rusqlite::params![CONFIG_KEY, json],
        )
        .map_err(map_sqlite_error)?;
    Ok(())
}

pub(crate) fn load_config_from_db(db: &HistoryDb) -> Result<Option<WebDavConfig>, SyncError> {
    let conn = db.conn();
    let guard = conn
        .lock()
        .map_err(|_| SyncError::Server("历史数据库锁损坏".to_string()))?;
    guard
        .execute_batch(CONFIG_TABLE_SQL)
        .map_err(map_sqlite_error)?;
    let row = guard
        .query_row(
            "SELECT value_json FROM sync_config WHERE key=?1",
            rusqlite::params![CONFIG_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(map_sqlite_error)?;
    match row {
        Some(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SyncError::Server(format!("配置解析失败: {e}"))),
        None => Ok(None),
    }
}

pub(crate) fn clear_config_from_db(db: &HistoryDb) -> Result<(), SyncError> {
    let conn = db.conn();
    let guard = conn
        .lock()
        .map_err(|_| SyncError::Server("历史数据库锁损坏".to_string()))?;
    guard
        .execute(
            "DELETE FROM sync_config WHERE key=?1",
            rusqlite::params![CONFIG_KEY],
        )
        .map_err(map_sqlite_error)?;
    Ok(())
}

// ============================================================================
// 本地 HistoryRecord ↔ SyncRecord 转换
// ============================================================================

fn record_to_sync(record: &HistoryRecord) -> SyncRecord {
    SyncRecord {
        id: record.id.clone(),
        content_id: record.content_id.clone(),
        content_type: record.content_type.as_str().to_string(),
        title: record.title.clone(),
        cover: record.cover.clone(),
        source_id: record.source_id.clone(),
        chapter_id: record.chapter_id.clone(),
        chapter_title: record.chapter_title.clone(),
        page_index: record.page_index,
        position_sec: record.position_sec,
        scroll_pct: record.scroll_pct,
        // DB 毫秒 → 传输秒（合并按秒粒度，spec §3.3 rule 3）。
        updated_at: record.updated_at / 1000,
        device_id: record.device_id.clone(),
        deleted: record.deleted,
    }
}

/// 传输格式 → DB 记录（仅测试验证 round-trip；落库路径 `upsert_from_sync` 直接
/// 以 `SyncRecord` 写列，不依赖本转换）。
#[cfg(test)]
fn sync_to_record(sync: &SyncRecord) -> Option<HistoryRecord> {
    let content_type = ContentType::from_str(&sync.content_type).ok()?;
    Some(HistoryRecord {
        id: sync.id.clone(),
        content_id: sync.content_id.clone(),
        content_type,
        title: sync.title.clone(),
        cover: sync.cover.clone(),
        source_id: sync.source_id.clone(),
        chapter_id: sync.chapter_id.clone(),
        chapter_title: sync.chapter_title.clone(),
        page_index: sync.page_index,
        position_sec: sync.position_sec,
        scroll_pct: sync.scroll_pct,
        updated_at: sync.updated_at * 1000,
        device_id: sync.device_id.clone(),
        deleted: sync.deleted,
    })
}

fn map_db_error(error: DbError) -> SyncError {
    SyncError::Server(format!("本地历史数据库错误: {error}"))
}

/// 直接把 `rusqlite::Error` 映射为同步错误（config 表操作直接持有 Connection）。
fn map_sqlite_error(error: rusqlite::Error) -> SyncError {
    SyncError::Server(format!("本地历史数据库错误: {error}"))
}

fn normalized_remote_dir(cfg: &WebDavConfig) -> String {
    let dir = cfg.remote_dir.trim().trim_matches('/');
    if dir.is_empty() {
        "moeplay-sync".to_string()
    } else {
        dir.to_string()
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================================
// run_sync 编排
// ============================================================================

/// 手动/自动同步统一入口：互斥锁 → 配置 + keyring 密码 → `WebDavClient` →
/// `finish_sync`。
pub async fn run_sync(state: &SyncState) -> Result<SyncResult, SyncError> {
    let _guard = state.lock.try_lock().map_err(|_| SyncError::Busy)?;
    let cfg = state.config()?.ok_or(SyncError::NotConfigured)?;
    let password = keyring_store::get_password()?.ok_or(SyncError::Auth)?;
    let client = webdav::WebDavClient::new(&cfg.base_url, &cfg.username, &password)?;
    finish_sync(state, &cfg, &client).await
}

/// 测试/编排入口：注入配置与客户端（集成测试用 wiremock，绕过 keyring 与 https_only）。
/// 生产路径请走 [`run_sync`]。
pub async fn run_sync_with(
    state: &SyncState,
    cfg: &WebDavConfig,
    client: webdav::WebDavClient,
) -> Result<SyncResult, SyncError> {
    let _guard = state.lock.try_lock().map_err(|_| SyncError::Busy)?;
    finish_sync(state, cfg, &client).await
}

async fn finish_sync(
    state: &SyncState,
    cfg: &WebDavConfig,
    client: &webdav::WebDavClient,
) -> Result<SyncResult, SyncError> {
    let result = run_sync_inner(state, cfg, client).await?;
    let mut last_result = state
        .last_result
        .lock()
        .map_err(|_| SyncError::Server("同步状态锁损坏".to_string()))?;
    *last_result = Some(result.clone());
    Ok(result)
}

/// 单次同步流程（spec §4.2 step d~j）：
/// 拉远端 → 读本地全量 → 内存合并 → purge 墓碑 → PUT（If-Match，412 重试 1 次）→
/// PUT manifest → 单事务落本地。
async fn run_sync_inner(
    state: &SyncState,
    cfg: &WebDavConfig,
    client: &webdav::WebDavClient,
) -> Result<SyncResult, SyncError> {
    let db = state
        .history
        .clone()
        .ok_or_else(|| SyncError::Server("历史数据库不可用".to_string()))?;
    let dir = normalized_remote_dir(cfg);
    let history_path = format!("{dir}/history.json");
    let manifest_path = format!("{dir}/manifest.json");
    let now = now_secs();

    // c. 目录不存在时自动创建。
    client.ensure_dir(&dir).await?;

    // d~g. 拉取 → 合并 → 条件 PUT history.json（412 时重拉并带最新 ETag 重试 1 次）。
    let prepared = prepare_pass(&db, client, &history_path).await?;
    let mut status = client
        .put_with_condition(
            &history_path,
            prepared.body.clone(),
            prepared.put_condition()?,
        )
        .await?;
    let final_pass = if status == webdav::PutStatus::PreconditionFailed {
        let retried = prepare_pass(&db, client, &history_path).await?;
        status = client
            .put_with_condition(
                &history_path,
                retried.body.clone(),
                retried.put_condition()?,
            )
            .await?;
        retried
    } else {
        prepared
    };
    if status == webdav::PutStatus::PreconditionFailed {
        return Err(SyncError::Server(
            "远端历史文件被并发修改，重试后仍冲突".to_string(),
        ));
    }

    // h. manifest 也使用条件写入，避免覆盖其他设备刚生成的清单。
    let manifest = serde_json::json!({
        "version": 1,
        "updated_at": now,
        "etag_hint": "",
    });
    let manifest_body = serde_json::to_vec(&manifest)
        .map_err(|e| SyncError::Server(format!("manifest 序列化失败: {e}")))?;
    let manifest_condition = match client.get(&manifest_path).await? {
        None => webdav::PutCondition::CreateOnly,
        Some((_, Some(etag))) => webdav::PutCondition::IfMatch(etag),
        Some((_, None)) => {
            return Err(SyncError::Server(
                "远端 manifest 文件缺少 ETag，无法安全并发写入".to_string(),
            ));
        }
    };
    if client
        .put_with_condition(&manifest_path, manifest_body, manifest_condition)
        .await?
        == webdav::PutStatus::PreconditionFailed
    {
        return Err(SyncError::Server(
            "远端 manifest 文件被并发修改，无法安全写入".to_string(),
        ));
    }

    // i. 原子落本地：单事务内 upsert 胜出记录 + 删除同键败方旧记录 + 清理过期墓碑
    //    （spec §4.2 step i / §6.3 双设备验收）。
    let (written, stale_deleted, purged_local) = db
        .apply_merged_set(
            &final_pass.merged,
            now_ms() - TOMBSTONE_RETENTION_DAYS * 86_400_000,
        )
        .map_err(map_db_error)?;

    // j. 审计日志（不含凭据）。
    let result = SyncResult {
        uploaded: final_pass.uploaded,
        downloaded: final_pass.downloaded,
        conflicts: final_pass.conflicts,
        tombstones_purged: final_pass.tombstones_purged,
        synced_at: now,
    };
    tracing::info!(
        uploaded = result.uploaded,
        downloaded = result.downloaded,
        conflicts = result.conflicts,
        written,
        stale_deleted,
        purged_local,
        "WebDAV history sync finished"
    );
    Ok(result)
}

/// 拉取远端 → 读本地 → 合并 → purge 墓碑 → 序列化，返回待 PUT 的载荷与 ETag。
async fn prepare_pass(
    db: &HistoryDb,
    client: &webdav::WebDavClient,
    history_path: &str,
) -> Result<PreparedPass, SyncError> {
    // d. 远端：404/None → 空数组；非法 JSON → Parse（立即返回，不写本地）。
    let (remote, etag, remote_exists) = match client.get(history_path).await? {
        Some((body, etag)) => {
            let parsed: Vec<SyncRecord> =
                serde_json::from_slice(&body).map_err(|_| SyncError::Parse)?;
            (parsed, etag, true)
        }
        None => (Vec::new(), None, false),
    };

    // e. 本地全量（含墓碑）。
    let local = db
        .list_all_with_deleted()
        .map_err(map_db_error)?
        .iter()
        .map(record_to_sync)
        .collect::<Vec<_>>();

    // f. 内存合并。
    let outcome = merge::merge_records(local, remote);
    let uploaded = outcome.uploaded;
    let downloaded = outcome.downloaded;
    let conflicts = outcome.conflicts;

    // g. purge 过期墓碑后序列化。
    let before = outcome.merged.len();
    let merged = merge::purge_tombstones(outcome.merged, now_secs(), TOMBSTONE_RETENTION_DAYS);
    let tombstones_purged = (before - merged.len()) as u32;
    let body = serde_json::to_vec(&merged)
        .map_err(|e| SyncError::Server(format!("历史数据序列化失败: {e}")))?;

    Ok(PreparedPass {
        etag,
        remote_exists,
        body,
        merged,
        uploaded,
        downloaded,
        conflicts,
        tombstones_purged,
    })
}

struct PreparedPass {
    etag: Option<String>,
    remote_exists: bool,
    body: Vec<u8>,
    merged: Vec<SyncRecord>,
    uploaded: u32,
    downloaded: u32,
    conflicts: u32,
    tombstones_purged: u32,
}

impl PreparedPass {
    fn put_condition(&self) -> Result<webdav::PutCondition, SyncError> {
        match (&self.etag, self.remote_exists) {
            (Some(etag), _) => Ok(webdav::PutCondition::IfMatch(etag.clone())),
            (None, false) => Ok(webdav::PutCondition::CreateOnly),
            (None, true) => Err(SyncError::Server(
                "远端历史文件缺少 ETag，已停止上传以避免覆盖并发修改".to_string(),
            )),
        }
    }
}

// ============================================================================
// Tauri Commands（spec §3.6 + 清除配置）
// ============================================================================

#[tauri::command]
pub async fn sync_now(state: State<'_, SyncState>) -> Result<SyncResult, SyncError> {
    run_sync(&state).await
}

#[tauri::command]
pub async fn test_webdav_connection(cfg: WebDavConfig, password: String) -> Result<(), SyncError> {
    // 前端可能不重新输入密码（已保存时不显示）；空密码回退到 keyring 已存凭据。
    let password = if password.trim().is_empty() {
        keyring_store::get_password()?.ok_or(SyncError::Auth)?
    } else {
        password
    };
    let client = webdav::WebDavClient::new(&cfg.base_url, &cfg.username, &password)?;
    let dir = normalized_remote_dir(&cfg);
    client.ensure_dir(&dir).await
}

#[tauri::command]
pub async fn save_webdav_config(
    cfg: WebDavConfig,
    password: Option<String>,
    state: State<'_, SyncState>,
) -> Result<(), SyncError> {
    // 先存 keyring 密码（若提供），成功后再写配置表（spec §4.2 step 8），
    // 避免"有配置无密码"的悬空态。
    if let Some(password) = password {
        if !password.trim().is_empty() {
            keyring_store::save_password(password.trim())?;
        }
    }
    state.set_config(Some(cfg.clone()))?;
    if let Some(db) = state.history.clone() {
        save_config_to_db(&db, &cfg)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_sync_config(
    state: State<'_, SyncState>,
) -> Result<Option<WebDavConfig>, SyncError> {
    // 任何情况下不返回密码。
    state.config()
}

#[tauri::command]
pub async fn get_sync_status(state: State<'_, SyncState>) -> Result<SyncStatus, SyncError> {
    let configured = state.config()?.is_some();
    let last_result = state
        .last_result
        .lock()
        .map(|guard| guard.clone())
        .map_err(|_| SyncError::Server("同步状态锁损坏".to_string()))?;
    let syncing = state.lock.try_lock().is_err();
    Ok(SyncStatus {
        configured,
        last_result,
        syncing,
    })
}

/// 清除配置 + keyring 凭据（spec §3.7"清除配置"，**不删除任何历史数据**）。
#[tauri::command]
pub async fn clear_webdav_config(state: State<'_, SyncState>) -> Result<(), SyncError> {
    keyring_store::delete_password()?;
    state.set_config(None)?;
    {
        let mut last_result = state
            .last_result
            .lock()
            .map_err(|_| SyncError::Server("同步状态锁损坏".to_string()))?;
        *last_result = None;
    }
    if let Some(db) = state.history.clone() {
        clear_config_from_db(&db)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> WebDavConfig {
        WebDavConfig {
            base_url: "https://dav.example.com/dav".to_string(),
            username: "alice".to_string(),
            mode: SyncMode::Auto,
            remote_dir: "moeplay-sync".to_string(),
        }
    }

    #[test]
    fn sync_error_serializes_as_kind_and_message() {
        for (error, kind, message_fragment) in [
            (SyncError::Auth, "auth", "认证失败"),
            (SyncError::Parse, "parse", "远端数据格式非法"),
            (SyncError::Busy, "busy", "同步正在进行中"),
            (SyncError::NotConfigured, "not_configured", "未配置"),
            (
                SyncError::Network("timeout".to_string()),
                "network",
                "网络连接失败",
            ),
            (SyncError::Server("500".to_string()), "server", "服务端错误"),
        ] {
            let value = serde_json::to_value(&error).expect("serializes");
            assert_eq!(value["kind"], kind);
            assert!(
                value["message"]
                    .as_str()
                    .unwrap()
                    .contains(message_fragment),
                "message {} should contain {message_fragment}",
                value["message"]
            );
        }
    }

    #[test]
    fn config_roundtrip_in_sqlite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = HistoryDb::open(dir.path()).expect("open history db");
        save_config_to_db(&db, &sample_config()).expect("save config");
        let loaded = load_config_from_db(&db).expect("load config");
        assert_eq!(loaded, Some(sample_config()));
        clear_config_from_db(&db).expect("clear config");
        assert_eq!(load_config_from_db(&db).expect("load after clear"), None);
    }

    #[test]
    fn config_never_contains_password() {
        let cfg = sample_config();
        let json = serde_json::to_value(&cfg).expect("serialize");
        assert!(
            json.get("password").is_none(),
            "config must not carry password"
        );
        assert_eq!(json["base_url"], "https://dav.example.com/dav");
        assert_eq!(json["mode"], "auto");
    }

    #[test]
    fn sync_to_record_roundtrips_updated_at_unit() {
        let history = HistoryRecord {
            id: "id".into(),
            content_id: "cid".into(),
            content_type: ContentType::Anime,
            title: "Title".into(),
            cover: None,
            source_id: "src".into(),
            chapter_id: None,
            chapter_title: None,
            page_index: 0,
            position_sec: 12.5,
            scroll_pct: 0.0,
            updated_at: 1_750_000_000_123,
            device_id: "dev".into(),
            deleted: false,
        };
        let sync = record_to_sync(&history);
        assert_eq!(sync.updated_at, 1_750_000_000);
        let back = sync_to_record(&sync).expect("convert back");
        assert_eq!(back.updated_at, 1_750_000_000_000);
        assert_eq!(back.content_type, ContentType::Anime);
        assert_eq!(back.position_sec, 12.5);
    }

    #[test]
    fn sync_to_record_rejects_unknown_content_type() {
        let bad = SyncRecord {
            id: "id".into(),
            content_id: "cid".into(),
            content_type: "game".into(),
            title: "Title".into(),
            cover: None,
            source_id: "src".into(),
            chapter_id: None,
            chapter_title: None,
            page_index: 0,
            position_sec: 0.0,
            scroll_pct: 0.0,
            updated_at: 1,
            device_id: "dev".into(),
            deleted: false,
        };
        assert!(sync_to_record(&bad).is_none());
    }
}
