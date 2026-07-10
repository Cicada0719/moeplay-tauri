# SPEC-02：游戏库与记录仪表盘优化

## 1. 现状与问题证据

- 游戏库主入口为 `home`，记录主入口为 `records`；二者产品上并列，但数据链仍不完全统一。
- 现有前端职责分散在 `SwitchHome.svelte`、`GameGrid.svelte`、`GameCard.svelte`、`GameDetailPage.svelte`、`gameLibrary.svelte.ts`、`games.svelte.ts`、`gameSelection.svelte.ts`。
- `PlayRecordsDashboard.svelte` 已达 1601 行，承担查询、聚合、状态、图表与布局，扩展成本高。
- `PlatformImportPage.svelte` 1288 行，平台发现、预览、冲突和 UI 混合。
- 后端 `commands/platform.rs` 2005 行，`db_sqlite.rs` 2014 行；命令层和领域逻辑边界不够清晰。
- 当前 SQLite 以 `games.data_json` 保真并投影少数字段，适合兼容，但缺少统一活动事件、字段来源和后台任务表。
- 已有 Steam/Epic/模拟器/本地导入、刮削、标签、合集、启动与游玩时长能力；本轮重点是可靠闭环、可解释数据和批量效率，而不是再加孤立按钮。


### 1.1 文件级审计证据（2026-07-10）

| 证据 | 当前行为 / 风险 | 对本 SPEC 的约束 |
|---|---|---|
| `src/lib/components/GameGrid.svelte:31-40,117-128` | 已实现按行虚拟化，而非“尚未虚拟化”；但依赖固定 `rowHeight`，卡片 mount 时仍执行 GSAP。 | `LibraryGrid` 任务定义为**校准、无障碍与大库压测**，不重复造轮子；验收 DOM 卡片数与帧耗时。 |
| `src/lib/stores/gameLibrary.svelte.ts:41-53,126-203` | `SmartCollection.filters` 声明了 `tags/tagMode/installed/hasPlayed`，过滤实现却未消费；合集又仅存 `localStorage`（57-66）。 | 先补齐现有筛选语义与兼容测试，再迁移持久化；旧合集导入必须幂等。 |
| `src/lib/stores/gameLibrary.svelte.ts:339-345` + `GameDetailPage.svelte:107-111` | store 吞掉启动异常，仅写 `loadError`；详情页仍提示“正在启动”。 | `launch` 必须返回显式 `LaunchOutcome` 或抛错，成功通知只能在命令成功后出现。 |
| `src-tauri/src/db_sqlite.rs:359-429` | 列表返回全部 `data_json`，全文搜索也是“全量反序列化后内存过滤”；现有投影索引只覆盖 `sort_name/game_type`。 | 5000 库预算不能只靠前端虚拟化，必须增加分页/投影查询与可解释查询计划。 |
| `src-tauri/src/commands/import.rs:69-147` | 本地预览仅有 `is_duplicate: bool`；批量导入把所有错误折叠成 skipped，无法展示冲突原因或重试。 | `ImportCandidate` 必须有稳定 ID、动作、原因、字段 diff；apply 返回逐项结果。 |
| `src-tauri/src/archive.rs:490-503` | 名称完全相同或互相包含即判重复，存在“同名不同作/版本”误合并风险。 | 名称只能做候选召回，不能单独触发自动合并。 |
| `src-tauri/src/commands/platform.rs:987-1128` | 平台导入逐项写库、无整批事务/持久化 job/cancel；新条目自动刮削为不可追踪后台 spawn。 | 扫描与 apply 分离；job、取消、重试和刮削子任务必须可恢复、可审计。 |
| `src-tauri/src/db_sqlite.rs:1265-1410` | 应用刮削会直接覆盖标题、标签、截图等；非 VNDB/Bangumi 评分会写成 `user_rating`。 | provenance 上线前禁止静默覆盖用户字段；评分来源与用户评分必须分离。 |
| `src-tauri/src/commands/play.rs:314-332` | 平台 URI 启动后 session 立即以 0 秒关闭；直接进程仅由内存 `ProcessMonitor` 跟踪。 | 平台启动需外部进程/客户端关联或明确标记 `untracked`；崩溃恢复依赖持久化 open session。 |
| `src-tauri/src/db_sqlite.rs:974-1077` | session 嵌在 `games.data_json`，跨游戏统计必须扫描全部游戏。 | `play_sessions` 是迁移主表，legacy JSON 只做兼容投影。 |
| `src/lib/utils/continue.ts:323-342` | 番剧/漫画时长由累计集数/话数乘常数估算，重复刷新会把累计进度当本次活动。 | 跨媒体仪表盘区分“精确时长、估算时长、仅进度”，默认总时长不得混算。 |
| `src/lib/components/StatsPage.svelte:18-30,50-55,145-232` vs `src-tauri/src/stats.rs:25-39` | 前端 DTO 使用 `total_playtime_hours/top_developers/recent_sessions/...`，后端返回 `playtime_hours/completion_distribution/collections/...`，契约已漂移。 | 第一优先级建立共享 DTO/契约测试；禁止页面内再声明同名私有接口。 |
| `src-tauri/src/stats.rs:121-127,217-245` | dashboard 请求同步递归统计磁盘；session `end_time` 按带秒格式解析，而模型实际写到分钟，月度小时可能为 0，且 last-played 会重复增加 session 数。 | 磁盘统计移出首屏并缓存；统一 RFC3339/毫秒 UTC；聚合以 session 主键去重。 |

