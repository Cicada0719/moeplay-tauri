# Changelog

## 0.19.5 - 2026-07-22

### 立方舞台长标题、小说搜索提速与继续游玩滚筒

- 修复立方舞台档案面板超长标题（罗马音长名、CJK 长名）把“继续游戏 / 打开档案 / 收藏”按钮和游戏目录挤出可视区的问题：标题按长度分三档缩小字号，并限制最大行数，操作区恒可见。
- 小说“全部书源”搜索改为前端逐源并发、先到先展示，慢源不再拖垮整体；单源搜索超时从 30–35 秒收紧到 10 秒，HTTP 客户端改为全局共享复用连接；搜索中可随时取消，并实时显示已返回书源进度。
- 游戏库“影像序列”升级为“继续游玩”封面滚筒：多张封面经裁切后拼接在水平滚筒上，以俯视角 3D 滚动（滚轮 / 拖拽 / 方向键），滚筒装入带上下出纸口的盒体中；原页面顶部的继续游玩横栏（ContinueHeroRail）暂时移除，组件保留备用。
- 模式切换器第三个标签由“影像序列”更名为“继续游玩”，本地保存的浏览模式不受影响。

## 0.19.4 - 2026-07-22

### Galgame 启动目标与游戏库界面修复

- 本地游戏启动现在严格使用用户保存的 `exe_path`，不再被引擎扫描、同目录原版程序或历史平台 URI 覆盖；启动后也不会再自动把猜测出的 EXE 写回数据库。
- 编辑档案改用字段级更新接口保存名称、简介和启动路径，避免旧详情快照覆盖最新游玩记录或刮削数据；启动路径移动到新目录时同步安装目录，并立即用于下一次启动和引擎检测。
- 无效或不存在的启动路径会明确报错，不再尝试其他启动项；详情页仅在后端确实启动成功后显示“正在启动”。
- 专注模式开关缩小为紧凑按钮并上移，针对手柄提示栏、移动端、安全区域和低高度窗口提供独立避让位置。
- 游戏卡片、列表和详情页补充超长标题截断、断行、低高度布局和固定操作区，避免收藏、删除、启动等按钮被挤出或裁切。
- Release 工作流不再硬编码英文 NSIS 文件名，会按版本匹配实际生成的中文产品名安装包并校验唯一结果。

## 0.19.3 - 2026-07-22

### 本地 Galgame 稳定性、小说多书源与界面体验
- 修复 Windows 上启动本地 Galgame 后萌游直接退出的问题：同步 Tauri 命令不再调用依赖 Tokio reactor 的 `tokio::task::spawn_blocking`，改为独立命名线程等待游戏进程结束；监控初始化失败时会安全清理运行记录和会话。
- 小说模块在笔趣阁、Project Gutenberg、中文维基文库基础上新增 Internet Archive、Open Library 与 Standard Ebooks，支持并行搜索、公开纯文本阅读以及上游明确提供的 EPUB/PDF 下载。
- 笔趣阁新增作品搜索、分页目录与分页正文解析；全部书源和单一来源会暴露可读错误，不再将网络或页面结构故障伪装成空结果。
- 漫画页面补齐超宽屏两侧背景，加入纯 CSS 渐变、纹理、卡片与详情层次，并为窄屏、高对比及减少动态效果模式提供降级。
- 专注模式改为不持久化的单一临时状态，切换栏目自动退出；统一恢复按钮和手柄提示，修复缩放、窄窗口、低高度窗口下的 UI 遮挡与横向溢出。
- 使用本机 Kirikiri Galgame《逐光柠檬协奏曲》完成发布构建实测，确认启动器可打开且萌游在游戏进程运行与退出后保持响应；本版本仍只提供 Windows x64 安装包、便携包和签名更新资源。

## 0.19.2 - 2026-07-20

