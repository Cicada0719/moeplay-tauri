# 萌游 MoeGame · Codex 重构方案 R2 —— 双主机 UI + 收尾后端

> 版本：R2 · 生成：2026-06-11 · 指挥/策划：Claude（实测 + 代码解析）· 执行：Codex
> 本文为**当前唯一权威施工说明**，取代已删除的 R1（`CODEX_REBUILD_PLAN.md`）。R1 的稳定性/设计系统任务已基本落地（见 §0），R2 只管**两件事**：① 把 UI 推倒重做成「主界面 NS Switch 风 + 大屏 PS5 风」；② 收尾仍开放的后端（灌库 / Steam·Epic 全库 / 模型收敛 / 巨石拆分）。
> 阅读顺序：§0 现状 → §1 设计总纲（最重要）→ §2 UI 任务卡 → §3 后端任务卡 → §4 里程碑。

---

## 0. R1 验收结论（实测 2026-06-11）

R1 由 Codex 执行，**稳定性 + 设计系统 + 主页渲染**这条线完成度不错；前端 `svelte-check` 0 error、`vitest` 5/5、注入样本数据后主界面可正常渲染全部 12 游戏 / 统计 / rail。

| 编号 | 项 | 状态 | 证据 |
|---|---|---|---|
| BUG-2 | 单实例锁 | ✅ 已修 | `Cargo.toml` 引入 `tauri-plugin-single-instance`，`lib.rs:64` 注册并聚焦已有窗口 |
| BUG-3 | 首页卡死 loading | ✅ 已修 | `App.svelte` 用 `onMount`+`booted` 守卫替代裸 `$effect`；`LibraryView` 三态链正确命中 |
| BUG-4 | 状态大小写错位 | ✅ 已修 | 新增 `utils/game.ts::normalizeCompletionStatus()`，全组件统一走它；含单测 |
| BUG-5 | 原始 TypeError 抛给用户 | ✅ 已修 | `userFacingErrorMessage()` 过滤；`LibraryView` 有 `loadError` banner + 重试 |
| BUG-9 | CSP 为 null | ✅ 已修 | `tauri.conf.json` 改为完整 CSP 串 |
| 设计系统 | 令牌 v4 + UI 组件库 | ✅ 已起步 | `app.css` 暗/亮/樱花三主题；`ui/` 有 Button/Card/Rail/Skeleton/StatBlock/Tag |
| **BUG-1** | **迁移桥灌库** | ⚠️ **代码就绪，未灌库** | `csharp_migration.rs` 已内嵌 `export_playnite_litedb.ps1`（LiteDB→JSON→SQLite + `verify_migration` + 烟雾测试）。**但真实 DB 仍 `games=0`，从未成功跑通**；旧 portable 库 `games.db` 仅 8KB（疑空），真库路径待锁定。 |
| BUG-6 | 数据模型三重冗余 | ⚠️ 部分 | `utils/game.ts` 加了取值器兜底，但 model 本身未收敛 |
| BUG-7 | Steam/Epic 全库 | ❌ 未做 | 无时长/最后游玩/成就导入；Epic 无账号全库 |
| BUG-8 | `commands.rs` 3.5k 行巨石 | ❌ 未做 | 仍单文件 |

**最致命的现实没变：应用里 0 游戏，作为"启动器"仍不可用。** R2 的 P0 不是"写迁移代码"（已写），而是**真机把旧库灌进去并验收**——这是所有 UI 验收的前提。

---

## 1. 设计总纲：双主机语言（R2 核心）

### 1.1 方向锁定【已与项目方确认 · 2026-06-11】
- **主界面（窗口态）= Nintendo Switch 主页风** —— **暗色（Basic Black）** + **竖版封面磁贴**。
- **大屏模式（Big Picture / 手柄 / TV 态）= PlayStation 5 主页风** —— **暗色** + 全屏电影化大图。
- 两套外壳**共用一套暗色令牌基底**，差异在"外壳层"。品牌强调色沿用克制玫红 `#E8557F`，**仅用于选中/主操作**，其余一律中性。中文 UI 全保留，**无 emoji**（全 SVG 图标）。

**为什么这样分：** Switch 主页的精髓是**克制**——扁平黑底上漂浮一排封面，底部一行系统坞，没有多余面板，最适合鼠标窗口操作；现有 `LibraryView` 把 hero 大图 + 焦点区 + 统计面板 + 三条 rail + 手柄提示栏全堆进主窗口，又挤又杂（正是"主界面奇差"的来源）。PS5 主页的电影化大图、侧栏活动卡、沉浸感，则**搬到大屏模式**那个真正需要沉浸的场景。

### 1.2 主界面 = NS Switch（暗色 · 竖封面）

**布局骨架（替换 `LibraryView` 的 home 分支，整屏重做）：**
```
┌──────────────────────────────────────────────────────────────┐
│  [用户头像]                              17:00   ⤢搜索  ⚙        │  ← 顶栏 ~48px，极简：左头像，右时钟/搜索/设置
│                                                                │
│   ┌─────────┐                                                  │
│   │         │  ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐  …  ┌──┐    │  ← 封面磁贴横排（核心）
│   │  选中    │  │    │ │    │ │    │ │    │ │    │     │全 │    │    选中=放大竖封面(约300×400)
│   │  竖封面  │  │idle│ │idle│ │idle│ │idle│ │idle│     │部 │    │    其余idle(约150×200)向右拖尾
│   │ 300×400 │  └────┘ └────┘ └────┘ └────┘ └────┘     └──┘    │    末尾"全部游戏"磁贴→全库网格
│   └─────────┘                                                  │
│   白色相簿2                                          ▶ 开始游戏  │  ← 选中游戏标题(大) + 一行副信息 + 主操作
│   Leaf · 2003 · 已通关 · 36h                                    │
│                                                                │
│  ─────────────────────────────────────────────────────────    │
│   ◎发现  ◎刮削  ◎下载  ◎存档  ◎统计  ◎导入  ◎模拟器  ◎设置  ◎大屏 │  ← 底部系统坞，单色线性圆形图标
└──────────────────────────────────────────────────────────────┘
```