现有测试主要覆盖 `gameLibrary` 基础筛选、`continue` 工具和少量 Rust 单元测试；未发现 `GameGrid/GameDetailPage/PlatformImportPage/ScrapeDialog/PlayRecordsDashboard/StatsPage` 的组件或端到端契约测试，因此“页面可渲染”不能视为现状已通过。

## 2. 目标

1. 新用户可在一次向导中导入库，明确看到新增、更新、重复、冲突和忽略项。
2. 老用户可快速发现未整理、路径失效、封面缺失和元数据冲突。
3. 每个元数据字段有来源、更新时间、置信度和回滚能力。
4. 游戏启动成功/失败、游玩 session 与状态变化写入统一 ActivityEvent。
5. 记录页可把游戏、番剧、漫画的“最近继续”和趋势放在同一模型中展示。
6. 1000–5000 游戏规模下仍能快速搜索、筛选、滚动和启动。

## 3. 非目标

- 不在本轮实现游戏平台账号交易、商店购买或成就作弊。
- 不自动移动/删除用户游戏目录；任何文件变更必须预览并确认。
- 不依赖 AI 才能完成导入、去重、刮削或搜索。
- 不把记录仪表盘做成开发者监控台。

## 4. 用户工作流

### 4.1 首次导入

1. 选择来源：Steam / Epic / 本地目录 / 模拟器 / 手动添加。
2. 后台扫描产生 `ImportCandidate`，UI 实时显示进度并允许取消。
3. 进入 diff preview：
   - 新增：绿色；
   - 更新：显示字段变化；
   - 冲突：要求选择身份或合并；
   - 忽略：展示原因；
   - 路径失效：提供重定位。
4. 用户确认后事务写入，失败可重试，不留下半导入状态。
5. 导入结束进入“待整理”集合，而不是直接淹没在全部游戏中。

### 4.2 日常整理

- 默认首页展示：继续、最近加入、待整理、收藏和全部游戏。
- 搜索支持名称、别名、标签、厂商、平台、路径和备注。
- 批量选择后可修改标签、状态、合集、成人显示、隐藏、封面和刮削策略。
- “库健康”页列出：路径失效、重复、无封面、无简介、来源过期、启动失败。

### 4.3 启动与记录

- 启动前解析 launcher descriptor：exe / Steam URI / Epic URI / emulator profile。
- 启动失败按原因显示：文件不存在、平台客户端缺失、权限、locale、参数错误。
- 成功启动创建 session；进程结束后写入时长、最后游玩和 ActivityEvent。
- 异常退出/应用重启后能恢复或关闭悬挂 session。

### 4.4 记录与继续

- 记录首页第一屏优先是“继续”，其次是近期节奏和内容构成。
- 用户可按 7/30/90 天、自定义时间、类型和来源筛选。
- 事件可更正、合并、删除；聚合会重算。
- 隐私模式可暂停记录，已有记录可导出或清除。

