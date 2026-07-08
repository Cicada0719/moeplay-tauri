# Changelog

## 0.12.0 - 2026-07-08

### 漫画、番剧播放与综合记录发布版
- 普通漫画页升级为默认入口：支持 MangaDex、DM5、1kkk 多源搜索、详情和章节阅读，PicACG 调整为右下角 `18+` 独立入口，避免普通漫画状态混入成人源结果。
- 修复 DM5 / 1kkk 章节解析：详情页章节数以真实章节列表为准，支持 `/manhua-xxx/`、`/manhua12345/`、`/m123/`、`/ch0-123/` 等路径，并避免推荐漫画章节误计入。
- 优化漫画详情页：展示更完整简介、来源、状态、真实章节数和最新章节；普通漫画隐藏 PicACG 专属点赞、收藏、评论和推荐。
- 增强 Kazumi 规则兼容与番剧网页播放兜底：提取失败、iframe 被阻止、反爬验证和源健康度均进入可操作流程，减少黑屏/灰屏卡死。
- 优化记录仪表盘：整合游戏游玩、番剧观看和漫画阅读历史，新增最近继续、媒体占比、番剧/漫画最近记录与全屏响应式布局。
- 补齐漫画运行时/扩展生态基础：加入 Keiyoushi、Suwayomi、Komga、LANraragi、Kavita、Mangayomi、Paperback、MangaDex 等源目录和连接器草案。
- 统一版本号至 `0.12.0`。

## 0.11.9 - 2026-07-08

### 普通漫画阅读器与多源入口
- 新增漫画源生态模型：补入 Keiyoushi、Suwayomi、Komga、LANraragi、Kavita、Mangayomi、Paperback、MangaDex 等通道，并记录连接器类型、索引格式、认证方式、NSFW 策略和运行时要求。
- 扩展扩展索引归一：Tachiyomi/Mihon 索引兼容 Keiyoushi，新增 Mangayomi `index.json` 归一，保留 `typeSource`、`sourceCodeUrl`、`apiUrl`、Cloudflare 与 NSFW 信息，不执行第三方插件代码。
- 新增漫画运行时连接器草案：为 Suwayomi、Komga、LANraragi、Kavita 提供只读探测、端点构建、认证头和库列表归一，GPL 服务保持外部 API 边界。
- 漫画页默认改为普通漫画阅读器：MangaDex 作为首个普通漫画源，搜索结果可直接进入详情、章节和统一漫画阅读器。
- 新增 `manga_fetch_json` Tauri 命令：普通漫画源请求走 Rust 侧白名单 HTTP，规避 WebView CORS，同时只允许 MangaDex API/图片域名。
- PicACG 不再作为漫画页门禁：改为右下角 `18+` 胶囊入口，点击后才弹出登录与 PicACG 搜索，和普通漫画搜索隔离。
- PR CI 调整：完整 Tauri 打包、portable 和发布 artifact 校验仅在 push/release 路径执行，PR 保留前端、Rust、Playwright 质量门。
- 统一版本号至 `0.11.9`。

## 0.11.8 - 2026-07-07

### Kazumi 规则兼容与网页播放兜底
- 补齐 Kazumi 规则字段兼容：`api` 支持字符串/数字，`antiCrawlerConfig` 完整持久化，`useWebview` 不再默认强制网页播放，只有 `useNativePlayer === false` 的源默认进入网页播放器。
- 统一番剧规则 URL 拼接：搜索、线路和播放地址共用 URL join helper，支持绝对 URL、协议相对 URL、根路径、相对路径和空路径，修复部分动漫网页打不开的问题。
- 搜索和线路请求接入反爬检测：命中 `antiCrawlerConfig` 时返回 `captchaRequired` 分层错误；SourceSheet 会显示“需要验证”，并可只对当前源执行“验证并重试”。
- 新增源站验证 WebView 命令：支持打开独立验证窗口，并按 Kazumi 配置尝试自动点击按钮或执行验证脚本；关闭窗口不会阻塞其它源。
- 正式化网页播放失败面板：iframe 加载超时或被源站禁止嵌入时，不再无限黑屏，改为提供重试原生提取、刷新网页、换源、外部浏览器打开和外部播放器播放。
- 新增源健康度记录：记录每个规则最近播放结果、失败类型、提取耗时和连续失败次数，自动换源优先选择最近成功、低失败率、非反爬的源。
- 优化视觉背景：全部游戏库底部改为黑绿纯色渐变背景；番剧网页播放器使用更稳定的深色加载背景，减少源站背景和应用 UI 混杂。
- 统一版本号至 `0.11.8`。

