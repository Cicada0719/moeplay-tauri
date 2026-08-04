# 子任务 5 开发规格说明：历史记录 WebDAV 同步

## 1. 任务目标

在历史数据模型 v2（SQLite，子任务 4 产出）之上，实现完整的 WebDAV 同步能力：凭据安全存储（keyring）、WebDAV 轻量客户端、last-write-wins 冲突合并、墓碑删除传播，以及配套设置界面。要求同步过程幂等、断网可恢复、任何失败路径不清空本地数据，并支持"仅手动 / 启动时自动 + 每 30 分钟"两种模式（默认手动）。

## 2. 上下文与约束

- 对应 PRD：**FR-09**（历史记录同步，P0）、§4.2 安全（keyring、HTTPS、禁止明文凭据）、§5.4（合并算法与墓碑策略）、风险 R5/R6。
- 技术栈：Tauri 2 + Svelte，Rust 侧 `reqwest`（rustls）网络出口、`tokio`、`rusqlite`/`sqlx`（沿用任务 4 选定方案）、`keyring` crate。
- 前置依赖：**子任务 4（历史模型 v2 + SQLite + 迁移）必须先完成**。本任务只消费其数据访问层，不修改迁移逻辑。
- 现有代码：仓库中已有 `src-tauri/src/cloud_save.rs`（旧的云端保存雏形）。本任务将其能力重构成 `src-tauri/src/sync/` 模块；`cloud_save.rs` 仅保留对已注册旧 Tauri command 的兼容转发（若存在），新逻辑一律放 `sync/`。
- 数据模型（任务 4 已建表，PRD §5.3）：`history(id, content_id, content_type, title, cover, source_id, chapter_id, chapter_title, page_index, position_sec, scroll_pct, updated_at, device_id, deleted)`。
- 约束：
  - WebDAV 密码**必须**存 keyring，配置文件中只允许出现 URL、用户名（非密）、同步模式等非敏感字段。
  - 网络请求全程 HTTPS；`reqwest` 客户端禁止设置 `danger_accept_invalid_certs(true)`。
  - 同步任务全局单实例运行（互斥锁），自动同步与手动同步不得并发。
  - 任何远端解析失败 / 认证失败 / 网络失败，均**不得写入或清空本地数据**（先合并到内存，成功后单事务落库）。

## 3. 输入 / 输出

### 3.1 远端文件布局（WebDAV）

```
{baseUrl}/moeplay-sync/
├── manifest.json        # {"version":1,"updated_at":..., "etag_hint":"..."}
└── history.json         # 全量记录数组（含墓碑），见 3.3
```

- 目录不存在时由首次同步自动 `MKCOL` 创建。
- 写策略：下载远端 `history.json` → 内存合并 → 生成新文件内容 → `PUT`（带 `If-Match` ETag 做乐观并发控制，冲突时重新拉取合并重试 1 次）→ `PUT manifest.json`。
- 该策略保证幂等：重复执行同一合并结果产生相同的最终远端文件，不产生重复记录（记录有唯一 `id` 与合并键）。

### 3.2 Rust 核心类型（`src-tauri/src/sync/`）

```rust
// sync/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncMode { Manual, Auto }              // 默认 Manual

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavConfig {                        // 不含密码！
    pub base_url: String,                        // 如 https://dav.jianguoyun.com/dav
    pub username: String,
    pub mode: SyncMode,
    pub remote_dir: String,                      // 默认 "moeplay-sync"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub uploaded: u32,      // 上传 n 条
    pub downloaded: u32,    // 下载 m 条（远端新增/更新到本地的条数）
    pub conflicts: u32,     // 同键冲突合并 k 条
    pub tombstones_purged: u32,
    pub synced_at: i64,     // unix 秒
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum SyncError {
    #[error("认证失败，请检查用户名与授权密码")] Auth,
    #[error("网络连接失败: {0}")] Network(String),
    #[error("远端数据格式非法")] Parse,
    #[error("服务端错误: {0}")] Server(String),
    #[error("同步正在进行中")] Busy,
    #[error("未配置 WebDAV")] NotConfigured,
}
```

### 3.3 同步记录传输结构（`history.json`）