**关键规格：**
- **背景**：纯近黑 `--bg-void:#07090F`，**扁平、无 hero 大图、无玻璃模糊、无渐变光晕**。可加极淡 vignette（径向，≤4% 黑）。这是与 PS5 最大的视觉区分点。
- **封面磁贴**：竖版 3:4（保留 galgame 美术），`object-fit:cover`。选中态放大 + **白色选择环**（Switch 招牌：`box-shadow:0 0 0 3px rgba(255,255,255,.92)`）+ 轻微抬升；idle 态略暗（`brightness(.82)`）。无封面→「首字母字母组 + 渐变占位」。
- **rail 内容 = 最近游玩**（按 `last_played` 倒序，未玩补后），上限 ~16，**末尾固定"全部游戏"磁贴** → 切到全库网格（G2）。
- **选中信息区**（rail 下方左侧）：游戏名（`--font-display`，约 28–34px）+ 一行 `开发商 · 年份 · 状态 · 时长` + **`▶ 开始游戏`** 主按钮（玫红，唯一彩色元素）+ 次级（收藏 / 详情）。
- **底部系统坞**：单色线性图标，圆形 hover 底（`rgba(255,255,255,.08)`），尺寸 ~44px。映射 moeplay 功能：发现 / 刮削 / 下载 / 存档 / 统计 / Steam·Epic导入 / 模拟器 / 设置 / **大屏模式（→PS5）**。
- **导航**：← →（或 A/D / 手柄左摇杆 / LB·RB）在 rail 内移动选中，选中磁贴 `scrollIntoView({inline:'center'})`；Enter/单击磁贴 = 打开详情；Space/双击/手柄 A = 直接启动；点底部图标进对应工具页。
- **搜索**：顶栏右侧放大镜 → 展开搜索框（pinyin-pro 已有）；输入时 rail 实时过滤。

### 1.3 大屏模式 = PS5（暗色 · 电影化）

**精炼现有 `BigPicturePage.svelte`（已是雏形）成正统 PS5：**
```
┌──────────────────────────────────────────────────────────────┐
│   游戏   媒体                                  [头像] ⤢ ⚙ 17:00 │  ← 顶栏分段导航
│                                                                │
│         ╔══════════════════════════════════════════╗          │
│         ║      选中游戏的全屏背景大图(background)      ║          │  ← 全出血电影化大图，随选中crossfade
│   白色相簿2                                                     │     左下：大标题/Logo
│   Leaf · 2003                                                  │
│   ▶ 开始游戏    ♡ 收藏    ⓘ 详情          ┌──────────────┐      │  ← 大主按钮 + 右侧活动/统计卡
│                                          │ 成就 18/20   │      │
│  ░░░░░░░░░░░░ 底部压暗 scrim ░░░░░░░░░░░░  │ 本周 8.0h    │      │
│   ┌──┐ ┌──┐ ┌──┐ ┌══┐ ┌──┐ ┌──┐ ┌──┐    │ 库完成度 33% │      │
│   │  │ │  │ │  │ ║选║ │  │ │  │ │  │      └──────────────┘      │  ← 封面卡横排，选中=白环+抬升
│   └──┘ └──┘ └──┘ ╚══╝ └──┘ └──┘ └──┘                          │     换选中→换整屏背景
└──────────────────────────────────────────────────────────────┘
```

**关键规格：**
- **背景**：选中游戏的 `background`/`screenshots[0]`（横版），全出血；底部 + 左侧 scrim 渐变压暗保证文字可读。**换选中 → 整屏背景 0.6s crossfade**（PS5 招牌）。这正是刮削必须抓**横版背景图**的原因（见 E-1）。
- **大标题**：`clamp(40px,6vw,72px)`；主按钮特大（手柄/10 尺可达）。
- **右侧活动卡**：把 R1 塞在主窗口的"成就 / 本周时长 / 库完成度 / 继续游玩"等富信息**移到这里**。
- **封面卡 rail**：竖版卡，选中 = 白色选择环 + 抬升 + 轻微 scale；可保留评分环（现有 `RatingRing`）。
- **导航**：手柄/方向键优先；LB·RB 翻页；A 启动，X/Y 收藏/详情。退出大屏回到 Switch 主界面。

### 1.4 共享设计令牌 v5（重写 `src/app.css`）

> 在 R1 v4 基础上**收敛**：强调色更克制、新增 void 底与"选择环"令牌、明确分出 Switch-shell / PS5-shell 两组外壳令牌。亮色/樱花主题保留但非 R2 重点。

```css
:root, [data-theme="dark"] {
  /* —— 底色（新增 void，最深，给 Switch 扁平黑） —— */
  --bg-void:    #07090F;
  --bg-deep:    #0B0E14;
  --bg:         #11151F;
  --bg-elev:    #161B27;
  --bg-card:    #1A2030;     /* Switch 用实色卡，非半透明玻璃 */
  --bg-hover:   #1E2433;

  /* —— 文本 —— */
  --text-primary:   #F2F4F8;
  --text-secondary: #AEB6C6;
  --text-muted:     #6B7488;
  --text-dim:       #4E586B;

  /* —— 强调：单一克制玫红，仅选中/主操作 —— */
  --accent:      #E8557F;
  --accent-hi:   #F07A9B;
  --accent-lo:   rgba(232,85,127,.14);

  /* —— 选择环（主机灵魂）—— */
  --ring-switch: 0 0 0 3px rgba(255,255,255,.92);   /* Switch 白环 */
  --ring-ps5:    0 0 0 3px #fff, 0 12px 40px -8px rgba(0,0,0,.7); /* PS5 白环+投影 */
  --focus-ring:  0 0 0 2px var(--accent);            /* 键盘焦点(非磁贴) */

  /* —— 边框：发丝级 —— */
  --border:       rgba(255,255,255,.07);
  --border-hover: rgba(255,255,255,.14);

  /* —— 圆角 —— */
  --radius-sm: 8px; --radius-md: 12px; --radius-lg: 16px; --radius-xl: 24px; --radius-full: 9999px;

  /* —— 阴影：空间感，非霓虹 —— */
  --shadow-tile:  0 8px 24px -14px rgba(0,0,0,.7);
  --shadow-lift:  0 18px 40px -18px rgba(0,0,0,.78);

  /* —— Switch 外壳 —— */
  --sw-bg:          var(--bg-void);
  --sw-tile-radius: var(--radius-lg);
  --sw-tile-idle-bright: .82;
  --sw-dock-icon:   44px;
  --sw-dock-hover:  rgba(255,255,255,.08);

  /* —— PS5 外壳 —— */
  --ps-scrim: linear-gradient(180deg, transparent 0%, transparent 42%,
              rgba(7,9,15,.55) 66%, rgba(7,9,15,.92) 86%, var(--bg-void) 100%);
  --ps-crossfade: .6s;

  /* —— 字体 —— */
  --font-display: "Outfit","Geist","M PLUS Rounded 1c","Noto Sans SC","Microsoft YaHei UI",sans-serif;
  --font-ui:      "Outfit","Geist","Noto Sans SC","Microsoft YaHei UI",system-ui,sans-serif;
  --font-mono:    "JetBrains Mono","Cascadia Code","Consolas",monospace;
}
```
**红线**：Switch 外壳禁用 `backdrop-filter` 玻璃、禁 hero 大图、禁渐变光晕（保持扁平）。PS5 外壳才用 scrim/crossfade/景深。**工具页走第三外壳 Aura（§1.7），玻璃/氛围光只允许出现在 Aura 作用域内。**强调色每屏出现面积尽量 ≤ 一处主按钮 + 选中环。

