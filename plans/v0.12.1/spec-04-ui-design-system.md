# MoePlay 0.12.1 UI/UX、设计系统、响应式、动效与可访问性专项规格

> 文档状态：Draft for implementation
> 专项目标版本：MoePlay 0.12.1
> 审计基线：`3ceb354`（2026-07-10）
> 负责人范围：整体 UI/UX、设计系统、响应式、大屏/手柄、动效、可访问性、视觉验收
> 非目标：不改业务数据模型、不重写番剧/漫画源协议、不以“全量换皮”代替交互与状态治理

---

## 1. 目标与发布判断

0.12.1 的 UI 专项不是重新选一套颜色、圆角和阴影，而是建立一条可持续的前端交付链：

1. **统一信息架构与页面骨架**：固定导航层级、返回语义、页面标题/工具栏/内容区结构。
2. **建立语义化设计令牌**：主题只改令牌，不允许页面自己重新发明颜色、密度、层级和动效。
3. **让四个主入口形成一个产品**：游戏库、记录、番剧、漫画共享导航、状态、卡片、筛选、详情和反馈规则，同时保留内容差异。
4. **覆盖真实状态**：加载、局部加载、空、错误、离线、无权限、陈旧数据、操作中和成功反馈都有统一契约。
5. **窗口、电视和输入方式都可用**：普通窗口、窄窗口、1080p/2K/4K 大屏，鼠标、键盘、手柄均能完成核心路径。
6. **动效可解释、可取消、可降级**：GSAP 只负责复杂编排，所有动效服从统一时长、性能与 `prefers-reduced-motion`。
7. **把视觉质量纳入自动化**：现有 Playwright 从“能打开”升级到可复现截图、主题、尺寸、状态和焦点矩阵。

### 1.1 发布成功定义

只有同时满足以下条件，专项才算完成：

- 四主入口均迁移到统一页面骨架和状态组件，而不是只替换背景/颜色。
- 普通模式在 `720×600` 到 `2560×1440` 无关键内容裁切、不可达控件或非预期横向滚动。
- Big Picture 在 `1920×1080`、`2560×1440`、`3840×2160` 完成手柄主路径并保持清晰焦点。
- 深色、浅色、纯黑、樱夜、高对比主题均通过核心截图与对比度门禁；Big Picture 使用明确的影院主题契约而非散落硬编码。
- 所有 P0/P1 交互满足 WCAG 2.2 AA 目标；键盘和手柄不依赖鼠标悬停信息。
- 视觉基线、焦点路径、reduced-motion、无横向溢出和性能预算进入 CI。

---

## 2. 审计方法与当前基线

### 2.1 审计范围

- 全局：`src/app.css`、`src/main.ts`、`src/App.svelte`
- 导航/状态：`src/lib/nav.ts`、`src/lib/stores/ui.svelte.ts`、`src/lib/stores/router.svelte.ts`
- 主题：`src/lib/utils/theme.ts`、`src/lib/stores/settings.svelte.ts`、`SettingsPage.svelte`
- UI primitives：`src/lib/components/ui/*`、`focus-trap.svelte.ts`、`spotlight.ts`
- 四主入口：`switch/SwitchHome.svelte`、`PlayRecordsDashboard.svelte`、`AnimePage.svelte`、`ComicPage.svelte`
- 详情/跨域：`GameDetailPage.svelte`、`anime/*`、`comic/*`、`ContinueHub.svelte`
- 大屏：`BigPicturePage.svelte`、`BigPictureDetail.svelte`、`BPSearch.svelte`、`BPMediaRail.svelte`、`bigpicture/*`、`useGamepad.svelte.ts`、`VirtualKeyboard.svelte`
- 动效：所有 GSAP 引用、Svelte transition、CSS animation/transition
- 测试：`playwright.config.ts`、`tests/visual/*`、UI primitive 单测

### 2.2 已执行验证

| 命令 | 结果 | 结论 |
|---|---:|---|
| `npm run check` | 0 error / 0 warning | 当前没有编译级或 Svelte 静态诊断阻塞，但不代表视觉/语义合格 |
| `npm run test:unit` | 155 passed / 1 skipped | 逻辑与少量 primitive 有回归保护 |
| `npm run test:visual` | 3 passed | 实际是功能型 E2E；没有 `toHaveScreenshot`，尚未形成视觉回归 |
| `npm run build` | 成功，存在 >500 kB chunk 警告 | UI 方案必须同时处理 CSS、字体、背景资源和首屏依赖预算 |

### 2.3 量化快照

- `src/app.css`：1,135 行、约 32 KB 源码。
- 81 个 Svelte 文件共 25,103 行，其中组件内 `<style>` 约 10,946 行，占 **43.6%**。
- Svelte 文件中约 **985** 处十六进制/RGB(A) 颜色引用；仅 `src/lib/components` 约 **883** 处。
- 代码中存在约 146 个 `transition:` 声明、62 处内联 style 绑定。
- 响应式宽度条件至少有 14 个不同阈值：520/560/600/620/640/700/720/760/900/940/960/1180/1280/1440，未形成窗口等级体系。
- 生产构建资产：JS 约 1,676.6 KB raw、CSS 约 298.4 KB raw、字体约 462.8 KB；初始 `index` JS 851.22 KB raw / 328.89 KB gzip，初始 CSS 130.95 KB raw / 38.50 KB gzip。
- `AnimePage` chunk 618.18 KB raw / 191.75 KB gzip，页面 CSS 53.66 KB raw；默认图库背景 PNG 约 1.55 MB。

这些数字不是要求一次性“删到最小”，而是说明当前设计规则主要存在于页面私有 CSS 中，新增功能会继续复制视觉与交互债务。

---

## 3. UI 审计证据与问题分级

| ID | 级别 | 证据 | 影响 | 0.12.1 决策 |
|---|---|---|---|---|
| UI-A01 | P0 | `App.svelte:221-284` 对路由组件使用 `{#await import(...) then}`，没有 pending/catch 内容 | 慢磁盘或 chunk 失败时主内容区空白，无法解释或重试 | 增加统一 `RouteBoundary`，提供延迟骨架、失败提示、重试和日志入口 |
| UI-A02 | P0 | `App.svelte:287-305` 工具抽屉是普通 `div` + 背景按钮；无 `role=dialog`、`aria-expanded`、焦点圈闭/归还 | 键盘和读屏用户可能进入抽屉后失去上下文 | 抽屉迁移为 `Drawer` primitive，强制焦点管理与返回触发点 |
| UI-A03 | P0 | `App.svelte:223`、289-290 使用 Svelte transition；`app.css:1114-1135` 只关闭少量 CSS transition | reduced-motion 用户仍会看到页面淡入、抽屉飞入和局部 GSAP | 建立单一 motion preference store，CSS/Svelte/GSAP 共用 |
| UI-A04 | P0 | `SystemDock.svelte:35-108` 有 nav 语义，但活动项没有 `aria-current`；工具按钮没有 `aria-expanded/controls` | 视觉高亮不能被辅助技术理解 | 导航 primitive 统一 current/expanded/shortcut 描述 |
| UI-A05 | P0 | `SegmentControl.svelte:20-36` 固定 `aria-label="切换视图"`，radio group 不支持方向键与 roving tabindex | 多处复用后语义错误，键盘操作低效 | 增加 label、disabled、方向键、Home/End、roving tabindex |
| UI-A06 | P0 | `Switch.svelte:20-24` 没有 label/ariaLabel/id/name props；可访问名称完全依赖外层写法 | 设置页开关存在无名风险 | Switch API 强制可访问名称或显式关联 label |
| UI-A07 | P1 | `Tooltip.svelte` 有 `role=tooltip`，但触发器与 tooltip 没有 `aria-describedby` 关联 | 读屏可能不朗读提示 | 生成稳定 id 并关联；触摸/手柄场景不得把必要信息只放 tooltip |
| UI-A08 | P1 | `BackgroundLayer.svelte:54-61` 同时设置 `aria-hidden=true`、`role=img`、`aria-label` | meaningful alt 永远不会被读屏获取，语义互相冲突 | 装饰背景只 `aria-hidden`；有意义图片改真实 `<img>`/figure |
| UI-A09 | P1 | `Card.svelte:32-52` 可输出带 `role=button` 的 div，键盘触发依赖调用方自行实现 | 容易出现“可聚焦但 Enter/Space 无效”的伪按钮 | 有点击行为默认输出 `<button>`/`<a>`；移除通用伪按钮路径 |
| UI-A10 | P1 | `app.css:15-314` 有五套主题，但组件仍有近千处硬编码色；Big Picture/媒体页大量 rgba/#hex | 浅色、樱夜、高对比无法稳定覆盖，主题回归难定位 | 页面只消费语义 token；例外必须登记为媒体遮罩/数据色 |
| UI-A11 | P1 | `app.css:449-793` 又建立 Aura 私有 token；`app.css:795-1135` 部分 primitive 样式在全局，Button/Dialog 等又在组件内 | token、工具页皮肤、primitive 样式所有权混杂 | 样式分层：tokens → base → primitives → patterns → page exception |
| UI-A12 | P1 | `body/#app` 使用 `100vh`（`app.css:331-343`），`App.svelte` 使用 `100dvh` | 窗口缩放/系统 UI 情况下高度策略不一致 | 全局改为 `100dvh` + `min-height:0` 链路，禁止页面自建全屏高度 |
| UI-A13 | P1 | 主页面体积与私有样式极高：记录 1601/948 行、番剧 1301/539 行、漫画 1223/693 行、SwitchHome 714/385 行 | 业务、布局、视觉和状态耦合，难以统一与测试 | 以壳层和 pattern 组件“绞杀式”迁移，不进行一次性重写 |
| UI-A14 | P1 | 番剧页没有宽度媒体查询；Big Picture 相关组件没有任何 `@media`；全仓库阈值碎片化 | 窄窗、低高度和 4K 缩放行为不可预测 | 采用窗口等级 + container query；Big Picture 增加高度/宽高比策略 |
| UI-A15 | P1 | `nav.ts:9` 注释写“6 个始终可见”，实际 `DOCK_ITEMS` 有 7 项；记录替代了旧“继续”的主位，工具抽屉仍含继续 | 信息架构和快捷键认知漂移 | 明确四内容主入口、跨域入口、工具和模式切换层级 |
| UI-A16 | P1 | `ContinueHub.svelte:51-63` GSAP 无 reduced-motion 和显式 cleanup；`SettingsPage.svelte:60-74` 同类问题 | 页面卸载/重入可能遗留 tween，reduced-motion 不完整 | 所有 GSAP 通过 motion helper/context，卸载 revert/kill |
| UI-A17 | P1 | `WhatToPlay.svelte:46-82` 每次创建 timeline，无持久引用、无取消/降级；`SavePanel.svelte` 导入 GSAP 但未使用 | 重复操作和关闭弹层时状态可能继续变化；无效依赖信号 | timeline 可取消，关闭时 kill；删除死 import |
| UI-A18 | P1 | `TileCard.svelte:102-133` 选中态同时 GSAP scale/y 与 CSS width transition | width 变化触发布局，长列表和手柄快速移动时易抖动 | 预留选中槽位或使用 FLIP；每帧动画限 transform/opacity |
| UI-A19 | P1 | Playwright 只有一个 Desktop Chrome project；smoke 只断言壳可见；没有截图、主题、尺寸、reduced-motion、焦点矩阵 | 视觉改动没有自动化门禁 | 建立确定性 fixture 和 P0 screenshot matrix |
| UI-A20 | P2 | 基础字号 14px，但大量关键导航/元数据使用 9-12px；大屏也存在 10-12px 文本 | 电视距离、缩放、高 DPI 下可读性不足 | 定义普通/紧凑/沙发密度的最小字号与触控/焦点尺寸 |
| UI-A21 | P2 | `glass`、Aura spotlight、多个 backdrop-filter 和 1.55 MB 背景图同时存在 | 低端 GPU/集显可能掉帧，纯黑/高对比主题不必要耗费 | 透明度/模糊可降级；背景转换 WebP/AVIF；设 GPU/帧预算 |

