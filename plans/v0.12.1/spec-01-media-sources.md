# MoePlay 0.12.1 SPEC-01：番剧 + 漫画内部观看/阅读与多源扩展

> 状态：规划基线（implementation-ready）  
> 版本基线：用户指定 **0.12.1**；仓库当前 `HEAD=3ceb354`，`package.json` 仍为 `0.12.0`。本专项不负责版本号修改。  
> 范围：番剧、漫画、源接入、内部播放器/阅读器、健康度、熔断、回退和测试。  
> 约束：本规格只规划合法、可维护的接入方式；**不得绕过付费、账号授权、地域限制、DRM/EME 或反自动化挑战**。第三方内容权利与开源许可证是两套独立审查项，均需记录。

## 1. 摘要与结论

MoePlay 已经具备两个可用但不统一的媒体栈：

- **番剧**已有 Kazumi 规则兼容、并发搜索、线路/分集、隐藏 WebView 视频地址提取、本地 HLS 代理、HLS.js/原生双模播放、进度、自动连播、弹幕、自动换源、网页/浏览器/外部播放器回退。
- **漫画**已有普通漫画聚合搜索（MangaDex、包子、DM5、1kkk）、PicACG 独立入口、统一详情形状，以及图片流或受限 iframe 两种阅读模式。

0.12.1 的主要问题不是“完全没有播放/阅读能力”，而是：

1. 番剧与漫画各自维护源模型、错误模型、健康状态和回退逻辑，无法形成统一控制面。
2. 番剧健康度同时写浏览器 `localStorage` 与 Rust JSON 文件，但排序只读前者，存在双真源和漂移。
3. 漫画多源仍是 `comicStore` 中的 provider 分支与硬编码源列表；扩展索引、目录和外部运行时连接器虽已有模型与测试，却没有接入产品 store/UI。
4. 漫画阅读器只有纵向图片流/iframe、章节级历史，缺少页级进度、预取、缩放、翻页方向、单页重试和源故障回退。
5. 当前源执行能力没有显式区分“内部直放/内部图片/网页嵌入/仅外部打开/DRM 不支持”，容易把技术失败、策略禁止和登录需求混为一类。

**0.12.1 应以“统一 source contract + 当前源迁移 + 健康/熔断 + 内部体验补齐”为发布主线。** 新源扩展优先选择公开 API、用户自托管或用户自有内容；第三方扩展索引只做目录发现，不在 MoePlay 内执行 APK、Dart、JS 或任意第三方代码。

---

## 2. 现状证据

### 2.1 番剧前端与内部播放器

| 证据位置 | 当前能力 | 审计结论 |
| --- | --- | --- |
| `src/lib/components/AnimePage.svelte` | 推荐、时间表、收藏/历史、规则仓库；详情和播放器使用 overlay；规则可从 GitHub 目录安装 | 页面已是完整番剧产品入口，但源管理仍以“规则列表”为中心，没有健康、信任、能力或禁用原因视图 |
| `src/lib/components/anime/SourceSheet.svelte` | 打开后对所有规则逐个发起 `anime_search`；每源独立 pending/success/error/captcha 状态；选择结果后再拉线路并明确选集 | 已避免盲播第 1 集；但无并发上限、无熔断跳过、无统一超时、无健康排序，组件内部再次实现一套源编排 |
| `src/lib/stores/anime.svelte.ts` | 约 80 KB 单体 store；规则、收藏、历史、播放器设置、弹幕、URL 缓存、源健康和自动换源均在一处 | 功能丰富但职责过载；源执行、状态、持久化与 UI 编排耦合，不利于继续扩源和并行开发 |
| `src/lib/stores/anime.svelte.ts` | `anime_search_all` 流式接收结果；`playEpisode` 45 秒提取超时；失败后 Phase 1 并行搜索/匹配，Phase 2 串行提取；使用 `findBestEpisodeMatch` 避免错误集数 | 当前自动换源是可复用基础；需要移入统一 orchestrator，并把候选、错误和取消代际类型化 |
| `src/lib/components/anime/AnimePlayer.svelte` | HLS.js/原生 `<video>` 双模；10 秒元数据和 15 秒可播放帧看门狗；HLS 网络/媒体恢复；失败保进度换源 | 内部播放已达到 0.12.1 可继续增强的水平，不能在重构中退化 |
| `src/lib/components/anime/AnimePlayer.svelte` | 续播、倍速、长按倍速、片头片尾跳过、自动连播、PiP、全屏、亮度/音量/进度手势、弹幕、章节评论、下载、外部播放器 | 这些均应列为兼容验收项；新 source contract 不得迫使播放器了解 provider 私有字段 |
| `src/lib/components/anime/AnimePlayer.svelte` | 原生失败后可 iframe 源站、浏览器打开或外部播放器；iframe 有 12 秒看门狗 | 回退路径齐全，但缺少显式策略：何时允许 iframe、何时必须外部打开、何时因 DRM/授权直接停止内部提取 |

### 2.2 番剧 Rust 规则、提取与健康度

| 证据位置 | 当前能力 | 审计结论 |
| --- | --- | --- |
| `src-tauri/src/anime.rs` | `AnimeRule` 兼容 Kazumi JSON；支持 GET/POST、UA、Referer、XPath→CSS、线路和分集解析、验证码页检测 | 兼容层应保留，但 XPath 转换覆盖范围有限；规则失败必须归类为 parseChanged/verificationRequired，而不是泛化网络错误 |
| `src-tauri/src/commands/anime.rs` | 单源搜索 12 秒、全部源 10 秒、线路 15 秒硬超时；结果通过 `anime-search-result` 流式 emit | 已有后端超时与流式基础；前端 SourceSheet 直接逐源调用形成第二套并发编排，应收敛 |
| `src-tauri/src/commands/anime.rs` | `anime_verify_rule_webview` 打开可见验证窗口；初始化脚本可自动点击或执行规则自带脚本 | 0.12.1 必须收紧：默认只允许用户在可见 WebView 手动完成验证；不对不受信规则执行任意 `captchaScript`，不把挑战自动化当成支持能力 |
| `src-tauri/src/commands/anime.rs` | 源健康记录写入 `anime_source_health.json`，每源保留 20 条；提供 `anime_get_source_health` | 前端没有消费该摘要，且另写 `anime-source-health-v1`；应迁移为单一后端真源 |
| `src-tauri/src/video_extractor.rs` | 隐藏 WebView 嗅探 m3u8/mp4/mpd，超时后可 legacy 页面解析；返回 URL 与最终页面 URL | 仅可用于无 DRM、允许访问的媒体。新增 `unsupportedDrm`/`policyBlocked` 检测后必须停止解析，不得尝试解密或规避授权 |
| `src-tauri/src/video_proxy.rs` | 本地代理处理 Referer、Range、m3u8 playlist/segment/key/media URI 重写，并有目标 URL 校验测试 | 应继续作为内部播放数据面；需要把日志脱敏、域名策略、会话和错误码纳入统一 source API |