### 1.5 运动规范（gsap-skill）

| 场景 | 外壳 | 规格 |
|---|---|---|
| 磁贴选中放大 | Switch | `scale 1→1.08` + 白环淡入，`power2.out` ~0.22s，**无 overshoot** |
| 选中信息换栏 | Switch | 标题/副信息 `y:8→0 + fade` 0.18s，错峰 0.03s |
| rail 入场 | Switch | 磁贴 `y:14, opacity:0 → `，`stagger 0.03`，`power3.out` |
| 背景换图 | PS5 | 整屏 `crossfade 0.6s power2.inOut`（双层叠化，不闪黑） |
| 卡片选中 | PS5 | 白环 + `y:-6 scale 1.06`，0.25s；可加极淡鼠标视差 |
| 通用 | 两者 | `prefers-reduced-motion` 一律降级为瞬切；`gsap.context()` 在 `onMount` 建、`onDestroy` revert |

### 1.6 界面重分配（务必执行）
- **删/替**：`LibraryView.svelte` 的 home「console-dashboard」整块（hero-backdrop / focus-section / stats-panel / lower-section / content-track / 手柄 footer）→ 由 G1 的 Switch 主壳替换。其中"继续游玩 / 活动 / 成就 / 库完成度"等富信息**迁入 G3 的 PS5 大屏右侧卡**，不要留在主窗口。
- **保留复用**：`GameCard`(竖封面、状态徽标已修)、`GameGrid`(全库网格)、`CachedImage`、`EmptyState`、`Skeleton`、`Icon`、`ui/*` 组件、`HeroArea` 的部分逻辑可并入 PS5。
- **新增**：`SwitchHome.svelte`(主壳) + `SystemDock.svelte`(底坞) + `TileRail.svelte`(磁贴排) + PS5 化的 `BigPicturePage`。

### 1.7 第三外壳：Aura 绮境（工具页 · 高端精致二次元）【G4 设计总纲 · 2026-06-12 与项目方确认】

> 三壳分工：**Switch 壳 = 克制纯黑**（主页）、**PS5 壳 = 电影化**（大屏）、**Aura 壳 = 高端二次元氛围**（九个工具页 + 迁移页 + 首启向导）。共享同一套 v5 基底令牌（字体/圆角/强调玫红/语义色），Aura 只新增"氛围层 + 表面分级 + 装饰文法"。参考气质：原神/星穹铁道/蔚蓝档案的**设置·图鉴·背包面板**——深色玻璃、发丝光边、双语字标、克制留白，**不是**霓虹赛博。

#### 1.7.1 作用域与红线
- 作用域 = 工具页根节点挂 `.aura-page`：发现 / 刮削 / 下载 / 存档 / 统计 / 平台导入 / 模拟器 / 诊断 / 设置 / 迁移 / 首启向导。**Switch 主页与 PS5 大屏严禁挂此类**，对比截图自检。
- **禁纯黑 `#000`**（底为 `--bg-deep #0B0E14`）；**禁霓虹外发光**（阴影一律向背景色相收敛）；**禁紫蓝渐变 AI 风**；强调色仍只玫红一枚，每屏 ≤ 主按钮 + 选中态。
- 玻璃嵌套 ≤ 1 层（玻璃面板内部用实色 `aura-card`，**禁** glass 套 glass）。
- **禁三等宽卡片横排**、**禁居中大标题页眉**（一律左对齐）；数字一律 `--font-mono` tabular。
- 全部动画只动 `transform`/`opacity`；`prefers-reduced-motion` 下氛围层（grain/樱瓣）整层移除、动效瞬切。

#### 1.7.2 Aura 令牌（追加进 `app.css` 暗色块，命名空间 `--aura-*`）
```css
/* —— Aura 外壳（工具页 · 二次元氛围）—— */
/* 氛围：右上玫瑰光池 + 左下月光银光池，叠在 bg-deep 上；克制 ≤8% */
--aura-pool-rose: radial-gradient(1200px 800px at 84% -12%, rgba(232,85,127,.08), transparent 62%);
--aura-pool-moon: radial-gradient(1000px 700px at -10% 110%, rgba(174,186,211,.05), transparent 58%);
--aura-grain: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='160' height='160'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/></filter><rect width='160' height='160' filter='url(%23n)' opacity='0.5'/></svg>");
--aura-grain-opacity: .03;

/* 表面三级：玻璃面板 / 实色卡 / 凹陷井 */
--aura-panel-bg: rgba(17, 21, 31, .58);
--aura-panel-border: rgba(255,255,255,.09);
--aura-panel-blur: blur(20px) saturate(1.15);
--aura-panel-highlight: inset 0 1px 0 rgba(255,255,255,.07);
--aura-inset-bg: rgba(7, 9, 15, .55);

/* 装饰文法 */
--aura-tick: linear-gradient(180deg, var(--accent), var(--accent-hi)); /* 标题左侧 4px 竖签 */
--aura-echo: rgba(255,255,255,.026);  /* 页眉巨型回声水印字色 */
--aura-bevel: 14px;                   /* 单角斜切量（每页 ≤1 处） */

/* 数据可视化：玫瑰单色 ramp（禁彩虹） */
--aura-data-1: var(--accent);
--aura-data-2: rgba(232,85,127,.62);
--aura-data-3: rgba(232,85,127,.34);
--aura-data-4: rgba(174,186,211,.45);
--aura-track:  rgba(255,255,255,.06); /* 进度/图表底轨 */

/* 运动 */
--aura-ease: cubic-bezier(0.16, 1, 0.3, 1);
--aura-stagger: 40ms;
```

