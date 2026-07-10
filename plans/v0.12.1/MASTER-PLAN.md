# MoePlay 0.12.1 → 0.13.0 综合优化 MASTER PLAN

> 规划日期：2026-07-10  
> 当前工作树：`codex/v0.12-anime-comic-acceptance` / `3ceb354`  
> 当前代码版本：`0.12.0`（`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 一致）

## 0. 基线说明

用户提出“在 0.12.1 的基础上”继续开发，但截至 2026-07-10，本仓库与远端引用中没有 `v0.12.1` tag，版本文件仍为 `0.12.0`。当前 HEAD 已比 `v0.12.0` 多 3 个番剧/漫画可靠性提交，因此本计划把当前 HEAD 视作 **0.12.1 候选基线**：先完成版本一致性、回归与打标，再进入下一阶段功能开发。

当前可验证基线：

- 四个主入口由 `src/lib/nav.ts` 明确定义为：**游戏库、记录、番剧、漫画**。
- 其他功能包括：继续、发现、刮削、下载、存档、统计、平台导入、模拟器、诊断、设置、大屏模式。
- 前端质量：`svelte-check` 0 错误/0 警告；155 个单元测试通过、1 个跳过；3 个 Playwright 冒烟通过。
- Rust 质量：`cargo fmt --check`、`cargo clippy -D warnings` 通过；126 个测试通过，2 个 live/环境测试跳过。
- 源目录已有 19 个 manifest：6 active / 5 planned / 8 reference；但“目录存在”不等于“用户可稳定使用”，需要把 reference/planned 转成可探测、可配置、可回退的连接器。
- 大文件风险明显：`anime.svelte.ts` 2019 行、`AnimePlayer.svelte` 1764 行、`PlayRecordsDashboard.svelte` 1601 行、`anime.rs` 2048 行、`db_sqlite.rs` 2014 行，后续必须在功能开发前建立稳定边界。
- AI 已有多 Provider/preset 和 AI 刮削基础，但 API Key 仍属于普通 Settings 字段，缺少系统凭据存储、结构化输出校验、任务审计和面向用户的统一 AI 工作台。
- Tauri command 契约存在 P0 漂移：实际注册 281 个，`build.rs` 声明 274 个，capability 覆盖 278 个；至少 `import_selected_candidates`、`pick_image_file`、`preview_directory_for_games` 已被前端调用但缺 capability。
- 数据故障路径需要先修：SQLite migration 失败路径可能删除主库并静默降级内存库，部分查询错误用 `unwrap_or_default()` 表现为空库/默认设置；新功能不得建立在该恢复策略上。
- 凭据不只 AI：Bangumi/PicACG token 存在 localStorage，AI/Steam Key 随 settings JSON 明文进入 SQLite；SecretStore 必须作为 0.12.1 的安全前置项。
- `StatsPage.svelte` 的前端 Dashboard DTO 与 Rust `DashboardData` 字段已发生漂移；必须先建立契约测试，不能在错误数据形状上继续堆仪表盘 UI。
- 现有“多 AI Provider”并未真正闭环：六个 preset 都绑定 OpenAI 路径、Ollama 被非空 Key 检查阻断、Claude 仍使用 OpenAI 请求体；且修改 endpoint 后可能把旧 Key 发往新 origin。Provider contract 与 Secret-origin 绑定属于 P0。

## 1. 产品目标

把 MoePlay 从“多个能力很强但彼此分散的页面”推进为 **本地优先、统一记录、内部消费优先、失败可恢复** 的游戏与二次元媒体中心。

### 1.1 核心结果

1. **游戏库**：导入、去重、刮削、整理、启动、记录和批量管理形成闭环。
2. **记录仪表盘**：统一游戏/番剧/漫画活动事件、进度与继续入口，图表可以解释数据而不是只展示数字。
3. **番剧**：搜索—详情—线路—播放—弹幕—进度—换源闭环，健康源尽量在软件内部直接播放。
4. **漫画**：搜索—详情—章节—阅读—预取—进度—换源闭环，优先接入公共 API、自托管服务和隔离连接器。
5. **AI**：从“刮削增强开关”升级为可控、可审计、可降级的助手层，服务于整理、搜索、推荐、摘要和笔记。
6. **整体 UI**：四主功能共享一致的信息架构、状态反馈、交互密度、响应式与大屏/手柄体验。

### 1.2 产品原则

- **Local-first**：核心库、记录、设置和缓存离线可用；远端失败不阻断本地功能。
- **Internal-first, fallback-always**：能在应用内合法播放/阅读则内部完成；失败时提供换源、验证、外部打开或本地服务回退。
- **No DRM/paywall bypass**：不设计绕过付费、DRM、账号限制或站点安全策略的实现。
- **Provider isolation**：第三方规则/索引只作为数据输入；不直接执行 Android/JVM/任意第三方插件代码。
- **Observable failure**：每次失败要归类、可重试、可诊断；不能只显示“加载失败”。
- **One source of truth**：版本、源状态、活动记录、AI 任务和数据库迁移只有一个权威模型。

## 2. 目标架构

### 2.1 统一领域模型

新增或固化以下跨功能模型：

- `ResourceIdentity`：`kind(game|anime|comic)`、规范标题、别名、外部 ID、内容分级。
- `ProviderManifest`：来源、能力、认证、许可证/隔离等级、健康状态、版本和配置 schema。
- `ProviderItemRef`：资源在某个 Provider 中的标识、URL/ID、最近成功时间。
- `ProgressRecord`：游戏秒数、番剧集数/时间点、漫画章节/页码统一存储。
- `ActivityEvent`：started / progressed / completed / rated / favorited / imported / failed。
- `ProviderHealth`：延迟、连续失败、验证码/限流/解析错误、熔断到期时间。
- `BackgroundJob`：刮削、AI、下载、索引同步、源探测统一任务状态与取消令牌。
- `SecretRef`：设置只保存凭据引用，实际密钥进入 Windows Credential Manager/系统 keyring。

### 2.2 分层

```text
Svelte 页面/组件
    ↓ view models / domain stores
