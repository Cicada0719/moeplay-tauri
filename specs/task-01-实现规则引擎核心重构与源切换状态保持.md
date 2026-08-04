# 子任务 1 开发规格说明：规则引擎核心重构与源切换状态保持

## 1. 任务目标

在 Rust 侧基于 `rquickjs` 构建沙箱化规则执行引擎，统一番剧/漫画/小说三类规则的「加载 → 校验 → 执行 → 错误上报」生命周期，单条规则异常不影响应用主体。前端配合实现源切换时的上下文（条目、集数、进度）保持与竞态取消，并提供 kazumi 兼容规则的导入/导出能力（FR-01、FR-02、FR-05）。

## 2. 上下文与约束

- **关联 PRD 章节**：FR-01（统一规则引擎，P0）、FR-02（源切换状态保持，P0）、FR-05（自定义规则导入/导出，P2）；§5.1 技术选型（rquickjs + reqwest rustls 统一网络出口）、§5.4（CancellationToken 竞态处理）、§4.2（沙箱安全约束）。
- **技术栈**：Tauri 2 + Svelte（不可变更）。Rust 侧新增依赖：`rquickjs`（启用 `loader` feature）、`reqwest`（rustls）、`tokio-util`（CancellationToken）、`serde`/`serde_json`/`serde_yaml`、`thiserror`、`tracing`、`uuid`。
- **现有代码结构假设**（编码代理需先 `ls` 验证，若目录名不同则以实际为准并沿用现有分层习惯）：
  - `src-tauri/src/main.rs`：Tauri 入口，注册命令。
  - `src-tauri/src/commands/`：Tauri command 模块目录。
  - `src-tauri/src/state.rs`（或 `main.rs` 内联）：`tauri::State` 管理。
  - `src/lib/stores/`：Svelte store（Svelte 4 用 writable store，Svelte 5 可沿用）。
  - `src/lib/components/`：现有 `SourceList.svelte` 等组件。
  - 现有规则执行逻辑（如有 `src-tauri/src/parser.rs` 或前端 JS eval 逻辑）**保留文件但停止调用**，由新引擎接管；具体废弃点见 §8 禁止修改清单的例外说明。
- **约束**：
  - 规则脚本在 QuickJS 沙箱内执行：禁止文件系统/进程访问；网络请求只能经由注入的 Rust `fetch` 桥接函数，走统一 reqwest 出口。
  - 单条规则加载超时阈值 **10s**，执行调用默认超时 **15s**（可在常量中配置）。
  - 规则执行必须在独立 OS 线程池（非 Tauri 主线程、非 async runtime 阻塞线程）运行，QuickJS runtime 不可跨线程共享。
  - 兼容 kazumi 规则格式：v2 引擎必须能加载 v1/kazumi 风格规则（JSON/YAML 描述 + JS 函数体字符串）。

## 3. 输入 / 输出

### 3.1 Rust 侧数据结构（`src-tauri/src/rules/schema.rs`）

```rust
/// 规则清单（kazumi 兼容，JSON/YAML 反序列化目标）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleManifest {
    pub name: String,                 // 规则名，唯一标识之一
    pub version: String,              // 语义化版本，如 "1.2.0"
    pub content_type: ContentType,    // anime | manga | novel
    pub base_url: String,             // 站点根地址
    #[serde(default)]
    pub language: String,             // 默认 "zh-CN"
    #[serde(default)]
    pub nsfw: bool,                   // 默认 false
    #[serde(default)]
    pub author: Option<String>,
    /// 各生命周期函数的 JS 源码（kazumi 风格：函数字符串）
    pub search: String,               // async function search(keyword, page) -> SearchItem[]
    pub detail: String,               // async function detail(url) -> Detail
    pub chapter: String,              // async function chapter(detailUrl) -> Chapter[]
    pub parse: String,                // async function parse(chapterUrl) -> ParseResult
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType { Anime, Manga, Novel }

/// 加载后的规则实体（含运行时状态）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedRule {
    pub id: String,                   // uuid，加载时生成；自定义规则持久化其 manifest
    pub manifest: RuleManifest,
    pub origin: RuleOrigin,           // Builtin | Custom
    pub status: RuleStatus,
    pub error: Option<RuleLoadError>, // status=Invalid 时填充
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleOrigin { Builtin, Custom }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleStatus { Ready, Invalid, Loading }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleLoadError {
    pub message: String,
    pub line: Option<u32>,            // QuickJS 解析出的错误行号
    pub phase: String,                // "schema" | "compile" | "timeout"
}
```