```rust
// sync/merge.rs — 与 DB 行一一对应的可序列化快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub id: String,
    pub content_id: String,
    pub content_type: String,        // anime | manga | novel
    pub title: String,
    pub cover: Option<String>,
    pub source_id: String,
    pub chapter_id: Option<String>,
    pub chapter_title: Option<String>,
    pub page_index: i64,
    pub position_sec: f64,
    pub scroll_pct: f64,
    pub updated_at: i64,
    pub device_id: String,
    pub deleted: bool,               // 墓碑
}

// 合并入口（纯函数，必须可无 IO 单测）
pub fn merge_records(local: Vec<SyncRecord>, remote: Vec<SyncRecord>)
    -> MergeOutcome;

pub struct MergeOutcome {
    pub merged: Vec<SyncRecord>,     // 最终一致集（双方都应落为这个集合）
    pub uploaded: u32,               // local 有而 remote 缺失/过旧的条数
    pub downloaded: u32,             // remote 有而 local 缺失/过旧的条数
    pub conflicts: u32,              // 合并键冲突次数（同键双方都有记录）
}
```

**合并规则（严格按 PRD §5.4）**：
1. 匹配键 = `(content_id, source_id)`。
2. 同键冲突：`updated_at` 新者胜。
3. `updated_at` 相同（同秒）：取进度更大者 —— 按类型取 `max(page_index)` / `max(position_sec)` / `max(scroll_pct)`。
4. 任一方 `deleted == true` 且其 `updated_at` 不旧于另一方 → 墓碑胜（删除传播）。
5. 墓碑保留 90 天：`purge_tombstones(now, 90)` 清理 `deleted=true 且 updated_at < now - 90*86400` 的记录（物理删除）。

### 3.4 WebDAV 客户端（`sync/webdav.rs`）

```rust
pub struct WebDavClient { /* reqwest::Client + base url + auth header */ }

impl WebDavClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Result<Self, SyncError>;
    pub async fn ensure_dir(&self, dir: &str) -> Result<(), SyncError>;   // MKCOL，忽略 405/已存在
    pub async fn get(&self, path: &str) -> Result<Option<(Vec<u8>, Option<String>)>, SyncError>; // 404 → None，返回 ETag
    pub async fn put(&self, path: &str, body: Vec<u8>, if_match: Option<&str>) -> Result<(), SyncError>;
    pub async fn propfind_exists(&self, path: &str) -> Result<bool, SyncError>;
}
```

- HTTP 401/403 → `SyncError::Auth`；超时（连接 10s / 总 60s）与 DNS/连接错误 → `SyncError::Network`；5xx → `SyncError::Server`。
- 认证方式：HTTP Basic（坚果云/Nextcloud 均支持应用密码 + Basic）。凭据只进 `Authorization` 头，不落盘、不进日志（日志必须脱敏）。

### 3.5 凭据存储（`sync/keyring_store.rs`）

```rust
const KEYRING_SERVICE: &str = "moeplay-tauri";
const KEYRING_ACCOUNT: &str = "webdav";

pub fn save_password(password: &str) -> Result<(), SyncError>;
pub fn get_password() -> Result<Option<String>, SyncError>;
pub fn delete_password() -> Result<(), SyncError>;
```

- 使用 `keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)`。
- 平台映射：Windows Credential Manager / macOS Keychain / Linux Secret Service（PRD §4.2）。

### 3.6 Tauri Commands（前端输入/输出）

```rust
#[tauri::command] async fn sync_now(state: State<'_, SyncState>) -> Result<SyncResult, SyncError>;
#[tauri::command] async fn test_webdav_connection(cfg: WebDavConfig, password: String) -> Result<(), SyncError>;
#[tauri::command] async fn save_webdav_config(cfg: WebDavConfig, password: Option<String>, state: ...) -> Result<(), SyncError>;
#[tauri::command] async fn get_sync_config(state: ...) -> Result<Option<WebDavConfig>, SyncError>;   // 永不返回密码
#[tauri::command] async fn get_sync_status(state: ...) -> Result<SyncStatus, SyncError>;

#[derive(Serialize)] pub struct SyncStatus {
    pub configured: bool,
    pub last_result: Option<SyncResult>,
    pub syncing: bool,
}
```

### 3.7 前端组件接口（`SyncSettings.svelte`）