## 5. 信息架构

### 游戏库

- `概览`：继续、最近加入、待整理、收藏。
- `全部`：虚拟化网格/列表、筛选、排序、批量选择。
- `合集`：手动合集与智能合集。
- `库健康`：重复、失效、缺失元数据、启动失败。
- `导入`：导入任务与历史 diff。

### 游戏详情

- 顶部：封面/标题/状态/启动主操作。
- Tabs：概览、记录、媒体、元数据来源、存档/备份、笔记。
- 元数据来源 tab 可以逐字段选择来源与回滚。

### 记录

- `继续`、`趋势`、`日历`、`明细` 四个视图。
- 游戏/番剧/漫画共享筛选和事件明细，媒体特有字段在 detail drawer 中展示。

## 6. 数据模型

```ts
interface GameIdentity {
  gameId: string;
  canonicalTitle: string;
  titleFingerprint: string;
  launchIdentity: { kind: "path" | "steam" | "epic" | "emulator"; value: string };
  externalIds: Record<string, string>;
}

interface FieldProvenance<T> {
  field: string;
  value: T;
  source: string;
  sourceItemId?: string;
  confidence?: number;
  fetchedAt: string;
  selected: boolean;
}

interface ImportCandidate {
  candidateId: string;
  identity: GameIdentity;
  action: "create" | "update" | "merge" | "ignore" | "conflict";
  changes: FieldChange[];
  reason: string;
}

interface ActivityEvent {
  id: string;
  resourceKind: "game" | "anime" | "comic";
  resourceId: string;
  eventType: "baseline" | "started" | "progressed" | "completed" | "rated" | "imported" | "failed";
  startedAt: string;
  endedAt?: string;
  durationSeconds?: number;
  durationQuality: "exact" | "estimated" | "baseline" | "none";
  sessionId?: string;
  sourceLegacyId?: string;
  payload: Record<string, unknown>;
}
```

建议 SQLite 新表：

- `game_identities(game_id, identity_kind, identity_value, normalized_value, confidence, created_at)`。
- `activity_events(id, resource_kind, resource_id, event_type, started_at, ended_at, duration_seconds, duration_quality, session_id, source_legacy_id, payload_json)`。
- `progress_records(resource_kind, resource_id, provider_id, progress_json, updated_at)`。
- `game_field_sources(game_id, field_name, source, source_item_id, value_json, confidence, fetched_at, selected)`。
- `import_runs(id, source_kind, status, started_at, completed_at, summary_json)`。
- `import_candidates(run_id, candidate_id, action, data_json)`。
- `play_sessions(id, game_id, legacy_session_id, process_identity, started_at, ended_at, duration_seconds, status)`。
- `migration_checkpoints(migration_key, cursor, status, updated_at, detail_json)`。

索引：activity 时间/类型/资源、game title fingerprint、launch identity、session status。


### 6.1 SQLite v2 → v3 迁移与兼容边界

当前 `src-tauri/src/db_sqlite.rs` 的 `SCHEMA_VERSION = 2`，且真实数据源是 `games.data_json`。v3 采用**仅新增表/列**的可回滚迁移，不在 0.12.1 首次升级时删除 JSON 字段：

1. 升级前执行 `PRAGMA quick_check`，记录 DB 大小与可用磁盘；创建带版本/时间戳的同目录备份，备份 fsync 成功后才迁移。
2. 单事务创建 `game_identities`、`game_field_sources`、`import_runs`、`import_candidates`、`play_sessions`、`activity_events`、`progress_records` 及索引；schema version 只在全部成功后改为 3。
3. 按 `game_id` 分批回填：
   - 真实 legacy `play_tracker.sessions` 原样写入 `play_sessions`，以 `(game_id, legacy_session_id)` 唯一约束保证幂等；时间解析兼容现有 `%Y-%m-%d %H:%M`，新写入统一 RFC3339 UTC。
   - 只有累计 `total_seconds`、没有 session 的游戏写 `activity_baseline`/baseline event，**不得伪造某天的 session**。
   - 番剧/漫画历史只回填 `progress_records`；没有可靠起止时间时不得生成 duration event。
   - 当前字段候选以 `source='legacy'`、`selected=1` 建档；无法证明历史来源时不猜测 VNDB/Steam。