### 3.1 已有可复用资产

本专项不否定现有实现，以下能力应保留并标准化：

- 已有 Outfit + JetBrains Mono 本地字体，无 CDN 依赖。
- `Dialog.svelte` 已有 `aria-modal`、Escape 和 focus trap 基础。
- `Button`、`Input`、`Tag`、`Skeleton` 等已有基础 token 接入和部分 reduced-motion。
- `TileRail`、`TileCard`、`GameCard`、`StatsPage` 多数 GSAP 使用 `gsap.context(...).revert()`。
- `useGamepad.svelte.ts` 已有连接/断开、边沿去抖和 RAF 生命周期清理。
- 四主入口已有真实加载/空/错误分支，问题主要是表达不一致，不需重做业务状态。
- 当前 check、unit、三条 E2E 均通过，可作为迁移期“不破坏业务”的底线。

---

## 4. 统一设计原则

### P1. 一个产品，不是一组主题页面

游戏、记录、番剧、漫画可以有不同内容结构，但不得各自拥有独立按钮语言、状态语言、阴影体系、字号比例和导航逻辑。差异应来自封面比例、媒体元数据、主要动作和内容密度，而非重新发明视觉系统。

### P2. 先解决任务，再表现氛围

- 第一视觉层：当前位置、当前内容、下一步动作、状态。
- 第二视觉层：筛选、排序、统计、来源、次要动作。
- 第三视觉层：背景图、氛围光、纹理、粒子。
- 氛围层不得降低文本对比度、抢占布局、阻塞加载或成为交互前提。

### P3. 语义令牌，不按页面命名

禁止新增 `--anime-pink`、`--comic-panel-blue`、`--records-card-bg` 一类页面令牌。必须使用 `--color-surface-*`、`--color-text-*`、`--color-action-*`、`--color-status-*` 等角色令牌。媒体特有色只可用于图表序列或来源品牌标识。

### P4. 状态是组件 API 的一部分

任何数据区域在实现时必须同时设计 `loading / empty / error / ready`；网络型区域额外设计 `offline / stale / retrying`。不接受最后用一段居中文字补空状态。

### P5. 输入等价，不追求像素等价

鼠标、键盘、手柄完成同一任务，但可以有适配后的交互：普通模式用 toolbar 和快捷键；Big Picture 用 roving focus、肩键分页和底部按键提示。任一必要功能不能只存在于 hover 或右键菜单。

### P6. 动效解释空间变化

动效用于：进入/离开、焦点移动、列表重排、状态确认、背景上下文切换。禁止无意义循环装饰、全页逐项长 stagger、同时改变布局属性和 transform。

### P7. 可测量优先

每条“更统一、更流畅、更易用”都必须对应截图、焦点路径、对比度、帧率、体积或完成步骤数指标。

---
## 5. 设计令牌、主题、字体、网格与密度

### 5.1 CSS 分层

建议把 `app.css` 从“大型规则集合”收敛为入口，按固定顺序导入：

```text
src/app.css
  ├─ lib/styles/tokens.css       # 尺寸、字体、动效、层级、语义角色
  ├─ lib/styles/themes.css       # dark/light/black/sakura/contrast
  ├─ lib/styles/base.css         # reset、body、selection、scrollbar、focus
  ├─ lib/styles/layout.css       # page/container/grid/window classes
  ├─ lib/styles/patterns.css     # page shell、toolbar、state、media patterns
  └─ 仅保留无法组件化的兼容层；0.12.2 删除
```

所有权规则：

- token 和基础元素只能在 styles 目录定义。
- primitive 的结构样式放在 primitive 自身，禁止同一 class 同时由全局和局部维护。
- pattern 可全局复用，但必须有文档化 class/API。
- 页面局部 CSS 只允许内容布局和真正的媒体差异，不再定义通用按钮、卡片、空状态、tab、focus ring。

### 5.2 核心 token 草案

```css
:root {
  /* typography */
  --font-ui: "Outfit", "Noto Sans SC", "Microsoft YaHei UI", system-ui, sans-serif;
  --font-display: "Outfit", "Noto Sans SC", "Microsoft YaHei UI", sans-serif;
  --font-mono: "JetBrains Mono", "Cascadia Code", Consolas, monospace;
  --text-2xs: 11px;
  --text-xs: 12px;
  --text-sm: 13px;
  --text-md: 14px;
  --text-lg: 16px;
  --text-xl: 20px;
  --text-2xl: 24px;
  --text-3xl: 32px;
  --text-4xl: 40px;
  --leading-tight: 1.2;
  --leading-ui: 1.4;
  --leading-copy: 1.6;

  /* 4px base spacing */
  --space-0: 0;
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;
  --space-16: 64px;

  /* shape */
  --radius-xs: 6px;
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --radius-xl: 24px;
  --radius-pill: 999px;
  --border-width: 1px;
  --focus-width: 3px;

  /* control sizing */
  --control-sm: 32px;
  --control-md: 40px;
  --control-lg: 48px;
  --target-min: 36px;
  --target-couch: 56px;

  /* motion */
  --motion-instant: 80ms;
  --motion-fast: 140ms;
  --motion-standard: 220ms;
  --motion-enter: 280ms;
  --motion-emphasis: 420ms;
  --ease-standard: cubic-bezier(.2, 0, 0, 1);
  --ease-enter: cubic-bezier(.16, 1, .3, 1);
  --ease-exit: cubic-bezier(.4, 0, 1, 1);

  /* layout */
  --page-max: 1440px;
  --page-gutter: 32px;
  --header-height: 72px;
  --dock-height: 76px;

  /* z-index: no arbitrary 80/81/90/1000 in pages */
  --z-base: 0;
  --z-sticky: 20;
  --z-dock: 40;
  --z-popover: 60;
  --z-drawer: 80;
  --z-dialog: 100;
  --z-toast: 120;
}
```

### 5.3 颜色角色

每个主题必须完整定义以下角色；组件不得直接依赖主题名：

