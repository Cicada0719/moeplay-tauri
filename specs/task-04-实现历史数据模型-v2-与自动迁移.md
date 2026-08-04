# 子任务 4 开发规格说明：历史数据模型 v2 与自动迁移

## 1. 任务目标

将现有的历史记录存储（v1，JSON 文件或轻量存储）统一迁移到基于 SQLite 的 v2 数据模型，覆盖番剧/漫画/小说三类内容。在应用首次启动时自动执行 v1→v2 迁移，迁移前强制备份，失败自动回滚，支持中断后断点续迁且保证幂等无重复记录。本任务是后续 WebDAV 同步（子任务 5）与阅读体验优化（子任务 6）的数据层基础。

## 2. 上下文与约束

### 2.1 关联 PRD 条目

- **FR-08**（P0）：统一历史数据模型 v2 + 自动迁移（核心验收来源）。
- **§5.3**：PRD 给出的 SQLite 表示意，本规格在其基础上做工程化细化（不得删减 PRD 中要求的字段）。
- **§5.1 技术选型**：本地存储建议采用 SQLite，使用 **`rusqlite`**（启用 `bundled` feature，避免三端构建时依赖系统 libsqlite3）。
- **§4.1 性能**：历史列表 1 万条数据下查询流畅（本任务负责索引设计与分页查询接口）。
- **§4.3 兼容性 / R4 风险**：旧版数据 100% 可迁移，迁移失败自动回滚，提供"从备份恢复"入口。
- **FR-09 / FR-10（下游依赖）**：`deleted` 墓碑字段、`device_id`、`updated_at` 为同步合并所必需，本任务必须建全。

### 2.2 技术栈与现有代码结构

- Tauri 2 + Rust（后端）+ Svelte（前端）。**技术栈不可变更**。
- 现有存储入口假定位于 `src-tauri/src/db.rs`（v1 JSON 存储，数据文件假定路径 `<app_data_dir>/history.json`，编码代理须先阅读该文件确认真实结构；若实际结构与本规格 §4 的假设不符，以实际为准编写适配映射，但 v2 目标 schema 不可变）。
- 本任务涉及的模块（新建或改造）：

| 文件 | 职责 | 新建/改造 |
|---|---|---|
| `src-tauri/src/db_sqlite.rs` | SQLite 连接管理、schema 初始化、`user_version` 管理 | 新建 |
| `src-tauri/src/domain/history.rs` | v2 历史记录领域模型与序列化定义 | 新建 |
| `src-tauri/src/migration/mod.rs` | 迁移框架：状态机、调度、备份/回滚 | 新建 |
| `src-tauri/src/migration/v1_to_v2.rs` | v1 JSON → v2 SQLite 的具体迁移实现 | 新建 |
| `src-tauri/src/db.rs` | 保留 v1 读取逻辑（只读），新增"读取 v1 原始数据供迁移"的公开函数；迁移完成后不再写入 | 改造 |

### 2.3 硬性约束

- 数据库文件路径：`<app_data_dir>/moeplay.db`（通过 `tauri::path::BaseDirectory::AppData` 解析）。
- 备份目录：`<app_data_dir>/backup/`，备份文件名格式 `history_v1_backup_<yyyyMMdd_HHmmss>.json`。
- 数据库连接必须启用 WAL 模式（`PRAGMA journal_mode=WAL`）与外键约束（`PRAGMA foreign_keys=ON`）。
- 所有迁移写入必须在**单个事务**内分批提交（见 §5 断点续迁设计），禁止逐条 autocommit。
- 禁止在迁移未完成时让前端读写 v2 表（通过迁移状态门控，见 §3.2）。

## 3. 输入 / 输出

