# Theme Spec · astral-rail「星穹旅人」

## 1. 设计参考
Honkai: Star Rail 官网与 UI（HoYoverse）。星穹铁道气质：深空靛紫、星轨金线、银河星光、优雅的柔光晕与史诗感留白。
**禁忌**：禁止 HSR logo、星轨图标、角色与任何截图；只取深空靛+星金的色彩语言。

## 2. Token 表（`[data-theme-pack="astral-rail"]`）

```css
[data-theme-pack="astral-rail"] {
  --accent: #d8b45a;
  --accent-hi: #eed390;
  --accent-lo: rgba(216, 180, 90, .15);
  --accent-ring: rgba(216, 180, 90, .50);
  --mascot-accent: #8ea2ff;
  --theme-ambient: rgba(126, 148, 255, .18);
  --wallpaper-scrim:
    linear-gradient(90deg, rgba(9,10,26,.90) 0%, rgba(9,10,26,.60) 47%, rgba(9,10,26,.24) 100%),
    linear-gradient(0deg, rgba(9,10,26,.85), transparent 52%);
  --wallpaper-brightness: .80;
  --wallpaper-saturation: 1.00;
}
```
dark 色模式底色沿用 dark 块。对比度自检：accent on #0b0e14 ≈ 7.6:1（AA+）。
defaultColorMode: `dark`；decoration: `light-particles`（星尘浮粒）。

## 3. 构图描述（`scripts/theme-art/astral-rail.mjs`）
- **wallpaper-1「银河铁道」**：#090a1a 深空底；两道金色（#d8b45a）轨道弧线自左下向右上方掠过（2px/1px 各一，大半径圆弧，opacity .8/.4）；星尘粒子场（白/淡金小点 60+，随机大小 1-3px，部分带 4px 光晕）；右上远方一颗淡紫行星（radial-gradient 球体，#8ea2ff 系）。
- **wallpaper-2「星图」**：深空底 + 中央淡紫星云渐变（opacity .2）；8-10 颗星点以 1px 金色细线连成星座折线；两个同心轨道圆环（1px，rgba(216,180,90,.3)，虚线 stroke-dasharray 4 6）；角落等宽小字坐标注记（rgba(255,255,255,.35)）。
- **wallpaper-3「晨曦跃迁」**：底部地平线辉光（#d8b45a → transparent 的大半径 radial-gradient，高度 35%）；上方深空渐变（#090a1a → #141a3a）；十余条上升星轨（1-2px 白色/金色短线，60°，opacity 随高度衰减）。
- **preview（800×500）**：金色轨道圆环 + 中央「星穹旅人」衬线感小标题（Georgia/serif，金色）+ 三颗星点。
- **mascot（512×512 透明 PNG）**：金色圆环行星（环以椭圆描边 6px 穿过球体，球体为 #8ea2ff → #4a5aa8 的径向渐变圆），右上一颗 4 角小星（自绘 polygon，金色）。扁平优雅。

## 4. 验收清单
- token 色值一致；AA 对比度实测。
- 资产 8 件（规格同前）；回读目检：弧线平滑、星点不过曝、文字无裁切。
- `theme-pack-astral-rail.test.ts` 断言：注册表含包、token 文件含 `--accent: #d8b45a`、资产 existsSync。

---

## 5. Wave T2 · 全套换肤扩容（覆写 scheme-c）

**选择器纪律**：所有规则必须以 `html[data-theme-pack="astral-rail"]` 为作用域（同特异性 (0,1,1)，靠 main.ts 末位导入取胜）。参照 `src/lib/styles/themes/shift-editorial.css`（已通过 QA 的参考实现）。

**全量调色板**（写入 `src/lib/styles/themes/astral-rail.css`，替换原低特异性块）：

```css
html[data-theme-pack="astral-rail"] {
  --c-black: #07081a; --c-void: #090a1a; --c-surface-1: #0d0f24; --c-surface-2: #12142e; --c-surface-3: #181a38;
  --c-paper: #eef0ff; --c-muted: #a3a8d4; --c-dim: #64689a;
  --c-line: rgba(238,240,255,.14); --c-line-strong: rgba(238,240,255,.36);
  --c-accent: #d8b45a; --c-accent-hi: #eed390;
  --accent: #d8b45a; --accent-hi: #eed390; --accent-lo: rgba(216,180,90,.15); --accent-ring: rgba(216,180,90,.50);
  --bg-void: #07081a; --bg-deep: #090a1a; --bg: #0d0f24; --bg-base: #0d0f24; --bg-surface: #12142e;
  --bg-secondary: #12142e; --bg-elev: #181a38; --bg-card: #1e2044; --bg-hover: #252858; --bg-active: rgba(216,180,90,.14);
  --text-primary: #eef0ff; --text-secondary: #c3c7ea; --text-muted: #a3a8d4; --text-dim: #64689a;
  --border: rgba(238,240,255,.12); --border-hover: rgba(238,240,255,.30); --border-glass: rgba(238,240,255,.10);
  --radius-sm: 4px; --radius-md: 6px; --radius-lg: 8px; --radius-xl: 12px;
  --focus-ring: 0 0 0 2px #090a1a, 0 0 0 4px rgba(216,180,90,.55);
  --glass-bg: rgba(9,10,26,.94); --glass-border: rgba(238,240,255,.14); --glass-highlight: inset 0 1px rgba(238,240,255,.06);
  --shadow-card: 0 2px 14px rgba(4,5,18,.5); --shadow-hover: 0 6px 22px rgba(4,5,18,.6);
  --mascot-accent: #8ea2ff; --theme-ambient: rgba(126,148,255,.18);
  --wallpaper-scrim: <保留现有>; --wallpaper-brightness: .80; --wallpaper-saturation: 1.00;
}
```

**控件签名（星穹优雅）**：柔圆角 + 金发丝线。
- `.ui-button`：8px 圆角；letter-spacing .04em。
- **`.ui-button--primary` 必须深字**：金底白字仅 2.05:1 → `color: #1a1533`（≈9:1 ✓），hover 用 --accent-hi。
- `.ui-button--secondary`：1px 金色发丝边框（var(--accent-ring)）+ 透明底 + 金色文字。
- `.ui-card`：8px 圆角 + var(--shadow-card)。
- 输入控件：6px 圆角。
- reduced-motion 双写收尾（照 shift-editorial.css 尾部写法）。

**QA 驱动**：QA spec 断言 `--accent === #d8b45a`、body 底色 `rgb(7, 8, 26)`。