#### 1.7.3 工具类（追加进 `app.css`，工具页统一用这套，不再各页自写）
```css
.aura-page {            /* 页根：氛围底（光池不随滚动重绘——挂在不滚动的页根上，内容区单独滚） */
  position: relative; height: 100%;
  background: var(--aura-pool-rose), var(--aura-pool-moon), var(--bg-deep);
}
.aura-page::after {     /* 胶片微噪点：固定层 + pointer-events:none（性能红线） */
  content: ""; position: absolute; inset: 0; z-index: 0;
  background: var(--aura-grain); background-size: 160px;
  opacity: var(--aura-grain-opacity); pointer-events: none;
}
.aura-panel {           /* 一级容器：液态玻璃 + 1px 内折射边 */
  background: var(--aura-panel-bg);
  backdrop-filter: var(--aura-panel-blur); -webkit-backdrop-filter: var(--aura-panel-blur);
  border: 1px solid var(--aura-panel-border);
  box-shadow: var(--aura-panel-highlight), var(--shadow-card);
  border-radius: var(--radius-lg);
}
.aura-card {            /* 二级：实色卡（玻璃面板内部一律用它） */
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}
.aura-inset {           /* 三级：凹陷井（日志/代码/路径展示） */
  background: var(--aura-inset-bg);
  border: 1px solid rgba(255,255,255,.05);
  box-shadow: inset 0 2px 8px rgba(0,0,0,.4);
  border-radius: var(--radius-md);
  font-family: var(--font-mono);
}
.aura-head { position: relative; padding: 28px 0 18px; }    /* 页眉锁定区 */
.aura-head .echo {       /* 巨型英文回声水印：右上、不可选中 */
  position: absolute; right: 0; top: -6px; z-index: 0;
  font-family: var(--font-display); font-weight: 800;
  font-size: clamp(72px, 11vw, 136px); line-height: 1; letter-spacing: -.04em;
  color: var(--aura-echo); user-select: none; pointer-events: none; white-space: nowrap;
}
.aura-kicker {           /* 双语小字标：11px 全大写 EN/JP */
  font-size: 11px; letter-spacing: .14em; text-transform: uppercase;
  color: var(--text-muted); font-weight: 600;
}
.aura-title {            /* 页标题：22–26px，禁巨型 H1，左侧 4px 玫红竖签 */
  position: relative; font-family: var(--font-display);
  font-size: 24px; font-weight: 700; letter-spacing: -.02em; padding-left: 14px;
}
.aura-title::before {
  content: ""; position: absolute; left: 0; top: 12%; bottom: 12%;
  width: 4px; border-radius: 2px; background: var(--aura-tick);
}
.aura-bevel {            /* 单角斜切（P5/BA 签名）：每页最多 1 处，用在页眉面板或特色卡 */
  clip-path: polygon(0 0, calc(100% - var(--aura-bevel)) 0, 100% var(--aura-bevel), 100% 100%, 0 100%);
}
.aura-num { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
.aura-divider { border: 0; border-top: 1px solid var(--border); }
.aura-enter {            /* 入场级联：每个面板设 style="--i:N"，最多前 8 个参与 */
  animation: fadeInUp .4s var(--aura-ease) both;
  animation-delay: calc(var(--i, 0) * var(--aura-stagger));
}
@media (prefers-reduced-motion: reduce) {
  .aura-enter { animation: none; }
  .aura-page::after { display: none; }
}
```

#### 1.7.4 聚光描边（交互面板的"高级感"签名 · 新增 `src/lib/actions/spotlight.ts`）
鼠标掠过玻璃面板时，1px 边框沿光标位置点亮（遮罩渐变边框，非外发光）：
```ts
// src/lib/actions/spotlight.ts —— Svelte action，rAF 节流写 CSS 变量
export function spotlight(node: HTMLElement) {
  let raf = 0;
  function onMove(e: PointerEvent) {
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      const r = node.getBoundingClientRect();
      node.style.setProperty("--mx", `${e.clientX - r.left}px`);
      node.style.setProperty("--my", `${e.clientY - r.top}px`);
    });
  }
  node.addEventListener("pointermove", onMove);
  return { destroy() { node.removeEventListener("pointermove", onMove); if (raf) cancelAnimationFrame(raf); } };
}
```
```css
.aura-panel--spot { position: relative; }
.aura-panel--spot::before {
  content: ""; position: absolute; inset: 0; border-radius: inherit; padding: 1px;
  background: radial-gradient(220px circle at var(--mx, 50%) var(--my, 0%), rgba(255,255,255,.22), transparent 70%);
  -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
  -webkit-mask-composite: xor; mask-composite: exclude;
  pointer-events: none; opacity: 0; transition: opacity .3s var(--aura-ease);
}
.aura-panel--spot:hover::before { opacity: 1; }
```

#### 1.7.5 页眉文法（每个工具页统一锁定，禁自由发挥）
```
┌──────────────────────────────────────────────┐
│ ▍统 计                      S T A T I S T I C S │ ← 玫红竖签+24px 标题；右上巨型回声水印(opacity .026)
│ 库存・时长・成就的全景概览                          │ ← 13px --text-secondary 一句话
│ ──────────────────────────────────────────── │ ← 发丝分隔
```
- 标题中文为主；`aura-kicker` 用英文（或日文假名）做氛围副标——**仅装饰，不承载信息**。
- 页眉左对齐；echo 水印每页一个、不遮内容（z-index 0，内容 z-index 1）。

#### 1.7.6 运动规范（Aura 专属，叠加 §1.5 通则）
| 场景 | 规格 |
|---|---|
| 页面进入 | 页眉先入（y:10→0 / .32s），面板 `.aura-enter` 级联 `--i`×40ms，**只前 8 个**参与 |
| 面板 hover | `translateY(-2px)` + 边框变亮 + 聚光描边淡入，.28s `--aura-ease` |
| 按下反馈 | `scale(.98)` .12s（全部可点元素，含 `ui/Button`） |
| 进度条 | 内胆 `transform: scaleX()`（origin left），**禁动 width**；轨道 `--aura-track` |
| 折线/环形图 | 入场 `stroke-dashoffset` 自绘一次（.8s），数据更新瞬切 |
| 状态点 | 复用现有 `.status-dot` 呼吸（pulse-dot） |
| 列表增删 | `fadeInScale .2s`；长列表禁全列表重排动画 |
| 樱瓣层 | 复用现有 `SakuraParticles.svelte`：**仅发现页**默认开、`count≤10`，其余页不开；reduced-motion 移除 |