### 2.3 漫画页面、store 与阅读器

| 证据位置 | 当前能力 | 审计结论 |
| --- | --- | --- |
| `src/lib/components/ComicPage.svelte` | 普通漫画搜索默认并行聚合 MangaDex、包子、DM5、1kkk；每源分区显示加载/错误/空结果并可单源重试 | 已具备多源搜索雏形；源标签和选项仍硬编码，尚未由 registry/manifest 驱动 |
| `src/lib/components/ComicPage.svelte` | PicACG 作为独立 18+ 登录入口，不影响普通漫画搜索 | 隔离方向正确；0.12.1 不应把 PicACG 认证、评论、排行直接混进普通源协议 |
| `src/lib/stores/comic.svelte.ts` | `ComicProvider = mangadex/baozi/dm5/picacg`，并通过 ID 前缀路由；普通源 `auto` 用 `Promise.all` 并发 | provider 分支会随源数量线性膨胀；`_mangaDexResults` 实际已承载所有普通源结果，是历史命名债务 |
| `src/lib/stores/comic.svelte.ts` | MangaDex、包子返回图片数组；DM5/1kkk 返回章节网页 URL；PicACG 由 Rust API 返回图片 | 同一阅读器已有 images/web 两种数据形态，但没有统一 `ResolvedReadTarget`，store 必须知道每个 provider 的执行细节 |
| `src/lib/components/comic/ComicDetail.svelte` | 统一详情和章节列表；PicACG 额外展示收藏、点赞、评论、推荐 | 普通源详情已可共用；来源显示目前从分类/汉化组猜测，缺少真实 source identity |
| `src/lib/components/comic/ComicReader.svelte` | 纵向连续图片、懒加载、上一话/下一话；web URL 使用 sandbox iframe | 阅读器最小可用，但 `loadedCount` 仅计数未反馈；无页级恢复、缩放、分页/RTL、预取、失败重试、签名 URL 刷新、全屏和键盘导航 |
| `src/lib/stores/comic.svelte.ts` | 历史最多 100 条，只记录 `last_order/last_title` | 只能回到章节，不能恢复到页或滚动位置；跨源同名漫画也没有稳定作品身份 |

### 2.4 漫画源与 Tauri 网关

| 证据位置 | 当前能力 | 审计结论 |
| --- | --- | --- |
| `src/lib/sources/mangadexProvider.ts` | 公开 API 搜索、详情、章节、At-Home 图片；搜索内容评级限制为 safe/suggestive，章节偏好语言 | 当前最可维护的公共 API 型漫画源，应作为 adapter contract 的基准实现 |
| `src/lib/sources/baoziProvider.ts` | HTML 搜索/详情/章节解析，跨分页收集图片 | 站点解析器易随 DOM 变化；必须隔离、可禁用、可探测，不能成为通用 fetch 放行理由 |
| `src/lib/sources/dm5Provider.ts` | DM5/1kkk 搜索与详情解析，章节交给网页阅读 | 应明确标记 `readMode=web`；若站点禁止嵌入或条款不允许，应降为 external-only，而不是尝试规避 |
| `src-tauri/src/commands/manga.rs` | `manga_fetch_json/text` 仅允许固定 HTTPS host；包子请求有有限重试和 Referer | 固定 allowlist 安全性好，但不可无限扩展；应迁移为基于启用 manifest 的 connector allowlist，并保留 SSRF 防护 |
| `src-tauri/src/comic.rs`、`src-tauri/src/commands/comic.rs` | PicACG 签名 API、登录 token、分类/搜索/详情/章节/图片/收藏/评论 | 是独立认证 provider，不应继续承担“所有漫画源”的后端命名空间 |

### 2.5 扩展目录与外部运行时

| 证据位置 | 当前能力 | 审计结论 |
| --- | --- | --- |
| `src/lib/sources/sourceRegistry.ts` | 已定义 media type、capability、lifecycle、ecosystem、adoption strategy、license risk、auth、NSFW policy；登记 Kazumi、现有漫画源、Suwayomi、Komga、LANraragi、Kavita、扩展索引等 | 这是统一 manifest 的良好起点，但当前 `version` 和 active/planned 状态仍混有 0.11.8/0.12.0 文案，且没有运行时 health/policy 字段 |
| `src/lib/sources/extensionIndex.ts`、`extensionCatalog.ts` | 可归一 Tachiyomi/Keiyoushi/Aniyomi/Mangayomi 索引，区分 discoverable/requiresRuntime/nativeAdapterPlanned/unsupported，并可筛选、统计 | 当前只做只读目录模型；这是正确安全边界，不应把“发现到扩展”误报为“可在 MoePlay 执行” |
| `src/lib/sources/suwayomiConnector.ts` | 本地 4567 GraphQL 配置、探测、认证状态、已安装扩展/源只读查询、候选映射 | 尚未接 Tauri 权限、凭据安全存储、产品 UI、搜索/详情/章节/页面；不能在 UI 宣称已支持阅读 |
| `src/lib/sources/mangaRuntimeConnector.ts` | Suwayomi/Komga/LANraragi/Kavita 的地址、认证 header、probe 和 library 归一模型 | 仍是连接探测层，不是完整 adapter；适合作为 0.12.1 的外部运行时控制面基础 |
| 全仓引用 | 上述 registry/index/runtime 模块除相互引用和测试外，未被 `ComicPage`、`comicStore`、`AnimePage` 或 `animeStore` 消费 | **核心缺口：目录层与实际产品/执行层断开。** |

### 2.6 测试现状

- 已有 provider/parser 单测：MangaDex、包子、DM5/1kkk。
- 已有 registry、extension index/catalog、Suwayomi、runtime connector 单测。
- 已有番剧标题归一、搜索排序、集数解析和跨线路精确匹配单测。
- Rust 已覆盖 Kazumi 反序列化、URL join、验证码检测、漫画 host allowlist、视频代理 URL/m3u8 重写等局部逻辑。
- 有可选 live comic acceptance：四个公共源中至少两个可搜索；默认通过 `MOEPLAY_LIVE_TESTS=1` 才运行。
- 有 ignored live anime test：候选规则中至少一个能完成 search→roads。
- **缺失**：anime/comic store 编排测试、SourceSheet/AnimePlayer/ComicReader 组件测试、熔断/半开测试、进度迁移测试、Tauri command 契约测试和稳定的播放器/阅读器 E2E。

本次审计实际运行结果（2026-07-10）：