## 0.11.7 - 2026-07-07

### 番剧播放提取与全屏稳定性修复
- 增强 Kazumi 风格视频地址嗅探：补充 PerformanceResourceTiming、Response.text、MediaSource、WebSocket 等路径，并在命中候选地址后保留短暂 settle 窗口，兼容需要播放页激活回调的源站。
- 修复播放源提取卡死：前端提取和自动换源增加超时保护、代际校验和错误兜底，避免一直停在“连接播放源”或灰屏/黑屏转圈。
- 优化网页播放兜底：规则声明 WebView / 非原生播放时直接使用源站播放器；提取失败或超时时可自动切换网页播放。
- 强化本地视频代理：补充代理端口查询命令及 Tauri ACL 权限，播放代理保留正确 Referer / Origin，降低防盗链导致的空白播放概率。
- 修复播放器灰屏/黑屏：HLS.js 与原生 video 增加元数据和可播放帧双阶段 watchdog，坏缓存会失效并触发重试或网页兜底。
- 重做播放器全屏路径：使用 Tauri 窗口级全屏作为主逻辑，iframe 增加 fullscreen / autoplay / encrypted-media / picture-in-picture 权限，`F` / `Esc` 与“返回详情”会正确恢复窗口状态。
- 统一版本号至 `0.11.7`。

## 0.11.6 - 2026-07-07

### 全仓代码质量与可维护性优化
- **Rust 格式化全量达标**：对 `src-tauri` 全部源码执行 `cargo fmt`，消除历史累积的格式差异，CI `cargo fmt --check` 通过。
- **Clippy 零警告**：修复 10 条 clippy lint（m3u8 解析手写切片改用 `strip_prefix`、分片合并/下载参数 `&PathBuf` → `&Path`、`sort_by_key`、刮削任务 `JoinHandle` 超长类型抽取为类型别名 `ScrapeJoinHandle`、嗅探脚本借用优化），`cargo clippy -- -D warnings` 通过。
- **前端调试日志治理**：新增 `src/lib/utils/debug.ts` 的 `debugLog`，将番剧播放 / 换源链路 28 处 `console.log` 改为仅在开发环境（`import.meta.env.DEV`）输出，生产构建经摇树消除，运行时控制台不再有噪声。
- 统一版本号至 `0.11.6`（package.json / Cargo.toml / tauri.conf.json）。

## 0.11.5 - 2026-07-07

### 导入体验与首页个性化增强
- 首次启动向导的本地文件夹导入重构为「扫描预览 → 勾选确认 → 导入」：支持全选、标记已存在候选、可重新扫描、可取消不需要的候选；预览阶段不展示压缩包候选。
- 游戏卡片新增删除（二次确认，不移除本地文件）与收藏 / 取消收藏操作。
- 设置页新增「首页看板娘」开关，支持选择自定义图片（png/jpg/jpeg/webp/svg）；系统命令新增看板娘图片选择对话框。
- 大屏模式（Big Picture）首页新增「开始游戏 / 收藏 / 详情」操作按钮。
- 导入扫描增强：新增常见 galgame 汉化 / 补丁 / 转区工具名过滤（汉化、中文化、补丁、修正、繁体、简体、转区），并补充对应单元测试。
- 统一版本号至 `0.11.5`。

## 0.11.4 - 2026-07-07

