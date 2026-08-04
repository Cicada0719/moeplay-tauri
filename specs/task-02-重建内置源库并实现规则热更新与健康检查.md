# 子任务 2 开发规格说明：重建内置源库并实现规则热更新与健康检查

## 1. 任务目标

参考 kazumi/animeko 的规则 schema 重建 ≥13 个开箱即用的内置源（番剧 ≥6、漫画 ≥4、小说 ≥3），规则文件与代码完全解耦。实现带签名校验的规则包热更新链路（远端拉取 → 校验 → 原子替换 → 失败回退本地缓存），并实现源健康检查机制（应用内探测 + 每日 CI 探测），健康状态在源列表 UI 中可见并影响排序。

---

## 2. 上下文与约束

### 2.1 关联 PRD 条目

- **FR-03 内置源重建（P0）**：规则数量、字段完整性、存放位置、健康检查通过率。
- **FR-04 规则热更新与源健康状态展示（P1）**：版本比较、签名校验、失败回退、异常源标识与沉底排序。
- 依赖 **FR-01（子任务 1）** 的规则引擎沙箱执行能力。
- 非功能约束：§4.2 安全（ed25519 签名 / HTTPS + SHA-256、证书校验不可关闭）；§4.1 性能（30 条规则加载 ≤500ms，不阻塞首屏）；§1.3 成功指标（CI 健康检查通过率 ≥80%）。

### 2.2 技术栈与现有结构

- Tauri 2 + Svelte（不变）。Rust 侧网络统一走 `reqwest`（rustls）。
- 子任务 1 已在 `src-tauri/src/rules/` 下建立规则引擎模块（`engine.rs`、`schema.rs`、`mod.rs`），本任务**新增** `update.rs`、`health.rs`，**不得修改** `engine.rs` 的公共 API。
- 新增依赖（加入 `src-tauri/Cargo.toml`）：
  - `ed25519-dalek = "2"`（签名校验）
  - `sha2 = "0.10"`（SHA-256 校验和）
  - `semver = "1"`（版本比较）
  - `base64 = "0.22"`（签名编码）
  - `wiremock = "0.6"`（dev-dependencies，测试用 mock HTTP）
  - 复用已有：`reqwest`、`tokio`、`serde`、`serde_json`、`thiserror`（若缺则补）

### 2.3 关键假设

- 远端规则仓库地址默认为 `https://raw.githubusercontent.com/Cicada0719/moeplay-tauri-rules/main/`（独立规则仓库，假设已存在或本任务附带创建指引），可在设置中修改，常量定义于 `update.rs::DEFAULT_REMOTE_BASE`。
- 应用数据目录通过 `tauri::path::PathResolver` 的 `app_data_dir()` 获取，记为 `$APPDATA`。
- 许可证风险（PRD R2）：内置规则**自研**，仅借鉴 kazumi schema 结构，不复制其规则内容；每条规则头部附 `licenseNote` 字段。

---

## 3. 输入 / 输出

### 3.1 规则文件 Schema（`resources/rules/*.json`，每源一个文件）

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `name` | string | ✅ | 源显示名，如 `"樱花动漫"` |
| `id` | string | ✅ | 全局唯一 slug，如 `"yhdm"`，仅允许 `[a-z0-9-]` |
| `version` | string | ✅ | semver，如 `"1.2.0"` |
| `contentType` | string | ✅ | `"anime" \| "manga" \| "novel"` |
| `baseUrl` | string(URL) | ✅ | 站点根地址 |
| `lang` | string | ✅ | 如 `"zh-CN"` |
| `nsfw` | bool | ✅ | NSFW 标记 |
| `probeKeyword` | string | ✅ | 健康检查搜索关键词，如 `"进击的巨人"` |
| `search` | object | ✅ | `{ "script": "function search(kw){...}", "timeoutMs": 10000 }` |
| `detail` | object | ✅ | 同上结构 |
| `chapter` | object | ✅ | 同上结构 |
| `parse` | object | ✅ | 同上结构 |
| `licenseNote` | string | ⬜ | 自研声明 |
| `extraHeaders` | object | ⬜ | 该源网络请求附加头（如 Referer，防盗链用） |