- `npx vitest run src/lib/sources src/lib/utils/animeSource.test.ts`：9 个文件通过、1 个 live 文件跳过；48 tests passed、1 skipped。
- `cargo test anime::tests`：3 passed。
- `cargo test commands::manga::tests`：1 passed。
- 未运行依赖公网的 live anime/live comic 验收。

---

## 3. 目标

### 3.1 产品目标

1. 用户从作品详情进入后，默认在 MoePlay 内完成番剧播放或漫画阅读；只有能力、策略或源站限制不允许时才退到网页/外部打开。
2. 多源结果和故障对用户可解释：显示来源、能力、健康、是否需登录/验证、为何被跳过以及可执行动作。
3. 番剧换源时保持作品、季/集和播放进度；漫画恢复时保持章节、页码和页内位置，不静默切到错误版本。
4. 当前 Kazumi、MangaDex、包子、DM5/1kkk、PicACG 能力不回退，同时为公开 API、自托管服务和外部运行时建立可扩展接口。

### 3.2 工程目标

1. 建立番剧/漫画共用的 `SourceManifestV2`、错误分类、健康快照、熔断状态和 resolved target 模型。
2. 将 source discovery/catalog、source execution、orchestration、player/reader 分层；页面和 store 不再包含新增 provider 的分支。
3. 源健康统一由 Rust 持久化，前端只通过 API 读取/订阅；移除 localStorage/Rust JSON 双写。
4. 每个 connector 有显式网络边界、认证模式、内容策略、执行模式和 kill switch。
5. 新增 adapter 时只需实现契约、注册 manifest 和测试，不需要修改播放器/阅读器核心分支。

### 3.3 可量化目标

- 已熔断源不参与自动搜索/换源，用户手动强制重试除外。
- 自动换源不得把编号明确的第 N 集匹配成其他集；无法确认时停止并要求用户选源。
- 番剧播放故障换源后恢复进度误差不超过 5 秒。
- 漫画重进阅读器恢复到正确章节，图片模式页码误差不超过 1 页，滚动模式恢复到目标页附近。
- 所有自动执行请求均受 connector host/scheme 策略和超时控制；不接受带 URL 凭据的目标，不在日志中输出 token/query secret。
- 任一源故障不得让整个媒体页面无限 loading；搜索、resolve、iframe 均有终态。

---

## 4. 非目标

1. 不在本专项中修改 `package.json`、Tauri bundle 或发布版本号。
2. 不实现或集成 DRM 解密、许可证提取、Widevine/FairPlay/PlayReady 绕过、付费墙绕过、账号共享或地域限制规避。
3. 不执行从 Tachiyomi/Mihon/Keiyoushi/Aniyomi/Mangayomi/Paperback 索引发现的 APK、Kotlin、Dart、JS 或任意第三方代码。
4. 不把验证码/Cloudflare challenge 自动化作为支持能力；只允许用户在可见页面手动完成站点允许的验证。
5. 不保证任意网页都能 iframe；禁止嵌入时应明确降级为系统浏览器/官方客户端。
6. 不在 0.12.1 完成所有自托管 connector。发布主线是统一架构和当前源迁移，外部运行时按优先级逐个完成。
7. 不重做 Bangumi 元数据、弹幕、PicACG 社交功能或统一下载任务中心；只保证新架构兼容现有调用。
8. 不做跨站点“同一作品”完全自动合并。低置信度时保留分源条目，不以误合并换取表面去重。

---

## 5. 合法性、信任与候选源优先级

### 5.1 接入门槛

每个源在进入 `active` 前必须记录：

- 软件/API 许可证与实现来源；是否复制代码、仅调用 API、仅参考契约或连接外部进程。
- 内容权利/服务条款状态：`approved`、`userProvided`、`unknown`、`restricted`。
- 认证方式、NSFW 策略、允许的 host/scheme、内部执行模式。
- 是否存在 DRM、付费或强账号授权；若存在，只能走源方允许的网页/外部路径。
- 维护责任人、健康探针、fixture、kill switch 和最后验证时间。

`licenseRisk=low` 仅表示软件许可证边界较低，**不等于内容来源天然合法**。

### 5.2 候选优先级

| 优先级 | 候选 | 0.12.1 定位 | 原因与边界 |
| --- | --- | --- | --- |
| P0 | **MangaDex 公共 API** | 迁移为标准 comic adapter，保留内部图片阅读 | 现有公开 API、结构化数据和 safe/suggestive 过滤，维护成本最低；仍需遵守 API 规则和内容删除/限流 |
| P0 | **用户本地/自托管内容入口** | 先完成统一 direct/image target 契约；具体 connector 优先 Komga 只读个人库 | 用户自有内容和自托管服务的权利边界最清晰；凭据仅存安全存储，不通过 iframe 暴露 |
| P0 | **KazumiRules 兼容层** | 迁移现有规则执行、健康、手动验证和网页回退，不扩大自动绕过能力 | 当前番剧主线，必须兼容；规则目录是 adapter 元数据，不代表每个站点自动获准启用。未知条款源默认需用户确认/可禁用 |
| P0 | **现有包子、DM5/1kkk、PicACG** | 仅做迁移、隔离、健康和 kill switch；不以 0.12.1 为由继续复制相近站点 parser | 保障现有用户，但 DOM parser/内容权利/稳定性风险高于公共 API；DM5/1kkk 不能嵌入时降为 external-only；PicACG 保持独立 18+ 入口 |
| P1 | **Komga** | 首选外部运行时端到端 pilot：probe→library→series/book→pages | 自托管、MIT、接口边界清晰；若 API 契约验证未完成，则 feature flag 下不宣称可读 |
| P1 | **Suwayomi 外部运行时** | 产品化连接状态、已安装源目录；搜索/阅读作为后续独立切片 | MoePlay 不内嵌扩展运行时；只连接用户自行运行的服务。不得自动安装扩展或把目录发现等同可读 |
| P1 | **LANraragi / Kavita** | 只读个人库 connector 候选 | 用户自托管优先；GPL 服务保持外部 API 边界，不复制实现 |
| P2 | **Keiyoushi、Tachiyomi/Mihon、Aniyomi、Mangayomi 索引** | 只读候选目录、语言/NSFW/Cloudflare/运行时需求筛选 | 用于发现和评估，不在 MoePlay 执行扩展。只有经单独许可与手写 adapter 评审后才可 active |
| P2 | **用户提供的无 DRM 直链/本地媒体** | 番剧新 source 类型候选 | 适合个人内容和测试；仅支持用户有权访问的普通文件/HLS，不代替认证或 DRM 客户端 |
| P3 | **Kotatsu、CloudStream 等高许可证/高执行风险生态** | 仅参考接口、错误分类和站点清单 | 不复制/链接其实现，不执行插件，不作为 0.12.1 可安装源 |
| 禁止 | 需要破解 DRM、绕过付费、伪造订阅、自动解挑战或规避地域限制的来源 | 不接入 | 返回 `unsupportedDrm` 或 `policyBlocked`，提供合法网页/官方客户端入口（若存在） |