### 汉化 exe 治理与首页视觉小升级
- 扫描器增加中文常见补丁/汉化/转区关键词过滤（`汉化`、`中文化`、`chinese`、`cn`、`补丁`、`patch`、`修正`、`繁体`、`简体`、`locale`、`转区`），减少汉化工具 exe 被误识别为独立游戏。
- 游戏库网格/封面墙卡片新增右键菜单，支持删除游戏；同时保留 Shift+Delete 快捷键，删除前二次确认。
- 首次启动向导的本地文件夹导入改为“扫描预览 → 勾选确认 → 导入”流程，可取消不需要的候选。
- 首页“最近游戏”底部增加更强的渐变遮罩，信息区改为玻璃卡片；右下角新增可开关/可替换的二次元看板娘立绘。
- 设置页增加“首页看板娘”开关与自定义图片路径选择。
- 统一版本号至 `0.11.4`。

## 0.11.3 - 2026-07-05

### Phase D/E/F · 依赖安全、后端健壮性与剩余 UI 基元化
- 统一版本号至 `0.11.3`，重建 `package-lock.json`。
- 升级 `vite` 6.4.3、`@sveltejs/vite-plugin-svelte` 5.1.1、`@tauri-apps/cli` 2.11.4、`@tauri-apps/api` 2.11.1；`npm audit` 0 vulnerabilities。
- 安装 Visual Studio Build Tools 2022 + C++ 工作负载，解决 Git Bash `link.exe` 冲突，恢复 Windows release 构建。
- 修复 Rust panic 点：
  - `downloader.rs` 测试写入错误使用 `expect`。
  - `anime_download.rs` 信号量 `acquire_owned` 失败时优雅跳过。
  - `comic.rs` HTTP client 构建失败回退到默认 client；HMAC 签名改为返回 `Result`。
  - `db.rs` 内存库打开失败时再尝试临时文件库兜底。
- 修复 `archive.rs` 测试在 `zip` 2.4 下的类型推断。
- 修复 Tauri v2 ACL 能力文件格式：自定义命令权限从 `app:<snake_case>` 改为 `allow-<kebab-case>`，并在 `build.rs` 中显式声明 275 条命令，使 release 构建通过 ACL 校验。
- 修复前端状态/错误处理：
  - `settingsStore.load()` 增加 catch 回退默认设置。
  - `addWatchDir` / `removeWatchDir` 增加错误处理。
  - `gameLibrary.batchDelete` 仅移除后端确认删除成功的条目。
  - `animeStore` 换源自愈增加集数标题数字匹配，降低集数错位概率。
- 子代理并行迁移剩余页面到 UI 基元：
  - `AnimePage.svelte`、`anime/AnimeDetail.svelte`、`anime/SourceSheet.svelte`、`anime/SearchDrawer.svelte`
  - `ComicPage.svelte`、`comic/ComicDetail.svelte`、`comic/ComicCard.svelte`、`comic/ComicReader.svelte`
  - `GameDetailPage.svelte`、`PlatformImportPage.svelte`
  - `StatsPage.svelte`、`BackupPage.svelte`、`DiagnosticsPage.svelte`
  - `DownloadPage.svelte`、`switch/SwitchHome.svelte`、`switch/SystemDock.svelte`
- 清理迁移产生的未使用 CSS，保持 `npm run check` 0 errors / 0 warnings。
- 完整 Windows 安装包产出：MSI、NSIS setup、便携 zip 与 `release-manifest.json`。

## 0.11.2 - 2026-07-05

### Phase C · 核心工具页 UI 基元重构
- 重做 `ContinueHub.svelte` / `ContinueCard.svelte`：
  - 统计区改用 `ui/StatBlock.svelte` Bento 网格（6/3/2 列响应式）。
  - 顶部继续卡片使用 `ui/Card.svelte`，支持聚焦与键盘 Enter 打开。
  - 类型筛选改用 `ui/SegmentControl.svelte`，标签内显示各类型计数。
  - `ContinueCard` 基于 `ui/Card` + `ui/Tag`，hover 显示操作图标。
  - 空状态接入 `ui/EmptyState.svelte`，操作按钮统一为 `ui/Button`。
  - GSAP 入场动画仅在首次挂载时触发一次，筛选切换不再重排。