> `search/detail/chapter/parse.script` 为 JS 函数字符串，由子任务 1 的 QuickJS 沙箱执行。脚本签名约定：
> - `search(keyword) → Array<{ id, title, cover, extra }>`
> - `detail(id) → { title, cover, desc, chapters: [{ id, title }] }`
> - `chapter(id) → { pages?: string[], content?: string }`（漫画返回图片 URL 数组，小说返回正文）
> - `parse(id) → { url, headers? }`（番剧播放地址解析）

**完整示例**（`resources/rules/anime/example.json`，编码代理按此模板产出其余规则）：

```json
{
  "name": "示例动漫源",
  "id": "example-anime",
  "version": "1.0.0",
  "contentType": "anime",
  "baseUrl": "https://example-anime.com",
  "lang": "zh-CN",
  "nsfw": false,
  "probeKeyword": "进击的巨人",
  "licenseNote": "Self-authored rule, site structure reference only.",
  "search": {
    "timeoutMs": 10000,
    "script": "function search(kw) { var res = http.get(baseUrl + '/search?q=' + encodeURIComponent(kw)); var items = []; res.querySelectorAll('.result-item').forEach(function(el){ items.push({ id: el.attr('data-id'), title: el.querySelector('.title').text(), cover: el.querySelector('img').attr('src'), extra: '' }); }); return items; }"
  },
  "detail": { "timeoutMs": 10000, "script": "function detail(id) { /* ... */ }" },
  "chapter": { "timeoutMs": 10000, "script": "function chapter(id) { /* ... */ }" },
  "parse": { "timeoutMs": 10000, "script": "function parse(id) { /* ... */ }" }
}
```

### 3.2 规则清单与包格式

**`resources/rules/manifest.json`**（本地清单，构建时随资源打包）：

```json
{
  "packageVersion": "2024.06.1",
  "publishedAt": 1718000000,
  "rules": [
    {
      "id": "example-anime",
      "path": "anime/example.json",
      "sha256": "<hex>",
      "name": "示例动漫源",
      "version": "1.0.0",
      "contentType": "anime",
      "lang": "zh-CN",
      "nsfw": false
    }
  ]
}
```

**远端包 `rules-package.json`**（位于 `{REMOTE_BASE}/rules-package.json`）：

```json
{
  "manifest": { "...": "同本地 manifest 结构" },
  "manifestSha256": "<hex of canonical manifest JSON bytes>",
  "signature": "<base64 ed25519 signature over manifestSha256 bytes>"
}
```

远端各规则文件位于 `{REMOTE_BASE}/rules/<path>`。

### 3.3 Rust 数据结构（`src-tauri/src/rules/update.rs` / `health.rs`）

```rust
// update.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFileEntry {
    pub id: String,
    pub path: String,
    pub sha256: String,
    pub name: String,
    pub version: String,
    pub content_type: String, // "anime" | "manga" | "novel"
    pub lang: String,
    pub nsfw: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleManifest {
    pub package_version: String,
    pub published_at: i64,
    pub rules: Vec<RuleFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePackage {
    pub manifest: RuleManifest,
    pub manifest_sha256: String,
    pub signature: String, // base64 ed25519
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesMetaInfo {
    pub package_version: String,
    pub source: RuleSource,        // Bundled | RemoteCache
    pub updated_at: i64,           // 上次成功热更新时间，bundled 时为打包时间
    pub last_check_at: Option<i64>,
    pub rule_count: usize,
    pub remote_base: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOutcome {
    pub status: UpdateStatus, // Updated | AlreadyLatest | FallbackCached { reason }
    pub from_version: Option<String>,
    pub to_version: String,
    pub updated_rules: usize,
}

// health.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus { Healthy, Degraded, Abnormal, Unknown }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthInfo {
    pub source_id: String,
    pub status: HealthStatus,
    pub consecutive_failures: u32,
    pub last_checked_at: Option<i64>,
    pub last_latency_ms: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthProbeResult {
    pub source_id: String,
    pub ok: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}
```

### 3.4 Tauri Commands（在 `src-tauri/src/lib.rs` 的 `invoke_handler` 注册）