### 3.2 执行结果类型（`src-tauri/src/rules/engine.rs`）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub title: String,
    pub url: String,                  // 详情页 URL（相对 base_url 亦可，引擎内归一化）
    pub cover: Option<String>,
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub title: String,
    pub cover: Option<String>,
    pub description: Option<String>,
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub title: String,                // 如 "第 5 集" / "第 12 话"
    pub url: String,
    pub index: u32,                   // 1-based 序号，源切换定位依赖此字段
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub urls: Vec<String>,            // 播放/图片/文本资源地址列表
    pub kind: String,                 // "video" | "images" | "text"
    pub headers: Option<std::collections::HashMap<String, String>>, // 播放所需 header（Referer 等）
}
```

### 3.3 引擎公开 API（`src-tauri/src/rules/engine.rs`）

```rust
pub struct RuleEngine { /* 内部：规则注册表 + 线程池 + CancellationToken 注册表 */ }

impl RuleEngine {
    pub fn new(http: reqwest::Client) -> Self;

    /// 并行加载一批规则；每条独立计时 10s，超时/失败仅影响该条。
    /// 返回全部规则的 LoadedRule（含失败项）。
    pub async fn load_rules(&self, inputs: Vec<RuleInput>) -> Vec<LoadedRule>;

    /// 校验单个 manifest（schema 完整性 + JS 语法预编译），不执行。
    pub fn validate(manifest: &RuleManifest) -> Result<(), RuleLoadError>;

    /// 生命周期执行。token 用于 FR-02 竞态取消。
    pub async fn search(&self, rule_id: &str, keyword: &str, page: u32,
                        token: CancellationToken) -> Result<Vec<SearchItem>, RuleExecError>;
    pub async fn detail(&self, rule_id: &str, url: &str,
                        token: CancellationToken) -> Result<Detail, RuleExecError>;
    pub async fn chapters(&self, rule_id: &str, detail_url: &str,
                          token: CancellationToken) -> Result<Vec<Chapter>, RuleExecError>;
    pub async fn parse(&self, rule_id: &str, chapter_url: &str,
                       token: CancellationToken) -> Result<ParseResult, RuleExecError>;

    /// 取消指定 scope（如 "play:{contentId}"）下所有未完成任务
    pub fn cancel_scope(&self, scope: &str);
    /// 为 scope 生成新 token（自动取消该 scope 旧 token）
    pub fn new_scope_token(&self, scope: &str) -> CancellationToken;
}