---

## 6. 源接入分层架构

```text
AnimePage / ComicPage
        │
        ▼
Product stores（导航、选中作品、播放器/阅读器 UI 状态）
        │
        ▼
MediaSourceOrchestrator
  - 搜索聚合与取消代际
  - 作品/集/章匹配
  - 健康排序、熔断、回退
  - 进度迁移与用户选择
        │
        ▼
Adapter contracts
  AnimeSourceAdapter / ComicSourceAdapter
  - search/detail/units/resolve
        │
        ├── Native/public API adapters
        ├── Kazumi rule adapter
        ├── Authenticated provider adapter（PicACG）
        ├── Self-hosted runtime adapter
        └── Web/external-only adapter
        │
        ▼
Tauri source gateway
  - 网络策略 / allowlist / SSRF 防护
  - 凭据与 cookie 边界
  - timeout / rate limit / health events
  - video extractor + local proxy
  - image/text/json fetch
        │
        ▼
Player / Reader
  只消费 ResolvedPlaybackTarget / ResolvedReadTarget
```

### 6.1 Layer 0：Policy / Trust

- manifest 决定 source 是否可启用、允许的 host、认证、NSFW、内部模式与执行来源。
- `policyBlocked`、`unsupportedDrm`、`authRequired` 不得被重试器伪装成普通网络失败。
- 任意规则/索引数据均为不可信输入；不直接执行脚本。现有 `captchaScript` 默认禁用，只有内置受审 adapter 才可声明有限初始化动作。
- 自托管地址仅允许用户显式配置；Rust 按 connector 绑定固定 base URL，不接受每次调用任意目标 URL。

### 6.2 Layer 1：Manifest / Catalog

扩展 `sourceRegistry.ts`，使 manifest 同时承担目录和运行时声明：

- 稳定 `sourceId`、media type、capabilities、lifecycle、ecosystem、版本。
- `executionMode`：`native | rule | publicApi | externalRuntime | web | externalOnly`。
- `playModes/readModes`：播放器/阅读器可消费的模式。
- `contentPolicy`、`authMode`、`allowedHosts`、`requiresUserVerification`。
- `defaultEnabled`、`killSwitch`、`maintainer`、`lastVerifiedAt`。
- 目录项的状态必须区分 `discoverable` 与 `executable`。

### 6.3 Layer 2：Adapter Contract

- 番剧 adapter：`search → detail/roads → resolve episode`。
- 漫画 adapter：`search → detail → chapters → resolve chapter`。
- adapter 返回统一模型，不返回 UI 文案或直接操作页面状态。
- adapter 的网络访问必须通过注入的 gateway client，便于测试、限流和记录健康。
- provider 私有 ID 只存在于 `ProviderRef`，不再依赖字符串前缀在 store 中分派。

### 6.4 Layer 3：Execution Gateway

- Rust 统一处理 fetch、认证 header、cookie profile、超时、最大响应、重定向、域名策略和日志脱敏。
- `manga_fetch_json/text` 作为兼容 wrapper 保留一版；新 adapter 使用 `media_source_request` 或 connector 专用 command。
- 视频 extractor 在开始嗅探前先检查 source policy；检测到 EME/DRM 信号时终止并返回 `unsupportedDrm`。
- 图片和 HLS 代理使用短期 session token，不把真实凭据暴露给前端 URL。

### 6.5 Layer 4：Orchestration

- 搜索并发有全局上限和 per-host 上限；优先返回健康源结果，慢源继续流式补充。
- detail/units/resolve 具有 Abort/cancel generation，旧请求不能污染新作品状态。
- 自动换源基于作品身份、集/章 identity 和健康排序；低置信度不自动切。
- 同一集的 direct/HLS/native/web 是“同源模式回退”；跨 source 是“跨源回退”，两者日志和用户提示分开。

### 6.6 Layer 5：Player / Reader

- 播放器不再读取 `AnimeRule` 私有字段，只读取 `ResolvedPlaybackTarget`。
- 阅读器不再判断 `currentProvider`，只读取 `ResolvedReadTarget`。
- UI 始终显示当前 source、模式、健康和回退状态。

---

## 7. 数据模型草案

```ts
export type MediaKind = "anime" | "comic";
export type SourceExecutionMode =
  | "native"
  | "rule"
  | "publicApi"
  | "externalRuntime"
  | "web"
  | "externalOnly";

export type SourceErrorKind =
  | "network"
  | "timeout"
  | "rateLimited"
  | "authRequired"
  | "verificationRequired"
  | "notFound"
  | "parseChanged"
  | "emptyResult"
  | "mediaUnavailable"
  | "proxyFailure"
  | "iframeBlocked"
  | "unsupportedDrm"
  | "policyBlocked"
  | "cancelled";

export interface SourceManifestV2 {
  id: string;
  name: string;
  mediaKind: MediaKind;
  version: string;
  capabilities: Array<"search" | "detail" | "units" | "play" | "pages" | "web" | "external" | "download" | "verify">;
  executionMode: SourceExecutionMode;
  playModes?: Array<"direct" | "hls" | "web" | "external">;
  readModes?: Array<"images" | "web" | "external">;
  authMode: "none" | "token" | "basic" | "api-key" | "session";
  contentPolicy: "approved" | "userProvided" | "unknown" | "restricted";
  nsfwPolicy: "safe-only" | "hide-by-default" | "user-controlled" | "unknown";
  allowedHosts: string[];
  defaultEnabled: boolean;
  requiresUserVerification: boolean;
  runtimeRequired: boolean;
  killSwitch?: { disabled: boolean; reason?: string; revision?: string };
  provenance?: { homepage?: string; license?: string; adapterSource?: string; lastVerifiedAt?: string };
}

export interface ProviderRef {
  sourceId: string;
  remoteId: string;
  canonicalUrl?: string;
}

export interface MediaWorkRef {
  kind: MediaKind;
  title: string;
  aliases?: string[];
  year?: number;
  season?: number;
  language?: string;
  provider: ProviderRef;
  identityConfidence?: number;
}

export interface MediaUnitRef {
  provider: ProviderRef;
  label: string;
  number?: number;
  season?: number;
  volume?: number;
  language?: string;
}

export type ResolvedPlaybackTarget =
  | { mode: "direct" | "hls"; url: string; headers?: Record<string, string>; expiresAt?: number; webFallbackUrl?: string }
  | { mode: "web"; url: string; embeddable: boolean }
  | { mode: "external"; url: string; reason: string };

export type ResolvedReadTarget =
  | { mode: "images"; pages: Array<{ id: string; url: string; width?: number; height?: number }>; refreshToken?: string }
  | { mode: "web"; url: string; embeddable: boolean }
  | { mode: "external"; url: string; reason: string };

export type CircuitStatus = "closed" | "open" | "halfOpen" | "disabled";

export interface SourceHealthSnapshot {
  sourceId: string;
  operation: "search" | "detail" | "units" | "resolve" | "stream" | "page";
  status: CircuitStatus;
  score: number;              // 0..100
  consecutiveFailures: number;
  successRate: number;
  avgLatencyMs: number;
  lastSuccessAt?: number;
  lastFailureAt?: number;
  lastFailureKind?: SourceErrorKind;
  retryAfter?: number;
}

export interface MediaProgressV2 {
  work: ProviderRef;
  unit: ProviderRef;
  sourceId: string;
  mode: "play" | "read";
  positionMs?: number;
  pageIndex?: number;
  pageProgress?: number;      // 0..1
  scrollAnchor?: string;
  updatedAt: number;
}
```