### 游戏库布局与手柄焦点修复
- 修复封面墙删除游戏时确认框被虚拟卡片和网格布局限制、导致弹窗偏移的问题；确认层现在挂载到应用最外层并始终相对整个窗口居中。
- “继续游玩”新增可折叠按钮，折叠状态会在本机保留；进入专注布局时会自动隐藏该区域，为封面墙和档案内容释放更多纵向空间。
- 修复隐藏控件后进入游戏详情再返回时，游戏标题被错误加上巨大焦点框的问题；Visual 与 Scene 舞台现在使用稳定的路由焦点锚点。
- 为游戏档案的折叠、媒体、封面、游戏目录、启动、档案与收藏操作补充稳定手柄焦点键和动态操作提示。
- 修复详情返回后封面窗口发生视觉位移的问题，并新增真实封面坐标、标题焦点框与删除弹窗视口定位回归测试。
- 本版本继续仅提供 Windows x64 安装包与便携包，不生成 APK 或 AAB。

## 0.19.1 - 2026-07-20

### 带界面截图的正式再发布
- 为 GitHub Release 新增 1920×1080 大屏模式实机界面截图，完整展示全屏背景、左下角作品信息、底部手柄封面滚轮与快捷操作提示。
- 全新重写 README，补齐 Windows 下载、签名自动更新、手柄、大屏模式、本地超清化和发布质量门槛说明。
- 删除已被正式产品替代的 Concept 原型与大体积演示素材、0.12.1 规划包、交接报告、历史审计文档和硬编码 Android 构建脚本。
- 移除 Android CI / Release job 与已失去调用方的签名辅助脚本；正式发布只生成 Windows 资产，不包含 APK/AAB。
- Release 工作流会上传截图并使用仓库内的 RELEASE_NOTES.md 生成发布正文，同时继续强制校验更新包、分离签名和 latest.json。
- 保留 v0.19.0 的大屏滚动剧场、手柄导航、响应式布局与 reduced-motion 修复。

## 0.19.0 - 2026-07-20

### 大屏滚动剧场与全屏作品舞台
- 全面重做大屏模式，将其与普通游戏库明确区分为面向电视与手柄的“滚动剧场”：压缩顶部导航和辅助信息，把作品本身作为主要视觉焦点。
- 主游戏封面改为底部横向手动滚轮，支持左摇杆与 D-Pad 左右逐项切换，不自动轮播；当前作品、相邻作品、序号、安装状态与收藏状态保持清晰可辨。
- 游戏标题、原名、说明、状态、启动、收藏、档案与游玩统计缩小并移动到左下角，减少对主画面的遮挡，同时保留完整手柄焦点与 A/X 等操作反馈。
- 背景图从右侧局部画框改为覆盖整个屏幕的扩散底稿；封面回退场景使用全屏模糊与综合色场，兼顾超宽屏、低高度与电视观看距离下的层次和可读性。
- 重构大屏媒体展厅、搜索和游戏档案的视觉语言与焦点区，补齐封面滚轮、信息操作区、顶部导航、媒体区和弹层之间的手柄往返导航。
- 新增并更新大屏手柄与响应式回归：5 项纯手柄流程通过，1080p、4K、1280×720、21:9 与 reduced-motion 等 7 项布局场景通过；Svelte 检查为 0 错误、0 警告。
- 本版本继续只发布 Windows 桌面安装包、便携包和签名自动更新资源；Android Release job 保持禁用，不生成 APK/AAB。
## 0.18.0 - 2026-07-19

### 普通模式手柄 UI、内部控件与专注布局
- 新增全局手柄操作提示条：检测到手柄后显示连接状态；开始使用手柄后，根据当前焦点动态标注 A 主操作、Y 卡片次操作、B 返回、X 搜索、LB/RB 切换主类、View 专注布局与 Start 大屏模式。
- 重做普通模式空间导航：支持原生按钮、链接、输入框、选择器、ARIA 按钮/标签页/菜单项，标签页采用稳定的手柄轮转；范围与下拉控件可用左右方向调整，并跳过遮罩、禁用项和无操作容器。
- 补齐游戏首页与档案内部操作：手柄可切换当前游戏、再次按 A 打开活动游戏档案，Y 执行收藏等次要操作；档案内可启动、日语环境启动、抓取元数据、编辑，并修复视觉边缘错误跳焦。
- 补齐番剧、漫画与小说内部控件：覆盖番剧主标签、历史记录、播放器选集/线路/弹幕/倍速/全屏/本地超清化，漫画来源、详情章节与阅读器翻页/换话/缩放，以及小说来源、目录、章节和阅读器显示设置。
- 新增按主类独立持久化的“专注布局”：游戏、记录、番剧、漫画和小说均可隐藏当前任务无关的辅助栏、筛选器或档案下半区；View 键和右下角按钮均可随时恢复，切换主类互不影响。
- 修复 X 搜索偶发聚焦到过渡期隐藏输入框的问题，并为动态路由、模态面板和 roving tab 增加回归测试；当前单元测试 653 项、Playwright 矩阵 145 项通过。
- 本版本继续只发布 Windows 桌面安装包、便携包和签名自动更新资源；Android Release job 保持禁用，不生成 APK/AAB。