| Command | 签名 | 说明 |
|---|---|---|
| `rules_get_meta` | `() -> Result<RulesMetaInfo, String>` | 设置页展示规则包版本/来源/更新时间 |
| `rules_check_and_update` | `(force: bool) -> Result<UpdateOutcome, String>` | 启动时自动调用 + 设置页"检查更新"按钮；`force=true` 跳过 24h 节流 |
| `rules_probe_health` | `(source_ids: Option<Vec<String>>) -> Result<Vec<HealthProbeResult>, String>` | `None` 表示全量探测；并发执行，单源超时 10s |
| `rules_get_health` | `() -> Result<Vec<SourceHealthInfo>, String>` | 源列表页读取持久化的健康状态 |

### 3.5 前端接口（`src/lib/api/rules.ts`，新增）

```ts
export interface SourceHealthInfo {
  sourceId: string;
  status: 'Healthy' | 'Degraded' | 'Abnormal' | 'Unknown';
  consecutiveFailures: number;
  lastCheckedAt: number | null;
  lastLatencyMs: number | null;
  lastError: string | null;
}
export const getRulesMeta = () => invoke<RulesMetaInfo>('rules_get_meta');
export const checkAndUpdateRules = (force = false) => invoke<UpdateOutcome>('rules_check_and_update', { force });
export const probeHealth = (ids?: string[]) => invoke<HealthProbeResult[]>('rules_probe_health', { sourceIds: ids ?? null });
export const getHealth = () => invoke<SourceHealthInfo[]>('rules_get_health');
```

---

## 4. 实现步骤

### Step 1：编写内置规则文件（`resources/rules/`）

1. 创建目录结构：`resources/rules/anime/`、`resources/rules/manga/`、`resources/rules/novel/`。
2. 按 §3.1 schema 编写规则文件，**数量底线**：anime 6 个、manga 4 个、novel 3 个。候选站点由实施者按"当前真实可访问 + 搜索接口稳定"原则选取并逐一人工验证；每个源的 `probeKeyword` 必须能在该站搜出 ≥1 条结果。
3. 每条规则四段脚本（search/detail/chapter/parse）调用子任务 1 沙箱注入的宿主 API（`http.get/http.post`、DOM 选择器，具体以子任务 1 产出为准，见 §5），禁止在脚本内做任何沙箱外操作。
4. 每个 `script` 字段脚本必须包含 `try/catch` 兜底，失败时返回空数组/抛出带站点上下文的 Error（便于健康检查归因）。
5. 在 `src-tauri/tauri.conf.json` 的 `bundle.resources` 中注册 `resources/rules/**`，确保打包产物携带。

### Step 2：生成并校验本地 manifest（构建脚本）

1. 新增 `src-tauri/build-rules-manifest.rs`（或在现有 `build.rs` 中追加函数 `generate_rules_manifest()`）：
   - 遍历 `resources/rules/{anime,manga,novel}/*.json`；
   - 对每条规则做 schema 静态校验（§3.1 必填字段缺失则 **build 失败**，防止残缺规则进包）；
   - 计算每个文件 SHA-256，输出 `resources/rules/manifest.json`（覆盖写）。
2. 在 `build.rs` `main()` 中调用该函数，并 `println!("cargo:rerun-if-changed=../../resources/rules")`。

### Step 3：规则包热更新（`src-tauri/src/rules/update.rs`）

实现以下函数（全部 `pub(crate)`，错误类型 `UpdateError` 用 `thiserror`）：

1. **常量**：
   ```rust
   pub const DEFAULT_REMOTE_BASE: &str = "https://raw.githubusercontent.com/Cicada0719/moeplay-tauri-rules/main/";
   const EMBEDDED_PUBKEY_B64: &str = "<构建期注入的 ed25519 公钥>";
   const MIN_CHECK_INTERVAL_SECS: i64 = 24 * 3600; // FR-04 每 24h
   ```
2. **`pub async fn fetch_remote_package(client: &reqwest::Client, base: &str) -> Result<RemotePackage, UpdateError>`**
   - GET `{base}/rules-package.json`，超时 15s，rustls 证书校验开启（禁止 `danger_accept_invalid_certs`）；
   - 反序列化为 `RemotePackage`。