### 7.1 兼容迁移

- `AnimeHistory` 原字段继续可读，首次保存时补 `sourceId/providerRef/unitRef`。
- `picacg-history` 迁移到 `media-progress-v2`；保留旧 key 至少一个版本，只读回退。
- 现有字符串前缀 ID 在 adapter 边界解析为 `ProviderRef`，不立即破坏收藏/历史。
- `AnimeRule` 保留为 Kazumi adapter 私有配置，不再作为全局 source model。

---

## 8. Tauri API 草案

新 API 采用统一命名；旧 command 在 0.12.1 保留并内部委托，以便渐进迁移。

```rust
#[tauri::command]
async fn media_source_list(media_kind: MediaKind) -> Result<Vec<SourceManifestV2>, SourceError>;

#[tauri::command]
async fn media_source_search(req: SourceSearchRequest) -> Result<Vec<SourceSearchSection>, SourceError>;

#[tauri::command]
async fn media_source_detail(source_id: String, provider: ProviderRef) -> Result<MediaWorkDetail, SourceError>;

#[tauri::command]
async fn media_source_units(source_id: String, provider: ProviderRef) -> Result<Vec<MediaUnitRef>, SourceError>;

#[tauri::command]
async fn media_source_resolve(source_id: String, unit: ProviderRef) -> Result<ResolvedMediaTarget, SourceError>;

#[tauri::command]
async fn media_source_health(source_ids: Option<Vec<String>>) -> Result<Vec<SourceHealthSnapshot>, SourceError>;

#[tauri::command]
async fn media_source_probe(source_id: String, force: bool) -> Result<SourceProbeResult, SourceError>;

#[tauri::command]
async fn media_source_set_enabled(source_id: String, enabled: bool) -> Result<(), SourceError>;
```

事件：

- `media-source-search-result`：单源流式结果。
- `media-source-health-changed`：熔断或半开状态变化。
- `media-source-resolve-progress`：视频提取/图片分页解析阶段。

兼容 wrapper：

- `anime_search`、`anime_search_all`、`anime_fetch_roads`、`anime_extract_video_url` 继续可用，逐步委托 Kazumi adapter。
- `comic_*` 继续服务 PicACG 私有功能。
- `manga_fetch_json/text` 继续服务旧 provider，但新 adapter 不再直接依赖任意 URL 参数。

---

## 9. 内部播放体验

### 9.1 保留能力

0.12.1 重构必须保留：HLS.js/原生双模、自动恢复、播放看门狗、续播、片头片尾、自动连播、倍速/长按倍速、手势、PiP、全屏、弹幕、章节评论、下载、选集、外部播放器和网页回退。

### 9.2 新行为

1. 播放器顶部显示“来源 · 模式 · 健康状态”，换源时显示旧源→新源。
2. resolve 进度使用后端阶段事件，不再仅用前端时间猜测“连接/嗅探/提取”。
3. 同源回退顺序：缓存 target → HLS.js/原生互换 → 刷新 target → web（仅 manifest 允许）→ external。
4. 跨源回退顺序：上次成功源 → 健康分最高且精确匹配的源 → 用户手动选源。
5. 跨源恢复携带目标集 identity 和 `positionMs`；匹配置信度不足时禁止自动播放。
6. `unsupportedDrm`、`policyBlocked`、`authRequired` 不进入提取重试循环；显示原因和合法动作。
7. iframe 仅在 `embeddable=true` 时尝试；超时或 CSP/X-Frame 限制后转 external，不反复重载。
8. 自动网页回退改为用户设置 + source policy 双重允许；不能因解析失败把 token 放入 iframe URL。

### 9.3 安全与隐私

- 播放 URL、Referer、cookie、query token 日志脱敏。
- 本地代理 URL 使用短期 session 标识，前端不可构造任意代理目标。
- 不向第三方页面注入 MoePlay 凭据。
- 规则自带脚本不默认执行；仅允许固定、审计过的初始化能力。

---

## 10. 内部阅读体验

### 10.1 阅读模式

- `images`：支持连续纵向、单页、双页；LTR/RTL；适宽/适高；缩放；背景；全屏。
- `web`：明确标注“源站网页阅读”，使用最小 sandbox；不可嵌入时直接 external。
- `external`：显示原因，不渲染空白阅读器。

### 10.2 进度与导航

- 保存章节、页索引、页内比例/scroll anchor、阅读模式和 source。
- 进入详情提供“继续阅读”，直接恢复上次位置。
- 键盘：左右/空格/PageUp/PageDown、Esc、Home/End；RTL 时方向映射可配置。
- 到章节末尾可自动进入下一章，但必须可关闭；预取下一章不提前写历史。

### 10.3 图片加载与回退

- 当前页优先，前后各预取有限页数；限制内存和并发。
- 单页失败显示原位 retry，不能只改 alt 文本。
- 对签名 URL 过期支持 adapter `refreshPages`，刷新后保持页索引。
- 同一 source 连续 page 失败可熔断 page operation；回到章节源选择，不把不同翻译/版本自动拼接。
- 跨源换章只在 work + chapter identity 高置信度时允许，且首次需用户确认。

### 10.4 iframe 收紧

- `allow` 去除与漫画无关能力；不默认允许 top navigation。
- token/session 不拼入可被页面读取的 URL；需要认证的网页源优先外部打开。
- 对来源未知或条款不明的网页源，默认 external-only。

---

## 11. 健康度、熔断与回退

### 11.1 单一真源

- 健康事件统一写 Rust 持久层；前端 localStorage 仅保留 UI 偏好，不再保存源健康。
- 维度：`sourceId + operation + optional host`。搜索成功不代表 resolve/stream 健康。
- 记录最近窗口和衰减摘要，不永久惩罚已修复源。