| 角色组 | token | 用途 |
|---|---|---|
| 背景 | `--color-canvas`, `--color-canvas-subtle` | 应用底、次级底 |
| 表面 | `--color-surface-1/2/3`, `--color-surface-hover`, `--color-surface-selected` | 卡片、浮层、提升层、交互态 |
| 文本 | `--color-text-1/2/3`, `--color-text-inverse` | 主、次、弱、反色 |
| 边界 | `--color-border`, `--color-border-strong`, `--color-divider` | 边框、强调边框、分隔 |
| 动作 | `--color-action`, `--color-action-hover`, `--color-action-soft`, `--color-focus` | 单一主强调色与焦点 |
| 状态 | `--color-success/warning/error/info` 及 soft 版本 | 状态，不得借用 action 色表示错误 |
| 遮罩 | `--color-scrim`, `--color-media-scrim` | Dialog 与媒体背景文字保护 |
| 数据 | `--color-data-1..6` | 图表专用，必须有非颜色辅助编码 |

兼容期保留现有 `--bg-*`、`--text-*`、`--accent-*` alias，但新代码只写新 token；迁移完成后移除 alias。

### 5.4 主题契约

- `dark`：默认生产主题，克制玫瑰为唯一主强调色。
- `light`：不是把背景变白；所有表面、遮罩、边界、图表和媒体卡必须有独立校准值。
- `black`：OLED 纯黑画布，减少大面积半透明模糊；表面通过边框/微亮度建立层级。
- `sakura`：只改变 token 与可选装饰层，不改变组件结构或信息密度；粒子必须可关闭并服从 reduced-motion。
- `contrast`：高对比不是更亮的暗色主题；关闭低对比玻璃、纹理和弱边框，焦点宽度至少 3px。
- `system`：只负责解析到 light/dark，系统变化时实时更新；视觉测试针对解析后的实际主题。
- Big Picture：使用 `data-surface-theme="cinema"` 的暗色影院语义层；当全局为 contrast 时必须进入 `cinema-contrast`，不能强制回普通暗色。

### 5.5 字体与字号规则

- Outfit 负责 UI/标题；JetBrains Mono 只用于时间、计数、路径、版本和等宽数据。
- 中文依赖系统高质量无衬线回退，不引入体积巨大的远程 CJK 字体。
- 普通模式正文不低于 13px；关键操作/导航不低于 12px；9-11px 只允许非关键快捷键角标。
- Big Picture 正文不低于 16px、次要信息不低于 14px、可操作标签不低于 16px。
- 单行标题必须有省略与完整 title/accessible name；多行简介使用 2-4 行截断并提供详情入口。
- 数字列和时长使用 tabular-nums，避免实时更新导致布局跳动。

### 5.6 窗口等级、网格与容器

普通模式按窗口而非设备名称设计：

| 等级 | 宽度 | 页面 gutter | 网格 | 主要行为 |
|---|---:|---:|---:|---|
| `narrow` | 720-899 | 16px | 4 列 | Dock 隐藏次要文字/进入可滚动；header 动作收进 overflow；双栏改单栏 |
| `compact` | 900-1199 | 24px | 8 列 | 两栏可保留但侧栏变窄；卡片最小宽度 150px |
| `standard` | 1200-1599 | 32px | 12 列 | 默认桌面布局，内容 max 1440px |
| `wide` | 1600-2559 | 40px | 12 列 | 增加列数/留白，不无限放大正文行长 |
| `ultra` | ≥2560 | 48px | 12 列 | 容器仍限宽；可增加背景和辅助统计，不拉伸控件 |

高度补充：

- `<720px`：进入 compact-density，减少垂直 padding，header 可 sticky，禁止关键操作掉到视口外。
- `720-899px`：默认密度。
- `≥900px`：可展示扩展摘要与更高封面。

实现要求：

- 页面级只使用上述窗口等级；组件内部优先 `@container`，不继续新增 620/700/940 等一次性阈值。
- 页面容器：`width:min(100%, var(--page-max)); margin-inline:auto; padding-inline:var(--page-gutter)`。
- 卡片网格：`repeat(auto-fill, minmax(var(--card-min), 1fr))`，禁止 flex 百分比算术。
- 所有 flex/grid 滚动链必须具备 `min-width:0` / `min-height:0`。
- 横向 rail 允许自身滚动，但页面根节点不得横向溢出。

### 5.7 密度模式

定义 `data-density="compact|comfortable|couch"`：

- `comfortable`：普通模式默认；40px 控件、12-16px gap。
- `compact`：窄/低窗口自动启用，可在设置中手动选择；32-36px 控件、8-12px gap，不降低正文字号。
- `couch`：Big Picture；56px 最小目标、16-24px gap、焦点 ring 与位移更明显。

密度改变空间，不改变信息优先级和可访问名称。

---

## 6. 导航与信息架构

### 6.1 层级定义

0.12.1 以当前固定 Dock 为事实来源，明确四个“内容级主入口”：

1. **游戏库**：本地游戏发现、筛选、详情、启动。
2. **记录**：游戏/番剧/漫画统一历史、继续和统计。
3. **番剧**：发现、搜索、详情、选集、播放。
4. **漫画**：来源、搜索、详情、章节、阅读。

其余入口分层：

- **跨域入口**：继续、发现。它们聚合四主入口，不创建第五套视觉语言。
- **资源操作**：刮削、下载、存档。
- **导入**：游戏导入、模拟器导入。
- **系统**：诊断、设置。
- **模式切换**：全屏、Big Picture；不属于内容导航。

### 6.2 Dock 与工具抽屉

- Dock 保留四主入口 + 工具 + 设置 + 大屏，视觉上分成 Content / Utility / Mode 三组。
- `记录`负责跨媒体历史；`继续`留在工具抽屉，但在游戏库/记录页可作为内容模块和快捷动作出现。
- 工具抽屉按“继续与发现 / 资源操作 / 导入 / 系统”分组，不再使用一张无标题 4 列图标墙。
- 窄窗下 Dock 只保留图标和 active indicator，文字进入 tooltip/accessible name；不得压缩到小于 36px 目标。
- active 页面使用 `aria-current="page"`；工具按钮使用 `aria-expanded`、`aria-controls`；大屏按钮使用“进入大屏模式”而非仅“大屏”。
- 快捷键角标由 `nav.ts` 的唯一映射生成，注释、帮助页、Dock 和测试不得各自维护。

### 6.3 返回与焦点语义

优先级从高到低：

1. 关闭当前 Dialog/Drawer/Search/VirtualKeyboard。
2. 详情返回来源列表并恢复原卡片焦点与滚动位置。
3. 子视图返回主入口首页。
4. 已在主入口首页时，Escape 不偷偷跳到游戏库。
5. Big Picture 的 B/Escape 依同一栈执行；只在栈为空时退出大屏。

`router.svelte.ts` 保存 `view + entity + focusKey + scrollOffset` 的轻量历史；不要求引入 URL 路由，但禁止只靠 `currentView = "home"` 丢失上下文。

### 6.4 全局搜索与命令

- `/` 聚焦当前主入口搜索；全局命令面板使用独立快捷键，不把两种搜索混在一起。
- 搜索框必须展示范围（游戏/番剧/漫画/全部）、清空按钮和结果数量。
- 0 结果是搜索状态，不等同于库为空；提供清除筛选/切换来源动作。
- Big Picture Search 使用同一查询模型，但视觉与输入适配 couch density。

---

## 7. 四主功能一致性

### 7.1 统一页面骨架 `PageShell`

公共 API 与 MASTER PLAN 对齐为 `PageShell / PageHeader / FilterBar / ContentGrid / DetailPanel / AsyncState`；本文中的 `Toolbar` 是 FilterBar 的容器行为，`StateBoundary` 是 AsyncState 的底层状态机，`EntityDetailShell` 是 DetailPanel 的实现模式。

每个主入口必须按以下顺序组装：

```text
PageShell
  ├─ PageHeader: kicker / h1 / summary / primary actions
  ├─ PageStatus: offline / stale / error / sync progress（按需）
  ├─ Toolbar: search / segment / filter / sort / view / overflow
  └─ PageBody
      ├─ optional Overview/Rail
      ├─ primary Collection/Grid/List
      └─ StateBoundary
```

共同规则：

- 每页恰好一个可见 `h1`。
- header 高度、标题比例、主动作位置一致；不再由每页定义 `anime-header`、`comic-header`、`records head` 的不同规则。
- toolbar 可 sticky，但不得遮挡内容或焦点。
- 主动作最多 1 个高强调按钮；其余为 secondary/ghost/overflow。
- 内容背景不默认套玻璃卡；卡片只用于真实分组和可点击实体。

### 7.2 游戏库

- `SwitchHome` 的横向舞台可作为品牌化“精选/最近”模式保留。
- “全部游戏”区域必须迁移到统一 PageHeader/Toolbar/MediaGrid；搜索、快速筛选、视图模式使用共享 primitives。
- selected、focused、favorite、installed、running 各自有明确状态，不用同一个 rose glow 表示全部。
- 详情页共享 `EntityDetailShell`：背景/封面、标题、元数据、主要动作、tabs/sections、错误区。
- 启动、编辑、刮削、存档等动作区按频率与风险分层；删除必须用确认 Dialog。