- Props：无（自成设置区块，挂在设置页"同步"分组下）。
- 状态：`config: WebDavConfig | null`、`passwordInput: string`、`status: SyncStatus`、`testing: boolean`。
- 交互：
  - 表单：WebDAV 地址、用户名、密码（密码 placeholder 显示"已保存（不显示）"当已配置）、同步模式单选（仅手动 / 启动时自动 + 每 30 分钟）。
  - 按钮：`测试连接`（调 `test_webdav_connection`，成功 Toast"连接成功"）、`保存配置`、`立即同步`（同步中禁用并显示 spinner，完成后展示"上传 n 条 / 下载 m 条 / 冲突 k 条"）、`清除配置`（二次确认，删除 keyring 凭据与配置，**不删除任何历史数据**）。
  - 错误展示：`SyncError` 的 message 直接作为红色提示文案显示（认证失败文案必须明确）。

## 4. 实现步骤

1. **添加依赖**（`src-tauri/Cargo.toml`）：`keyring = "3"`、`thiserror`、`serde_json`（已有则跳过）。确认 `reqwest` 启用的特性不含 `native-tls`（保持 rustls）。
2. **新建 `src-tauri/src/sync/mod.rs`**：
   - 定义 §3.2 的 `SyncMode / WebDavConfig / SyncResult / SyncError / SyncStatus`。
   - 定义 `pub struct SyncState { lock: tokio::sync::Mutex<()>, config: Mutex<Option<WebDavConfig>>, last_result: Mutex<Option<SyncResult>> }`。
   - 实现 `pub async fn run_sync(app: &AppHandle, state: &SyncState) -> Result<SyncResult, SyncError>` 编排函数：
     a. `try_lock` 失败返回 `SyncError::Busy`；
     b. 读配置（无 → `NotConfigured`），从 keyring 取密码（无 → `Auth`）；
     c. 构建 `WebDavClient`，`ensure_dir("moeplay-sync")`；
     d. `get("moeplay-sync/history.json")`：404/None → 远端为空数组；解析失败 → `Parse`，**立即返回不写本地**；
     e. 从任务 4 的数据访问层拉取本地全量（含墓碑）→ 转 `SyncRecord`；
     f. `merge_records` 得到 `MergeOutcome`；
     g. 将 merged 序列化为新 `history.json`，`put(..., if_match=etag)`；若 412（ETag 冲突）→ 重新执行 d~f 一次后无条件 `put`（最多重试 1 次，仍失败返回 `Server`）；
     h. `put manifest.json`；
     i. **单事务**将 `downloaded` 涉及的远端胜出记录 upsert 进本地 SQLite，并执行 `purge_tombstones(90 天)`；
     j. 更新 `last_result`，写审计日志（不含凭据）。
   - **关键不变量**：步骤 d~g 全部成功后才能执行 i；i 失败只影响本地写入，返回 `Server` 但不回滚远端（下次同步幂等收敛）。
3. **新建 `src-tauri/src/sync/webdav.rs`**：按 §3.4 实现。`reqwest::Client::builder().timeout(60s).connect_timeout(10s).https_only(true)`；Basic Auth 用 `base64` 手动拼头（避免把密码存进 client 可 clone 结构外的位置）。`put` 支持 `If-Match` 头。所有 `reqwest::Error` 分类映射到 `SyncError`。
4. **新建 `src-tauri/src/sync/merge.rs`**：按 §3.3 实现 `merge_records` 与 `purge_tombstones(records, now, retention_days)`，纯函数、不依赖 DB/网络。合并键用 `HashMap<(String,String), SyncRecord>` 实现 O(n)。
5. **新建 `src-tauri/src/sync/keyring_store.rs`**：按 §3.5 实现三个函数；Linux 无 Secret Service 环境下 `get_password` 返回 `Ok(None)` 并记 warn 日志（不 panic）。
6. **改造 `src-tauri/src/cloud_save.rs`**：删除其中旧的同步实现逻辑（如有），改为 `pub use crate::sync::*` 的兼容转发或保留旧 command 名字但内部调用 `run_sync`；不得保留任何明文写凭据的代码路径。
7. **注册与接线**（`src-tauri/src/lib.rs` 或 `main.rs`）：
   - `mod sync;`，`.manage(SyncState::new())`，注册 §3.6 全部 command。
   - 启动时：`SyncState::load_config_from_db()` 读取配置；若 `mode == Auto`，立即 spawn 一次 `run_sync` 并启动 `tokio::time::interval(30min)` 循环任务（AppHandle clone 持有，退出时随 runtime 结束）。
   - 配置存储位置：复用任务 4 的 SQLite（新增 `sync_config` 单行表或 settings KV 表，key=`webdav_config`，value=JSON 序列化的 `WebDavConfig`）。**密码绝不进该表**。