#### 1.7.7 布局文法（DESIGN_VARIANCE≈8 · 拒绝 AI 模板脸）
- **bento 不对称网格**：统计页用 `grid-template-columns: 2fr 1fr 1fr`、设置页 `220px 1fr`（左 sticky 锚点目录），禁等分三栏。
- **列表页反卡片化**：下载/诊断/模拟器的行项目用 `.aura-divider` 发丝分隔 + hover 底色，**不要每行包卡**；只有需要"层级抬升"的容器才用 `.aura-panel`。
- 窄窗 `<760px` 一律塌单列（`grid-template-columns: 1fr`），禁横向滚动溢出。
- 留白节奏：页内边距 28–36px，面板内 20–24px，组间 16px；密度对标"日常应用"非驾驶舱。

#### 1.7.8 三态（沿用 §8 思路，Aura 皮肤）
- **载入**：`ui/Skeleton` 按真实布局占位（bento 块对 bento 块），禁圆形 spinner。
- **空态**：`EmptyState` 加 Aura 处理——echo 水印字 + 一句话 + 唯一主按钮（玫红）；例：下载页空 →「暂无下载任务 / 从发现页找点新游戏」。
- **错误**：行内 banner（`--color-error` 左签 + 可读文案 + 重试按钮），禁原始异常串。

#### 1.7.9 现有资源盘点（直接复用，不重造）
| 资源 | 位置 | 用法 |
|---|---|---|
| `--glass-*` 令牌 + `.glass/.glass-card` | `app.css` | Aura 面板的前身，G4 时**收敛进 `.aura-panel`**（保留别名防回归） |
| `shimmer/fadeInUp/fadeInScale/pulse-dot` | `app.css` | 入场/骨架/状态点全复用 |
| `ui/*`（Button/Card/Rail/Skeleton/StatBlock/Tag） | `lib/components/ui` | G4 校准到 Aura（按下 scale、聚光描边可选挂） |
| `RatingRing` / `StatusBadge` / `TagPill` / `Toast` / `Modal` | `lib/components` | 配色对齐 ramp/语义色即可 |
| `SakuraParticles`（`count` prop 已有） | `lib/components` | 发现页氛围层（§1.7.6 约束） |
| `Icon.svelte`（含新 tv/steam/epic/arrowRight） | `lib/components` | 全部图标走它，禁 emoji |
| GSAP + `gsap.context` 范式 | TileRail/TileCard 已示范 | 复杂编排才用 GSAP，简单进出场用 CSS 级联 |

---

## 2. UI Agent 任务卡

> 工作法沿用 R1：每卡一分支一 PR；DoD 见 §4.3；自验证据（截图/日志/测试）贴进 PR。
> **依赖**：F 先行（令牌），G1/G3 依赖 F；G1/G2/G3 的"好看验收"依赖 §3-C 真机灌库（否则空库无从评判）。

### F · 设计令牌 v5 + 外壳基础
- 重写 `src/app.css` 落地 §1.4 全部令牌；分出 Switch-shell / PS5-shell 两组工具类。
- 校准 `ui/*` 现有组件到 v5（实色卡、发丝边、克制强调）。
- **验收**：全站构建无回归；暗色三屏（主界面/大屏/任一工具页）令牌取值一致；无遗留霓虹光晕。

### G1 · Switch 主界面外壳 ⭐（R2 最大可见收益）
- 新建 `SwitchHome.svelte` + `TileRail.svelte` + `SystemDock.svelte`，按 §1.2 实现：顶栏 / 竖封面磁贴排（选中放大+白环）/ 选中信息区+开始游戏 / 底部系统坞 / 扁平 void 背景。
- 接 `gameStore`（最近游玩排序 + 末尾"全部游戏"磁贴）；键盘 ← → / Enter / Space + 手柄映射；GSAP 选中动效（§1.5）。
- 替换 `App.svelte` 中 home 分支为 `SwitchHome`；删除旧 console-dashboard。
- **验收**：真机灌库后，主界面即一排竖封面，← → 流畅切换、选中放大+白环、标题/信息实时更新、开始游戏可拉起进程；底坞各图标进对应页；空库走 `EmptyState`（引导导入）。截图贴 PR。

### G2 · 全库网格 + 游戏详情页
- "全部游戏"磁贴 → 全库**竖封面网格**（复用/精炼 `GameGrid`）：快捷筛选条（全部/最近/已安装/游玩中/已通关/收藏/待补全）+ 排序 + 搜索；密度按 §1 克制。
- 重做 `GameDetailPage`：竖封面 + 横版背景 + `▶开始游戏` + 元数据/标签/评分/简介/游玩会话/存档入口；走 §3-E 取值器，避免空白。
- **验收**：网格滚动顺滑、筛选/排序/搜索正确；详情页字段齐全无空白；从磁贴/网格进出详情动效一致。

### G3 · PS5 大屏模式
- 将 `BigPicturePage` 精炼为 §1.3 正统 PS5：全出血背景 + **0.6s crossfade 换图** + 大标题 + 特大主按钮 + 右侧活动/统计卡（接 R1 已算好的成就/周时长/完成度）+ 竖封面卡 rail（白环选中）。
- 手柄/方向键导航；退出回 Switch 主界面。
- **验收**：切换选中时整屏背景平滑叠化不闪黑；手柄全程可操作；10 尺可读；截图/录屏贴 PR。

### G4 · 工具页 Aura 绮境皮肤（按 §1.7 设计总纲施工）
> 拆 4 个子 PR，禁一锅端：**G4a 基建** → G4b/G4c/G4d 按页分批。每页根节点挂 `.aura-page`，页眉走 §1.7.5 文法，三态走 §1.7.8。