### 7.3 记录

- 记录是四主入口中的跨媒体数据页，不能成为独立“控制台皮肤”。
- metric card、时间线、排行榜、继续项使用共享 `StatBlock`、`Card`、`MediaRow`、`EmptyState`。
- 图表颜色只使用 data tokens，并以标签、纹理/图例或文本值提供非颜色编码。
- 空记录时仍保留 PageHeader 和解释性 onboarding；不要渲染一整页空白大卡。
- `最近继续` 与 `ContinueHub` 共用同一 item model/pattern，避免两套卡片。

### 7.4 番剧

- 推荐、搜索、收藏/追番、详情、播放属于同一入口下的子视图；header/返回/toolbar 不因视图切换完全重建。
- 搜索来源和播放线路是 domain control，使用共享 Select/Segment/Sheet 语义与状态。
- 推荐封面、搜索结果、收藏统一 `MediaCard`，通过 `variant="poster"` 和 badge slot 表达差异。
- 播放器可全屏沉浸，但退出后恢复详情焦点；错误必须区分“源失败 / 解析失败 / 媒体失败 / 已切换备用源”。
- 加载使用封面骨架和局部 section skeleton，不用多套 spinner。

### 7.5 漫画

- 普通多源与 PicACG 可以有来源选择，但不得维护两套完全不同 header/toolbar/状态视觉。
- 来源 section 使用 `AsyncSection`：标题、健康度、刷新、loading/error/empty/ready 同位展示。
- 搜索结果、排行、随机、收藏、历史统一 `MediaCard`/`MediaRow`。
- 详情与章节列表迁移到 `EntityDetailShell`；Reader 是沉浸子模式，保留一致的返回、进度、设置和错误反馈。
- 成人内容模糊/隐藏是内容策略，不得破坏焦点顺序；被隐藏项要有可理解的占位和设置入口。

### 7.6 允许保留的差异

| 领域 | 可保留差异 | 不允许差异 |
|---|---|---|
| 游戏 | 横向舞台、启动态、平台/安装状态、3:4 封面 | 私有按钮/空状态/焦点体系 |
| 记录 | 图表、时间轴、统计密度 | 独立字体、颜色和页面壳 |
| 番剧 | 集数、线路、播放器、弹幕 | 私有搜索框/tab/dialog 语言 |
| 漫画 | 章节、阅读器、来源分区、成人内容策略 | 两套 header、错误和卡片系统 |

---

## 8. 空、加载、错误与异步状态

### 8.1 `StateBoundary` 契约

新增统一状态层，API 至少包含：

```ts
type ViewState =
  | "loading"
  | "refreshing"
  | "empty"
  | "no-results"
  | "error"
  | "offline"
  | "stale"
  | "ready";
```

必须支持 `title`、`description`、`primaryAction`、`secondaryAction`、`details`、`ariaLive`、`preserveContent`。

### 8.2 表达规则

- **首次加载**：200ms 内不闪 skeleton；超过 200ms 显示与最终布局同形骨架。
- **局部刷新**：保留旧内容并显示 section 级 progress，不把整页清空。
- **空库**：说明为什么为空，并提供导入/添加/设置来源的主要动作。
- **无搜索结果**：展示查询与筛选条件，首选“清除筛选”，不建议“添加内容”。
- **错误**：提供人类可读摘要、重试；技术详情折叠展示，可复制，不直接把异常字符串当标题。
- **离线**：区分本地仍可用与网络不可用；不阻断本地游戏/缓存漫画/已下载番剧。
- **陈旧数据**：内容可读，标注上次成功时间并允许刷新。
- **操作中**：按钮局部 loading、`aria-busy`、避免重复提交；成功后在原位置确认，toast 只作补充。
- **危险操作**：Dialog 内明确对象名和后果，默认焦点不落在危险按钮。

### 8.3 公告策略

- 页面首屏 loading 不使用 assertive live region。
- 搜索结果数量、保存成功、自动切源等使用 `aria-live="polite"`。
- 不可恢复错误或即将覆盖存档可使用 `role="alert"`。
- toast 需要按 info/success/error 分离 live politeness，并允许暂停自动消失。

---
## 9. 响应式、Big Picture 与手柄

### 9.1 普通模式最小窗口

0.12.1 支持下限定义为 `720×600`。低于该尺寸可以提示窗口过小，但仍必须能访问设置/退出；不得出现不可关闭的裁切 Dialog。

验收规则：

- 页面根节点无横向滚动。
- Dock、header、primary action 始终可达。
- Dialog 最大高度使用 `min(85dvh, ...)`，内容区滚动，标题/关闭/确认固定可见。
- 详情双栏在 compact/narrow 改为单栏；封面不占满首屏。
- 表格在窄窗转为 card row 或自身滚动，并提供列标题语义。
- 任何功能不得以 `display:none` 直接消失而没有 overflow/menu 替代入口。

### 9.2 Big Picture 布局策略

Big Picture 不是普通页面放大版，采用独立 `CouchShell`，但消费同一 token、状态和实体组件数据：

- 参考设计基准：16:9，`1920×1080`。
- 21:9：主内容限制安全宽度，背景延展；操作和字幕不贴边。
- 4K：字号/控件按 clamp 增长，内容容器不按 2 倍无限扩张。
- 低高度窗口：wheel/hero 改为 compact couch layout，不依赖 `16vh` 等单一视口比例。
- 安全区：左右至少 `max(48px, 3vw)`，上下至少 `max(32px, 3vh)`。
- 当前 `BigPictureWheel` 固定 194px 侧栏、`padding-bottom:16vh`、详情固定右侧 42vw，需要替换为 couch tokens 和容器查询。
- 当前 Big Picture 组件无媒体查询；0.12.1 必须覆盖宽度、低高度、宽高比和 4K 四类变化。

### 9.3 焦点与手柄状态机

每个 couch screen 定义一个显式 focus zone：`top-nav / wheel / hero-actions / media-rail / detail / search / keyboard`。

- D-pad/左摇杆：同 zone 内 roving。
- LB/RB：切换主 tab 或分页；必须在底部按键提示可见。
- A：激活；B：按返回栈关闭/返回；X/Y 仅分配高频次动作并显示提示。
- 焦点项必须进入视口；滚动完成后焦点与 selected id 一致。
- 手柄断开时保留焦点，键盘/鼠标可无缝接管。
- 持续按方向键需要可控 repeat：初始延迟约 320ms，之后 90-120ms；当前 edge-only 逻辑仅适合单次移动。
- Search/VirtualKeyboard 打开时暂停底层 gamepad handler，关闭后恢复触发按钮焦点，避免多个 `attachGamepad` 同时响应。
- `role=application` 只用于确有必要的虚拟键盘区域，并提供退出方式；普通列表不使用 application role。

### 9.4 couch 可读性

- 操作文本 ≥16px，正文 ≥16px，元数据 ≥14px。
- 选中态至少包含 3px 高对比轮廓 + 轻微 scale/位置变化，不能只改变颜色。
- 底部按键提示使用图形 + 文本，不依赖控制器品牌；可在设置中切换 Xbox/PlayStation/Nintendo glyph。
- 背景图上的文字必须通过稳定 scrim 保证对比，不允许依赖图片本身恰好较暗。

---

## 10. 动效系统与 reduced-motion

### 10.1 动效等级

| 等级 | 时长 | 用途 |
|---|---:|---|
| instant | 80ms | pressed/hover 微反馈 |
| fast | 140ms | tooltip、focus indicator、局部显隐 |
| standard | 220ms | tab、drawer 小位移、卡片状态 |
| enter | 280ms | 页面/大面板进入 |
| emphasis | 420ms | 背景交叉淡化、一次性结果揭示 |

- 列表 stagger 总时长不得超过 480ms；超过 12 项只动画首屏可见项或整体容器。
- exit 比 enter 快约 20%-30%。
- 循环动画只允许加载指示和明确运行状态；页面离开或不可见时暂停。

### 10.2 技术边界

**CSS/Svelte transition**：简单 hover、focus、opacity/transform 显隐。
**GSAP**：复杂序列、计数、可逆时间线、FLIP/焦点编排。
**禁止**：用 GSAP 替代普通 CSS hover；动画 width/height/top/left/margin；全局 selector 无 scope；卸载后继续运行。

GSAP 标准模板：

```ts
const motion = useMotionPreference();
let ctx: gsap.Context | undefined;

$effect(() => {
  if (!root || motion.reduced) return;
  ctx = gsap.context(() => {
    gsap.from(items, {
      opacity: 0,
      y: 12,
      duration: 0.28,
      ease: "power3.out",
      stagger: 0.03,
      clearProps: "transform,opacity",
    });
  }, root);
  return () => ctx?.revert();
});
```

### 10.3 reduced-motion 行为

`prefers-reduced-motion: reduce` 时：

