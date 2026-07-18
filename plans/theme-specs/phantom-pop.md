# Theme Spec · phantom-pop「魅影波普」

## 1. 设计参考
Persona 5 视觉体系（Atlus）。怪盗波普：激进斜切构图、黑/红/白三色撞色、漫画 halftone 网点、撕裂拼贴与高能量节奏。
**禁忌**：禁止 P5 logo、星形图标商标造型、角色与任何截图；只取黑红白波普语言。

## 2. Token 表（`[data-theme-pack="phantom-pop"]`）

```css
[data-theme-pack="phantom-pop"] {
  --accent: #e6242f;
  --accent-hi: #ff4d57;
  --accent-lo: rgba(230, 36, 47, .18);
  --accent-ring: rgba(230, 36, 47, .55);
  --mascot-accent: #ffffff;
  --theme-ambient: rgba(230, 36, 47, .20);
  --wallpaper-scrim:
    linear-gradient(90deg, rgba(10,8,9,.90) 0%, rgba(10,8,9,.62) 47%, rgba(10,8,9,.28) 100%),
    linear-gradient(0deg, rgba(10,8,9,.86), transparent 52%);
  --wallpaper-brightness: .78;
  --wallpaper-saturation: 1.05;
}
```
dark 色模式底色沿用 app.css dark 块（近黑），不额外覆盖。对比度自检：accent on #0b0e14 ≈ 4.8:1（AA）。
defaultColorMode: `dark`；decoration: `petals`（红色碎纸飘落，读作波普 confetti）。

## 3. 构图描述（`scripts/theme-art/phantom-pop.mjs`）
- **wallpaper-1「斜切红白黑」**：#0a0809 黑底；一条 40vh 宽的红色（#e6242f）对角色带自左上切至右下（-18°）；色带边缘一条 12px 白色平行细带；黑色区域铺 halftone 圆点（rgba(255,255,255,.06)，radial-gradient 24px 网格）；角落旋转 -8° 的白色等宽小字两行（mono，opacity .7，无裁切）。
- **wallpaper-2「拼贴框」**：黑底；三个旋转的矩形块（红实心、白描边、墨灰 #1a1617 实心 + halftone 阴影）错位叠放如撕裂海报；中央偏右一个白色八角爆炸形（clip-path polygon，自绘通用爆炸框，非商标造型）。
- **wallpaper-3「红黑渐变噪点」**：#0a0809 → #3d0b10 的对角渐变；全幅噪点纹理（多层 box-shadow 点或 SVG feTurbulence data-uri，opacity .05）；一条 8px 白色对角细线（24°）贯穿。
- **preview（800×500）**：wallpaper-1 变体 + 底部黑色信息条（白色粗体小标题「魅影波普」+ 红色斜切角）。
- **mascot（512×512 透明 PNG）**：黑色八角爆炸形（边长 70%），外描 10px 红色粗边，中心一个白色实心五角星（自绘 polygon）。扁平高对比。

## 4. 验收清单
- token 色值一致；AA 对比度实测。
- 资产 8 件（规格同 shift-editorial spec）；回读目检：斜切边缘锐利、文字无裁切。
- `theme-pack-phantom-pop.test.ts` 断言：注册表含包、token 文件含 `--accent: #e6242f`、资产 existsSync。

---

## 5. Wave T2 · 全套换肤扩容（覆写 scheme-c）

**选择器纪律**：所有规则必须以 `html[data-theme-pack="phantom-pop"]` 为作用域（与 scheme-c 同特异性 (0,1,1)，靠 main.ts 末位导入在源码序上取胜）。参照 `src/lib/styles/themes/shift-editorial.css`（已通过 QA 的参考实现）。

**全量调色板**（写入 `src/lib/styles/themes/phantom-pop.css`，替换原低特异性块）：

```css
html[data-theme-pack="phantom-pop"] {
  --c-black: #0a0809; --c-void: #0d0a0b; --c-surface-1: #120e10; --c-surface-2: #1a1416; --c-surface-3: #221a1d;
  --c-paper: #f5f0f1; --c-muted: #a89a9e; --c-dim: #6e5f63;
  --c-line: rgba(245,240,241,.16); --c-line-strong: rgba(245,240,241,.40);
  --c-accent: #e6242f; --c-accent-hi: #ff4d57;
  --accent: #e6242f; --accent-hi: #ff4d57; --accent-lo: rgba(230,36,47,.18); --accent-ring: rgba(230,36,47,.55);
  --bg-void: #0a0809; --bg-deep: #0d0a0b; --bg: #120e10; --bg-base: #120e10; --bg-surface: #171114;
  --bg-secondary: #1a1416; --bg-elev: #1e1518; --bg-card: #241a1e; --bg-hover: #2a1e23; --bg-active: rgba(230,36,47,.16);
  --text-primary: #f5f0f1; --text-secondary: #cfc0c4; --text-muted: #a89a9e; --text-dim: #6e5f63;
  --border: rgba(245,240,241,.14); --border-hover: rgba(245,240,241,.30); --border-glass: rgba(245,240,241,.10);
  --radius-sm: 0px; --radius-md: 0px; --radius-lg: 0px; --radius-xl: 0px;
  --focus-ring: 0 0 0 2px #0a0809, 0 0 0 4px rgba(230,36,47,.60);
  --glass-bg: rgba(10,8,9,.92); --glass-border: rgba(245,240,241,.16); --glass-highlight: none;
  --shadow-card: none; --shadow-hover: 0 4px 0 rgba(230,36,47,.25);
  --mascot-accent: #ffffff; --theme-ambient: rgba(230,36,47,.20);
  --wallpaper-scrim: <保留现有>; --wallpaper-brightness: .78; --wallpaper-saturation: 1.05;
}
```

**控件签名（怪盗波普）**：锐角切角 + 硬边无阴影。
- `.ui-button`：`clip-path: polygon(0 0, calc(100% - 9px) 0, 100% 9px, 100% 100%, 0 100%)`（右上 9px 切角）；`letter-spacing: .06em`；无 box-shadow；primary 红底白字（#e6242f/white，对比度 4.5:1 ✓），hover 用 --accent-hi，禁用 opacity .5。
- `.ui-button--secondary/.ui-button--ghost`：1px var(--border) 边框 + 透明底。
- `.ui-card`：锐角（radius 0）+ 1px var(--border)，无阴影。
- 输入控件：锐角。
- reduced-motion 双写收尾（照 shift-editorial.css 尾部写法）。

**QA 驱动**：`tests/visual/theme-packs-qa.spec.ts` 已断言该包 `--accent === #e6242f`、body 底色 `rgb(10, 8, 9)`。