### 3.1 领域模型（`src-tauri/src/domain/history.rs`）

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Anime,
    Manga,
    Novel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,               // uuid v4
    pub content_id: String,       // 条目标识（源内唯一 ID）
    pub content_type: ContentType,
    pub title: String,
    pub cover: Option<String>,
    pub source_id: String,
    pub chapter_id: Option<String>,    // 集/话/章 ID
    pub chapter_title: Option<String>,
    pub page_index: i64,          // 漫画页码（原子单位=单页，与渲染模式无关）；番剧/小说为 0
    pub position_sec: f64,        // 番剧播放进度秒数
    pub scroll_pct: f64,          // 小说滚动百分比 0.0~100.0
    pub updated_at: i64,          // Unix 毫秒时间戳
    pub device_id: String,        // 设备唯一标识，见 §3.4
    pub deleted: bool,            // 墓碑标记，同步用
}

/// 前端展示用（对应 FR-08 验收中的 camelCase 字段名；pageIndex + progress 的
/// "progress" 在序列化层由 content_type 决定映射 position_sec 或 scroll_pct，
/// 存储层不新增冗余列）
```

> 字段说明对照 FR-08 验收：`contentId→content_id`、`contentType→content_type`、`title`、`cover`、`chapterId→chapter_id`、`chapterTitle→chapter_title`、`pageIndex→page_index`、`progress→（番剧=position_sec / 小说=scroll_pct，由序列化适配函数输出）`、`sourceId→source_id`、`updatedAt→updated_at`、`deviceId→device_id`。

### 3.2 SQLite Schema（`db_sqlite.rs` 中常量 `SCHEMA_V2_SQL`）

```sql
CREATE TABLE IF NOT EXISTS history (
  id            TEXT PRIMARY KEY,
  content_id    TEXT NOT NULL,
  content_type  TEXT NOT NULL CHECK (content_type IN ('anime','manga','novel')),
  title         TEXT NOT NULL,
  cover         TEXT,
  source_id     TEXT NOT NULL,
  chapter_id    TEXT,
  chapter_title TEXT,
  page_index    INTEGER NOT NULL DEFAULT 0,
  position_sec  REAL    NOT NULL DEFAULT 0,
  scroll_pct    REAL    NOT NULL DEFAULT 0,
  updated_at    INTEGER NOT NULL,
  device_id     TEXT    NOT NULL,
  deleted       INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_history_type_updated
  ON history(content_type, updated_at DESC);
-- 同步合并与去重键（子任务 5 依赖）
CREATE INDEX IF NOT EXISTS idx_history_merge_key
  ON history(content_id, source_id);

-- 迁移状态表：断点续迁与幂等的核心
CREATE TABLE IF NOT EXISTS migration_state (
  id              INTEGER PRIMARY KEY CHECK (id = 1),  -- 单行表
  from_version    INTEGER NOT NULL,
  to_version      INTEGER NOT NULL,
  status          TEXT NOT NULL CHECK (status IN ('pending','in_progress','completed','failed','rolled_back')),
  total_count     INTEGER NOT NULL DEFAULT 0,
  migrated_count  INTEGER NOT NULL DEFAULT 0,          -- 断点：已提交的记录数
  last_offset     INTEGER NOT NULL DEFAULT 0,          -- 断点：v1 数据读取游标
  backup_path     TEXT,
  error_message   TEXT,
  started_at      INTEGER,
  finished_at     INTEGER
);
```

数据库版本使用 `PRAGMA user_version` 管理：`0`=空库、`2`=v2 schema。`user_version = 1` 保留给"检测到 v1 JSON 存在但 SQLite 未初始化"的语义判断（实际判断逻辑以文件存在性 + user_version 组合为准，见 §4.2 步骤 2）。

### 3.3 公开接口签名

**`src-tauri/src/db_sqlite.rs`**

```rust
pub struct SqliteDb { /* 内部持有 Arc<Mutex<rusqlite::Connection>> */ }

impl SqliteDb {
    /// 打开（不存在则创建）数据库，应用 PRAGMA，执行 schema 初始化至当前版本。
    pub fn open(app_data_dir: &Path) -> Result<Self, DbError>;

    pub fn conn(&self) -> Arc<Mutex<Connection>>;

    pub fn user_version(&self) -> Result<i64, DbError>;
    pub fn set_user_version(&self, v: i64) -> Result<(), DbError>;
}

/// v2 历史的 CRUD（供 Tauri command 层与子任务 5/6 调用）
pub trait HistoryRepo {
    fn upsert(&self, rec: &HistoryRecord) -> Result<(), DbError>;          // INSERT OR REPLACE，同步合并也用此入口
    fn get(&self, id: &str) -> Result<Option<HistoryRecord>, DbError>;
    fn find_by_merge_key(&self, content_id: &str, source_id: &str)
        -> Result<Vec<HistoryRecord>, DbError>;
    /// 分页列表：content_type=None 表示全部；不包含 deleted=1 的记录（墓碑仅供同步拉取）
    fn list(&self, content_type: Option<ContentType>, keyword: Option<&str>,
            limit: u32, offset: u32) -> Result<Vec<HistoryRecord>, DbError>;
    fn count(&self, content_type: Option<ContentType>) -> Result<i64, DbError>;
    fn tombstone(&self, id: &str) -> Result<(), DbError>;                  // deleted=1 + 刷新 updated_at
    fn list_tombstones_since(&self, since_ms: i64) -> Result<Vec<HistoryRecord>, DbError>; // 供子任务 5
}
```

**`src-tauri/src/migration/mod.rs`**

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MigrationStatus { NotNeeded, Pending, InProgress, Completed, Failed(String), RolledBack }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub status: MigrationStatus,
    pub total: i64,
    pub migrated: i64,
    pub backup_path: Option<String>,
}

pub struct Migrator { /* 持有 SqliteDb + app_data_dir */ }

impl Migrator {
    pub fn new(db: SqliteDb, app_data_dir: PathBuf) -> Self;

    /// 启动时调用：判断是否需要迁移。
    /// 需要迁移的情形：
    ///  a) v1 JSON 存在 且 migration_state 无 completed 记录；
    ///  b) migration_state.status == 'in_progress'（上次中断）→ 断点续迁；
    ///  c) migration_state.status == 'failed'（上次失败已回滚）→ 从头重试。
    pub fn check(&self) -> Result<MigrationStatus, MigrationError>;

    /// 执行迁移（同步阻塞，调用方需在 tokio::task::spawn_blocking 中运行）。
    /// 内部流程：备份 → 分批事务写入 → 校验条数 → 标记 completed。
    /// 任何步骤失败 → 自动回滚（清空本次写入 + status='rolled_back'）并返回错误。
    pub fn run(&self) -> Result<MigrationReport, MigrationError>;

    /// "从备份恢复"入口（R4 应对）：将指定备份 JSON 重新作为 v1 数据源执行迁移。
    pub fn restore_from_backup(&self, backup_path: &Path) -> Result<MigrationReport, MigrationError>;

    pub fn list_backups(&self) -> Result<Vec<PathBuf>, MigrationError>;
}
```

**`src-tauri/src/migration/v1_to_v2.rs`**

```rust
/// v1 原始记录的反序列化结构（字段以 db.rs 实际结构为准，此处为占位示意）
#[derive(Debug, Deserialize)]
pub struct V1HistoryEntry { /* ... */ }

/// 纯函数：单条 v1 → v2 映射。单独抽出以便 100% 单测。
/// - 生成 uuid；device_id 从全局设备标识读取（§3.4）；
/// - v1 缺失字段按 §4.1 映射表填默认值；
/// - 非法记录（如 title 为空）记入 skipped 列表而非中断迁移。
pub fn map_v1_to_v2(entry: &V1HistoryEntry, device_id: &str) -> Result<HistoryRecord, MapError>;

/// 幂等写入：以 (content_id, source_id, chapter_id) 判定重复——
/// 若目标行已存在且 updated_at >= 待写入值则跳过，否则覆盖。
pub fn upsert_idempotent(tx: &rusqlite::Transaction, rec: &HistoryRecord) -> Result<UpsertOutcome, DbError>;
```

### 3.4 Tauri Commands（注册进 `lib.rs` / `main.rs` 的 invoke_handler）

```rust
#[tauri::command] async fn migration_status(state: State<'_, Migrator>) -> Result<MigrationStatus, String>;
#[tauri::command] async fn migration_run(state: State<'_, AppState>) -> Result<MigrationReport, String>;
#[tauri::command] async fn migration_restore_backup(path: String, state: State<'_, AppState>) -> Result<MigrationReport, String>;
#[tauri::command] async fn history_list(content_type: Option<String>, keyword: Option<String>,
                                        limit: u32, offset: u32, state: State<'_, AppState>) -> Result<Vec<HistoryRecord>, String>;
#[tauri::command] async fn history_delete(id: String, state: State<'_, AppState>) -> Result<(), String>; // 墓碑删除
```

**设备标识（`device_id`）**：在 `db_sqlite.rs` 旁新增工具函数 `get_or_create_device_id(app_data_dir) -> String`——首次生成 uuid 写入 `<app_data_dir>/device.id`，之后读取。子任务 5 的同步合并依赖该值稳定不变。

### 3.5 迁移门控

`AppState` 中持有 `Arc<RwLock<MigrationStatus>>`。`history_*` 系列 command 在状态非 `Completed | NotNeeded` 时返回 `"MIGRATION_PENDING"` 错误，前端据此展示迁移进度页而非历史列表（前端页面不属于本任务范围，仅需保证 command 门控行为正确并在文档中说明约定）。

## 4. 实现步骤

### 4.1 v1 → v2 字段映射表（实现前先核对 `db.rs` 实际结构）

| v2 字段 | v1 来源 | 缺失时默认值 |
|---|---|---|
| `id` | 新生成 uuid v4 | — |
| `content_id` | v1 条目 ID 字段 | 缺失则该条跳过并记录 |
| `content_type` | v1 类型字段（如 `"bangumi"`→`anime` 的枚举映射） | 无法识别则跳过并记录 |
| `title` / `cover` / `source_id` | 直映射 | `source_id` 缺失填 `"unknown"` |
| `chapter_id` / `chapter_title` | 直映射 | `NULL` |
| `page_index` | 漫画页码 | `0` |
| `position_sec` | 番剧进度（若 v1 单位是毫秒或百分比需换算为秒） | `0` |
| `scroll_pct` | 小说滚动位置（若 v1 是字符偏移，按 `offset/total*100` 换算；无 total 则 `0`） | `0` |
| `updated_at` | v1 时间戳（统一换算为 Unix 毫秒） | 迁移执行时刻 |
| `device_id` | 全局设备标识 | 见 §3.4 |
| `deleted` | — | `0` |

### 4.2 编号实现步骤

1. **`Cargo.toml`**：新增依赖 `rusqlite = { version = "0.31", features = ["bundled"] }`、`uuid = { version = "1", features = ["v4", "serde"] }`、`thiserror = "1"`（若已有则复用）。`serde`/`serde_json` 复用现有。
2. **新建 `src-tauri/src/domain/mod.rs` 与 `domain/history.rs`**：按 §3.1 定义 `ContentType`、`HistoryRecord`；实现 `ContentType` 的 `FromStr`/`Display`（与 DB TEXT 互转）；实现序列化适配函数 `progress(&self) -> f64`（按类型返回 position_sec 或 scroll_pct）。
3. **新建 `src-tauri/src/db_sqlite.rs`**：
   - 定义 `DbError`（thiserror，包装 `rusqlite::Error` 与 `io::Error`）。
   - 实现 `SqliteDb::open`：创建父目录 → `Connection::open` → 执行 `PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;` → 若 `user_version < 2` 则执行 `SCHEMA_V2_SQL` 并 `set_user_version(2)`（注意：schema 初始化与数据迁移解耦，schema 建空表不算迁移完成）。
   - 实现 `HistoryRepo` trait 的全部方法（§3.3）。`list` 使用参数化查询，`keyword` 走 `title LIKE '%' || ? || '%'`；分页固定 `ORDER BY updated_at DESC`。
   - 实现 `get_or_create_device_id`。
4. **改造 `src-tauri/src/db.rs`**：将 v1 读取逻辑抽出为 `pub fn load_v1_history(app_data_dir: &Path) -> Result<Vec<V1HistoryEntry>, _>`（只读）。**不删除** v1 文件，迁移完成后仅在文件中写入标记注释或重命名为 `history.json.migrated`（二选一，推荐后者，且该操作必须在迁移 `completed` 之后执行）。
5. **新建 `src-tauri/src/migration/mod.rs`**：
   - 定义 `MigrationStatus`、`MigrationReport`、`MigrationError`。
   - 实现 `Migrator::check`：读 `migration_state` 单行；无记录且 v1 JSON 存在 → `Pending`；`in_progress` → `InProgress`（断点续迁）；`failed`/`rolled_back` → `Pending`（重试）；`completed` 或无 v1 数据 → `NotNeeded`。
   - 实现 `Migrator::run`，严格按以下顺序：
     a. 写入/更新 `migration_state`（`status='in_progress'`, `started_at=now`），先 `SELECT migrated_count, last_offset` 作为断点；
     b. **备份**：若 `backup_path` 为空，将 v1 JSON 原样复制到 `backup/history_v1_backup_<ts>.json`，校验备份文件 SHA-256 与源文件一致后才继续；已有 `backup_path`（断点场景）则复用；
     c. 读取 v1 全量记录，从 `last_offset` 开始按 **500 条一批**处理：每批一个事务，批内逐条 `map_v1_to_v2` → `upsert_idempotent`，批提交后更新 `migration_state.migrated_count/last_offset`（状态更新与数据写入在**同一事务**内，保证崩溃一致性）；
     d. 全部批次完成后校验：`SELECT COUNT(*) FROM history`（含本批次前已有数据需换算）与 v1 有效记录数比对，不一致 → 回滚；
     e. 成功 → `status='completed', finished_at=now`，执行步骤 4 的 v1 文件重命名。
   - 实现回滚：任一失败 → 在事务外执行 `DELETE FROM history WHERE device_id = ? AND id IN (本次写入的 id 集合)`（实现上更稳妥的做法：迁移期间在 `migration_state` 旁维护 `migrated_ids` 临时表 `migration_staging(id TEXT PRIMARY KEY)`，回滚时按 staging 表删除再清空；该表随 schema 一并创建）。
   - 实现 `restore_from_backup`：将备份 JSON 复制回 v1 路径（或直接从备份路径读取），重置 `migration_state` 后复用 `run` 的主体逻辑（抽公共私有函数 `run_with_source(path)`）。
6. **新建 `src-tauri/src/migration/v1_to_v2.rs`**：实现 `V1HistoryEntry`、`map_v1_to_v2`（纯函数，覆盖 §4.1 映射表）、`upsert_idempotent`（先 `SELECT updated_at`，存在且 ≥ 新值则返回 `Skipped`，否则 `INSERT OR REPLACE` 返回 `Inserted/Replaced`）。
7. **接入应用启动流程**（`lib.rs` 的 `setup` 钩子）：
   - `SqliteDb::open` → `Migrator::new` → `check`；
   - 若为 `Pending/InProgress`：`spawn_blocking` 执行 `run`，期间通过 `app.emit("migration-progress", report)` 推送进度（每批 emit 一次）；
   - 完成后将 `Migrator`、`HistoryRepo` 实现、`MigrationStatus` 注入 `AppState`，注册 §3.4 的 commands。
8. **进度事件**：定义事件名 `migration://progress`（payload 为 `MigrationReport`），前端子任务可订阅；本任务只保证 Rust 侧 emit 正确。
9. **日志**：全程使用现有日志方案（如无则用 `log` crate + `env_logger`/`tauri-plugin-log`，取仓库已有者），关键节点（备份路径、每批提交、回滚原因、完成条数）必须落日志。

## 5. 依赖的外部接口

本任务 `depends_on: []`，无上游依赖。对下游的接口承诺如下：

| 消费方 | 本任务提供 | 对接方式 |
|---|---|---|
| 子任务 5（WebDAV 同步） | `HistoryRepo::upsert / find_by_merge_key / list_tombstones_since`、`device_id` 稳定标识、`updated_at` 毫秒时间戳、墓碑 `deleted` 字段 | 直接以 Rust trait/类型复用，子任务 5 不得自行绕过 `HistoryRepo` 写库 |
| 子任务 6（历史列表/续读） | `history_list` command（分页 + 类型筛选 + 关键词）、`HistoryRecord` 的 camelCase JSON（含 `pageIndex`、`progress` 字段） | 前端通过 `invoke('history_list', {...})` 调用 |
| 前端（迁移进度页） | `migration://progress` 事件 + `migration_status` / `migration_run` / `migration_restore_backup` commands | 前端在启动时先 `migration_status`，非 `NotNeeded/Completed` 时进入迁移页并监听事件；`Failed` 时展示"从备份恢复"按钮调用 restore |

**接口假设声明**：v1 JSON 的具体字段名以 `db.rs` 实际代码为准，编码代理须先读取该文件再落实 `V1HistoryEntry`；若 v1 实际为其他轻量存储（如 sled/自格式二进制），同样只允许在 `v1_to_v2.rs` 的加载层适配，v2 schema 与迁移框架不变。

## 6. 测试要求

所有测试放 `src-tauri/src/migration/tests.rs`（或对应模块内 `#[cfg(test)]`），使用 `tempfile` 创建隔离 app_data_dir。迁移逻辑覆盖率要求 **100%**（行覆盖，`cargo tarpaulin` 或 `llvm-cov` 验证）。

### 正常路径

1. `test_open_creates_schema_and_version`：新库打开后 `user_version == 2`，`history`/`migration_state`/`migration_staging` 表与两个索引存在。
2. `test_migrate_basic_records`：构造三类（番剧/漫画/小说）各 2 条 v1 记录，迁移后条数一致、字段映射符合 §4.1（重点断言漫画的 `page_index`、番剧的 `position_sec`、小说的 `scroll_pct`）。
3. `test_backup_created_with_checksum`：迁移成功后 `backup/` 存在备份文件，内容与 v1 源文件字节一致，且迁移结束后源文件被重命名为 `.migrated`。
4. `test_migration_report_counts`：`MigrationReport.total/migrated` 与输入一致，`status == Completed`。
5. `test_idempotent_rerun`：迁移完成后再次 `run`，返回 `NotNeeded` 或零新增，记录数不变、无重复行（按 `content_id+source_id+chapter_id` 检查唯一性）。
6. `test_history_repo_list_filter_search_pagination`：插入混合数据，验证类型筛选、`keyword` 模糊匹配、`limit/offset` 分页与 `updated_at DESC` 排序。
7. `test_tombstone_and_list_exclusion`：墓碑删除后 `list` 不可见、`list_tombstones_since` 可见、`deleted=1` 且 `updated_at` 被刷新。

### 边界 / 异常路径

8. `test_crash_resume_no_duplicates`：模拟中断——第 2 批提交后、第 3 批前注入 panic（用可注入的批钩子或 fault-injection 回调），重建 `Migrator` 再 `run`：从断点继续，最终条数正确且**无重复记录**（FR-08 核心验收）。
9. `test_migrate_failure_rollback`：注入第 3 批写库失败（如 mock 磁盘错误/关闭连接），断言：`status='rolled_back'`、`history` 表中本次写入的记录全部被清除、`migration_staging` 清空、备份文件保留。
10. `test_restore_from_backup`：回滚后调用 `restore_from_backup`，迁移可重新成功。
11. `test_corrupted_v1_json`：v1 文件为非法 JSON → 迁移失败、回滚、错误信息含解析失败详情、备份仍生成。
12. `test_partial_invalid_entries_skipped`：10 条中 2 条缺 `content_id`/`title` → 8 条入库，2 条记入 skipped 日志，迁移整体成功。
13. `test_empty_v1`：v1 文件存在但为 `[]` → `status` 直接 `completed`，`total=0`，无备份或仅空备份（实现任选，测试锁定行为）。
14. `test_no_v1_data`：无 v1 文件 → `check() == NotNeeded`，不创建备份。
15. `test_missing_fields_defaults`：v1 记录缺 `cover/chapter_id/position_sec` 等可空字段 → 默认值符合 §4.1。
16. `test_device_id_stable`：两次调用 `get_or_create_device_id` 返回相同值；删除 `device.id` 后重新生成不同值。
17. `test_updated_at_unit_normalization`：v1 秒级时间戳正确换算为毫秒（若 v1 实为毫秒则断言不重复放大——以实际结构写正反两例）。
18. `test_command_gating`：`MigrationStatus` 为 `InProgress/Failed` 时 `history_list` 返回 `"MIGRATION_PENDING"`。
19. **性能测试** `test_migrate_10k_records_perf`：生成 10,000 条 v1 记录，断言：迁移总耗时 < 10s（CI 中等机型）、迁移后 `history_list(limit=50)` 查询 < 50ms、`count` == 10000。标记 `#[ignore]` 默认跳过，CI 中用 `cargo test -- --ignored` 单独跑。

## 7. 完成定义（Definition of Done）

- [ ] §4.2 步骤 1~9 全部完成，`history` 表 schema 与 §3.2 完全一致（字段、索引、CHECK 约束）。
- [ ] v1→v2 迁移在含旧数据的启动中自动触发，迁移后备份文件位于 `backup/` 且校验一致，条数一致。
- [ ] 失败自动回滚：回滚后 `history` 无残留本次写入记录，`migration_state.status='rolled_back'`，`restore_from_backup` 可用。
- [ ] 断点续迁：中断后重启从 `last_offset` 继续，无重复记录（测试 8 通过）。
- [ ] 漫画历史记录经 `history_list` 返回的 JSON 包含 `contentId/contentType/title/cover/chapterId/chapterTitle/pageIndex/progress/sourceId/updatedAt/deviceId` 全部字段。
- [ ] §6 全部 19 个测试用例编写并通过；迁移模块行覆盖率 100%；1 万条性能测试通过。
- [ ] Tauri commands 已注册并被 invoke_handler 引用；迁移进度事件 `migration://progress` 正常 emit。
- [ ] 关键节点日志（备份、分批提交、回滚、完成）已接入。
- [ ] `cargo build` 与 `cargo test` 在三端目标（至少 Windows x64 + macOS）无新增告警/错误；`rusqlite/bundled` 不引入系统依赖。
- [ ] 迁移逻辑对子任务 5/6 的接口（`HistoryRepo`、commands、事件）有 rustdoc 注释说明。

## 8. 禁止修改清单

- ❌ `src-tauri/src/db.rs` 中 **v1 数据文件的读取格式定义**（只允许新增只读导出函数与迁移后重命名逻辑；不得改动 v1 原有结构体字段，否则旧版本回退兼容被破坏）。
- ❌ 前端 `src/` 下所有页面与组件（迁移进度 UI 属其他任务；本任务仅提供 commands/事件）。
- ❌ 规则引擎相关目录（`src-tauri/src/rules/` 或等价目录，子任务 1/2 产物）。
- ❌ 播放器相关代码（子任务 3 范围）。
- ❌ `.github/workflows/release.yml` 及 CI 主流程（PRD §6.3 明确本期不改动；如 CI 需加 `cargo test` 步骤须单独提审）。
- ❌ `tauri.conf.json` 中的应用标识、版本号与产物配置（新增 fs 权限 scope 如需读取 backup 目录，仅限在 `capabilities`/`permissions` 中追加，不得修改既有项）。
- ❌ 任何第三方源的规则文件与本任务无关的资源目录。