- 页面、Drawer、Dialog：无位移/缩放，只允许 ≤80ms opacity，或直接切换。
- 列表：无 stagger。
- 背景：直接替换或 ≤100ms dissolve；关闭 spotlight、粒子和持续漂移。
- Skeleton：静态色块或低频 opacity，不使用横向 shimmer。
- 计数：直接显示最终值。
- 随机选择：直接选择结果，不进行 16 次快速翻牌。
- 焦点反馈保留，不能因为 reduced-motion 移除 focus ring。

偏好必须由统一 store 监听 MediaQuery 变化，不能每个组件 mount 时读取一次后永不更新。

### 10.4 现有 GSAP 迁移清单

| 文件 | 当前状态 | 迁移 |
|---|---|---|
| `switch/TileCard.svelte` | 有 context/reduced；同时 width transition | 保留 context，选中布局改预留槽位或 FLIP，清理永久 will-change |
| `switch/TileRail.svelte` | 有 context/reduced/cleanup | 接入 motion token；长列表只动画首屏 |
| `GameCard.svelte` | 有 context/reduced/cleanup | 由 Grid 容器统一入场，避免每卡独立 timeline |
| `GameGrid.svelte` | 有 context/reduced/cleanup | 与虚拟化/筛选重排统一，防重复动画 |
| `GameDetailPage.svelte` | gallery 入场有 cleanup | 只动画可见截图，详情打开恢复焦点 |
| `RatingRing.svelte` | 有 reduced/context | reduced 直接最终值，颜色改 data/status token |
| `StatsPage.svelte` | 有 reduced/context | 数字动画使用 motion helper，离屏不启动 |
| `ContinueHub.svelte` | 无 reduced/cleanup | 改 context + cleanup；或由 PageShell 统一容器入场 |
| `SettingsPage.svelte` | onMount tween 无 cleanup/reduced | 移除整页 stagger或使用 scoped context |
| `WhatToPlay.svelte` | timeline 无引用/kill/reduced | 持久 timeline；关闭/重选 kill；reduced 直接出结果 |
| `SavePanel.svelte` | GSAP import 未使用 | 删除 import |
| `App.svelte` | Svelte fade/fly 未统一降级 | 使用 motion-aware transition params 或关闭位移 |
| `BackgroundLayer.svelte` | timeout 驱动 crossfade | timeout 可清理；reduced 直接 swap；统一 420ms token |

---

## 11. 可访问性规格

目标：核心用户路径达到 WCAG 2.2 AA；高对比主题与 Big Picture 在电视距离上额外提高可读性。

### 11.1 语义与结构

- 每个页面一个 `main` 上下文和一个 `h1`；section 使用可见标题或 `aria-labelledby`。
- Dock 使用 `nav`；活动入口 `aria-current=page`。
- Drawer/Dialog/Sheet 使用正确 role、label、modal 状态、Escape、focus trap、焦点归还。
- 可点击实体优先 `<button>` 或 `<a>`；不使用只有 click 的 div。
- listbox/option 仅用于真正的单选复合控件；如果每个卡片本身是按钮，避免嵌套 button/listbox 混合语义。
- 加载/错误/状态区域使用最小必要 live region，避免页面更新时连续轰炸。

### 11.2 键盘

| 场景 | 必须支持 |
|---|---|
| Dock | Tab 进入；Enter/Space 激活；当前项可识别；快捷键不拦截输入框 |
| Segment/Tabs | Left/Right、Home/End、roving tabindex；激活策略一致 |
| Grid/Rail | Tab 可逐项；可选提供方向键加速，但不得截断标准 Tab |
| Dialog/Drawer | 初始焦点合理；Tab 圈闭；Escape 关闭；关闭后返回触发器 |
| Search | `/` 聚焦；Escape 先清除建议/关闭，再返回页面；清空按钮有名称 |
| Detail/Reader/Player | 返回动作可键盘触发；全屏状态可退出；控制条不只靠 hover |
| Dangerous action | Enter/Space 生效；确认 Dialog 默认焦点为取消/安全动作 |

### 11.3 焦点样式

- 所有主题焦点轮廓与相邻颜色对比至少 3:1。
- 普通模式 3px ring + 2px offset；Big Picture 3-4px ring + 非颜色位移/scale。
- hover、selected、active、focus 四种状态不能共用一个模糊 glow。
- `outline:none` 只能与等价或更清晰的 focus style 同时出现。
- 滚动容器获得焦点时，不显示与选中卡片竞争的第二个大 ring，但仍要有屏幕阅读器可理解的容器 label。

### 11.4 文本、颜色与图标

- 正文对比 ≥4.5:1；大文本 ≥3:1；非文本控件边界/状态 ≥3:1。
- 状态不能只靠红/绿；同时显示图标与文字。
- Icon-only button 必须有 `aria-label`，装饰图标 `aria-hidden=true`。
- tooltip 不承载必要操作说明；触摸/手柄必须能看到同等信息。
- 封面 alt 使用作品名 + 类型上下文；纯背景/氛围图 alt 为空。
- 图表必须有文本摘要或可访问数据表。

### 11.5 尺寸、缩放与媒体

- 普通模式目标最小 36×36px；主要动作 40×40px；Big Picture 56×56px。
- 200% 文本缩放下核心操作不重叠、不被截断；长中文/英文/日文名称均测试。
- 播放器控制有可见标签/名称、键盘焦点和音量状态；弹幕开关不只用图标。
- 漫画 Reader 的页码、阅读方向、缩放、退出可键盘操作；图片加载失败显示页级重试。

### 11.6 自动化与人工检查

- 建议新增 `@axe-core/playwright`（当前未安装，实施时显式加入 devDependency）跑 P0 页面；严重/关键 violation 为 0。
- 自动检查：accessible name、aria-current、Dialog focus、无键盘陷阱、颜色对比、无横向 overflow。
- 人工检查：NVDA + Windows、键盘-only、Xbox 类手柄、200% 缩放、高对比主题、reduced-motion。

---

## 12. 组件迁移清单（文件级）

迁移遵循“先基础、后壳、再页面、最后删兼容”的顺序；每个 checkbox 对应可独立评审的提交。为与 `AGENT-WORK-PACKAGES.md` 的 WP-A7 写入边界一致，0.12.1 新设计系统首先落在 `src/lib/components/ui-v2/**`、`src/lib/styles/**`、`src/lib/actions/a11y/**` 和 `tests/visual/baselines/**`。现有 `ui/**`、App 壳和 domain 页面通过适配器与集成提交迁移；若文件由其他工作包持有，不得直接覆盖其并发改动。

### 12.1 样式与主题基础

- [ ] `src/app.css`：改为分层入口；删除重复 primitive/pattern 规则；保留短期 alias 注释。
- [ ] `src/lib/styles/tokens.css`（新增）：字体、空间、尺寸、圆角、层级、动效、布局 token。
- [ ] `src/lib/styles/themes.css`（新增）：dark/light/black/sakura/contrast/cinema 完整语义 token。
- [ ] `src/lib/styles/base.css`（新增）：reset、`100dvh`、scrollbar、selection、全局 focus、reduced-motion fallback。
- [ ] `src/lib/styles/layout.css`（新增）：窗口等级、PageContainer、grid、density。
- [ ] `src/lib/styles/patterns.css`（新增）：仅稳定 pattern，不接收页面名 class。
- [ ] `src/main.ts`：仅导入需要的字体权重/格式与样式入口；评估减少 emitted font files。
- [ ] `src/lib/utils/theme.ts`：增加 theme contract、cinema/contrast 解析和测试辅助。
- [ ] `src/lib/stores/settings.svelte.ts`：持久化 density、motion override（system/reduce/full）与可选 transparency preference。
- [ ] `src/lib/utils/theme.test.ts`：覆盖全部主题、system 动态变化、无效值、cinema contrast。

### 12.2 UI v2 primitives 与兼容层

- [ ] `src/lib/components/ui-v2/index.ts`（新增）：冻结公开 API，并导出 PageShell/PageHeader/FilterBar/ContentGrid/DetailPanel/AsyncState。
- [ ] `src/lib/components/ui-v2/compat/**`（新增）：为旧 `ui/**` 提供短期 adapter，页面迁移完成后删除。