3. **`pub fn verify_package(pkg: &RemotePackage) -> Result<(), UpdateError>`**
   - 将 `pkg.manifest` 以 canonical JSON（key 排序、无空白，用 `serde_json::to_string` 配合 `BTreeMap` 或约定序列化顺序）重新序列化，计算 SHA-256，与 `manifest_sha256` 比较；不等 → `UpdateError::ChecksumMismatch`；
   - base64 解码 `signature`，用 `ed25519_dalek::VerifyingKey`（`EMBEDDED_PUBKEY_B64`）对 `manifest_sha256` 的字节做 `verify()`；失败 → `UpdateError::InvalidSignature`。
4. **`pub fn is_newer(remote: &str, local: &str) -> bool`**：用 `semver` 或日期戳格式 `YYYY.M.N` 比较（manifest 采用后者时写自定义比较器，二选一并在代码注释固定）。
5. **`pub async fn apply_update(app: &AppHandle, pkg: &RemotePackage) -> Result<usize, UpdateError>`**：
   - 对每个 `manifest.rules` 条目：GET `{base}/rules/{path}`，校验文件 SHA-256 与条目一致；
   - 全部下载成功后写入 `$APPDATA/rules-cache/.tmp-<ts>/`，再原子 `rename` 到 `$APPDATA/rules-cache/current/`（**先下载全量再切换**，部分失败不动现有缓存）；
   - 写入 `$APPDATA/rules-cache/manifest.json` 与 `$APPDATA/rules-meta.json`（`{ packageVersion, updatedAt }`）；
   - 调用子任务 1 的 `RuleEngine::reload_all(rules_dir)` 热重载（见 §5）；返回更新条数。
6. **`pub async fn check_and_update(app: &AppHandle, force: bool) -> Result<UpdateOutcome, String>`**（command 入口）：
   - 读取 `$APPDATA/rules-meta.json`；非 force 且距上次检查 < 24h → 直接返回 `AlreadyLatest`；
   - `fetch_remote_package` → 任何网络错误：记 `log::warn!`，返回 `FallbackCached { reason }`（不报错给前端）；
   - `verify_package` 失败：记 `log::warn!`（含规则包版本），返回 `FallbackCached`；
   - `is_newer` 为否 → `AlreadyLatest`；是 → `apply_update` → `Updated`；
   - 更新 `lastCheckAt`。
7. **`pub fn resolve_rules_dir(app: &AppHandle) -> PathBuf`**：若 `$APPDATA/rules-cache/current/manifest.json` 存在且可解析 → 用缓存目录；否则用打包资源目录 `resources/rules/`。应用启动加载规则时（子任务 1 的加载入口）改为调用此函数——**这是与子任务 1 的唯一集成点**。
8. 在 `src-tauri/src/lib.rs`：`rules/mod.rs` 加 `pub mod update; pub mod health;`，`invoke_handler` 注册 4 个 command，`setup()` 中 `tauri::async_runtime::spawn` 调用 `check_and_update(force=false)`（不阻塞首屏）。

### Step 4：健康检查（`src-tauri/src/rules/health.rs`）

1. **持久化**：`$APPDATA/rules-health.json`，结构 `HashMap<String /*source_id*/, HealthRecord>`，`HealthRecord { results: VecDeque<ProbeRecord> (最多保留 10 条), consecutive_failures: u32, last_checked_at, last_latency_ms, last_error }`。提供 `load_health(path)` / `save_health(path, &map)`。
2. **`pub async fn probe_one(engine: &RuleEngine, rule_id: &str, keyword: &str) -> HealthProbeResult`**：
   - 调用子任务 1 引擎接口执行 `search` 动作（带 10s 超时，由引擎保证）；
   - 成功判定：无异常且返回数组长度 ≥ 1；
   - 记录 `latency_ms`。
3. **`pub async fn probe_all(app, source_ids: Option<Vec<String>>) -> Vec<HealthProbeResult>`**：
   - 读取当前规则目录，筛出目标源（含其 `probeKeyword`）；
   - `futures::future::join_all` 并发探测（并发上限 8，用 `tokio::sync::Semaphore`）；
   - 逐条更新 `HealthRecord`：成功清零 `consecutive_failures`，失败 +1；
   - 落盘 `rules-health.json`。
4. **状态推导** `pub fn derive_status(rec: Option<&HealthRecord>) -> HealthStatus`：
   - 无记录 → `Unknown`；
   - `consecutive_failures >= 3` → `Abnormal`；
   - `consecutive_failures >= 1` → `Degraded`；
   - 否则 `Healthy`。