统一前端 API client（typed invoke + cancellation + normalized errors）
    ↓
Tauri commands（薄命令层）
    ↓
Domain services（Library / Activity / Media / AI / Jobs）
    ↓
Provider connectors + Repository/SQLite + Cache + HTTP client
```

禁止继续把页面、store、Tauri command 和站点解析逻辑写进单个超大文件。每个 Provider 必须通过契约测试，不允许页面直接判断具体站点的 URL 结构。

### 2.3 Provider 能力契约

统一能力：

- `probe(config)`：配置校验、版本与延迟。
- `search(query, filters, cursor, signal)`。
- `detail(ref)`。
- `children(ref)`：番剧 episodes/roads 或漫画 chapters。
- `resolve(ref)`：播放流、网页播放描述、图片页或本地文件。
- `progress(read/write)`：可选；仅自托管/账号源实现。
- `verify()`：可选；验证码/登录/Token 验证。
- `health()`：标准错误分类与退避信息。

标准错误枚举：`Network`、`Timeout`、`RateLimited`、`AuthRequired`、`CaptchaRequired`、`ParseChanged`、`GeoBlocked`、`EmbedBlocked`、`Unsupported`、`Cancelled`、`Unknown`。

## 3. 版本路线图

### Phase 0 — 0.12.1 基线固化（1–2 天）

- 把 `package.json`、`Cargo.toml`、`tauri.conf.json`、CHANGELOG 同步到 0.12.1。
- 把当前 3 个媒体可靠性提交纳入 changelog，并执行完整质量门。
- 修复 command 注册 / `build.rs` / capability 三方漂移，新增自动一致性测试，禁止前端调用未授权 command。
- 修复 Dashboard Rust/TypeScript DTO 漂移并建立序列化契约测试；在此之前不扩展统计 UI。
- 为 AI endpoint 建立 origin policy，禁止旧 Secret 在用户修改目标地址后被静默发送；修复无 Key 本地 Provider 被阻断的问题。
- 禁止数据库迁移失败后删除主库或静默伪装为空库；先落地备份与只读 recovery mode。
- 修正本机 updater signing 配置：CI 构建与正式签名发布使用不同配置；不把私钥放仓库。
- 生成 `v0.12.1-rc.1`，完成升级/回滚和安装包冒烟。

**Gate P0**：现有功能不回退，质量门全绿，0.12.0 数据可无损启动 0.12.1。

### Phase 1 — 平台契约与数据底座（3–5 天）

- 提取 Provider 接口、标准错误、健康度和后台任务模型。
- 收敛 JSON migration 与 SQLite schema version 两套版本概念，建立显式 SQL migration 表。
- 建立 ActivityEvent / ProgressRecord / ProviderHealth 表与兼容迁移。
- 建立 keyring SecretStore，迁移 AI/Steam/Bangumi/自托管 Token。
- 拆分 `api/index.ts`、`anime.svelte.ts`、`anime.rs` 的基础边界，不改变用户行为。

**Gate P1**：Provider contract 测试、迁移往返测试、取消/重试测试、密钥不落明文日志。

### Phase 2 — 四主功能并行开发（8–12 天）

- Track A：游戏库。
- Track B：记录仪表盘。
- Track C：番剧内部播放与源平台。
- Track D：漫画内部阅读与源平台。

四条 Track 共用 Phase 1 契约，但写入范围分离；先以 feature flag 接入，不直接替换旧路径。

**Gate P2**：每条 Track 具备独立 E2E、空/加载/错误/离线状态和数据迁移验证。

### Phase 3 — AI 助手产品化（5–8 天）

- 建立 provider-agnostic AI Gateway、结构化输出、重试/预算/取消。
- 上线三个高价值入口：库整理建议、自然语言搜索、个性化“玩什么/看什么”。
- 再上线摘要/标签/笔记辅助；所有写入必须预览并由用户确认。
- 本地模型/Ollama 与远端 OpenAI-compatible API 同等支持。

**Gate P3**：AI 关闭或不可用时所有主功能完全可用；AI 不可自动删除、移动或覆盖用户内容。

### Phase 4 — UI/UX 统一与小功能收口（5–7 天）

- 统一四主功能页面 shell、标题栏、筛选、状态反馈、详情面板、继续入口。
- 整理主题令牌、密度、字体、焦点、动效和 reduced-motion。
- 大屏/手柄、窄屏、125%/150% Windows 缩放专项。
- 发现/刮削/下载/备份/导入/诊断/设置迁移到同一 Job/Provider 状态体系。

**Gate P4**：视觉回归、键盘操作、手柄主流程、性能预算均通过。

### Phase 5 — Release Candidate 与发布（3–5 天）

- 真实源 acceptance（非 PR 必跑，允许隔离不稳定源）。
- 1000/5000 游戏库、20000 活动事件、长漫画章节性能测试。
- 安装升级、便携版、自动更新、数据库备份与降级演练。
- 生成 manifest、签名 updater artifacts、release notes 与已知问题清单。

## 4. 四主功能详细交付方向

### 4.1 游戏库

- 导入向导统一 Steam/Epic/本地目录/模拟器；导入前展示新增、更新、冲突、忽略。
- 去重使用路径 identity + 平台 ID + 规范标题组合，不只依赖名称。
- 刮削采用 field-level provenance：每个字段记录来源、置信度、更新时间，可单字段回滚。
- 支持批量标签、状态、合集、封面修复、路径重定位和失效项扫描。
- 首页区分“继续”“最近导入”“未整理”“随机推荐”，不把所有信息塞进一个 Bento。
- 启动失败提供路径修复、平台客户端缺失、权限和 locale 诊断。

### 4.2 记录仪表盘

- ActivityEvent 统一游戏 session、番剧观看、漫画阅读。
- 提供时间范围、类型、设备/来源筛选和数据解释。
- “继续”是主操作；图表是辅助，不做无意义 KPI。
- 支持记录编辑/合并/删除、隐私模式和导出。
- 大数据量查询使用聚合表/索引，页面不加载全部原始事件。

### 4.3 番剧

- 搜索聚合采用并行源 + 去重分组，展示源数、可播放状态和健康度。
- 详情以 Bangumi/AniList 等元数据与 Provider 播放身份分离。
- 播放 resolve 输出 `native-hls` / `native-file` / `webview` / `external`，播放器只消费统一 descriptor。
- 自动换源必须保持集数身份，不以弱字符串匹配盲换。
- 第一优先新增：Jellyfin 本地/自托管库连接器、KazumiRules 增量同步与校验；其次是用户授权的 Emby/Plex 或 WebDAV/本地文件适配。
- Aniyomi 扩展仓库已归档，不作为新执行引擎；只保留契约参考。CloudStream/Kazumi 本体等 GPL 项目只参考行为，不复制实现。

### 4.4 漫画

- MangaDex、Baozi/DM5/1kkk 与自托管库通过统一 Provider 展示；PicACG 继续作为独立受控入口。
- 第一优先新增：Komga、Kavita、LANraragi、Suwayomi 的只读连接器；第二优先：OPDS/本地 CBZ/PDF/WebDAV。
- Keiyoushi 只导入索引和能力元数据，不执行 APK/Kotlin 扩展。
- 阅读器支持长条/分页、预取窗口、图片失败单页重试、章节切换和阅读进度恢复。
- 缓存可见、可清理、有限额；离线下载复用 BackgroundJob。

## 5. AI 功能交付方向

### 5.1 首批场景

1. **库整理 Copilot**：识别标题、重复项、标签和缺失元数据，输出可审阅 change set。
2. **自然语言搜索**：把“最近想玩轻松的全年龄短篇”转换为本地结构化过滤器；结果必须可解释。
3. **玩什么/看什么**：综合用户显式偏好、最近活动、未完成度与可用源，不只依赖 LLM 幻觉。
4. **摘要/翻译/笔记**：对已有元数据进行增强，保留原文与来源。

### 5.2 安全边界

- API Key 进入 SecretStore；前端只得到 `configured: true/false`。
- Prompt、模型、温度、schema 都有版本号；AI 输出必须通过 JSON schema 校验。
- 输入上下文在发送前展示隐私摘要；默认不发送本地路径、存档内容、账户 Token。
- 所有 AI 写操作先生成 patch，用户确认后应用；支持撤销。
- 支持全局月预算、单任务 token/时间预算、并发限制和取消。

## 6. UI/UX 目标

- 视觉参数基线：高变化但不过度装饰（variance 8）、中等动效（6）、中低密度（4）。
- 保留中文 UI；字体继续使用 Outfit + JetBrains Mono，不引入新的字体体系。
- 主色只保留一个语义 accent；不同媒体类型使用低饱和辅助色，不形成四套设计。
- 页面统一 `PageShell / PageHeader / FilterBar / ContentGrid / DetailPanel / AsyncState`。
- 卡片只用于可操作或可分组信息；纯文本和分隔优先用留白层级。
- GSAP 动画只使用 transform/opacity，必须 cleanup，并响应 `prefers-reduced-motion`。
- 关键流程全键盘可达；焦点可见；Dialog/Drawer 有焦点陷阱与返回焦点。
- 大屏模式不是桌面 UI 放大，而是 10-foot UI：更少项目、更大目标、手柄焦点稳定。

## 7. 量化验收预算

| 维度 | Release Gate |
|---|---|
| 游戏库启动 | 1000 游戏冷启动到可交互 ≤ 1.5s；5000 游戏 ≤ 3s（基准机） |
| 搜索取消 | 用户取消后 500ms 内停止 UI 更新；过期请求不得覆盖新结果 |
| Provider 搜索 | 3 个健康源下首批结果 p95 ≤ 4s；单源故障不阻断其他源 |
| 番剧起播 | 健康测试源 p50 ≤ 8s、p95 ≤ 15s；失败必须进入可操作状态 |
| 漫画首屏 | 健康测试源首张图 p95 ≤ 3s；单页失败可重试且不清空章节 |
| 仪表盘 | 20000 ActivityEvent 首屏聚合 ≤ 800ms；不向前端传全部事件 |
| UI | 主滚动/过渡目标 60fps；无持续 layout thrashing |
| 包体/构建 | release build、MSI、NSIS、portable、manifest 全部通过自检 |
| 安全 | Secret 不出现在 settings 导出、日志、panic、前端 state dump |
| 可访问性 | 键盘完成四主流程；reduced-motion 下无强制大幅动画 |

## 8. 测试与发布门禁

每个 PR 必须运行：

```powershell
npm run check
npm run test:unit
npm run build
npm run test:visual
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