8. **设置落库函数**（可放 `sync/mod.rs`）：`save_config_to_db(db, &WebDavConfig)` / `load_config_from_db(db)`；`save_webdav_config` command 内：先存 keyring 密码（若提供），成功后再写配置表，保证顺序（避免有配置无密码的悬空态）。
9. **前端新建 `src/lib/components/settings/SyncSettings.svelte`**：按 §3.7 实现；用 `invoke` 调后端；同步模式切换即时调用 `save_webdav_config`（不带密码字段则后端保留旧密码）。
10. **接入设置页路由**：找到现有设置页容器（如 `src/lib/components/settings/` 下的 index/Settings 页），引入并渲染 `<SyncSettings />`。只追加，不改动其他设置区块。
11. **任务 4 数据访问层补充**（若任务 4 未提供，在 `src-tauri/src/db/history.rs` **以新增函数方式**补齐，不改既有函数签名）：
    - `list_all_with_deleted() -> Vec<HistoryRow>`；
    - `upsert_from_sync(records: &[SyncRecord]) -> Result<usize>`（事务内逐条 INSERT OR REPLACE，仅在 `updated_at` 更新或不存在时覆盖，防止本地新数据被旧同步结果回写）；
    - `purge_tombstones_db(older_than: i64)`。
12. **前端类型文件**（可选）：新建 `src/lib/types/sync.ts`，定义 `SyncMode/WebDavConfig/SyncResult/SyncStatus` 的 TS 镜像类型。

## 5. 依赖的外部接口

| 依赖方 | 接口假设 | 对接方式 |
|---|---|---|
| 子任务 4（历史模型 v2） | `history` 表结构与 PRD §5.3 一致，含 `deleted` 墓碑列与 `device_id`；存在设备 ID 持久化（首次启动生成 uuid） | 通过 `src-tauri/src/db/history.rs` 的数据访问函数调用；若函数缺失，按步骤 11 新增，不改任务 4 已有代码 |
| 任务 4 的 DB 连接管理 | 存在可传入的 SQLite 连接/连接池句柄（如 `State<DbPool>`） | `SyncState` 持有同一 pool 的克隆，所有同步写操作走单事务 |
| 现有 `cloud_save.rs` | 可能注册了旧 Tauri command（名称未知） | 保留 command 名做兼容转发；若 grep 后确认无前端调用，可直接删除旧实现 |
| 前端设置页容器 | `src/lib/components/settings/` 存在设置页入口组件 | 仅追加 `<SyncSettings />` 挂载点 |

## 6. 测试要求

### 6.1 合并算法单测（`sync/merge.rs` 内 `#[cfg(test)]`，必须全部覆盖）

- [ ] 正常路径：本地 3 条 + 远端 2 条全新 → merged=5，downloaded=2，uploaded=3，conflicts=0。
- [ ] LWW：同键（content_id+source_id）本地 `updated_at=100`、远端 `=200` → 远端胜，downloaded+1，conflicts+1。
- [ ] 反向 LWW：本地较新 → 本地胜，uploaded+1。
- [ ] 同秒冲突：本地 page_index=5、远端 page_index=8、updated_at 相同 → 取 8（漫画）；番剧取 position_sec 大者；小说取 scroll_pct 大者。
- [ ] 墓碑传播：远端 deleted=true 且 updated_at 较新 → merged 中该键为墓碑，本地方记录被墓碑覆盖。
- [ ] 墓碑 vs 较新正常记录：本地正常记录 updated_at 更新 → 正常记录胜（"复活"）。
- [ ] `purge_tombstones`：updated_at 早于 91 天的墓碑被物理移除，89 天的保留。
- [ ] 幂等性：merge(merge(A,B), B) 与 merge(A,B) 结果集合相同；重复合并两次 uploaded 第二次为 0。