- 重构 `SettingsPage.svelte`：
  - 各设置章节使用 `ui/Card.svelte` 包裹，统一 padding 与 hover 反馈。
  - 主题、NSFW、同步优先级、播放器倍速等改用 `ui/SegmentControl.svelte`。
  - 所有开关（数据源、自动刮削、AI、自启动、播放器选项）统一使用新增 `ui/Switch.svelte`。
  - 文本输入框（代理、AI 地址/Key/模型、Bangumi Token）统一使用新增 `ui/Input.svelte`。
  - 扫描目录列表项使用 `ui/Card.svelte`；操作与关于区域按钮全部使用 `ui/Button`。
- 打磨 `DiscoveryPage.svelte` / `DiscoveryDetail.svelte`：
  - 新增 `DiscoveryCard.svelte`，基于 `ui/Card` + `ui/Tag`，复用封面、来源徽章、评分标签。
  - 顶部 Tab 使用 `ui/SegmentControl.svelte`；搜索框使用 `ui/SearchInput.svelte` + `ui/Button`。
  - 搜索结果横向轨道使用 `ui/Rail.svelte`，加载与空状态使用 `ui/EmptyState.svelte`。
  - `DiscoveryDetail` 内部标签/类型使用 `ui/Tag`，操作按钮使用 `ui/Button`，资源卡片使用 `ui/Card`。
- 优化 `SmartCollectionEditor.svelte`：
  - 名称输入使用 `ui/Input.svelte`。
  - 图标选择使用 `ui/Tag.svelte`；快速筛选与状态使用 `ui/SegmentControl.svelte`。
  - 已安装 / 已玩过开关使用 `ui/Switch.svelte`；保存/取消/删除按钮使用 `ui/Button`。
- 新增通用 UI 基元：`ui/Input.svelte`、`ui/Switch.svelte`。
- 新增 `Input.test.ts`、`Switch.test.ts` 单元测试，当前共 107 个测试。

## 0.11.1 - 2026-07-05

### Phase B · 首页 Bento、卡片与 Store 拆分
- 新增 `src/lib/components/home/HomeBento.svelte`：首页改为 Bento Grid 布局，含精选游戏 Hero、最近游戏 Rail、游戏库统计、继续入口、全部游戏、随机推荐六个 widget。
- `SwitchHome.svelte` 接入 `HomeBento`，保留动态背景、顶部栏、搜索与全库网格模式。
- 重构 `GameCard.svelte`：基于 `ui/Card.svelte` 容器，优化信息层级，保留 grid/compact/list 三种形态、NSFW、收藏、多选、hover 上浮动效。
- `Card.svelte` 增加 `ref`、`onkeydown`、`focusable` 支持，focusable 时自动补全 `role="button"` 与 `tabindex`。
- 拆分 `src/lib/stores/games.svelte.ts`：
  - `gameLibrary.svelte.ts` 负责数据、筛选排序、Smart Collection。
  - `gameSelection.svelte.ts` 负责选中与批量选择。
  - `games.svelte.ts` 保留为兼容门面，组件 import 路径与 `gameStore` API 不变。
- 新增 `gameLibrary.test.ts`、`gameSelection.test.ts` 单元测试。
- 重构 `BigPicturePage.svelte`：拆分为 `BigPictureBackground`、`BigPictureWheel`、`BigPictureHero`、`BigPictureMediaTab` 子组件，交互与手柄逻辑保持不变。
- 修复 `continue.svelte.ts` 在静态导入场景下的 `$effect` 作用域错误，改为由 `App.svelte` 调用 `continueStore.start()` 启动。
- `Icon.svelte` 新增 `dice` 图标。