新增门禁：

- Provider contract fixtures：搜索、详情、子项、resolve、错误归一。
- DB migration golden files：0.12.0 → 0.12.1/下一 schema、升级失败恢复、导出导入。
- Secret redaction test。
- Activity aggregation benchmark。
- AI JSON schema/failure/cancel/budget 测试。
- live acceptance 放 nightly/manual，不让第三方源波动阻塞普通 PR；连续失败自动降级源状态。

## 9. 风险与决策

| 风险 | 决策 |
|---|---|
| 第三方站点经常改版 | Provider 隔离、fixture、健康度、feature flag、快速禁用 |
| 来源许可证不兼容 | 只消费公开 API/索引；GPL 实现保持进程/API 边界；不复制代码 |
| 源站账号/隐私泄漏 | SecretStore + redaction + 最小化请求上下文 |
| 旧数据库损坏 | 升级前备份、事务迁移、版本化 migration、失败自动恢复 |
| 超大文件继续膨胀 | 功能开发前先抽 contract/service；单文件建议 ≤ 700 行，超出需说明 |
| AI 幻觉覆盖元数据 | field provenance + preview patch + 人工确认 + undo |
| UI 大改导致回归 | feature flag、页面 shell 先行、逐页迁移、截图矩阵 |
| 自动更新签名失败 | CI smoke 与正式签名分离；release secret 预检；禁止仓库私钥 |

## 10. 完成定义

本计划不是以“新增页面/新增源数量”作为完成标准。只有同时满足以下条件才可宣布目标完成：

- 四主功能均形成可使用闭环，并有独立 E2E/验收证据。
- 至少一个番剧自托管连接器、两个漫画自托管/公共连接器达到 active；planned/reference 不计入交付。
- AI 的三个首批场景可用，且关闭 AI 不影响任何主功能。
- 统一活动/进度模型完成迁移，0.12.0 数据升级与回滚验证通过。
- 所有安全、性能、可访问性、构建和发布门禁通过。
- 文档、版本、CHANGELOG、安装包与 updater artifacts 一致。