4. feature flag 期间新表为规范源，同时 dual-write 必要的 legacy `Game.play_tracker/metadata` 投影；每次启动抽样比较 session 总时长、最后活动和 selected metadata checksum。
5. 所有 backfill 有 `migration_checkpoint`，支持中断续跑；相同 v2 fixture 连续迁移两次，行数与聚合结果必须不变。
6. 回滚仅切回 `library_v2=false/activity_v2=false` 并读取旧 JSON；v3 表保留但停止写入。发生新表独有的用户编辑后，不做破坏性 schema downgrade，只允许从升级前备份整体恢复或导出后重建。

关键约束：`activity_events` 与 `play_sessions` 不得对同一次游戏游玩重复计时；建议 session 是游戏时长事实表，activity 仅引用 `session_id`。`game_field_sources` 对 `(game_id, field_name)` 只能有一个 selected 候选，并用事务切换。

## 7. 前端改动计划

### 新模块

- `src/lib/features/library/model.ts`
- `src/lib/features/library/libraryStore.svelte.ts`
- `src/lib/features/library/importStore.svelte.ts`
- `src/lib/features/library/components/LibraryOverview.svelte`
- `src/lib/features/library/components/LibraryGrid.svelte`
- `src/lib/features/library/components/LibraryHealth.svelte`
- `src/lib/features/library/components/ImportDiff.svelte`
- `src/lib/features/library/components/FieldSourcePicker.svelte`
- `src/lib/features/activity/activityStore.svelte.ts`
- `src/lib/features/activity/components/ActivityOverview.svelte`
- `src/lib/features/activity/components/ContinueRail.svelte`
- `src/lib/features/activity/components/ActivityTimeline.svelte`
- `src/lib/features/activity/components/ActivityFilters.svelte`

### 迁移策略

- 旧 stores 暂时作为 compatibility adapter，避免一次重写全部页面。
- 把 `PlayRecordsDashboard.svelte` 拆成数据 store、query DTO、图表组件和页面 shell。
- `SwitchHome.svelte` 先迁移查询/虚拟化，再迁移视觉，避免行为与 UI 同时变化。
- 所有异步操作接入统一 `BackgroundJob`，页面不维护独立 retry/cancel 状态机。

## 8. Rust 改动计划

### 新 service

- `services/library/import_service.rs`
- `services/library/identity_service.rs`
- `services/library/metadata_service.rs`
- `services/library/launch_service.rs`
- `services/activity/activity_service.rs`
- `repositories/activity_repository.rs`
- `repositories/import_repository.rs`

### 薄命令

- `library_scan_import`
- `library_preview_import`
- `library_apply_import`
- `library_cancel_import`
- `library_health_report`
- `library_relocate_path`
- `library_get_field_sources`
- `library_select_field_source`
- `activity_query_summary`
- `activity_query_events`
- `activity_update_event`
- `activity_delete_event`
- `activity_export`

命令只做参数校验和 DTO 转换，不包含平台解析或 SQL 细节。

## 9. 任务拆分

### LIB-1 Identity 与去重

- 建立 title fingerprint、launch identity、external ID 组合规则。
- 覆盖同名不同游戏、路径移动、Steam 同游戏重复扫描。
- 验收：重复扫描幂等；不同 identity 不误合并。

### LIB-2 Import Job 与 Diff Preview

- 扫描、取消、任务持久化、preview、事务 apply。
- 验收：取消后无半写入；重新打开可查看上次结果。

### LIB-3 Field Provenance

- 刮削 merge 输出字段候选，不直接覆盖。
- UI 可逐字段选择并回滚。
- 验收：来源切换后兼容 legacy 字段投影。

### LIB-4 Library Health / Batch

- 失效路径、重复、缺图、缺元数据、启动失败。
- 批量操作带 undo token。

### LIB-5 Launch Session

- launcher descriptor、进程跟踪、悬挂 session 恢复。
- 验收：Steam URI、本地 exe、模拟器三个 fixture。

### ACT-1 Activity Migration