- [ ] `src/lib/components/ui/Button.svelte`：统一事件 API；icon-only label；loading announcement；couch size。
- [ ] `Card.svelte`：移除通用伪按钮；明确 surface/interactive/selected variants。
- [ ] `Dialog.svelte`：标题 id、description id、初始焦点、关闭策略、scroll body、reduced-motion。
- [ ] `Overlay.svelte`：scrim token、pointer/escape 策略、透明度降级。
- [ ] `Input.svelte`：id/name/label/error/description/invalid/autocomplete API。
- [ ] `SearchInput.svelte`：scope、result count、clear 后焦点、Escape 行为。
- [ ] `SegmentControl.svelte`：自定义 label、方向键、Home/End、roving tabindex、disabled。
- [ ] `Switch.svelte`：强制 label/ariaLabel、id/name、description、disabled reason。
- [ ] `Tooltip.svelte`：id + aria-describedby；hover/focus delay；reduced-motion；边界碰撞。
- [ ] `EmptyState.svelte`：区分 empty/no-results/error/offline；主次动作；live 策略。
- [ ] `Skeleton.svelte`、`LoadingSkeleton.svelte`：统一尺寸、静态 reduced 模式、父级 busy contract。
- [ ] `BackgroundLayer.svelte`：修复 aria 冲突、清理 timer、主题 scrim、reduced swap。
- [ ] `Rail.svelte`：方向键/滚动按钮/边缘渐隐/容器查询/焦点恢复。
- [ ] `StatBlock.svelte`、`Chart.svelte`：data token、文本摘要、loading/error 统一。
- [ ] `Tag.svelte`：区分 tag、filter chip、status badge；避免一个组件承担三种语义。
- [ ] `src/lib/components/ui-v2/Drawer.svelte`（新增）：工具抽屉/移动 panel 的统一语义和 focus 管理。
- [ ] `src/lib/components/ui-v2/PageShell.svelte`、`PageHeader.svelte`、`FilterBar.svelte`、`ContentGrid.svelte`（新增）：四主入口壳。
- [ ] `src/lib/components/ui-v2/AsyncState.svelte`、`AsyncSection.svelte`（新增）：异步状态契约；内部可使用本文 `StateBoundary` 状态机。
- [ ] `src/lib/components/ui-v2/MediaCard.svelte`、`MediaRow.svelte`、`DetailPanel.svelte`（新增）：跨域内容 pattern。
- [ ] `src/lib/components/ui/index.ts`：只导出稳定组件；标记 deprecated 兼容组件。
- [ ] `src/lib/actions/a11y/focusTrap.ts`（新增）：支持 initialFocus selector、关闭后触发器回焦、嵌套层级；旧 action 通过 adapter 迁移。
- [ ] `src/lib/actions/a11y/spotlight.ts`（新增）：监听 preference 变化；页面不可见/降透明时停止 pointer work；旧 action 通过 adapter 迁移。
- [ ] 新增 primitive 单测：键盘、aria、focus restore、reduced-motion、disabled/loading。

### 12.3 应用壳、导航与跨域

- [ ] `src/App.svelte`：RouteBoundary、motion-aware view transition、Drawer、焦点/返回栈、动态 import error UI。
- [ ] `src/lib/nav.ts`：修正文档与快捷键唯一映射；明确 content/utility/mode/group。
- [ ] `src/lib/nav.test.ts`：校验 id/view/shortcut 唯一性、四主入口顺序、可访问 label。
- [ ] `src/lib/stores/ui.svelte.ts`：移除重复 drawer 状态或与 App 单一化；增加 density/motion 派生状态。
- [ ] `src/lib/stores/router.svelte.ts`：保存 focusKey/scrollOffset/overlay stack；实现分层返回。
- [ ] `src/lib/components/switch/SystemDock.svelte`：aria-current、expanded/controls、窄窗策略、统一 target。
- [ ] `src/lib/components/ShortcutHelp.svelte`：快捷键来自 nav/shortcut 配置，不手写漂移。
- [ ] `src/lib/components/CommandDrawer.svelte`：与新 Drawer/Search 语义对齐。
- [ ] `src/lib/components/Notifications.svelte`：live region、暂停消失、操作反馈去重。
- [ ] `src/lib/components/ContinueHub.svelte`、`ContinueCard.svelte`：复用 MediaCard/Row/Stat，迁移 GSAP。

### 12.4 四主入口与详情

- [ ] `switch/SwitchHome.svelte`：PageShell（全部游戏）、共享 toolbar/state；保留精选舞台作为 pattern。
- [ ] `switch/TileRail.svelte`、`TileCard.svelte`：焦点模型、FLIP/预留布局、长列表动效与 couch/desktop target。
- [ ] `GameGrid.svelte`、`GameCard.svelte`：MediaCard/grid、入场统一、状态 badge。
- [ ] `GameDetailPage.svelte`：EntityDetailShell、Dialog/StateBoundary、窄窗单栏、焦点归还。
- [ ] `PlayRecordsDashboard.svelte`：PageShell、Stat/Chart/MediaRow；拆出 section，删除控制台私有视觉。
- [ ] `AnimePage.svelte`：PageShell、toolbar、AsyncSection、MediaCard；补窗口/container 响应式。
- [ ] `anime/AnimeDetail.svelte`：EntityDetailShell、线路/选集共享控件、错误层级。
- [ ] `anime/SearchDrawer.svelte`、`SourceSheet.svelte`：Drawer/Dialog primitive 与 focus restore。
- [ ] `anime/AnimePlayer.svelte`、`DanmakuOverlay.svelte`：控制条可访问性、reduced-motion、错误/备用源反馈。
- [ ] `ComicPage.svelte`：统一普通/PicACG header、toolbar、source sections、MediaCard/StateBoundary。
- [ ] `comic/ComicDetail.svelte`：EntityDetailShell、章节状态、焦点恢复。
- [ ] `comic/ComicReader.svelte`：沉浸壳、键盘/阅读方向、页级加载失败、窄窗/大屏。

### 12.5 Big Picture

- [ ] `BigPicturePage.svelte`：CouchShell、focus zones、返回栈、cinema theme、宽高比/低高度布局。
- [ ] `bigpicture/BigPictureBackground.svelte`：统一 BackgroundLayer 与 scrim/reduced 策略。
- [ ] `bigpicture/BigPictureWheel.svelte`：容器查询、4K/低高度、roving tabindex、滚动与选中同步。
- [ ] `bigpicture/BigPictureHero.svelte`：couch typography、action target、内容截断与详情入口。
- [ ] `BigPictureDetail.svelte`：couch Drawer/DetailShell、操作分层、焦点圈闭/归还。
- [ ] `BPSearch.svelte`：共享搜索模型、VirtualKeyboard 互斥、结果焦点与清空。
- [ ] `BPMediaRail.svelte`、`bigpicture/BigPictureMediaTab.svelte`：共享 rail、媒体状态、肩键提示。
- [ ] `switch/useGamepad.svelte.ts`：handler scope/优先级、repeat、active zone、测试注入。
- [ ] `VirtualKeyboard.svelte`：可访问名称、退出、手柄 handler 独占、布局/符号切换公告。

### 12.6 动效与装饰

- [ ] `src/lib/stores/motion.svelte.ts`（新增）：system preference、用户 override、透明度/粒子派生值。
- [ ] `src/lib/actions/a11y/gamepadFocus.ts`（新增）：focus zone、handler scope、焦点归还与手柄 repeat 共用逻辑。
- [ ] `src/lib/utils/motion.ts`（新增）：GSAP context helper、duration/ease 映射、test disable。
- [ ] `SakuraParticles.svelte`：reduced/visibility/主题条件、粒子数量性能上限。
- [ ] `SettingsPage.svelte`、`WhatToPlay.svelte`、`SavePanel.svelte`、`RatingRing.svelte`、`StatsPage.svelte`：按第 10.4 节迁移。

---
## 13. 视觉验收基线与测试/截图矩阵

### 13.1 测试基础设施

改造 `playwright.config.ts`：

- `desktop-standard`：1440×900，DPR 1。
- `desktop-compact`：960×640，DPR 1。
- `desktop-narrow`：720×600，DPR 1。
- `couch-1080p`：1920×1080，DPR 1。
- `couch-4k`：3840×2160，DPR 1（CI 可按 0.5 screenshot scale 或独立 nightly）。
- 每个 project 可设置 `reducedMotion: "reduce"` 的对应检查，不需与全部页面笛卡尔积。

新增确定性 fixture：

```text
tests/visual/fixtures/
  app-fixture.ts       # mock Tauri invoke、时钟、随机数、图片响应
  games.ts
  records.ts
  anime.ts
  comic.ts
  themes.ts
```

截图前固定：

- `Date.now` / 时钟 / timezone。
- 随机数和排序种子。
- 动画完成或通过 test flag 关闭；caret 隐藏。
- 网络图片使用本地 fixture，不依赖公网。
- 字体加载完成：`document.fonts.ready`。
- 等待页面明确的 `data-ui-ready=true`，禁止只 `waitForTimeout`。

### 13.2 P0 截图矩阵