### 11.2 建议熔断规则

- `closed → open`：同 operation 连续 3 次失败，或最近至少 5 次中失败率 ≥ 60%。
- `open` 冷却：默认 10 分钟；429 按 `Retry-After`；parseChanged 可 6 小时；policyBlocked/kill switch 为 disabled。
- `open → halfOpen`：冷却后仅允许 1 个探测请求。
- `halfOpen → closed`：连续 2 次成功；失败则重新 open，并指数退避，上限 24 小时。
- `authRequired/verificationRequired`：只暂停该用户会话的自动请求，不降低所有用户共享的源质量；用户完成认证后手动 half-open。
- 用户可以“强制本次尝试”，但不得绕过 policyBlocked/unsupportedDrm。

### 11.3 健康评分

建议按 operation 单独计算：

- 最近成功率 45%。
- 最近成功时间 20%。
- 延迟 15%。
- 连续失败惩罚 15%。
- 验证/登录摩擦 5%。

上次对该作品成功的 source 可获得小幅偏好，但不能越过 open/disabled 或精确匹配约束。

### 11.4 回退状态机

**番剧**：

```text
resolve current source
  → refresh same target
  → alternate playback engine
  → permitted web target
  → exact-match next healthy source（preserve progress）
  → manual source sheet
  → external/open official page
```

**漫画**：

```text
load current page
  → retry page
  → refresh chapter page URLs
  → continue unaffected pages
  → mark page operation unhealthy
  → manual source/version choice
  → web/external target
```

所有路径必须在 UI 上进入 success/error/external 之一，禁止无限 spinner。

---

## 12. 文件级改动计划

> 下表描述后续实现；本次规划提交本身只修改本文件。

| 文件/目录 | 计划改动 |
| --- | --- |
| `src/lib/sources/sourceRegistry.ts` | 升级 `SourceManifestV2`，补 execution/read/play modes、content policy、allowed hosts、启用/kill switch、维护信息；修正版本基线文案 |
| `src/lib/sources/mediaSourceTypes.ts`（新） | 放统一 ProviderRef、work/unit、resolved target、错误、健康与进度类型 |
| `src/lib/sources/mediaSourceClient.ts`（新） | 封装统一 Tauri commands/event，避免 store 直接散落 `invokeCmd` |
| `src/lib/sources/mediaSourceOrchestrator.ts`（新） | 搜索聚合、取消代际、健康排序、熔断跳过、作品/集/章匹配与回退 |
| `src/lib/sources/adapters/comic/*`（新） | 为 MangaDex、包子、DM5/1kkk、PicACG 建统一 adapter wrapper；逐步迁移现有 provider 文件 |
| `src/lib/sources/adapters/anime/kazumiAdapter.ts`（新） | 对现有 anime commands 建前端 adapter，隔离 AnimeRule 私有字段 |
| `src/lib/sources/extensionIndex.ts`、`extensionCatalog.ts` | 保持只读目录，补 executable=false/policy 信息，禁止 UI 把候选显示为已安装可用 |
| `src/lib/sources/suwayomiConnector.ts`、`mangaRuntimeConnector.ts` | 接统一 manifest/health；凭据句柄化；先产品化 probe/catalog，再单独实现端到端查询 |
| `src/lib/stores/anime.svelte.ts` | 拆出源执行、健康、URL 缓存和 failover；保留产品状态/兼容 getter；停止 localStorage 健康双写 |
| `src/lib/components/anime/SourceSheet.svelte` | 改由 orchestrator 提供 section；显示健康/熔断/验证/模式；限制并发；支持强制本次尝试 |
| `src/lib/components/anime/AnimePlayer.svelte` | 只消费 resolved target；显示来源/模式；处理 unsupportedDrm/policyBlocked；保留现有播放能力 |
| `src/lib/components/AnimePage.svelte`、`AnimeDetail.svelte` | 增加 source settings/health 入口和继续观看信息，不改变现有推荐/日历/Bangumi 结构 |
| `src/lib/stores/comic.svelte.ts` | 移除新增 provider 分支需求；普通源由 registry/orchestrator 驱动；迁移页级历史；PicACG 私有社交能力保留独立模块 |
| `src/lib/components/ComicPage.svelte` | 源 tabs 改为 manifest 驱动；聚合结果支持健康、熔断、能力、单源重试和可解释排序 |
| `src/lib/components/comic/ComicDetail.svelte` | 显示真实 source identity；增加继续阅读、版本/源选择；普通源不再从分类猜来源 |
| `src/lib/components/comic/ComicReader.svelte` | 加页级进度、分页/RTL/缩放/全屏、预取、原位重试、refresh pages 和严格 web/external 模式 |
| `src-tauri/src/media_sources.rs`（新） | 统一 manifest、错误、health/circuit、policy 和 resolved target Rust 类型 |
| `src-tauri/src/commands/media_sources.rs`（新） | 注册 list/search/detail/units/resolve/health/probe/enabled commands 与事件 |
| `src-tauri/src/commands/anime.rs`、`src-tauri/src/anime.rs` | Kazumi adapter 委托、typed error、健康事件；验证窗口禁用不可信任脚本自动执行 |
| `src-tauri/src/commands/manga.rs` | 从固定通用 fetch 迁移到 connector policy；保留兼容 wrapper；加强重定向/响应大小/日志脱敏 |
| `src-tauri/src/commands/comic.rs`、`src-tauri/src/comic.rs` | PicACG 作为 authenticated adapter；token 不进入统一普通源 UI；错误类型化 |
| `src-tauri/src/video_extractor.rs` | source policy 前置、DRM/EME 不支持分类、阶段事件、取消和脱敏；不实现解密 |
| `src-tauri/src/video_proxy.rs` | session 化代理目标、域名绑定、错误事件；保留 m3u8/Range 能力 |
| `src-tauri/src/lib.rs`、`src-tauri/src/commands/mod.rs` | 注册新 commands/state，同时保留旧命令一个兼容周期 |
| `src/lib/sources/*.test.ts`、`src/lib/components/**/*.test.ts`、`src-tauri/src/** tests` | 增加契约、store、组件、熔断、迁移、安全和回退测试 |

---

## 13. 任务拆分（可独立并行）

### T01：统一契约与迁移规则

- Owner files：`mediaSourceTypes.ts`、`sourceRegistry.ts`、类型 fixture。
- 产出：manifest v2、错误、resolved target、progress v2、兼容转换函数。
- 依赖：无；所有任务的先决合约。
- 验收：类型测试覆盖现有 Kazumi/MangaDex/包子/DM5/PicACG manifest。

### T02：Rust policy + health + circuit 核心