## 0.11.0 - 2026-07-05

### Phase A · 重塑基础（UI 组件、API 核心、后端安全）
- 新增通用 UI 组件：`Card`、`SearchInput`、`SegmentControl`、`EmptyState`、`LoadingSkeleton`、`BackgroundLayer`、`Tooltip`。
- `Icon.svelte` 补齐 `user`、`settings`、`image`、`square`、`calendar` 图标。
- `Dialog.svelte` 接入焦点陷阱 action，打开时聚焦、Tab 循环、关闭后恢复焦点。
- 新增 `src/lib/actions/focus-trap.svelte.ts` 焦点管理工具。
- 拆分 `src/lib/api/index.ts`：类型定义迁移至 `src/lib/api/types.ts`，新增 `src/lib/api/core.ts` 统一 invoke 封装与测试 mock 注入点。
- 将 `stores/anime.svelte.ts`、`stores/comic.svelte.ts`、`components/anime/AnimePlayer.svelte`、`components/anime/SourceSheet.svelte`、`components/StatsPage.svelte` 中的直接 `invoke` 调用迁移至 `invokeCmd`。
- 新增 `src/lib/testing/vitest-setup.ts` 与组件测试 harness；新增 `Button`、`Dialog` 组件测试（当前共 83 个单元测试）。
- 后端安全 P0：新增 `src-tauri/src/security.rs` 路径作用域校验；修复 `src-tauri/src/archive.rs` Zip Slip；`downloader.rs` / `anime.rs` / `comic.rs` / `gal_download.rs` 等默认启用 TLS 校验，仅 `MOEGAME_INSECURE_TLS=1` 时关闭。
- 新增 `src-tauri/src/http_client.rs` 统一 HTTP 客户端构造。
- `commands/system.rs::open_path` 与 `commands/saves.rs::backup_save` / `restore_save` 增加路径白名单校验。
- `capabilities/default.json` 从通配权限改为显式列出全部 275 个自定义命令；新增 `capabilities/sensitive.json` 单独声明高风险命令。
- 新增 `playwright.config.ts` 与 `tests/visual/smoke.spec.ts` 视觉测试骨架。

## 0.10.12 - 2026-07-05

### Phase 1 前端基础清理
- 删除已确认无引用的废弃组件：`LibraryView.svelte`、`HeroArea.svelte`、`GameDetail.svelte`、`StatusBadge.svelte`、`Modal.svelte`、`Toast.svelte`、`Topbar.svelte`。
- 将 `BigPictureDetail.svelte`、`EmulatorImportDialog.svelte`、`GameDetailPage.svelte`、`MigrationPage.svelte`、`PlatformImportPage.svelte`、`SavePanel.svelte`、`SettingsPage.svelte` 的按钮统一迁移至 `src/lib/components/ui/Button.svelte`，删除旧版 `src/lib/components/Button.svelte`。
- 修复剩余 6 个 `svelte-check` a11y 警告，当前 `npm run check` 0 errors / 0 warnings。
- `Icon.svelte` 新增 `ariaHidden`、`ariaLabel`、`role` 属性，默认对装饰性图标隐藏。
- 完善浅色主题令牌（`--bg`、`--bg-void`、`--text-dim`、`--accent-ring`、阴影与玻璃变量）。
- 补齐纯黑/高对比主题缺少的 `--glass-blur`、`--accent-pink*` 等别名与语义色。
- 新增 `sakura` 主题 CSS 令牌。
- 新增共享底层组件 `Overlay.svelte` 与 `Dialog.svelte`，`SmartCollectionEditor.svelte` 已接入 `Dialog`。
- 修复 Windows 下 Git Bash 小写盘符导致 `vitest` 报 `Cannot read properties of undefined (reading 'config')` 的问题，调整 `test:unit` 脚本与 `vitest.config.ts`。

## 0.10.11 - 2026-07-05