- **G4a · 基建**：`app.css` 追加 §1.7.2 令牌 + §1.7.3 工具类；新建 `src/lib/actions/spotlight.ts`（§1.7.4）；校准 `ui/*` 六组件（Button 按下 scale、Card→aura 三级表面、Skeleton 对齐布局）。旧 `.glass/.glass-card` 收敛进 `.aura-panel`（留别名）。
- **G4b · 数据三页**：
  - **统计 StatsPage**：bento `2fr 1fr 1fr` 不对称网格；折线/环形图用玫瑰单色 ramp（`--aura-data-*`）+ 入场 stroke 自绘；全部数字 `.aura-num`；echo 水印 "STATISTICS"。
  - **下载 DownloadPage**：任务行反卡片化（发丝分隔 + hover 底色）；进度条 scaleX + `--aura-track` 轨道；速度/ETA mono；空态走 §1.7.8。
  - **诊断 DiagnosticsPage**：检查项列表 + `.aura-inset` 日志井（mono）；状态点呼吸；echo "DIAGNOSTICS"。
- **G4c · 库务三页**：
  - **设置 SettingsPage**：`220px 1fr` 双栏，左 sticky 锚点目录，右分组玻璃面板（组内行用发丝分隔）；页眉面板可用唯一一处 `.aura-bevel`。
  - **存档 BackupPage**：快照时间线（左侧轴线 + 节点圆点），每条快照一行非一卡；操作按钮收进行尾。
  - **平台导入 PlatformImportPage**：顶部步骤条（mono 序号）+ Steam/Epic 平台卡（双卡 `1.4fr 1fr` 不对称，icon 已有 steam/epic 键）；导入进度复用下载页规格。
- **G4d · 内容三页 + 迁移**：
  - **发现 DiscoveryPage**：氛围最浓的一页——横滚编辑型 rail + 大图卡（CachedImage）；`SakuraParticles count≤10` 默认开（全应用唯一开启处）。
  - **刮削 ScraperPage**：`1.4fr 1fr` 双栏（候选结果列表 / 右侧预览面板带聚光描边）；匹配度数字 mono。
  - **模拟器 EmulatorImportDialog 相关页**：列表 + 行尾操作，反卡片化。
  - **迁移 MigrationPage / 首启向导 FirstRunWizard**：同套页眉 + 进度规格；这是新用户第一眼，单角斜切 + echo 可用足。
- **验收（每子 PR）**：
  1. `svelte-check` 0 error；`npm run build` 通过。
  2. 真机灌库后逐页截图贴 PR（含 hover 态 + 空态各一张）。
  3. 红线自检：Switch 主页 / PS5 大屏对比截图证明**未受 Aura 影响**；玻璃嵌套 ≤1 层；无三等宽卡排、无居中页眉、无纯黑、无霓虹外发光。
  4. `prefers-reduced-motion` 录屏：氛围层移除、动效瞬切。
  5. 工具页滚动 60fps（DevTools Performance 一段录制截图）。

---

## 3. 后端收尾 Agent 任务卡

### C · BUG-1 真机灌库 ⭐⭐（P0，全局阻塞）
- 迁移**代码已就绪**（`csharp_migration.rs`：找库 → PS1 调 LiteDB.dll 导出 JSON → 映射 upsert SQLite → `verify_migration`）。R2 任务是**让它真的跑通并灌进真实库**：
  1. **锁定真实库**：portable dist 的 `…\MoeGame-Portable\library\games.db` 仅 8KB（疑空）。先确认用户真实 500+ 库的 `games.db` 位置（优先 `%LOCALAPPDATA%\MoeGameSetup\…` 或安装版工作目录），把 `find_litedb_*`/`find_export_json` 的搜索路径与优先级补全。
  2. **打通 LiteDB.dll**：系统内仅 nuget 缓存有 `LiteDB.dll`（4.1.4）。确保导出脚本能定位到与旧库匹配的 DLL（随安装版/或回退 nuget 缓存），PowerShell 只读模式打开。
  3. **端到端验收**：`migrate_from_csharp` 真机跑通，`get_game_count ≈ 旧库数量`；封面/背景复制到新 `covers/`、路径重写为新库相对；解析 `Notes` 的 `<!--moe:cn …-->` 中文名/简介；`<!--moe:cn-->` 缺失时回退原名。
  4. **接 UI**：`MigrationPage` 走通进度事件 `migration-progress`，失败有可读提示 + 重试；首启向导提供"从旧版 MoeGame 导入"入口。
- **验收**：点一次迁移 → 主界面（G1）即出真实数百竖封面；`verify_migration` 报告封面存在率 ≥95%、中文名/简介/时长齐全；重入幂等（再点不翻倍）。

### D · BUG-7 Steam / Epic 全库导入（P1，对标 Playnite）
- **Steam**：现有本地 appmanifest + Web API GetOwnedGames + OpenID 已搭好。补：**时长（`playtime_forever`）/ 最后游玩（`rtime_last_played`）→ `play_tracker`**；成就（GetPlayerAchievements）→ `achievements_total/unlocked`；**竖版库图**（`library_600x900`）优先于横版 capsule，喂给磁贴/网格。再同步幂等（按 app_id upsert，不覆盖用户手动改的中文名/评分）。
- **Epic**：现仅扫本地已安装 `.item`。补**账号全库**：EGS OAuth（authorizationCode → exchange → 拉 `LauncherLibrary`/Catalog），导入未安装的拥有游戏；竖版图。
- **验收**：登录后一键导入，库中 Steam/Epic 游戏带时长/最后游玩/成就/竖封面；断网/失效 key 有可读提示；再同步不产生重复或覆盖人工修改。

### E · BUG-6 数据模型收敛（P1）
- 定**单一真相**：游戏字段集中到 `metadata.*`（封面/开发商/评分/年份/平台/背景…），顶层仅留 `id/name/路径/状态/收藏`。弃用 `cover/developer/rating/release_year/last_played` 顶层重复字段（保留读取兼容，写入只走 metadata）。
- 写 `normalizeGame()` 兜底旧数据；前端统一经 `utils/game.ts` 取值器（`coverOf/developerOf/ratingOf/heroImageOf` 已部分存在，补齐并全组件改用）。
- **验收**：刮削/导入写入后磁贴/网格/详情立即显示封面·厂商·评分，无空白；`svelte-check` 0 error；取值器有单测。