## 0.17.1 - 2026-07-18

### 普通模式手柄导航修复
- 修复 0.17.0 中主界面方向键、摇杆与 LB/RB 不生效、仅 A 键偶尔可用的问题：移除不可见首页横轨常驻的高优先级手柄作用域，避免它在没有内容时仍吞掉全局输入。
- 普通模式手柄导航范围现在覆盖顶部全局导航和当前内容页；首次按方向键会从当前模块建立稳定焦点，随后可在页面按钮、搜索框、卡片和全局工具间连续移动。
- LB/RB 可在主页、记录、番剧、漫画、小说之间循环切换；A/Y 确认，B 返回或关闭抽屉，X 聚焦当前页面搜索，Start 进入大屏模式。
- 新增真实主界面手柄端到端回归测试，覆盖五个主页面、方向键、A/B/X、LB/RB、工具抽屉和可见焦点，防止再次出现“只有 A 键可用”的回归。
- 本版本仍仅发布 Windows 桌面安装包、便携包和自动更新资源，不生成 APK/AAB。

## 0.17.0 - 2026-07-18

### 桌面可靠性、全局手柄与本地画质增强
- 修复番剧首页推荐在接口短暂失败后永久空白的问题：Bangumi 请求现在校验 HTTP 状态，推荐区采用本地快照、过期重验、部分成功保留和显式重试，离线或服务波动时仍可显示最近内容。
- 修复正式安装包自动更新下载 404：Release 统一生成并上传 `MoeGame_<version>_x64-setup.exe`、签名与 `latest.json`，清单 URL 与实际资源名保持一致，并增加发布契约测试。
- 将手柄导航扩展到普通桌面模式的主页、游戏、番剧、漫画、记录及全部主要内容页：方向键/摇杆执行空间焦点移动，A/Y 确认，B 返回，X 聚焦搜索，LB/RB 切换主要页面，并提供清晰的手柄焦点环。
- 修复大屏模式“全部 / 本地”游戏切换：View 键或 `F` 可直接切换，全部游戏不再继承普通页面的搜索与筛选结果，本地模式只展示已安装游戏。
- 新增完全本地的 GPU 视频超清化：提供“均衡”与“质量优先”模式，使用 WebGL2 边缘自适应锐化与 1.5×/2× 分辨率提升，最高输出 1080p/1440p；视频不上传，GPU 或跨域纹理不可用时自动回退原始播放。
- 新增方向：番剧首页 stale-while-revalidate 缓存、统一 DOM 空间导航和播放器增强能力检测，减少网络波动、输入设备差异与硬件兼容性导致的不可用状态。
- 本版本仅发布 Windows 桌面安装包、便携包和更新资源；Android Release job 保持禁用，不生成 APK/AAB。

## 0.13.8 - 2026-07-12

### WebView2 全屏状态重建修复
- 不再只信任 Tauri 的 `isFullscreen()` 标志：同时比较窗口外框位置、尺寸与当前显示器物理边界，识别“状态为全屏但实际已恢复边框”的 Windows 状态不同步。
- 退出番剧播放器全屏时强制执行一次原生全屏状态重建（false → true），并在延迟守护中再次校正，重新应用 Win32 窗口样式。
- 顶部全屏按钮遇到状态不同步时优先修复全屏，而不是误执行退出全屏。
## 0.13.7 - 2026-07-12