- Owner files：新 `media_sources.rs`、`commands/media_sources.rs`、注册文件。
- 产出：单一健康真源、熔断状态机、启用/禁用、probe、脱敏错误。
- 依赖：T01 的序列化契约冻结。
- 可与 T03/T05 前端 adapter 并行，先用 mock command。

### T03：番剧 adapter 与 orchestrator 迁移

- Owner files：`kazumiAdapter.ts`、`mediaSourceOrchestrator.ts` 的 anime 部分、`anime.svelte.ts`。
- 产出：统一 search/roads/resolve、健康排序、精确集匹配、保进度换源、旧 getter 兼容。
- 依赖：T01；T02 可后接。
- 冲突约束：此任务独占 `anime.svelte.ts`，其他任务不在同一阶段改 store。

### T04：番剧 SourceSheet / Player UX

- Owner files：`SourceSheet.svelte`、`AnimePlayer.svelte`、必要的 AnimeDetail 小改。
- 产出：来源/健康/熔断展示、typed failure、DRM/policy 终态、阶段进度。
- 依赖：T03 的 store API。
- 必须以现有播放器能力回归清单为门槛。

### T05：漫画现有 provider adapter 化

- Owner files：comic adapter 新目录、现有 provider 文件、`comic.svelte.ts`。
- 产出：registry 驱动聚合、统一 detail/chapters/read target、移除新 provider 分支需求。
- 依赖：T01；可与 T03 并行。
- 冲突约束：此任务独占 `comic.svelte.ts`。

### T06：内部漫画阅读器 0.12.1

- Owner files：`ComicReader.svelte`、`ComicDetail.svelte`、progress helper。
- 产出：页级恢复、连续/单页/双页、RTL、缩放、预取、单页重试、web/external 终态。
- 依赖：T05 resolved read target。

### T07：外部运行时产品化

- Owner files：`suwayomiConnector.ts`、`mangaRuntimeConnector.ts`、独立设置/源目录组件。
- 产出：连接配置、probe、认证状态、只读源/library 目录；端到端 reader pilot 优先 Komga。
- 依赖：T01/T02；不阻塞 T03-T06。
- 发布规则：只有完成 search/detail/units/resolve 全链路的 connector 才标 `active/executable`。

### T08：安全边界收紧

- Owner files：`commands/manga.rs`、`commands/anime.rs` 验证窗口、`video_extractor.rs`、`video_proxy.rs`。
- 产出：manifest allowlist、SSRF/重定向/大小限制、凭据句柄、日志脱敏、DRM/policy 分类、禁用不可信脚本。
- 依赖：T01/T02；可与 UI 并行。

### T09：测试、迁移与发布验收

- Owner files：新增测试/fixture，不直接重写 store 实现。
- 产出：测试矩阵、旧历史迁移、live 诊断、手动 Tauri checklist、feature flag rollback 验证。
- 依赖：各任务提供稳定接口；可从 T01 开始持续补。

### 并行建议

- Wave 1：T01。
- Wave 2：T02、T03、T05、T07、T08 并行。
- Wave 3：T04、T06，T09 集成。
- Wave 4：live acceptance、性能、回滚演练和 release gate。

---

## 14. 验收标准

### 14.1 通用

- [ ] 页面展示的 source 列表来自 registry，不再由 AnimePage/ComicPage 硬编码新增源。
- [ ] 每个 source 显示 capability、执行模式、启用状态和健康状态。
- [ ] `discoverable/requiresRuntime` 目录项不会被标成“可播放/可阅读”。
- [ ] source 被 kill switch/熔断后自动流程跳过，用户可查看原因。
- [ ] 搜索、detail、units、resolve、iframe 都有明确 timeout 和终态。
- [ ] `unsupportedDrm`/`policyBlocked` 不重试、不换解析器、不进入代理。
- [ ] 前端不保存源健康真值；重启后 health/circuit 一致。
- [ ] 网络命令受 scheme/host/重定向/响应大小限制，日志无敏感 query/token。

### 14.2 番剧

- [ ] 当前已安装 Kazumi 规则仍可搜索、加载线路和选择具体集。
- [ ] 全源搜索流式显示；熔断源不占用自动并发槽。
- [ ] 精确编号集找不到时不播放第 1 集或相邻集。
- [ ] 提取失败、黑屏、断流均能进入同源恢复→跨源恢复→手动/网页/外部终态。
- [ ] 跨源恢复后进度误差 ≤ 5 秒，并更新历史到新 source/unit。
- [ ] HLS.js/原生切换、自动连播、片头片尾、倍速、手势、PiP、全屏、弹幕、评论、下载、外部播放器无功能回退。
- [ ] 验证窗口由用户手动完成；不受信规则脚本不会自动执行。

### 14.3 漫画

- [ ] MangaDex、包子、DM5、1kkk 仍能独立搜索并显示分源状态；一个源失败不阻塞其他源。
- [ ] PicACG 仍保持独立 18+ 登录入口和私有功能。
- [ ] ComicPage/ComicReader 不再通过新增 `if provider === ...` 才能支持新 adapter。
- [ ] 图片 source 在 MoePlay 内阅读；web source 明确标记且不可嵌入时转 external。
- [ ] 继续阅读恢复到正确章节和目标页附近；切模式后进度不丢。
- [ ] 单页失败可原位重试，刷新签名 URL 不改变页索引。
- [ ] 支持连续、单页、双页、LTR/RTL、适宽/适高、缩放和键盘导航。
- [ ] 不自动把不同语言/版本章节拼接为一章。

### 14.4 外部运行时

- [ ] Suwayomi/Komga 等连接状态能区分 online/authRequired/offline/schemaMismatch。
- [ ] 凭据不以明文 localStorage 作为最终方案；前端只持有配置状态/credential handle。
- [ ] 未完成 resolve 全链路的 runtime 只显示“已发现/需要运行时”，不进入普通搜索默认源。
- [ ] 至少一个自托管 connector pilot 通过端到端 contract 后，才可作为 0.12.1 新 active source；优先 Komga。

---

## 15. 测试矩阵