### H · BUG-8 拆分 `commands.rs`（P1）
- 把 3,479 行巨石按域拆 `commands/`：`games.rs / metadata.rs / play.rs / scrape.rs / platform.rs / download.rs / backup.rs / system.rs`，`mod.rs` 汇出；`lib.rs` 的 `generate_handler!` 相应分组。
- 顺手统一错误：可失败路径 `Result + ?`，收敛到 `MoeError`（`thiserror`），禁裸 `unwrap/expect`（除 main/测试）。
- **验收**：`cargo fmt --check && cargo clippy -- -D warnings && cargo test` 全绿；命令注册数不变、行为不回归（并发烟雾测试通过）。

---

## 4. 里程碑与关键路径

### 4.1 关键路径
**C（真机灌库）解锁一切 UI 验收**（空库无从评判好看）；**F（令牌）解锁所有 G**。故 **C 与 F 立即并行起步**。

### 4.2 里程碑
| M | 目标 | 含卡 | 出口 |
|---|---|---|---|
| **M1** | 有数据 + 有底座 | **C** 灌库、**F** 令牌 v5 | 主界面能看到真实数百游戏；令牌全站生效 |
| **M2** | 主界面立住 | **G1** Switch 主壳 | 一排真实竖封面，切换/启动/底坞可用——"启动器"首次真正可用 |
| **M3** | 浏览闭环 | **G2** 网格+详情、**E** 模型收敛 | 全库网格 + 详情无空白；刮削写入即显 |
| **M4** | 沉浸 + 全库 | **G3** PS5 大屏、**D** Steam/Epic 全库 | 大屏电影化；Steam/Epic 带时长/成就/竖封面 |
| **M5** | 收口 | **G4** 工具页 Aura（§1.7）、**H** 巨石拆分 + 测试 | 十一页 Aura 绮境皮肤（九工具页+迁移+向导）；后端绿灯；打包冒烟 |

### 4.3 每卡 Definition of Done
1. `cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings` 绿；涉及命令的卡 `cargo test` 绿。
2. 前端 `npm run build` + `npx svelte-check` 零 error；改动取值/状态的卡补 `vitest`。
3. **自验证据**：截图 / 录屏 / 日志 / 测试输出贴 PR。
4. 禁裸 `unwrap/expect`（除 main/测试）；不把原始异常文案抛给用户。
5. 一卡一 PR，禁攒大改；UI 卡必须在**真机灌库后**截图验收，不接受空库截图。

---

## 5. 新组件契约（G1 / G3 照此建文件）

> Svelte 5 runes：用 `$props()` + **回调 prop**（非 `createEventDispatcher`）。所有组件走 `gameStore`/`uiStore`/`settingsStore`，不自持游戏数据。文案优先走 `i18n.svelte.ts` 已有键，缺键再补，**不要新增硬编码语言分支**。

**G1 文件树**
```
src/lib/components/switch/
  SwitchHome.svelte     主壳：编排顶栏 + TileRail + 选中信息区 + SystemDock + 空/错/载三态
  TileRail.svelte       竖封面磁贴横排（选中放大+白环+键盘/手柄导航）
  TileCard.svelte       单个竖封面磁贴（复用 CachedImage + 占位字母组）
  SystemDock.svelte     底部系统坞（单色圆形图标行）
  useGamepad.svelte.ts  手柄轮询 composable（rAF + navigator.getGamepads）
```

**组件 props（TypeScript 形态）**
```ts
// TileRail.svelte
let { items, selectedId, onSelect, onActivate, onLaunch }: {
  items: Game[];                 // gameStore 已排序的最近游玩 + 末尾“全部”哨兵
  selectedId: string | null;
  onSelect: (id: string) => void;     // 移动选中（不离开主页）
  onActivate: (id: string) => void;   // Enter/单击 → 打开详情
  onLaunch: (id: string) => void;     // Space/双击/手柄A → 直接启动
} = $props();

// SystemDock.svelte
type DockItem = { id: string; label: string; icon: string; view: string };
let { items, onPick }: { items: DockItem[]; onPick: (view: string) => void } = $props();

// TileCard.svelte
let { game, selected, idle }: { game: Game | null; selected: boolean; idle: boolean } = $props();
// game=null 表示“全部游戏”哨兵磁贴（虚线框）
```

**SwitchHome 内部状态机**：`selectedId`（默认 `gameStore.selectedGame?.id ?? items[0]?.id`）；`onSelect` 写 `gameStore.selectGame(id)`；`onActivate` → `uiStore.currentView="game-detail"`；`onLaunch` → `gameStore.launch(id)` + `uiStore.notify`。"全部"哨兵 `onActivate` → `uiStore.viewMode="grid"`（进 G2 全库网格）。

**G3**：把 `BigPicturePage.svelte` 拆出 `BigPictureHero.svelte`（全出血背景 + crossfade + 大标题 + 主按钮 + 右侧活动卡）与 `BigPictureRail.svelte`（竖封面卡 + 白环选中）；复用现有 `RatingRing`、`BigPictureDetail`。

---

## 6. 输入映射（两壳必须一致可预期）

> 手柄用 Gamepad API（Tauri WebView 支持）。`useGamepad` 在 `requestAnimationFrame` 里读 `navigator.getGamepads()`，做边沿去抖（按下→释放才触发一次），映射到与键盘相同的回调。窗口失焦时停止轮询。

| 动作 | 键盘 | 手柄 | Switch 主页 | PS5 大屏 |
|---|---|---|---|---|
| 移动选中 | ← → / A D | 左摇杆 / 十字键 | 在 rail 内移动 | 在卡轨内移动（换背景） |
| 翻页 | PageUp/Down | LB / RB | 跳 6 个 | 跳 6 个 |
| 打开详情 | Enter | ▣ (Y/△) | onActivate | 详情 |
| 启动游戏 | Space | ⓐ (A/✕) | onLaunch | 启动 |
| 收藏 | F | ⓧ (X/□) | toggleFavorite | toggleFavorite |
| 搜索 | / | — | 展开搜索 | 展开搜索 |
| 进/出大屏 | Ctrl+B | 长按 ☰ | 进大屏 | 退大屏 |
| 返回 | Esc | ⓑ (B/○) | 关搜索/无操作 | 退大屏 |

`prefers-reduced-motion` 下所有移动动画降级为瞬切；焦点必须有可见环（`--ring-switch` / `--focus-ring`），键盘与手柄共享同一 `selectedId`。

---

## 7. 系统坞规格（SystemDock 数据 + 待补图标）