- 从现有游戏时长、番剧/漫画 history 生成初始事件/进度。
- 迁移必须幂等并记录 source legacy ID。

### ACT-2 Aggregate Queries

- 7/30/90 日、趋势、分布、continue candidates。
- 20k 事件 benchmark。

### ACT-3 Dashboard UI

- Continue-first、过滤、timeline、detail drawer、编辑/删除。

### ACT-4 Privacy / Export

- 暂停记录、导出 JSON/CSV、清理范围预览。


### 9.1 依赖关系与并行执行

先完成 **F0（2–3 人日）契约冻结**：统一 Rust/TypeScript DTO、时间格式、identity 规则版本、v3 DDL 与 golden fixtures。F0 未完成前不得并行落库，避免两个分支各自定义事件和导入结构。

| 工作流 | 可并行内容 | 前置 / 汇合点 |
|---|---|---|
| A：Identity + Import（LIB-1/2） | Rust identity、job repository、ImportDiff mock UI 可并行 | 依赖 F0；与 C 在 `ImportCandidate/ImportApplyResult` 契约汇合。 |
| B：Session + Activity（LIB-5、ACT-1/2） | migration/backfill、SQL aggregate、launch outcome 可并行 | 依赖 F0；迁移与写路径汇合后才允许 Dashboard 读新表。 |
| C：Library UI（LIB-4 + LIB-3 UI） | 现有虚拟网格压测、筛选修复、详情 tab/field picker 可并行 | 可先用冻结 DTO fixture；真实 apply 依赖 A 与 provenance repository。 |
| D：Dashboard UI（ACT-3/4） | 页面 shell、图表、筛选、a11y 可并行 | 可先用冻结 summary fixture；真实数据依赖 B。 |
| E：集成/迁移/性能 | golden DB、Playwright、benchmark、rollback drill | 依赖 A+B 主路径；C/D 可持续修复。 |

关键路径为 `F0 → v3 migration/session write → aggregate queries → Dashboard integration → migration/perf gate`。`Field Provenance` 与完整 `Import Job` 不应同时由同一人从 UI 到 DB 串行包办；建议按 repository/command 与 UI/store 两条线拆分。单人顺序实施预计 15–20 人日；两名工程师并行并保留 3 天集成窗口，目标 10–12 个工作日，原“10 天全部完成”只适用于无迁移返工的理想情况。

## 10. 性能预算

基准环境固定为 Windows 11、4 核 CPU、16 GB RAM、NVMe、release Tauri 包；每项报告 10 次运行的 p50/p95，并区分应用冷启动与同进程热查询。预算不含网络刮削耗时。

- **库启动**：1000 游戏冷启动至可交互 p95 ≤ 2.0s；5000 游戏 p95 ≤ 5.0s。首屏不得等待磁盘目录递归统计或在线图片。
- **查询**：5000 游戏本地筛选/排序 p95 ≤ 150ms；走 SQLite 投影/分页的查询 p95 ≤ 250ms；输入 debounce 100–150ms，过期请求不可覆盖新结果。
- **虚拟网格**：标准/紧凑/列表三模式 DOM 中 `GameCard` ≤ 200；30 秒连续滚动的 p95 frame time ≤ 20ms、长任务（>50ms）≤ 3 次，不以“肉眼 60fps”验收。
- **导入**：扫描开始后 500ms 内出现首个进度；取消请求 2s 内停止新增工作；1000 个已扫描候选 apply（不含网络/图片下载）p95 ≤ 10s，并逐项返回结果。
- **活动查询**：20k events / 5k sessions 下，SQL summary p95 ≤ 300ms，完整首屏（含序列化与渲染）p95 ≤ 800ms；timeline keyset 分页每页 ≤ 100。
- **磁盘/图片**：磁盘占用作为可取消后台 job 并缓存，不阻塞 `get_dashboard_data`；缩略图复用磁盘缓存，不在网格加载原图。
- **内存/稳定性**：5000 游戏 fixture 加载后前端 heap 相对空库增量 ≤ 100MB；快速切换筛选 100 次后无持续增长，后台 listener/job 均在页面卸载或取消时清理。

## 11. 验收与测试矩阵