### 桌面窗口与游戏视觉修正版
- 修复番剧播放器退出全屏后 WebView2 将宿主窗口还原为带边框窗口的问题；播放器期间持续守护原有宿主全屏状态，并在进入、退出与跨域播放器状态变化后自动恢复。
- 新增系统托盘与明确关闭策略：可在设置中选择右上角关闭后彻底退出进程，或驻留托盘并从菜单恢复。
- 游戏首页 Visual 改为 3:2 媒体/档案比例，扩大 Hero 主视图，并在档案侧加入独立裁切封面窗口，避免封面遮挡标题。
- 游戏详情改为全宽电影档案页，加入视觉接触表、Hero、封面、评分、简介、元数据、截图、存档和会话信息，消除左侧大面积空白。
- 修复设置页旧数据缺少代理默认值时无法渲染的问题；分类目录改为页内滚动按钮，避免破坏路由，并修复 AI 增强开关未真正保存。
- 修复存档候选与快照接口返回空值时详情组件崩溃的问题。

## 0.13.6 - 2026-07-12

### 正式视觉收束版
- 建立统一正式视觉系统：按钮、输入框、标签、卡片、抽屉、详情面板、弹窗、空状态、滚动条和动效使用同一套几何、材质与焦点语言。
- 游戏库 Visual 与 Scene 恢复评分、简介、标签、年份、开发商/发行商和档案摘要，强化滚轮切换后的内容反馈。
- 游戏详情新增作品档案、私人评价与完整元数据区，重新展示用户评分、评论、引擎、系列、语言、分级、首次游玩和完成次数。
- 漫画首页改为搜索档案、来源能力与继续阅读组成的分割式工作台，减少未搜索状态的大面积无意义留白。
- 发现页按开发商、标签、年份与评分采用编辑索引、比较条、标签地貌和时间/评分切片，不再统一使用稀疏大卡片。
- Activity v2 解析结构化事件 payload，恢复作品标题、评分、观看集数、阅读章节、备注、完成与收藏说明。
- 设置页升级为宽屏双栏工作台与分类索引；任务中心增加任务来源侧栏、能力矩阵和有意义的待命状态。
- 动画统一限制在 transform 与 opacity，并为 reduced-motion 提供完整降级。
## 0.13.5 - 2026-07-12

- 游戏库新增两套独立创意浏览体验：nodate 风格双面立方媒体档案与斜向连续影像序列，完整索引继续保留。
- 游戏目录与影像序列按软件身份去重，Scene 改为一款游戏一个镜头，并支持滚轮、拖拽、键盘和目录点击切换。
- 记录档案 001/002 列表按软件身份保留最新记录，同时保留完整活动统计与历史时长。
- 播放器全屏改为播放器内部 CSS/DOM 全屏，不再调用 Tauri 主窗口全屏，修复点击播放器导致整个应用切回窗口的问题。
- 番剧详情升级为全宽媒体布局：左侧封面视觉、标题、简介、评分、线路和观看进度，右侧保留完整资料与选集。
- 补充重复软件、创意游戏视图、播放器全屏隔离和番剧详情响应式回归测试。

## 0.13.4 - 2026-07-12

- 重构游戏库 Visual：固定五个语义媒体槽，修复媒体来源与点击对象错位，放大主视觉并加入同源模糊背景和自适应配色。
- 重构游戏库 Scene：加入连续媒体场景流、拖拽吸附、滚轮/键盘导航、缩略图地图和焦点定位框。
- 为传统番剧播放器与 Provider V2 引入共享自适应媒体舞台，利用封面背景、播放信息和宽屏信息栏减少黑边与空白。
- 强化 Adaptive Chroma 的缓存、竞态、高对比度和 reduced-motion 行为，并统一背景与提色来源。
- 修复路由切换后搜索快捷键的焦点重试竞态，补充游戏媒体与番剧播放视觉回归测试。

## 0.12.7 - 2026-07-12