### 6.2 WebDAV 客户端测试（使用 `wiremock` 或手写 hyper mock server）

- [ ] 200 GET 返回 body + ETag 解析正确；404 → `Ok(None)`。
- [ ] 401 → `SyncError::Auth`；500 → `Server`；连接超时 → `Network`。
- [ ] PUT 携带 `If-Match` 头；412 响应被上层识别触发一次重拉重试。
- [ ] MKCOL 对已存在目录（405）不报错。

### 6.3 集成测试（`src-tauri/tests/sync_integration.rs`，可用 `tempfile` + 内存 SQLite + wiremock）

- [ ] 端到端：本地 3 条、远端 2 条 → `run_sync` 返回 `{uploaded:3, downloaded:2, conflicts:0}`，本地库新增远端 2 条，远端文件含 5 条。
- [ ] 双设备场景：模拟 A（第 8 集，较新）与 B（第 5 集，较旧）先后同步 → 两次后 B 本地为第 8 集，远端为第 8 集。
- [ ] 断网幂等：mock 服务端首次 PUT 后返回 500 中断 → 返回错误且本地未被清空；恢复后重跑 → 最终一致、无重复记录。
- [ ] 认证失败：mock 恒 401 → 返回 `Auth`，本地数据逐条比对无变化。
- [ ] 远端损坏：`history.json` 为非法 JSON → 返回 `Parse`，本地零写入。
- [ ] 并发保护：并发调两次 `sync_now` → 其一返回 `Busy`。

### 6.4 keyring 与前端

- [ ] keyring save/get/delete 冒烟测试（CI 无系统密钥环时 `#[ignore]` 或环境变量门控，本地手动验证三平台）。
- [ ] 前端（Vitest 或手动验收清单）：认证失败时红字显示"认证失败"；"立即同步"进行中按钮禁用；同步结果条展示 n/m/k 三数。
- [ ] 手动验收：坚果云 + Nextcloud 各跑一次真实端到端同步（PRD R5 兼容矩阵）。

## 7. 完成定义（Definition of Done）

- [ ] `src-tauri/src/sync/{mod,webdav,merge,keyring_store}.rs` 四个模块实现完毕，`cloud_save.rs` 完成兼容化改造，无残留明文凭据代码路径。
- [ ] §3.6 全部 Tauri command 注册并可被前端调用；`get_sync_config` 任何情况下不返回密码。
- [ ] 合并算法单测 §6.1 全部通过；集成测试 §6.3 全部通过（`cargo test` 绿）。
- [ ] 凭据经 keyring 存取，配置文件/SQLite 中 grep 不到密码字段（安全自查）。
- [ ] 同步模式"仅手动 / 启动时自动 + 每 30 分钟"均生效，默认手动；Auto 模式启动时触发一次同步。
- [ ] 全部失败路径（认证失败/断网/远端损坏/并发）本地历史数据零丢失、零重复（有测试断言）。
- [ ] 删除传播：A 删除一条 → 同步 → B 同步后该条不可见，墓碑 90 天清理逻辑生效。
- [ ] 设置页 SyncSettings 上线：测试连接、保存、立即同步、结果展示、清除配置均可用。
- [ ] 坚果云 / Nextcloud 至少各 1 次真实手动端到端验证通过并留有记录。
- [ ] 无 `unwrap/expect` 于 IO 路径（全部映射为 `SyncError`）；日志不含凭据。

## 8. 禁止修改清单

- `.github/workflows/release.yml` 及 CI 主流程（PRD §6.3 假设）。
- 规则引擎与源相关：`src-tauri/src/rules/`、`src-tauri/src/engine/`（或任务 1/2 产出的等效目录）、`resources/rules/`。
- 播放器相关：播放器组件、控制栏隐藏逻辑（任务 3 范围）。
- 任务 4 的迁移模块：v1→v2 迁移脚本、备份/回滚逻辑（仅允许在其数据访问层**新增** §4 步骤 11 的函数，禁止修改既有函数签名与迁移代码）。
- 漫画阅读器与双页相关文件（任务 6 范围）。
- `Cargo.toml` 中除新增本任务依赖外，不得升级/降级任何既有依赖版本。
- 前端除设置页容器追加挂载点外，不得改动其他设置区块与全局 store 结构。