pub enum RuleInput {
    Manifest { manifest: RuleManifest, origin: RuleOrigin },
    File { path: PathBuf, origin: RuleOrigin }, // .json/.yaml 按扩展名解析
}

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuleExecError {
    #[error("规则不存在或无效: {0}")] RuleNotFound(String),
    #[error("规则执行超时")] Timeout,
    #[error("任务已取消")] Cancelled,
    #[error("脚本异常: {message}")] ScriptError { message: String, line: Option<u32> },
    #[error("网络错误: {0}")] Network(String),
    #[error("返回结构不合法: {0}")] BadReturn(String),
}
```

### 3.4 Tauri Commands（`src-tauri/src/commands/rules.rs`）

```rust
#[tauri::command] pub async fn rules_load_all(state: State<'_, RuleEngineState>) -> Result<Vec<LoadedRule>, String>;
#[tauri::command] pub async fn rules_search(state: State<'_, RuleEngineState>, rule_id: String, keyword: String, page: u32) -> Result<Vec<SearchItem>, RuleExecError>;
#[tauri::command] pub async fn rules_detail(state: State<'_, RuleEngineState>, rule_id: String, url: String) -> Result<Detail, RuleExecError>;
#[tauri::command] pub async fn rules_chapters(state: State<'_, RuleEngineState>, rule_id: String, detail_url: String) -> Result<Vec<Chapter>, RuleExecError>;
/// scope 由前端传入，如 "play:{contentId}"，实现 FR-02 取消语义
#[tauri::command] pub async fn rules_parse(state: State<'_, RuleEngineState>, rule_id: String, chapter_url: String, scope: String) -> Result<ParseResult, RuleExecError>;
#[tauri::command] pub async fn rules_cancel_scope(state: State<'_, RuleEngineState>, scope: String) -> Result<(), String>;
/// 导入本地规则文件（前端弹文件对话框拿到路径后传入）
#[tauri::command] pub async fn rules_import(state: State<'_, RuleEngineState>, path: String) -> Result<LoadedRule, RuleLoadError>;
/// 删除自定义规则（内置规则拒绝删除）
#[tauri::command] pub async fn rules_remove_custom(state: State<'_, RuleEngineState>, rule_id: String) -> Result<(), String>;
/// 导出全部规则（含内置+自定义）为 JSON 数组到指定路径
#[tauri::command] pub async fn rules_export(state: State<'_, RuleEngineState>, path: String) -> Result<u32, String>; // 返回导出条数
```

### 3.5 前端 API（`src/lib/stores/sourceSwitch.ts`）

```ts
export interface SwitchContext {
  contentId: string;        // 条目标识（搜索项 url 归一化后的稳定 id）
  title: string;
  chapterIndex: number;     // 当前集/话/章 1-based
  positionSec: number;      // 当前播放进度（秒），漫画/小说为 0
}

export interface SwitchResult {
  status: 'ok' | 'fallback' | 'failed';
  chapters: Chapter[];
  targetChapter: Chapter;   // 实际定位到的章节
  parseResult?: ParseResult;
  resumeSec: number;        // 续播秒数（fallback 时为 0）
  message?: string;         // fallback/failed 时的用户提示文案
}

/** 核心入口：切换源并保持上下文。并发调用时仅最后一次生效。 */
export function switchSource(targetRuleId: string, ctx: SwitchContext): Promise<SwitchResult>;