### SHIFTBRAIN UI 正式整合版
- 将 SHIFTBRAIN 风格骨架合并到生产应用：移除常驻左侧栏，游戏、番剧、漫画、记录与管理页面统一使用顶部全局导航和右上角工具入口。
- 游戏库采用 Cinematic 视觉工作区并支持滚轮逐项切换、Visual / Index / Scene；番剧采用暗色 Editorial，漫画采用 Kinetic 媒体流，记录页采用 FOAM 风格 Activity Archive。
- 接入封面驱动的媒体氛围、响应式布局、键盘焦点与 reduced-motion 降级，同时保留原有导入、搜索、播放、阅读、任务和设置功能。
- 修复 MangaDex 当前 API 兼容与请求可靠性问题，并修复包子漫画封面 URL/防盗链加载问题。
- 统一应用版本号至 `0.12.7`。
## 0.13.0 - 开发中

### Stage 4：统一来源中心
- 新增 Anime、Comic 和外部运行时的统一来源投影、启用状态、优先级与来源健康度管理。
- 引入 schema v7 `source_preferences`，来源开关与排序不存储凭据。
- 新增可审计的来源验证、批量验证、熔断清除、健康度评分及自动选源策略。
- 新增仅导入元数据的远程扩展目录缓存：ETag、Last-Modified、24 小时 TTL 和离线快照；绝不下载或执行第三方扩展代码。
## 0.12.2 - 开发中

### Stage 3：任务生产者与可观测性
- SQLite schema 升级至 v6：新增脱敏的 `background_job_events` 事件时间线、按任务保留上限与终态保留策略。
- 任务中心新增详情抽屉、事件时间线、全局活动徽标和可复制的脱敏错误上下文，失败任务可显示针对性恢复入口。
- 引入版本化 `JobOperation` 与后端 dispatcher 边界；导入、刮削、来源验证、备份/恢复、诊断导出和更新检查均通过受控生产者创建任务。
- 强化取消与重启恢复语义：取消后的迟到结果不会覆盖终态，下载与可重放任务按安全策略恢复。
- 统一产品版本号至 `0.12.2`。

## 0.12.1 - 开发中