| 场景 | 层级与通过标准 |
|---|---|
| Rust/TS dashboard DTO | contract test：序列化 fixture 可被 `StatsPage` 直接消费；禁止页面私有同名 DTO；覆盖当前字段漂移回归。 |
| 重复 Steam/Epic 导入 | integration：相同 `(source, library_id)` 第二次 0 create；只更新允许字段，用户封面/标题不被覆盖。 |
| 本地同名不同路径/版本 | identity fixture：名称相同或包含关系不得自动合并；路径大小写、符号链接与路径移动有明确结果。 |
| 导入 preview/apply/cancel | repository + command：preview 零写入；apply 事务边界明确；取消后无新写入；逐项错误可重试；重开应用可见历史。 |
| 自动刮削子任务 | integration：任务状态可查询/取消/重试；应用关闭后不会留下“成功但无结果”的假状态。 |
| 字段来源切换 | repository roundtrip + component：用户值、legacy、平台、刮削候选切换可回滚；未知源评分不写 `user_rating`。 |
| 智能合集 | store test：`tags any/all`、installed、hasPlayed 与导入旧 localStorage 合集均覆盖；激活合集不污染其他筛选。 |
| 启动成功/失败 | command + component：exe 缺失、locale 缺失、平台 URI、模拟器 fixture；失败时详情页不得提示启动成功。 |
| session 正常/异常退出 | direct child、launcher child、平台 `untracked`、应用崩溃四类；恢复 open session 不重复累计。 |
| 时间格式与月度聚合 | Rust：兼容 legacy 分钟格式并写 RFC3339 UTC；跨月、时区、DST；同一 session 不因 last-played 重复计数。 |
| 跨媒体继续/时长 | unit + component：精确、估算、仅进度三种质量标签；番剧/漫画累计集数变化不重复累计历史时长。 |
| 旧数据迁移 | v2 golden DB（空库、仅总时长、有 open session、损坏时间、5000 游戏）→ v3；重复执行幂等；checksum/总时长守恒。 |
| 回滚演练 | upgrade → 新写入 → flag 回退；旧页面可读 legacy 投影；升级前备份可完整恢复，记录 RTO 与文件校验。 |
| 20k 活动 / 5000 库 | benchmark：执行计划命中索引并满足第 10 节 p95；测试禁止把全部 timeline/event DTO 一次送前端。 |
| 键盘、读屏、批量操作 | Playwright + axe：网格/列表焦点、Shift/Ctrl 选择、删除确认、进度 announce、取消/重试。 |

## 12. 风险与回滚

- **误合并游戏**：只自动合并高置信 identity；其余进入 conflict。
- **字段覆盖**：先写 provenance，selected value 单独投影；支持回滚。
- **大库卡顿**：虚拟化、分页、SQLite 聚合，不把全部事件送前端。
- **进程跟踪不可靠**：session 状态允许 `unknown`，用户可修正。
- **迁移失败**：升级前复制 DB；事务回滚；保留只读 legacy importer。
- **UI 改版回归**：feature flag `library_v2` / `activity_v2`，旧页面保留到 RC。

## 13. 里程碑

- **M0 / 契约闸门（2–3 人日）**：冻结 DTO、时间/identity 规范、v3 DDL、golden fixtures；先修复 `StatsPage` 契约漂移和启动错误传播。退出条件：contract tests 全绿。
- **M1 / 数据底座（4–5 人日，可 A/B 并行）**：identity/import repository、session/activity 表、幂等 backfill、dual-write。退出条件：迁移/回滚与总时长守恒通过。
- **M2 / 可靠闭环（4–5 人日）**：持久化 import job/diff/cancel、field provenance、launch outcome/open-session recovery、SQL aggregates。退出条件：命令集成测试与 20k benchmark 达标。
- **M3 / UI 接入（4–5 人日，可 C/D 并行）**：修复现有 store 筛选，拆分 Library/Detail/Records，接入真实 DTO、任务进度和字段来源。退出条件：关键 Playwright/axe 场景通过。
- **M4 / 发布闸门（3 人日）**：5000 库性能、磁盘统计后台化、故障注入、备份恢复、feature flag 回退、RC 修复。退出条件：第 10/11/12 节全部有证据；未达标则保留旧页面为默认，不强行切换。