5. **`rules_get_health` command**：加载 health 文件，对当前规则列表逐源生成 `SourceHealthInfo`（用 `derive_status`），返回。
6. **触发时机**：`setup()` 中启动后延迟 30s 执行一次全量探测（spawn、不阻塞）；之后每 6 小时一次（`tokio::time::interval` 循环任务）。源列表页打开时不自动探测，只读缓存状态（性能约束）。

### Step 5：健康检查 CLI（供 CI 使用）

1. 新增 `src-tauri/src/bin/rules_health.rs`（在 `Cargo.toml` 注册 `[[bin]] name = "rules-health" path = "src/bin/rules_health.rs"`）：
   - 无 GUI 依赖：直接构造 `RuleEngine`（子任务 1 引擎须支持 headless 构造，见 §5），加载 `resources/rules/`；
   - 对每条规则执行 `probe_one`；
   - 输出 JSON 报告到 `--out <path>`：`{ "total": N, "passed": M, "passRate": 0.xx, "failures": [{ "id", "error" }] }`；
   - 进程退出码：passRate < 0.8 → exit 1；否则 exit 0。
2. 该 bin 不链接 Tauri 运行时，仅复用 `rules` 模块中不依赖 `AppHandle` 的纯逻辑（`health.rs` 的 probe 逻辑需拆出 AppHandle 无关的核心函数 `probe_with_engine(engine, rule_json, keyword)`）。

### Step 6：CI 工作流（`.github/workflows/rules-health.yml`）

```yaml
name: rules-health
on:
  schedule: [{ cron: "0 2 * * *" }]  # 每日 UTC 02:00
  workflow_dispatch:
jobs:
  probe:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run health probe
        working-directory: src-tauri
        run: cargo run --release --bin rules-health -- --rules-dir ../resources/rules --out ../health-report.json
      - uses: actions/upload-artifact@v4
        with: { name: health-report, path: health-report.json }
      - name: Alert on low pass rate
        if: failure()
        uses: actions/github-script@v7
        with:
          script: |
            // 查找标题含 "[rules-health]" 的 open issue，有则评论，无则新建
            // 正文含 passRate、失败源列表（从 health-report.json 读取）
```

- 退出码 1 即 job 失败 → 产生告警 issue（FR-03"失败产生告警"）。不改动 `release.yml`。

### Step 7：前端组件与集成

1. **`src/lib/api/rules.ts`**：按 §3.5 实现。
2. **`src/lib/components/SourceHealth.svelte`**（新增，纯展示组件）：
   - Props：`status: HealthStatus`、`latencyMs?: number | null`、`lastError?: string | null`、`size?: 'sm' | 'md'`；
   - 渲染徽标：`Healthy` 绿点"可用"、`Degraded` 黄点"波动"、`Abnormal` 红点"异常"、`Unknown` 灰点"未知"；
   - `Abnormal` 时徽标 `title` 提示 `lastError`（tooltip 用原生 `title` 即可）。
3. **排序工具 `src/lib/utils/sourceSort.ts`**（纯函数，便于单测）：
   ```ts
   export function sortByHealth<T extends { id: string }>(sources: T[], health: SourceHealthInfo[]): T[]
   ```
   规则：状态权重 `Healthy=0 < Unknown=1 < Degraded=2 < Abnormal=3`，权重升序；同权重按 `lastLatencyMs` 升序（无延迟数据的排后）；**Abnormal 自然沉底**（FR-04）。
4. **源列表组件集成**（找到现有源列表组件文件，如 `src/lib/components/SourceList.svelte`；若名称不同以实际为准）：`onMount` 调 `getHealth()`，每个源条目旁挂 `<SourceHealth ... />`，列表经 `sortByHealth` 排序后渲染。异常源条目额外置灰 30% 透明度但**仍可点击**（不阻止用户手动尝试）。
5. **设置页规则管理区块**（在现有设置页追加 section）：
   - 展示 `getRulesMeta()` 的 `packageVersion`、`source`（内置/远端缓存）、`updatedAt`（本地化时间）；
   - "检查更新"按钮 → `checkAndUpdateRules(true)`，结果 Toast：`Updated` → "规则已更新至 vX（n 条）"；`AlreadyLatest` → "已是最新"；`FallbackCached` → "更新失败，已继续使用本地规则"（用户无感知中断，符合 FR-04）；
   - "立即健康检查"按钮 → `probeHealth()`，进行中按钮 loading，完成后刷新列表。