| Screenshot ID | 页面/状态 | 视口 | 主题 | 输入/动效 | 关键断言 |
|---|---|---:|---|---|---|
| shell-standard-dark | 游戏库 populated | 1440×900 | dark | keyboard/full | Dock 分组、active、无 overflow |
| shell-narrow-dark | 游戏库 populated | 720×600 | dark | keyboard/full | Dock/toolbar 折叠，主动作可达 |
| shell-standard-light | 游戏库 populated | 1440×900 | light | keyboard/full | 无暗色硬编码块、文本对比 |
| shell-contrast | 游戏库 populated | 1440×900 | contrast | keyboard/reduce | 无玻璃弱边框、焦点清晰 |
| tools-drawer | 工具抽屉 open | 960×640 | dark | keyboard/reduce | 分组、焦点圈闭、背景不可聚焦 |
| games-loading | 游戏库首次加载 | 1440×900 | dark | reduce | 同形 skeleton、header/dock 保留 |
| games-empty | 游戏库为空 | 1440×900 | dark | full | 导入/添加动作与说明 |
| games-error | 游戏库失败 | 960×640 | dark | reduce | 摘要、重试、details |
| games-detail | 游戏详情 populated | 1440×900 | dark | keyboard/full | 主要动作、详情层级、focus |
| records-populated | 记录 populated | 1440×900 | dark | reduce | stats/chart/timeline 一致性 |
| records-empty | 记录 empty | 960×640 | light | reduce | onboarding，不出现空白大卡 |
| anime-home | 番剧推荐 | 1440×900 | dark | reduce | PageShell/section/card |
| anime-search-empty | 番剧无结果 | 960×640 | dark | keyboard/reduce | 查询、清除筛选、来源 |
| anime-detail | 番剧详情/选集 | 1440×900 | sakura | keyboard/reduce | 详情壳、线路/集数焦点 |
| anime-player-error | 播放失败自动切源 | 1440×900 | dark | keyboard/reduce | 错误层级、备用源反馈 |
| comic-auto-sources | 漫画多源并行 | 1440×900 | dark | reduce | section loading/error/ready 同构 |
| comic-empty | 漫画无结果 | 960×640 | light | keyboard/reduce | 空状态与工具栏 |
| comic-detail | 漫画详情/章节 | 1440×900 | dark | keyboard/reduce | 详情壳、章节可见焦点 |
| comic-reader | Reader controls visible | 1440×900 | black | keyboard/reduce | 页码/方向/退出可达 |
| bp-home-1080 | Big Picture 游戏 | 1920×1080 | cinema | gamepad/full | focus ring、安全区、按键提示 |
| bp-media-1080 | Big Picture 媒体 | 1920×1080 | cinema | gamepad/reduce | rail、tab、焦点同步 |
| bp-search-1080 | Big Picture 搜索 | 1920×1080 | cinema | gamepad/reduce | keyboard 独占、结果焦点 |
| bp-detail-1080 | Big Picture 详情 | 1920×1080 | cinema | gamepad/reduce | Drawer、返回焦点 |
| bp-home-4k | Big Picture 游戏 | 3840×2160 | cinema | gamepad/reduce | 字号/容器不失衡，资源清晰 |
| bp-low-height | Big Picture 游戏 | 1280×720 | cinema | gamepad/reduce | hero/wheel/提示不裁切 |

### 13.3 Pairwise 扩展矩阵

不做 5 主题 × 25 场景 × 5 尺寸的全组合；采用 pairwise：

- 五主题至少各覆盖 shell、一个数据页、一个 overlay。
- 三普通窗口等级至少各覆盖四主入口之一。
- reduced-motion 覆盖页面进入、Drawer、Dialog、Skeleton、背景、GSAP 结果动画。
- keyboard 覆盖 Dock→Toolbar→Grid→Detail→Back；gamepad 覆盖 Big Picture 主路径。
- 每个异步状态至少在一个主入口截图，状态组件本身另做 component screenshot。

### 13.4 视觉阈值与评审

- 核心壳与 primitive：`maxDiffPixelRatio <= 0.003`。
- 媒体封面/视频占位：`<= 0.005`，图片使用固定 fixture。
- 抗锯齿敏感区域可 mask 动态文本，但不得 mask 整个组件。
- snapshot 更新必须附变更理由和 before/after；禁止 CI 失败后无审查批量更新。
- 截图之外必须断言：无横向 overflow、焦点元素可见、active/current aria、Dialog focus restore。

### 13.5 视觉验收 checklist

- [ ] 对齐使用 4px grid，无随机 13/17/19px 间距。
- [ ] 页面标题、toolbar、首个内容块在四主入口位置一致。
- [ ] 同类按钮、卡片、标签、空状态在主题间结构不变。
- [ ] 窄窗没有关键文字重叠、按钮消失、不可见滚动条承载唯一入口。
- [ ] 主题切换后没有明显暗色残留块、白字白底或低对比边框。
- [ ] focus 与 selected 可同时辨认；鼠标 hover 不覆盖键盘 focus。
- [ ] reduced-motion 截图中没有中间动画态。
- [ ] Big Picture 在沙发距离可读，底部按键提示与焦点始终可见。

---

## 14. 性能预算

### 14.1 构建预算

| 指标 | 当前审计值 | 0.12.1 预算 | 门禁 |
|---|---:|---:|---|
| 初始 JS gzip | 328.89 KB | ≤300 KB，且不得因设计系统增加 | build size report |
| 初始 CSS gzip | 38.50 KB | ≤34 KB | build size report |
| 全部 CSS raw | 298.4 KB | ≤250 KB | build size report |
| Anime route JS gzip | 191.75 KB | ≤170 KB 或拆出播放器/图表异步 chunk | route chunk report |
| 字体 emitted raw | 462.8 KB / 43 files | ≤300 KB；运行时首屏加载 ≤120 KB | asset report + network |
| 默认背景图 | 1.55 MB PNG | ≤600 KB WebP/AVIF，保留合理分辨率 | asset lint |
| 单个新增 UI 图片 | — | ≤250 KB；4K 背景可例外但必须响应式 source | asset lint |

设计系统迁移不应通过把所有 primitives 打进首屏来“统一”；按路由使用的复杂组件仍需 code split。

### 14.2 运行时预算

参考机：Windows 11、集成显卡、1920×1080、release build。

- 冷启动到壳可交互：≤1.5s；到首个内容 skeleton：≤800ms。
- 已加载路由切换到可交互：p95 ≤250ms。
- 键盘/手柄输入到焦点反馈：p95 ≤100ms。
- 连续 rail 导航：平均 ≥55 FPS，1% low ≥45 FPS。
- 页面切换期间 >50ms long task：最多 1 个；>100ms：0。
- 背景 crossfade 不触发 layout；图片 decode 不阻塞主线程明显输入。
- 静止页面 CPU 接近空闲；粒子、时钟、gamepad RAF 在不可见/未连接时停止。
- 一次页面 mount 后，卸载时 GSAP global timeline 不残留该页面 tween。

### 14.3 性能实现策略

- 背景图提供 WebP/AVIF 和尺寸变体，预解码下一张；只保留 current/previous 两层。
- `backdrop-filter` 仅用于 Drawer/Dialog/Dock 等少数层；contrast/low-transparency 关闭。
- 长网格/rail 使用窗口化或至少可见范围渲染；不为每张卡长期设置 `will-change`。
- GSAP 只动画 transform/opacity；动画结束 `clearProps`/移除 will-change。
- Chart/HLS/复杂 reader/player 在进入子视图时动态加载。
- 字体只保留实际权重和 woff2；校验 unicode-range 与运行时请求，不只看 emitted 数量。

---

## 15. 任务拆分与依赖

| Task | 内容 | 依赖 | 规模 | 完成产物 |
|---|---|---|---:|---|
| UI-01 | 冻结审计基线、fixture、现状截图、size report | 无 | S | baseline 报告与现状 screenshots |
| UI-02 | tokens/themes/base/layout 分层；兼容 alias | UI-01 | M | styles 目录、主题测试 |
| UI-03 | motion store/helper、reduced-motion 全链路 | UI-02 | M | CSS/Svelte/GSAP 共用偏好 |
| UI-04 | primitive a11y/API 迁移 | UI-02/03 | L | Button/Input/Dialog/Segment/Switch 等稳定 API |
| UI-05 | PageShell/Toolbar/StateBoundary/Media patterns | UI-04 | L | 页面共用 patterns + component tests |
| UI-06 | App shell、RouteBoundary、Dock、Drawer、返回/焦点栈 | UI-04/05 | L | 壳层与 IA 完成 |
| UI-07 | 游戏库/详情迁移 | UI-05/06 | L | game P0 screenshots |
| UI-08 | 记录/Continue 迁移 | UI-05/06 | L | records/continue screenshots |
| UI-09 | 番剧列表/详情/播放器状态迁移 | UI-05/06 | XL | anime screenshots/E2E |
| UI-10 | 漫画多源/详情/Reader 迁移 | UI-05/06 | XL | comic screenshots/E2E |
| UI-11 | 普通窗口响应式与 container query 收敛 | UI-07..10 | L | narrow/compact/wide matrix |
| UI-12 | CouchShell、Big Picture、gamepad zones/repeat | UI-03/04/06 | XL | 1080p/4K gamepad matrix |
| UI-13 | axe/键盘/NVDA/contrast 人工验收与修复 | UI-07..12 | L | a11y report，P0 violation=0 |
| UI-14 | 视觉截图 CI、pairwise matrix、snapshot review 规则 | UI-01/05 | L | Playwright screenshot suite |
| UI-15 | CSS/字体/背景/chunk/帧率优化 | 全程并行，收尾 UI-07..12 | M | budget report，无超标 |
| UI-16 | 删除兼容 alias/旧 page CSS/死 GSAP import | UI-07..15 | M | debt 清理与 final diff audit |