### URL hash 路由持久化
- 新增 `src/lib/stores/router.svelte.ts`：hash 解析、视图白名单、生效应用、双向同步、`hashchange`/`popstate` 监听。
- 支持 `#home`、`#settings`、`#stats`、`#game-detail?id=xxx` 等 hash 格式。
- 切换视图时自动同步地址栏 hash；刷新页面后恢复到上一个合法视图与选中的游戏。
- `uiStore` 移除 `?view=` 查询参数初始化，统一由路由 Store 管理。
- `App.svelte` 启动时调用 `initRouter()`。
- 新增 `src/lib/stores/router.test.ts`（11 个测试）。

## 0.10.10 - 2026-07-05

### 主题系统扩展
- 新增主题模式：跟随系统、纯黑（OLED）、高对比。
- 抽取 `src/lib/utils/theme.ts` 集中管理主题类型、生效主题解析、`system` 模式监听与持久化。
- `src/lib/stores/settings.svelte.ts` 改用主题工具初始化与应用主题。
- 设置页主题选择器使用 `APP_THEMES`，共 6 个选项。
- `Icon.svelte` 新增 `moon`、`contrast` 图标。
- `app.css` 新增 `[data-theme="black"]`、`[data-theme="contrast"]` 与对应 Aura 变量覆盖。
- 新增 `src/lib/utils/theme.test.ts`，覆盖 8 个单元测试。

## 0.10.9 - 2026-07-05

### 修复与优化
- 修复 Chart.js  typings：使用 `border.display: false` 替代 `drawBorder: false`，消除 `svelte-check` 类型错误
- 清理 `app.css` 中已弃用的 `.status-fill` 样式选择器
- `npm run check` 0 errors，`npm run test:unit` 57 个测试全部通过

## 0.10.8 - 2026-07-05

### 统计页 Chart.js 图表化
- 接入 `chart.js` 图表库
- 新增 `Chart.svelte` 通用图表组件，自动销毁与响应式更新
- 统计页替换原有 SVG 手绘图表：
  - 月度热力图 → Chart.js 折线图（支持 hover 提示、平滑曲线）
  - 状态分布 → Chart.js 水平条形图
  - 完成率 → Chart.js doughnut 图，保留中心百分比
- 新增 `src/lib/utils/chart.ts` 数据转换工具与单元测试

## 0.10.7 - 2026-07-05

### 全局键盘快捷键系统
- 接入 `@svelte-put/shortcut`，建立可扩展的快捷键体系
- 新增快捷键：
  - `1`~`5` 快速切换 Dock 前 5 个主视图（游戏库、继续、番剧、漫画、工具）
  - `?` 打开/关闭快捷键帮助浮层
  - `/` 或 `Ctrl/Cmd + K` 聚焦游戏库搜索框
- 底部 Dock 显示对应数字快捷键提示角标
- 新增 `ShortcutHelp` 组件集中展示所有可用快捷键
- 保留原有 Escape 返回首页、手柄回退行为

## 0.10.6 - 2026-07-05

### 今日中枢 Dashboard 升级
- 将「继续」页升级为全功能 Dashboard，每个展示栏位均有实际作用
- 新增今日/本周活跃时长、连续活跃天数、游戏/番剧/漫画计数等统计卡片
- 智能排序「最该继续」的内容，优先展示做到一半的游戏、最近更新的番剧/漫画
- 每个卡片新增进度条、副标题、明确操作意图（继续游玩/观看/阅读）
- 空状态提供「导入游戏」「去追番」「去看漫」一键跳转入口

### 架构与质量
- 抽取 `src/lib/utils/continue.ts` 纯函数模块：进度计算、时长聚合、连续天数、优先级评分
- 重构 `src/lib/stores/continue.svelte.ts`，数据逻辑下沉到可测试函数
- 新增 `src/lib/utils/continue.test.ts` 单元测试，覆盖核心计算逻辑
- 引入 `date-fns` 替代手写 `timeAgo` 与日期聚合逻辑

## 0.10.5 - 2026-06-17