6. **应用启动**：`src/routes/+layout.ts`（或现有启动逻辑处）`invoke('rules_check_and_update', { force: false })`，`catch` 静默忽略。

### Step 8：日志

- `update.rs`：`log::info!` 记录更新成功（from→to 版本）；`log::warn!` 记录签名/校验/下载失败原因。
- `health.rs`：探测失败记 `log::debug!`（含 source_id 与 error），状态跃迁（如 Healthy→Abnormal）记 `log::info!`。

---

## 5. 依赖的外部接口（子任务 1 产出，接口假设）

> 若子任务 1 实际接口与此处假设有出入，**以子任务 1 实际导出为准做适配，禁止反向修改子任务 1 文件**；在 `rules/mod.rs` 内写适配层。

| 假设接口 | 用途 | 对接方式 |
|---|---|---|
| `RuleEngine::load_from_dir(dir: &Path) -> LoadReport` | 加载规则目录，返回每规则校验结果（缺字段规则被拒并记原因） | `update.rs::resolve_rules_dir` 选定目录后调用；Step 2 的构建期校验复用同一 schema 校验函数 |
| `RuleEngine::reload_all(dir: &Path) -> Result<LoadReport>` | 热更新后重载 | `apply_update` 成功后调用 |
| `RuleEngine::execute(rule_id: &str, action: RuleAction, arg: Value) -> Result<Value, RuleError>`（内部带 10s 超时与取消） | 健康探测执行 `search` | `health.rs::probe_one` |
| `RuleEngine::new_headless(rules_dir) -> Result<Self>` | CI bin 无 AppHandle 构造 | Step 5 的 `rules-health` bin |
| 沙箱宿主 API：`http.get/post`、DOM 查询 | 规则脚本内使用 | Step 1 编写脚本时对齐子任务 1 文档 |

另依赖子任务 1 提供的"当前启用源列表"读取方式（`RuleEngine::list_rules() -> Vec<RuleMeta>`，含 `id/name/contentType`），用于 `rules_get_health` 关联。

---

## 6. 测试要求

### 6.1 Rust 单元/集成测试（`src-tauri/src/rules/update.rs`、`health.rs` 的 `#[cfg(test)]` 及 `tests/`）

**update.rs（配合 `wiremock` mock HTTP server）：**

1. ✅ 正常路径：mock 远端返回合法包 + 合法签名 → `Updated`，缓存目录文件齐全、sha256 正确、`rules-meta.json` 更新。
2. ✅ 版本相同/更低 → `AlreadyLatest`，缓存目录未被触碰。
3. ✅ 签名被篡改（改一字节）→ `FallbackCached{reason}`，原缓存完好，warn 日志产生。
4. ✅ `manifestSha256` 与 manifest 不匹配 → `FallbackCached`。
5. ✅ 单个规则文件 sha256 不符 → 整体放弃更新，缓存目录原子性验证（`current/` 内容与更新前逐字节一致，`.tmp-*` 已清理）。
6. ✅ 网络超时/500 → `FallbackCached`，不返回 Err 给前端。
7. ✅ 24h 节流：`force=false` 且 `lastCheckAt` 在 24h 内 → 不发起 HTTP 请求（用 mock server 请求计数断言）。
8. ✅ `resolve_rules_dir`：无缓存 → bundled 目录；有损坏 manifest 的缓存 → 回退 bundled。
9. ✅ 密钥固化测试：用测试密钥对签名 + 验证通过，防止"验证函数恒真"的假实现。

**health.rs：**

10. ✅ `derive_status` 全分支：无记录→Unknown；失败 1/2 次→Degraded；3 次→Abnormal；成功后清零回 Healthy。
11. ✅ 探测成功但返回空数组 → 记失败。
12. ✅ mock 引擎单源超时 → 该源记失败，其余源结果不受影响（并发隔离，对应 FR-01 超时语义）。
13. ✅ 结果队列只保留最近 10 条；`rules-health.json` 损坏时降级为空记录不 panic。
14. ✅ 并发上限验证：探测 20 源时在飞请求 ≤8。

