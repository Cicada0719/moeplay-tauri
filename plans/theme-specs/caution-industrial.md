# Theme Spec · caution-industrial「警戒工业」

## 1. 设计参考
Arknights 明日方舟 UI（Hypergryph）。工业机能风：枪灰金属底、技术 HUD 刻度、模具喷字（stencil）编号、警戒橙功能色、冷峻分区。
**禁忌**：禁止方舟 logo、阵营标志、角色立绘与任何游戏截图；只取枪灰+警戒橙的 HUD 语言。

## 2. Token 表（`[data-theme-pack="caution-industrial"]`）

```css
[data-theme-pack="caution-industrial"] {
  --accent: #f59e0b;
  --accent-hi: #ffb93d;
  --accent-lo: rgba(245, 158, 11, .15);
  --accent-ring: rgba(245, 158, 11, .50);
  --mascot-accent: #9aa4b0;
  --theme-ambient: rgba(245, 158, 11, .14);
  --wallpaper-scrim:
    linear-gradient(90deg, rgba(11,13,16,.92) 0%, rgba(11,13,16,.68) 47%, rgba(11,13,16,.32) 100%),
    linear-gradient(0deg, rgba(11,13,16,.88), transparent 50%);
  --wallpaper-brightness: .72;
  --wallpaper-saturation: .90;
}
```
dark 色模式底色沿用 dark 块。对比度自检：accent on #0b0e14 ≈ 7.2:1（AA+）。
defaultColorMode: `dark`；decoration: `digital-rain`。

## 3. 构图描述（`scripts/theme-art/caution-industrial.mjs`）
- **wallpaper-1「蓝图网格」**：#0d1014 枪灰底；1px 工程网格（rgba(154,164,176,.08)，48px）；橙色（#f59e0b）1px 测量线两条（水平/垂直各一，带端点刻度短杠）；中央大号空心数字 "07"（800px 等宽粗体，仅 2px 描边 rgba(154,164,176,.35)，无填充）；四角白色对齐十字（rgba(255,255,255,.25)）。
- **wallpaper-2「警示条纹」**：枪灰底；下 1/3 一条 120px 高警示条纹带（45° 橙/深灰 #1c2128 相间斜纹，repeating-linear-gradient）；上部 HUD 刻度尺（白色小竖线阵列，opacity .3）；右侧三行等宽小字数据读数（#9aa4b0）。
- **wallpaper-3「金属渐变」**：#10141a → #0a0c10 的垂直拉丝渐变（细密水平 1px 线，opacity .04）；微弱六边形网格（stroke rgba(154,164,176,.05)）；一条发光橙色线路（3px 带 12px 光晕）自左边缘折线延伸至中部。
- **preview（800×500）**：HUD 细边框（橙色 1px，四角加粗）+ 中央「警戒工业」等宽小标题 + 警示条纹角标。
- **mascot（512×512 透明 PNG）**：六边形徽章（边长 72%，2px 枪灰 #9aa4b0 描边，填充 #14181e），中心一个橙色实心 chevron（› 形箭头，朝右，占高 45%）。扁平。

## 4. 验收清单
- token 色值一致；AA 对比度实测。
- 资产 8 件（规格同前）；回读目检：条纹方向统一、数字描边不糊、文字无裁切。
- `theme-pack-caution-industrial.test.ts` 断言：注册表含包、token 文件含 `--accent: #f59e0b`、资产 existsSync。

---

## 5. Wave T2 · 全套换肤扩容（覆写 scheme-c）

**选择器纪律**：所有规则必须以 `html[data-theme-pack="caution-industrial"]` 为作用域（同特异性 (0,1,1)，靠 main.ts 末位导入取胜）。参照 `src/lib/styles/themes/shift-editorial.css`（已通过 QA 的参考实现）。

**全量调色板**（写入 `src/lib/styles/themes/caution-industrial.css`，替换原低特异性块）：

```css
html[data-theme-pack="caution-industrial"] {
  --c-black: #0b0d10; --c-void: #0d1014; --c-surface-1: #10141a; --c-surface-2: #161b22; --c-surface-3: #1c2128;
  --c-paper: #e8ecf0; --c-muted: #9aa4b0; --c-dim: #5d6772;
  --c-line: rgba(232,236,240,.14); --c-line-strong: rgba(232,236,240,.36);
  --c-accent: #f59e0b; --c-accent-hi: #ffb93d;
  --accent: #f59e0b; --accent-hi: #ffb93d; --accent-lo: rgba(245,158,11,.15); --accent-ring: rgba(245,158,11,.50);
  --bg-void: #0b0d10; --bg-deep: #0d1014; --bg: #10141a; --bg-base: #10141a; --bg-surface: #141920;
  --bg-secondary: #161b22; --bg-elev: #1a2029; --bg-card: #232a33; --bg-hover: #2a333d; --bg-active: rgba(245,158,11,.14);
  --text-primary: #e8ecf0; --text-secondary: #b6bfc9; --text-muted: #9aa4b0; --text-dim: #5d6772;
  --border: rgba(232,236,240,.12); --border-hover: rgba(232,236,240,.28); --border-glass: rgba(232,236,240,.10);
  --radius-sm: 2px; --radius-md: 3px; --radius-lg: 4px; --radius-xl: 4px;
  --focus-ring: 0 0 0 2px #0b0d10, 0 0 0 4px rgba(245,158,11,.55);
  --glass-bg: rgba(11,13,16,.94); --glass-border: rgba(232,236,240,.14); --glass-highlight: none;
  --shadow-card: none; --shadow-hover: 0 2px 10px rgba(0,0,0,.5);
  --mascot-accent: #9aa4b0; --theme-ambient: rgba(245,158,11,.14);
  --wallpaper-scrim: <保留现有>; --wallpaper-brightness: .72; --wallpaper-saturation: .90;
}
```

**控件签名（工业 HUD）**：小切角 + 等宽工程字。
- `.ui-button`：`clip-path: polygon(0 0, calc(100% - 7px) 0, 100% 7px, 100% 100%, 0 100%)`（右上 7px 切角）；等宽字体栈（var(--font-mono)）；`text-transform: uppercase; letter-spacing: .08em; font-size: .78em`；1px 边框。
- **`.ui-button--primary` 必须深字**：橙底白字仅 2.15:1 不达标 → `color: #14181e`（对比度 ≈8:1），hover 用 --accent-hi。
- `.ui-card`：3px 圆角 + 1px var(--border)，无阴影。
- 输入控件：2px 圆角、--bg-void 底。
- reduced-motion 双写收尾（照 shift-editorial.css 尾部写法）。

**QA 驱动**：QA spec 断言 `--accent === #f59e0b`、body 底色 `rgb(11, 13, 16)`。