### 基线、安全与契约
- 统一 `package.json`、Cargo、Tauri 和 lockfile 版本为 `0.12.1`，新增版本一致性检查并接入 CI/Release。
- 修复 Tauri 命令注册、`build.rs` 与 capability 权限漂移，新增命令契约验证器和 fixture 测试。
- SQLite 打开/迁移失败改为 fail-closed：不再删除主库或静默回退可写内存库；迁移使用事务并保留唯一恢复备份。
- 修复统计页 Rust/TypeScript DTO 漂移、时间解析和重复 session 聚合；磁盘递归统计移出首屏同步路径。
- AI Provider 增加 endpoint/origin 安全策略，Provider DTO 不再返回 API Key；Ollama 支持无 Key，未正确实现的 Claude 暂停启用。
- 新增系统 SecretStore 基础设施，使用 OS credential store 并按 SecretKind/origin 隔离；AI、Steam、Bangumi、PicACG 凭据已迁移，前端 Settings/LocalStorage 不再持有明文 Key/Token。
- 新增 Provider、Progress、Activity、Health、ResolvedTarget 与 BackgroundJob 的跨端领域契约基础。
- SQLite schema 升级至 v3，新增迁移 ledger/checksum、Activity/Progress/ProviderHealth/BackgroundJob 表和仓储，包含 20k 活动聚合/分页验证。
- 新增 UI v2 基础：PageShell、PageHeader、FilterBar、StateBoundary、DetailPanel、语义 token 与 reduced-motion/GSAP cleanup，并接入全局 motion 初始化。
- UI v2 公共 API 补齐 ContentGrid、AsyncState/Section、MediaCard/Row、Drawer 与嵌套 focus trap；App 新增 view/entity/focusKey/scrollOffset/overlay 返回栈，当前页面搜索快捷键和 Dock 可访问语义。
- Gamepad runtime 新增 scope/priority/overlay 独占、四向输入和 320ms/100ms repeat；Big Picture 建立 top-nav/wheel/hero/media/detail/search/keyboard focus zones，修复隐藏游戏误操作和 overlay 输入冲突。
- 新增持久化 BackgroundJob 队列，使用 SQLite 恢复任务状态并提供真实取消句柄；前端新增统一任务状态基础。
- Activity v2 统一游戏/番剧/漫画活动与进度，提供继续中心、时间线分页、编辑/删除/导出及旧记录幂等回填，失败时保留旧仪表盘回退。
- Library v2 接入导入预览/应用、字段级 diff、冲突处理、provenance 与库健康面板，旧导入流程可通过 feature flag 回退。
- 新增 Anime Provider 注册表与 LocalMedia/Jellyfin/Kazumi adapter 命令边界，凭据仅从 SecretStore 按 origin 读取并记录来源健康状态。
- Anime Provider v2 已接入番剧主页面：支持本地目录扫描、Jellyfin、按来源搜索、详情/剧集、内部本地/HLS 播放、受保护 loopback 会话和安全 WebView/外部回退。
- Kazumi 规则引擎新增 API 模式兼容：支持嵌套 search/chapter API config、数值型反爬字段、受限 JSON path 和 episode page 变量；实时双源验收已由 TvTFun 与 aafun 完成搜索到剧集线路闭环。
- Anime/Comic 非敏感 Provider 配置写入 SQLite v4 并在运行时注册表重启后自动恢复；凭据仍只存在 SecretStore，配置仓储递归拒绝 token/API Key/password 等字段。
- 新增 Comic Provider 注册表与 Local/Komga/Kavita adapter 命令边界，支持探测、搜索、详情、章节与安全解析，远端凭据不进入前端状态。
- Comic Provider v2 已接入漫画主页面：支持 Local/Komga/Kavita 配置、探测、搜索、详情、章节、内部图片阅读、安全回退与旧页面 fallback。
- 实时来源验收新增可重复入口：番剧要求至少两个来源完成搜索→线路，漫画要求至少两个独立公开源返回结果；本次实测 TvTFun/aafun 与 Baozi/DM5/1kkk 通过。
- AI Gateway 新增 OpenAI-compatible/Ollama 方言、endpoint/origin 策略、版本化 prompt/schema、预算/限流/取消/脱敏与“用户确认前零写入”的 change-set 基础。
- AI v2 已接入 6 条统一异步任务命令（provider/budget/start/status/result/cancel）和 3 条 change-set 命令（preview/apply/undo）；发现页 AI 工作台改为 literal `invokeCmd` 契约，资料库写入携带完整已验证 change set 与 provenance，并支持原子应用和撤销。
- SQLite schema 升级至 v5，新增仅保存结构化验证结果的 AI task result 仓储；不保存原始 prompt、provider 原始响应、请求头或凭据，结果保留 7 天并可在应用进程重启后继续读取。
- SQLite schema 升级至 v4，新增拒绝 secret 字段的非敏感 Provider 配置仓储，为来源配置重启恢复提供单一事实源。
- 诊断导出改为默认脱敏 ZIP，使用并发隔离临时目录并清理；日志执行 7 天/100 MiB 保留策略，诊断页不再误导为数据库导出。
- 存档恢复增加文件差异预览、破坏性变更提醒和恢复前安全检查点确认。
- 运行时 User-Agent 统一由 Cargo 包版本生成，版本检查会拒绝源码中的硬编码旧版本 User-Agent。
- 修复通用下载器状态未注册问题，恢复默认下载目录与并发控制；Rust `fmt`、`clippy -D warnings` 和全量测试重新全绿。
- 前端单测启动改为直接使用本地 Vitest CLI 和绝对配置路径，移除 Windows `shell: true`/Node DEP0190 风险，并补充路径与启动契约测试。
- 统一 BackgroundJob Task Center 投影，新增按状态/类型/数量过滤、结构化控制错误、真实下载暂停/恢复/重试命令，以及重启后下载暂停恢复和其他任务失败收口语义。
- 新增全局任务中心页面与工具入口，提供状态摘要、筛选、脱敏消息、恢复标识和 capability 驱动操作；下载页复用统一任务控制面并保留旧列表回退。
- 自动更新发布新增签名 Secret gate：签名可用时强制验证 `latest.json`、本地产物和 detached signature；缺少密钥时仅保留明确标记的 installer-only Draft，并阻止发布晋级。
- 发布证据新增 CycloneDX SBOM、构建 commit/toolchain metadata 与自动验证，release 构建前清理 Rust target。

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