**构建期校验：**

15. ✅ 构造缺 `parse` 字段的规则放临时目录，`generate_rules_manifest` 报错；完整规则集合（真实 `resources/rules/`）能通过且 manifest 中条目数 ≥13、anime≥6/manga≥4/novel≥3（写一个测试直接断言数量底线，防止后续删源失守）。

### 6.2 前端测试（Vitest）

16. ✅ `sortByHealth`：混合状态输入 → Abnormal 沉底、Healthy 置顶、同状态按延迟排序；空 health 数组 → 全部 Unknown 保持原序稳定。
17. ✅ `SourceHealth.svelte` 快照/渲染测试：四种状态各渲染正确文案与样式类。

### 6.3 手动/集成验证（写进 PR 描述 checklist）

18. ✅ 断网启动应用 → 源列表正常（本地规则），设置页显示内置包版本，无错误弹窗。
19. ✅ 在 mock/真实远端发布高一版本规则包 → 启动后静默更新，设置页版本变化。
20. ✅ 设置页点"立即健康检查"，人为屏蔽某源域名（改 hosts）3 次 → 该源显示"异常"并沉底。
21. ✅ `cargo run --bin rules-health` 本地跑通，输出报告 JSON，通过率 <80% 时 exit 1。

---

## 7. 完成定义（Definition of Done）

- [ ] `resources/rules/` 下 anime ≥6、manga ≥4、novel ≥3，每条规则含全部必填字段且 probeKeyword 实测可搜出结果；`licenseNote` 已填写。
- [ ] `build.rs` 构建期 schema 校验生效：缺字段规则导致 build 失败；`manifest.json` 自动生成且 sha256 正确。
- [ ] `update.rs` 四段链路（拉取→校验→原子应用→回退）实现，签名使用 ed25519，网络层强制证书校验；24h 节流与 `force` 生效。
- [ ] `health.rs` 探测、状态推导、持久化实现；启动后自动探测 + 6h 周期任务运行。
- [ ] 4 个 Tauri command 注册并可被前端调用；`resolve_rules_dir` 集成进规则加载入口。
- [ ] `rules-health` headless bin 实现，`.github/workflows/rules-health.yml` 上线（手动 `workflow_dispatch` 触发验证通过一次），通过率 <80% 时 job 失败并自动建/更新告警 issue。
- [ ] `SourceHealth.svelte` 与 `sortByHealth` 接入源列表：异常源标识 + 沉底；设置页展示规则包版本/更新时间/来源，含"检查更新""立即健康检查"按钮。
- [ ] §6 全部自动化测试通过（`cargo test`、`pnpm test`/等价命令），CI 绿。
- [ ] 实测：当前内置源集合本地探测通过率 ≥80%（若个别站点临时不可用，替换源直至达标）。
- [ ] 热更新端到端演练通过（§6.3 第 18~20 项）并附截图/日志于 PR。

---

## 8. 禁止修改清单

| 路径 | 原因 |
|---|---|
| `src-tauri/src/rules/engine.rs`、`schema.rs`（子任务 1 产出） | 引擎核心属于子任务 1；如需集成，仅在 `rules/mod.rs` 写适配层 |
| `.github/workflows/release.yml` | PRD §6.3 明确本期不改动 CI 主流程 |
| 历史记录/数据库相关代码（子任务 4 的领域，如 `src-tauri/src/history/`、`db.rs`） | 健康数据用独立 JSON 文件，禁止引入 SQLite 依赖耦合 |
| 播放器组件与控制栏逻辑（子任务 3 领域） | 本任务仅向源列表组件追加徽标与排序，不动播放链路 |
| `src-tauri/Cargo.toml` 中已有依赖的版本号 | 仅允许**新增** §2.2 列出的依赖；升级既有依赖需另行评审 |
| 任何第三方站点的规则内容直接复制自 kazumi/animeko 仓库 | 许可证风险（PRD R2），仅可参考 schema 结构自研 |
| 凭据/密钥存储相关代码 | 规则包公钥以常量内嵌于 `update.rs`，禁止引入 keyring 或把私钥提交进仓库 |