### 刮削改进
- 增加重试机制（3 次重试 + 800ms 退避），连接超时 10s → 请求超时 30s
- 新增 HTTP 代理设置：设置页可配置刮削代理地址，解决部分源被墙无法访问的问题
- 搜索结果展示每个数据源的状态（成功/失败/结果数量），失败时显示错误原因
- 刮削对话框：搜索后显示各源连接状态，连接失败时提示配置代理
- 刮削中心：搜索后显示各源状态标签及结果计数

## 0.10.4 - 2026-06-17

### 新增功能
- 底部导航栏增加全屏/窗口切换按钮，随时在无边框全屏与窗口模式间切换

## 0.10.3 - 2026-06-17

### Bug 修复
- 修复退出番剧播放器后窗口自动退出全屏的问题
  - 播放器全屏切换改为纯 CSS 实现，不再使用 DOM Fullscreen API 以避免影响 Tauri 窗口状态
  - 原生 video controls 触发的 DOM 全屏退出时自动恢复 Tauri 窗口全屏
  - 修复 AnimePage Escape 键捕获阶段拦截导致播放器无法先退出全屏再关闭的问题

## 0.10.2 - 2026-06-17

### Bug 修复
- 修复视频播放器卡在"正在连接播放页… 0s/30s"无法播放的问题
  - 修复 Svelte 5 `$effect` 依赖循环：`extractTimer` 从 `$state` 改为普通变量，消除 effect 无限重触发
  - 修复视频代理 ureq 2 错误处理：区分 HTTP 状态错误与连接错误，透传 CDN 响应而非静默返回 502

### 新增功能
- 大屏模式媒体导轨、搜索组件
- 续播中心 + 续播卡片
- 游戏笔记面板
- 存档管理面板
- 智能合集编辑器
- 虚拟键盘（大屏模式）
- "玩什么"推荐组件
- 应用更新对话框

## 0.10.1 - 2026-06-17

### 播放源优化
- 共享 HTTP 客户端单例（reqwest + ureq Agent），减少连接开销
- 视频嗅探 JS 注入：相对 URL 解析、HTMLSourceElement setter hook、JSONP 检测
- 嗅探轮询 500ms → 250ms，迭代次数 70 → 140，提升捕获率
- 视频代理增加 Origin 头、读缓冲 8KB → 16KB、AtomicU16 替换 static mut
- 视频 URL 缓存（30 分钟 TTL），重复播放秒开
- 播放成功后台预提取下一集 URL
- 并行换源：Phase 1 多源并发搜索+线路，Phase 2 顺序提取，优先上次成功源

### UI 优化
- 播放器工具栏增加分组分隔线，全屏时隐藏底栏
- 倍速菜单/弹幕设置点击外部自动关闭
- 提取进度指示器增加脉冲动画
- 详情页 FAB 按钮显示续播信息 + 呼吸动画
- 选集面板标记已观看集数、高亮续播集
- 移除搜索抽屉无功能按钮
- Tab 切换增加过渡动画
- 进度条宽度响应式适配

### 项目清理
- 移除 AI 工具配置、开发临时文件
- 标准化开源项目结构

## 0.1.2 - 2026-06-16

- 自动换源：播放提取失败时自动搜索替代源
- 提取进度可视化：分步状态与计时器
- 播放器超时/错误状态新增「手动选源」按钮
- Tauri 自动更新集成
- 批量操作：Ctrl 多选、批量收藏/隐藏/标签/删除
- 智能合集：保存筛选条件为命名合集
- 修复多个页面加载/错误状态缺失问题

## 0.1.1 - 2026-06-11

- 从 MoeGame C# 版迁移 LiteDB 数据至 Tauri SQLite
- Steam 平台导入：游戏时间、最后游玩、封面、图标、成就
- Epic 本地清单封面提取与平台导入去重
- 主页/详情视图重构为 PS5 风格导轨布局
- 添加前端单元测试与 Rust 并发抓取回归测试
- 发布打包脚本与构建校验