建议并行边界：

- Foundation lane：UI-02/03/04/05。
- Shell lane：UI-06/14。
- Domain lane：UI-07/08 与 UI-09/10 分开，但共享 pattern 后再开工。
- Couch lane：UI-12。
- Quality lane：UI-13/15 持续跟进，不在最后一天集中补。

任何 domain lane 若需要修改 primitive API，先回 Foundation lane 更新和测试，禁止在页面内复制临时组件。

---

## 16. 验收标准

### 16.1 设计系统

- AC-DS-01：新代码中无页面命名颜色 token；P0 页面硬编码颜色引用较基线下降至少 80%，剩余例外有注释。
- AC-DS-02：五主题和 cinema/contrast 完整定义语义 token，运行时不存在未解析 CSS variable。
- AC-DS-03：空间、字号、圆角、层级、动效均来自 token；新增一次性 z-index/断点需评审。
- AC-DS-04：primitive 样式所有权唯一，不再同时由 `app.css` 与组件私有规则维护同一职责。

### 16.2 信息架构与四主入口

- AC-IA-01：Dock 清晰显示四内容入口、工具/设置、模式切换；active/current/expanded 语义正确。
- AC-IA-02：四主入口均使用 PageShell；标题、toolbar、state、内容层级一致。
- AC-IA-03：详情返回后恢复列表焦点与滚动位置；Escape/B 按层级关闭，不无条件回游戏库。
- AC-IA-04：Continue/发现/工具不再维护另一套页面语言。

### 16.3 状态与反馈

- AC-ST-01：每个四主入口至少有 loading/empty/error/ready 自动化覆盖。
- AC-ST-02：局部刷新保留旧数据；首次加载骨架与最终布局近似，CLS 不明显。
- AC-ST-03：错误提供重试与可折叠详情；离线不阻塞可用本地内容。
- AC-ST-04：所有异步按钮防重复提交并提供 `aria-busy`/局部反馈。

### 16.4 响应式与大屏

- AC-RS-01：720×600、960×640、1440×900、2560×1440 均无页面根横向 overflow 和关键控件裁切。
- AC-RS-02：仓库页面级宽度断点只使用窗口等级；新组件优先 container query。
- AC-BP-01：Big Picture 在 1080p/2K/4K 和 1280×720 低高度布局通过截图。
- AC-BP-02：仅手柄可完成进入大屏→选择→详情→启动/返回→搜索→退出。
- AC-BP-03：焦点永远可见且与 selected item 一致；overlay 打开时底层 handler 不响应。

### 16.5 动效与可访问性

- AC-MO-01：所有 GSAP tween/timeline 有 scope、cleanup 和 reduced 路径；无页面卸载残留。
- AC-MO-02：reduced-motion 下无位移/缩放/stagger/粒子/shimmer/快速翻牌。
- AC-A11Y-01：P0 Playwright axe critical/serious violation 为 0。
- AC-A11Y-02：键盘完成 Dock→四主入口→列表→详情→Dialog→返回；无键盘陷阱。
- AC-A11Y-03：正文/控件/焦点对比达到 WCAG 2.2 AA；状态不只靠颜色。
- AC-A11Y-04：200% 文本缩放和长标题下核心操作仍可达。

### 16.6 测试与性能

- AC-QA-01：第 13.2 节 P0 screenshots 全部进入 CI，并有 review 规则。
- AC-QA-02：现有 155 个通过单测和三条功能 E2E 不回退；新增 primitive/nav/motion 测试通过。
- AC-QA-03：无不稳定公网图片、真实时钟、随机排序导致的截图漂移。
- AC-PERF-01：满足第 14 节构建预算；超标必须有带期限豁免。
- AC-PERF-02：参考机 rail 导航平均 ≥55 FPS，输入反馈 p95 ≤100ms。

---

## 17. 风险与缓解

| 风险 | 概率/影响 | 触发信号 | 缓解 |
|---|---|---|---|
| 大页面迁移引入业务回归 | 高/高 | Anime/Comic E2E 失败、状态分支丢失 | 绞杀式迁移；先抽 pattern，不改 store/API；保持现有 E2E |
| 全局 token 替换导致主题连锁回归 | 高/高 | 浅色/contrast 大面积不可读 | 先加 alias、逐页切换、每批跑主题 screenshots，不全局搜索替换 |
| primitive API 频繁变化阻塞并行代理 | 高/中 | 页面分支各自 fork Button/Card | Foundation 先冻结 API；变更走单独 RFC/commit，不在页面内复制 |
| Big Picture 焦点与多 handler 冲突 | 中/高 | 一次按键触发两层、关闭后焦点丢失 | gamepad scope stack；overlay 独占；自动化注入 gamepad events |
| 动效 cleanup 漏洞 | 中/中 | 重入页面重复动画、global timeline 增长 | motion helper + lifecycle test + DevTools/测试计数 |
| 视觉测试脆弱 | 高/中 | CI 像素随机漂移 | 固定字体/时间/图片/动画；小而明确的 mask；pairwise 而非全组合 |
| CSS 迁移短期体积反增 | 高/中 | 新旧规则并存、initial CSS 超预算 | 每个里程碑必须删除对应旧 CSS；size report 按 PR 对比 |
| 玻璃/背景/粒子导致低端 GPU 掉帧 | 中/高 | rail/背景切换 <45 FPS | transparency preference、contrast 降级、资产压缩、静止暂停 |
| 小字号在电视上不可读 | 中/高 | 1080p 沙发测试无法读元数据 | couch typography 独立下限；真实电视/远距人工验收 |
| IA 调整让旧快捷键失效 | 中/中 | 帮助、Dock、实际行为不一致 | nav 单一映射 + 单测；保留兼容快捷键并显示迁移提示 |
| 并发工作覆盖他人改动 | 中/高 | 同一大页面冲突、生成文件变化 | 文件 owner 划分；小提交；不回退无关改动；合并前重新审计 |

---

## 18. 里程碑与门禁

### M0 — Baseline Freeze

- 完成 UI-01。
- 固定审计 commit、现状截图、功能测试和 size report。
- Gate：现有 check/unit/E2E 全绿；任何后续差异可追踪。

### M1 — Foundation Ready

- 完成 UI-02/03/04/05 的 P0 部分。
- token、主题、motion、PageShell、StateBoundary、Drawer、核心 primitives 可用。
- Gate：primitive 单测、五主题 component screenshots、reduced-motion 通过；API 冻结。

### M2 — Shell & Four Pillars

- 完成 UI-06/07/08/09/10 的列表与详情主路径。
- Gate：四主入口 PageShell/状态一致；普通模式 P0 screenshots 通过；原 E2E 不回退。

### M3 — Responsive & Couch

- 完成 UI-11/12。
- Gate：四普通尺寸 + Big Picture 1080p/4K/低高度；键盘/手柄主路径通过。

### M4 — Accessibility & Performance Release Candidate

- 完成 UI-13/14/15/16。
- Gate：axe P0 无 serious/critical、人工键盘/NVDA/手柄通过、性能预算通过、兼容 CSS 已删除。

若 M1 未冻结 primitive API，不允许同时大规模迁移四个 domain；若 M3 手柄焦点状态机未通过，不允许以“鼠标可用”签署 Big Picture 验收。

---

## 19. 实施守则与非目标

### 必须做

- 以现有业务能力为准，补齐壳、状态、语义、响应式和测试。
- 每次迁移同时删除对应私有通用样式，避免新旧双份长期存在。
- 所有视觉改动附状态/尺寸/主题证据。
- 大屏在真实手柄和真实 1080p 显示设备至少人工验收一次。

### 不做

- 不在 0.12.1 引入新的大型 UI framework 或第二套 icon 库。
- 不因统一设计而抹平游戏舞台、播放器、Reader 等必要沉浸差异。
- 不把所有页面一次性改成相同卡片 Dashboard。
- 不用更多渐变、玻璃、粒子掩盖 IA、状态和可读性问题。
- 不为了截图通过而隐藏动态区域、关闭真实错误或大面积 mask。
- 不回退与本专项无关的并发改动；冲突时只重放本文件/本专项明确拥有的变更。

---

## 20. 最终交付物

1. 语义 token + 主题 + 密度 + motion 基础。
2. 稳定的 UI primitives、PageShell、StateBoundary、Media patterns。
3. 四主入口和详情的一致壳层/状态/响应式实现。
4. 可用的 Big Picture/CouchShell 与手柄焦点状态机。
5. WCAG 2.2 AA 核心路径报告。
6. P0 视觉截图矩阵、pairwise 扩展和 CI 门禁。
7. 构建体积、字体/背景资源、路由交互和动画帧率报告。
8. 旧 CSS/token/GSAP 债务删除清单和 0.12.2 后续项。

本规格的核心判断是：**0.12.1 应把 MoePlay 从“多个功能页各自做得像应用”推进为“一个应用稳定承载多个媒体领域”**。视觉统一只是结果；真正的交付是信息架构、状态契约、输入等价、响应式规则、生命周期和自动化验收都能复用。