/** 内部状态 store */
export const sourceSwitchState: Writable<{
  switching: boolean;
  currentScope: string | null;   // "play:{contentId}"
  lastError: string | null;
}>;
```

**切换算法（`switchSource` 内部逻辑，编码代理必须按此实现）**：
1. 生成 `scope = "play:" + contentId`，调用 `rules_cancel_scope(scope)`（取消前序任务）。
2. `rules_search(targetRuleId, ctx.title, 1)` 找到匹配条目（标题归一化比较：去空白/大小写/全半角），取首个匹配项 `detail_url`。
3. `rules_chapters(targetRuleId, detail_url)` 获取章节列表。
4. 定位：`chapters.find(c => c.index === ctx.chapterIndex)`；
   - 命中 → `resumeSec = ctx.positionSec`；
   - 未命中 → 取 `chapters` 中 `index` 最大者，`resumeSec = 0`，`status = 'fallback'`，`message = "当前源暂无第 N 集，已跳转至最新一集"`。
5. `rules_parse(targetRuleId, targetChapter.url, scope)` 解析播放地址；失败返回 `status='failed'` 并携带错误（由播放器错误 UI 处理，错误 UI 本体属任务 3，本任务仅保证错误结构化透传）。
6. 任一步骤收到 `Cancelled` 错误 → 静默丢弃结果（不更新 UI、不报错）。

### 3.6 前端组件（`src/lib/components/SourceList.svelte`）

Props / 事件：

```ts
export let rules: LoadedRule[];                 // 来自 rules_load_all
export let activeRuleId: string | null;
// 事件：dispatch('select', { ruleId }) —— 由父组件调用 switchSource
// 事件：dispatch('import') / dispatch('remove', { ruleId })
```

UI 要求：
- `status === 'invalid'` 的源：置灰（`opacity-50`）、禁止点击、`title` 提示 `error.message`。
- `origin === 'custom'` 的源：显示「自定义」徽标 + 删除按钮。
- 顶部提供「导入规则」按钮（触发 Tauri 文件对话框，过滤 `.json,.yaml,.yml`）。
- 切换中（`sourceSwitchState.switching`）整列表显示 loading 遮罩，禁止重复点击。

## 4. 实现步骤

> 前置：编码代理先执行 `ls src-tauri/src src/lib/stores src/lib/components`，确认现有结构；`Cargo.toml` 中确认 `tauri` 为 2.x。以下步骤按顺序执行。

**Step 1 — 依赖引入**（`src-tauri/Cargo.toml`）

新增：
```toml
rquickjs = { version = "0.8", features = ["loader", "macro", "array-buffer", "futures"] }
tokio-util = { version = "0.7", features = ["rt"] }
serde_yaml = "0.9"
thiserror = "2"
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }
```
（`reqwest`/`tokio`/`serde_json` 若已存在则复用，确保 reqwest 启用 `rustls-tls`。）

**Step 2 — Schema 模块**（新建 `src-tauri/src/rules/schema.rs`）

按 §3.1 定义全部类型。实现：
- `impl RuleManifest { pub fn from_str(s: &str, format: RuleFileFormat) -> Result<Self, RuleLoadError> }`，`RuleFileFormat::{Json, Yaml}` 按扩展名判定。
- schema 校验函数 `validate_manifest(&RuleManifest) -> Result<(), RuleLoadError>`：检查 `name/version/base_url/search/detail/chapter/parse` 非空、`base_url` 为合法 http(s) URL、四个脚本字段均含 `function` 关键字；缺失字段在错误 `message` 中列明具体字段名（PRD FR-03 要求缺字段拒绝加载，此处先行保证）。

**Step 3 — 沙箱模块**（新建 `src-tauri/src/rules/sandbox.rs`）

- `pub struct Sandbox;` 封装 `rquickjs::Runtime + Context`。
- `Sandbox::new(http: reqwest::Client) -> Result<Self>`：
  - 创建 Runtime，设置内存上限 `64MB`（`runtime.set_memory_limit`）与最大栈 `1MB`。
  - 创建 Context，**不注册** 任何文件系统/os 模块（QuickJS 默认无 std/os 即满足沙箱要求，需确认未启用 `rquickjs` 的 `std` feature）。
  - 注入全局 `fetch(url, options?)`：通过 `rquickjs::Function` 桥接到 Rust，内部调用统一 reqwest client（30s 超时、默认 UA `moeplay/2.0`、记录 `tracing::info!` 审计日志：rule_id + url + status），返回 `{ status, body, headers }`。支持 `options.headers/method/body`。
  - 注入 `console.log/warn/error` → `tracing` 转发。
- `Sandbox::compile_check(script: &str, name: &str) -> Result<(), RuleLoadError>`：`ctx.eval` 编译函数体（包装为 `(function(){ return ( <script> ); })()` 形式仅做 `Function::new` 语法检查，不执行），捕获 `rquickjs::Error::Exception` 并从 exception message 中正则提取行号填充 `RuleLoadError.line`。
- `Sandbox::call(script: &str, fn_name: &str, args: Vec<serde_json::Value>, http_headers: Option<HashMap>) -> Result<serde_json::Value, RuleExecError>`：eval 脚本注册函数 → 取全局函数 → 调用 → 若为 Promise 则 `ctx.execute_pending_job()` 驱动至完成 → 结果转 JSON。所有异常映射为 `ScriptError{message,line}`。

**Step 4 — 引擎模块**（新建 `src-tauri/src/rules/engine.rs`）

- 按 §3.3 定义 `RuleEngine`。内部结构：
  - `rules: Arc<RwLock<HashMap<String, LoadedRule>>>`（仅存放 `status=Ready` 的规则 manifest 用于执行；完整列表含 Invalid 项由 `load_rules` 返回值及 state 缓存维护）。
  - `scopes: Arc<Mutex<HashMap<String, CancellationToken>>>`。
  - 专用线程池：`std::thread` 派生 N=4 个常驻 worker，每 worker 持有独立 `Sandbox`（QuickJS runtime 不可跨线程，**必须一线程一沙箱**），通过 `tokio::sync::mpsc` 分发任务、`oneshot` 回收结果。
- `load_rules`：`futures::future::join_all`，每条规则流程：`parse 文件 → validate_manifest → （带 10s tokio::time::timeout）在线程池 compile_check 四个脚本 → Ready/Invalid`。单条失败/超时仅标记该条，绝不影响其他（验收第 3 条）。内置规则从 `resources/rules/*.json|yaml`（目录不存在则创建空目录并 warn）读取；自定义规则从 `app_config_dir()/custom_rules/` 读取。
- 执行方法（`search/detail/chapters/parse` 共享私有方法 `execute(rule_id, script_field, fn_name, args, token, timeout=15s)`）：
  1. 查表取规则，不存在 → `RuleNotFound`。
  2. `tokio::select!` 监听 `token.cancelled()` → 立即返回 `Cancelled`（同时向 worker 发中断：对正在执行的任务调用 `runtime.set_interrupt_handler` 触发 JS 中断）。
  3. `tokio::time::timeout(15s, ...)` → `Timeout`。
  4. 结果 JSON 反序列化为对应返回类型，失败 → `BadReturn`。
- `new_scope_token` / `cancel_scope`：对 `scopes` map 操作，`new_scope_token` 插入前若已存在旧 token 先 `cancel()`。

**Step 5 — Tauri Commands**（新建 `src-tauri/src/commands/rules.rs`）

- 按 §3.4 实现 9 个 command，全部薄封装到 `RuleEngine`。
- `rules_import`：读取文件 → `RuleManifest::from_str` → `validate_manifest` → `compile_check` → 成功则拷贝到 `app_config_dir()/custom_rules/{uuid}.json` 并注册为 `origin=Custom`；任何一步失败返回结构化 `RuleLoadError`（前端展示具体校验错误）。
- `rules_remove_custom`：仅允许 `origin=Custom`，删文件 + 注册表移除；内置规则返回 `Err("内置规则不可删除")`。
- `rules_export`：序列化全部 manifest 为 pretty JSON 数组写入指定路径。

**Step 6 — 状态注册**（修改 `src-tauri/src/main.rs` 或现有入口）

- 新建 `src-tauri/src/rules/mod.rs`：`pub mod schema; pub mod sandbox; pub mod engine;` 并 re-export。
- 新建 `src-tauri/src/commands/mod.rs`（若不存在）：`pub mod rules;`。
- 在 `main.rs`：`pub struct RuleEngineState(pub RuleEngine);`，`setup` 中构建 `RuleEngine::new(reqwest::Client::builder().user_agent("moeplay/2.0").build()?)` 并 `.manage(RuleEngineState(...))`；`invoke_handler` 追加 `commands::rules::*` 全部 9 个命令。**仅追加注册，不删除现有命令。**

**Step 7 — 前端 API 封装**（新建 `src/lib/api/rules.ts`，若已有 api 目录惯例则沿用）

`invoke` 的 TS 封装：`loadAllRules / search / detail / chapters / parse / cancelScope / importRule / removeCustomRule / exportRules`，导出 §3.1/3.2 对应的 TS 类型（与 Rust `camelCase` serde 输出字段一一对应）。

**Step 8 — 源切换 store**（新建 `src/lib/stores/sourceSwitch.ts`）

按 §3.5 实现 `switchSource` 及 `sourceSwitchState`。要点：
- 标题归一化函数 `normalizeTitle(s: string): string`（trim、toLowerCase、全角转半角、去标点）单独导出便于单测。
- 每次调用入口先 `cancelScope(scope)`，并递增本地 `callSeq` 计数器，回调落地前比对 seq 防止 Rust 层之外的 JS 竞态（双保险）。
- 全程 try/catch：`Cancelled` 静默；其他错误置 `lastError` 并返回 `{status:'failed', ...}`，**绝不向上抛未捕获异常**（防白屏）。

**Step 9 — 源列表组件**（修改/新建 `src/lib/components/SourceList.svelte`）

按 §3.6 实现。导入使用 Tauri 2 `@tauri-apps/plugin-dialog` 的 `open({ filters: [{name:'规则文件', extensions:['json','yaml','yml']}] })`；若项目未安装该插件，在 `package.json` 与 `src-tauri` capabilities（`src-tauri/capabilities/default.json`）中追加 `dialog:default` 权限。导入失败时用项目现有 Toast/通知组件展示 `error.message` + `error.line`。

**Step 10 — 接线现有播放器入口**

在现有播放页（代理需先定位现有「点击源」的处理函数，预计在某个 `*.svelte` 或 store 中）将原来的直接调用改为 `switchSource(ruleId, ctx)`，并根据 `SwitchResult`：
- `ok`：设置播放器 source + `seek(resumeSec)`；
- `fallback`：同上 + Toast `message`；
- `failed`：把错误写入现有播放器错误展示位（任务 3 会做完整降级 UI，本任务仅在现有容器内显示 `lastError` 文本 + 「切换源」按钮，按钮重新打开 SourceList）。

**Step 11 — 旧规则逻辑下线**

找到旧版规则解析入口（前端 eval 或旧 Rust parser），保留文件，将调用点注释并加 `// DEPRECATED(task-1): 由 RuleEngine 接管`，确保无残留双跑逻辑。

## 5. 依赖的外部接口

本任务无上游子任务依赖（`depends_on: []`）。对下游任务暴露的契约：

| 消费方 | 接口 | 承诺 |
|---|---|---|
| 任务 2（内置源/热更新） | `RuleManifest` schema、`RuleEngine::load_rules`、`resources/rules/` 目录约定 | 热更新只需写入新规则文件后调用 `rules_load_all`；schema 字段不再变更（只增不删） |
| 任务 3（播放器修复） | `switchSource()` → `SwitchResult`、`RuleExecError` 的 `kind` tag | 错误结构化透传稳定，`status:'failed'` 时必有 `message` |
| 任务 5/6（历史） | `SwitchContext.contentId/chapterIndex` 语义 | `chapterIndex` 恒为 1-based 单页原子单位 |

外部第三方依赖假设：kazumi 规则字段名以 kazumi 官方 schema 为准，若实际字段名与 §3.1 有出入（如 `contentType` vs `type`），在 `schema.rs` 中通过 `#[serde(alias = "...")]` 兼容，**不得改动对外暴露的 camelCase 输出**。

## 6. 测试要求

### 6.1 Rust 单元/集成测试（`src-tauri/src/rules/tests.rs` 及 `src-tauri/tests/`）

1. `schema_validate_ok`：合法 manifest 通过校验。
2. `schema_missing_field_rejected`：缺 `parse` 字段 → 拒绝，`message` 含 "parse"。
3. `schema_yaml_and_json_both_parse`：同一规则 JSON/YAML 双格式加载结果一致。
4. `compile_syntax_error_has_line`：注入语法错误脚本（第 3 行 `let x = ;`）→ `status=Invalid`，`error.line == Some(3)`，`phase == "compile"`。
5. `load_rules_partial_failure`：构造 20 条规则，其中 5 条脚本内死循环（或 mock sandbox 延时 11s）→ 返回 20 条，5 条 `Invalid`，15 条 `Ready`，总耗时 < 15s（并行）。
6. `exec_timeout`：脚本 `while(true){}` → `RuleExecError::Timeout`（≤16s 返回）。
7. `exec_script_error_propagates`：脚本 `throw new Error("boom")` → `ScriptError`，message 含 "boom"。
8. `exec_cancelled`：parse 进行中 `cancel_scope` → 返回 `Cancelled`，且 sandbox interrupt handler 生效（JS 中断，worker 不被永久占用：随后同 worker 再执行正常脚本成功）。
9. `scope_token_replacement`：连续 `new_scope_token("play:x")` 两次 → 第一个 token `is_cancelled() == true`。
10. `sandbox_no_fs_access`：脚本尝试 `std.open` / `os.system` → 抛错（符号不存在），证明沙箱无 std/os。
11. `fetch_bridge_audit`：mock http server（`wiremock` 或 `httpmock`），脚本内 `fetch` → 收到正确响应；UA 为 `moeplay/2.0`。
12. `bad_return_shape`：脚本返回字符串而非数组 → `BadReturn`。
13. `import_export_roundtrip`：导入合法规则 → 出现在列表且 `origin=Custom`；导出后再导入条数一致；删除自定义规则后列表移除、文件删除；删除内置规则报错。
14. `kazumi_compat_fixture`：将一份真实 kazumi 风格规则 fixture 放入 `tests/fixtures/`，加载成功。

### 6.2 前端测试（`src/lib/stores/sourceSwitch.test.ts`，vitest + mock invoke）

15. `switch_ok_resume`：源 B 含第 5 集 → `status='ok'`，`resumeSec=750`，`targetChapter.index=5`。
16. `switch_fallback_to_latest`：源 B 仅 3 集 → `status='fallback'`，`targetChapter.index=3`，`resumeSec=0`，message 含「第 5 集」「最新一集」。
17. `switch_race_last_wins`：连续调用 3 次（mock 前两次延迟返回）→ 前两次结果为 `Cancelled` 静默丢弃，store 只反映第三次结果，`cancelScope` 被调用 3 次。
18. `switch_never_throws`：mock `rules_parse` reject → `switchSource` resolve `{status:'failed'}`，不抛异常，`sourceSwitchState.lastError` 已设置。
19. `normalizeTitle`：全角/大小写/空白变体归一化相等。
20. `SourceList` 组件测试（`@testing-library/svelte`）：invalid 源置灰且不可点击；custom 源显示徽标与删除按钮；导入失败展示错误行号。

### 6.3 手动验收（对照 acceptance 清单逐条执行）

- 故意放入语法错误规则 → 源列表置灰 + 日志含行号。
- 解析抛异常 → 提示含「切换源」按钮，应用不白屏。
- 源 A 第 5 集 12:30 → 切源 B → 12:30±10s 续播；源 B 仅 3 集 → 第 3 集 + Toast。
- 快速连切 3 次 → 仅最后一次生效（验证最终播放 URL 属于最后选择的源）。

## 7. 完成定义 Definition of Done

- [ ] §6 全部自动化测试（1~20）通过：`cargo test`（src-tauri）与 `pnpm/npm test`（前端）全绿。
- [ ] §3 全部 Tauri commands 注册并可通过前端 invoke 调用；`RuleEngine` 单条规则异常不影响其他规则（含测试证据）。
- [ ] 沙箱验证：规则脚本无文件系统/进程访问能力；网络请求全部经由 reqwest 桥接且有审计日志。
- [ ] 源切换四项验收（ok 续播 ±10s / fallback 到最新集 + Toast / 竞态仅末次生效 / 错误结构化不白屏）手动通过并录屏或留测试记录。
- [ ] 导入/导出/删除自定义规则链路可用，非法规则给出含字段名/行号的具体错误；内置与自定义规则 UI 可区分。
- [ ] 旧规则执行调用点已注释下线，无双重执行。
- [ ] 三端至少本机 + CI `cargo check` 通过（若触发 R3 风险——rquickjs 三端构建失败——立即在 PR 中记录并评估切换 boa_engine，不静默阻塞）。
- [ ] 代码 `cargo clippy` 无新增 warning；新增公开 API 均有 doc 注释。

## 8. 禁止修改清单

- ❌ `src-tauri/Cargo.lock` 以外的**发布/CI 相关文件**：`.github/workflows/release.yml`（本期明确不动主 CI 流程）、`src-tauri/tauri.conf.json` 的 `version`/`bundle` 产物配置（capabilities 追加 dialog 权限除外）。
- ❌ 历史数据存储相关代码与数据文件（属任务 4：SQLite 化与迁移），本任务不得改动历史记录读写逻辑。
- ❌ 播放器内核代码（视频组件本体、画质切换逻辑属任务 3），本任务仅允许在播放页「源点击处理函数」处做 §4 Step 10 的最小接线。
- ❌ 任何 `resources/rules/` 下既有规则文件的**删除**（可新增 fixture，不允许清空重写，内置源重建属任务 2）。
- ❌ `package.json` 中 Svelte / Tauri 主版本号升级（仅允许追加 `@tauri-apps/plugin-dialog` 及测试依赖）。
- ❌ 旧规则解析文件本体（仅允许在调用点加 `DEPRECATED` 注释下线，不得删除文件）。