> `view` 对应 `uiStore.currentView`（"home" 之外的值已在 `App.svelte` 懒加载分支里存在）。`icon` 为 `Icon.svelte` 现有键。

```ts
const dock: DockItem[] = [
  { id:"discovery", label:"发现",  icon:"compass",  view:"discovery"   },
  { id:"scraper",   label:"刮削",  icon:"star",     view:"scraper"     },
  { id:"downloads", label:"下载",  icon:"download", view:"downloads"   },
  { id:"backup",    label:"存档",  icon:"save",     view:"backup"      },
  { id:"stats",     label:"统计",  icon:"chart",    view:"stats"       },
  { id:"import",    label:"导入",  icon:"database", view:"steam-import"},
  { id:"emulator",  label:"模拟器", icon:"gamepad",  view:"emulator"    },
  { id:"diag",      label:"诊断",  icon:"toolbox",  view:"diagnostics" },
  { id:"settings",  label:"设置",  icon:"gear",     view:"settings"    },
  { id:"bigpic",    label:"大屏",  icon:"tv",       view:"__bigpicture"},  // 特例→uiStore.setBigPicture(true)
];
```
**F 卡顺手给 `Icon.svelte` 补 4 个键**：`tv`（大屏）、`steam`、`epic`、`arrowRight`（现仅有 `arrowLeft`）。坞图标单色 `--text-secondary`，hover 圆形底 `--sw-dock-hover`，当前页高亮 `--accent`。

---

## 8. 三态与空库引导（SwitchHome 必须显式处理）

| 态 | 条件 | 渲染 |
|---|---|---|
| 载入 | `gameStore.loading && allGames.length===0` | 顶栏 + rail 处放 6 张 `Skeleton variant="card"`，不卡死 |
| 错误 | `gameStore.loadError` | rail 上方 `loadError` banner + "重试"（调 `gameStore.load()`），下方仍渲染已有数据 |
| 空库 | `!loading && allGames.length===0` | `EmptyState` 居中："从旧版 MoeGame 导入" → `uiStore.currentView="migration"`；并列 "Steam/Epic 导入"、"添加本地游戏"（`gameStore.importGame()`） |
| 有数据 | `!loading && allGames.length>0` | 正常 Switch 主页 |

空库态是当前真实状态（DB=0），**必须先于"好看"做对**：它是用户灌库前看到的第一屏，要把"从旧版导入"做成最显眼的主行动。灌库（§C）跑通后此态自然消失。

---

## 9. 性能预算（真实库 500+ 游戏）

- **主页 rail**：上限 16 张，无压力。
- **G2 全库网格（重点）**：500+ 竖封面**必须虚拟滚动**（`@tanstack/svelte-virtual` 或自写 IntersectionObserver 窗口化），首屏只渲染可视行；`CachedImage` 已是 `loading="lazy"`，再加 `decoding="async"`。封面走**缩略图**（后端 `thumbnail.rs` 已有缓存）按磁贴尺寸取图，禁止全分辨率原图进网格。
- **排序/搜索**：`allGames` 不在每次按键重算；搜索 200ms 防抖（`HeroArea` 已有范式），`$derived` 缓存派生列表。
- **GSAP**：入场 `stagger` 仅作用于可视磁贴；`will-change:transform` 用完即撤；大屏 crossfade 仅动 `opacity`（不动 `filter`/`box-shadow`）。
- **预算线**：主页冷启 → 可交互 < 1.5s（已灌库）；网格滚动稳定 60fps；切换选中换背景无掉帧。

---

## 10. 关键动效片段（gsap-skill · 可直接粘）

```ts
// Switch 磁贴选中：放大 + 白环淡入（无 overshoot）
function animateSelect(node: HTMLElement) {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  gsap.to(node, { scale: 1.08, duration: 0.22, ease: "power2.out" });
  gsap.fromTo(node, { "--ring-a": 0 }, { "--ring-a": 0.92, duration: 0.18 });
}
// idle 复位
function animateDeselect(node: HTMLElement) {
  gsap.to(node, { scale: 1, duration: 0.2, ease: "power2.out" });
}

// PS5 换背景：双层叠化，不闪黑
function crossfadeHero(incoming: HTMLElement, outgoing: HTMLElement) {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
    outgoing.style.opacity = "0"; incoming.style.opacity = "1"; return;
  }
  gsap.set(incoming, { opacity: 0 });
  gsap.to(incoming, { opacity: 1, duration: 0.6, ease: "power2.inOut" });
  gsap.to(outgoing, { opacity: 0, duration: 0.6, ease: "power2.inOut" });
}
```
全部包进 `gsap.context(() => {…}, node)`，`onMount` 建、`onDestroy` `ctx.revert()`。

---

## 11. 风险登记（施工前先读）

| 风险 | 影响 | 缓解 |
|---|---|---|
| LiteDB.dll 版本与旧 Playnite 库不匹配（系统仅见 nuget 4.1.4） | 导出脚本崩 → 灌库失败 | 优先用随安装版打包的 DLL；回退 nuget；导出脚本失败给**可读错误**而非静默 |
| 真实库路径未知（portable `games.db` 仅 8KB 疑空） | 迁移找到空库 → 仍 0 游戏 | `MigrationPage` 提供**手动选目录**；自动扫已知位置（安装版工作目录 / `%LOCALAPPDATA%`）；导入前显示"检测到 N 个游戏"二次确认 |
| Steam Web API 限流 / key 失效 | 全库导入半途失败 | 指数退避 + 断点续传；失败项可重试；key 校验前置 |
| 500+ 竖封面内存/首屏 | 网格卡顿、内存涨 | 强制虚拟滚动 + 缩略图（§9） |
| crossfade 在弱 GPU 掉帧 | 大屏换图卡 | 仅动 opacity；`reduced-motion` 瞬切兜底 |
| 手柄在 WebView 失焦时仍轮询 | 误操作 / 耗电 | 失焦停轮询；只在大屏/主页激活 |

---

## 12. 一句话给 Codex
**C / F / G1 / G3 已完成**（灌库 519 游戏、令牌 v5、Switch 主壳、PS5 大屏均已落地）。当前主攻：**G2（全库网格+详情）→ G4（工具页 Aura 绮境，严格照 §1.7 总纲，先 G4a 基建再分批）**，后端并行 **E / D / H**。**任何 UI PR 不接受空库截图；Aura 类禁止泄漏到 Switch/PS5 两壳。**