| 层级 | 范围 | 关键用例 | 执行方式 |
| --- | --- | --- | --- |
| TS 单元 | manifest/types | 旧 manifest→v2、capability、policy、候选不可执行状态 | Vitest |
| TS 单元 | orchestrator | 并发上限、取消代际、流式顺序、熔断跳过、手动强制、精确集/章匹配 | Vitest + fake clock |
| TS 单元 | providers | MangaDex/包子/DM5 fixture、DOM 变化、空结果、429、签名 URL 刷新 | Vitest，延续现有 injected fetcher |
| TS 单元 | progress | AnimeHistory/picacg-history 迁移、页级恢复、跨 source identity | Vitest |
| Svelte 组件 | SourceSheet | per-source 状态、captcha/auth、open/halfOpen、选线路/选集、不闪旧数据 | Testing Library Svelte |
| Svelte 组件 | AnimePlayer | HLS/native fallback、watchdog、typed failure、保进度换源、web/external 终态 | mock Hls + fake timers |
| Svelte 组件 | ComicPage | registry 驱动 tabs、聚合流式结果、单源 retry、熔断提示 | Testing Library Svelte |
| Svelte 组件 | ComicReader | 连续/分页/RTL、页错误 retry、refresh pages、恢复位置、键盘 | happy-dom + mocked observers |
| Rust 单元 | health/circuit | 阈值、429 Retry-After、half-open、auth 不污染全局、持久化重载 | `cargo test` |
| Rust 单元 | policy | scheme/host、redirect、credentials、private network connector、大小限制、日志脱敏 | `cargo test` |
| Rust 单元 | anime | Kazumi parse、验证码分类、脚本禁用、typed error、URL join | `cargo test anime` |
| Rust 单元 | proxy/extractor | m3u8/Range、session target、DRM/EME 终止、取消、过期 | `cargo test video_*` |
| Command contract | Tauri IPC | list/search/detail/units/resolve/health 序列化与旧 wrapper 兼容 | Rust/TS contract fixtures |
| Live opt-in | 漫画 | 当前源至少两个 search 健康；active 新 connector 全链路 | `MOEPLAY_LIVE_TESTS=1` |
| Live opt-in | 番剧 | 至少一个已允许规则 search→roads→无 DRM resolve 或明确 web/external | ignored/手动启用 |
| E2E | Tauri 桌面 | 详情→内部播放/阅读→故障→换源→恢复→重启续播/续读 | Playwright/Tauri 手动或驱动 |
| 性能 | 多源 | 20/50 源目录、6 个启用源搜索、图片长章内存、代理并发 | 指标脚本 + 手动 profile |

发布前命令：

```powershell
npm run check
npm run test:unit
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

公网 live 测试必须可选，不应让普通 CI 因第三方源波动随机失败；失败时输出每源诊断而非只有总断言。

---

## 16. 风险与回滚

| 风险 | 缓解 | 回滚 |
| --- | --- | --- |
| 大 store 拆分引发播放器/阅读器回归 | 兼容 getter/action；先 adapter 包装再迁移 UI；组件回归测试 | `media_source_v2` feature flag 切回旧 store 路径 |
| 源站 DOM/API 变化 | fixture + live probe + parseChanged 熔断 + per-source kill switch | 禁用单源，不回滚整版 |
| 健康误判导致可用源被熔断 | operation 分维度、half-open、用户强制本次尝试、短窗口衰减 | 清除单源 health/circuit，不删除用户数据 |
| 跨源误匹配错误集/章 | 编号/季/语言/标题多字段匹配；低置信度禁止自动切 | 回到手动 SourceSheet/版本选择 |
| localStorage/Rust 健康迁移冲突 | 后端为新真源；旧 localStorage 只做一次性导入，记录 migration version | 删除新 health 文件/表并重新从事件积累；不影响收藏历史 |
| iframe/CSP/认证泄漏 | manifest embeddable、严格 sandbox、credential handle、external fallback | 全局关闭 web embed，仅保留 external |
| SSRF 或任意代理 | connector 固定 base URL、scheme/host/redirect 校验、session token | 禁用新 gateway，保留当前固定 allowlist wrapper |
| 第三方条款或内容权利不清 | contentPolicy、默认禁用、来源说明、kill switch、只接公开/自托管优先 | 远程/本地 kill switch 立即禁用源并移出默认搜索 |
| GPL/MPL 等边界 | 外部进程/API 边界；不复制实现；索引只读 | 移除 connector/catalog 项，不影响核心 |
| 并行代理互相覆盖 | 按 T01-T09 owner files 分区；store 文件单 owner；新增文件优先 | 不回退他人提交，基于最新分支重放小提交 |
| 新 connector 未完成却被宣传 | executable gate 必须通过全链路 contract | 降级为 discoverable/requiresRuntime，不在默认源出现 |

数据库/持久化变更只做可加可回滚迁移，不在 0.12.1 删除旧历史 key；至少保留一个版本读取兼容。

---

## 17. 预计里程碑

按 2–3 名开发者并行估算，约 **15–21 engineer-days，8–12 个工作日历天**；单人约 3–4 周。

| 里程碑 | 时间 | 交付 |
| --- | --- | --- |
| M0：契约冻结 | Day 1–2 | T01；SourceManifestV2、错误、target、progress、兼容 fixture |
| M1：控制面与安全 | Day 3–5 | T02/T08；health/circuit、policy gateway、typed errors、兼容 commands |
| M2：番剧迁移 | Day 3–7（并行） | T03/T04；Kazumi adapter、SourceSheet、播放器 typed target 与回退 |
| M3：漫画迁移与阅读器 | Day 3–8（并行） | T05/T06；当前源 adapter 化、页级进度和阅读模式 |
| M4：外部运行时 pilot | Day 5–9（并行，非阻塞核心） | T07；连接 UI、只读目录；Komga 优先端到端候选 |
| M5：集成与回滚演练 | Day 9–12 | T09；全量 tests、live opt-in、Tauri 手动验收、性能、feature flag rollback |

### 17.1 Release gate

0.12.1 可以发布的最低条件：

1. 当前番剧/漫画源完成统一契约迁移或兼容 wrapper 接入。
2. health/circuit 单一真源生效。
3. 番剧内部播放和漫画内部图片阅读无现有功能回退。
4. 漫画页级进度与失败重试可用。
5. DRM/付费/策略错误有明确终态，不进入绕过流程。
6. 单元/组件/Rust 契约测试通过，Tauri 手动验收和回滚演练通过。
7. 外部运行时若未完成端到端，只能以“连接/目录预览”发布，不计为可阅读源。

---

## 18. 关键决策记录

1. **统一的是契约与控制面，不强行统一所有 provider 私有业务。** PicACG 社交、Bangumi 元数据保持独立能力。
2. **播放器/阅读器只消费 resolved target。** provider、规则、认证和网页细节留在 adapter/gateway。
3. **扩展索引是目录，不是执行器。** MoePlay 0.12.1 不运行第三方扩展代码。
4. **健康度按 operation 分维度且由 Rust 持久化。** 解决当前 localStorage 与 JSON 双写漂移。
5. **内部优先但不强行内部化。** DRM、授权、禁止嵌入或条款限制时，合法网页/外部入口是正确结果。
6. **新源优先公开 API和用户自托管。** 站点 HTML parser 只维护现有兼容面，不以数量作为成功指标。
7. **低置信度不自动跨源。** 宁可让用户选择，也不播放错集或拼错章节。
