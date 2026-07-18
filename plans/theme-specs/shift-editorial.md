# Theme Spec · shift-editorial「素纸编集」

## 1. 设计参考
shiftbrain.com（SHIFTBRAIN，日本数字设计机构）。纸面编辑美学：米白纸底、发丝线（hairline）、超大字号与严谨字距的编辑排版、单一信号红点睛、克制的不对称网格。
**禁忌**：不复制其站任何具体作品图、logo、字体文件；只取配色与版式气质。

## 2. Token 表（写入 `src/lib/styles/themes/shift-editorial.css` 的 `[data-theme-pack="shift-editorial"]` 块）

```css
[data-theme-pack="shift-editorial"] {
  --accent: #d4293c;
  --accent-hi: #e8485a;
  --accent-lo: rgba(212, 41, 60, .10);
  --accent-ring: rgba(212, 41, 60, .45);
  --mascot-accent: #1a1a1a;
  --theme-ambient: rgba(212, 41, 60, .08);
  --wallpaper-scrim:
    linear-gradient(90deg, rgba(250,248,243,.88) 0%, rgba(250,248,243,.62) 47%, rgba(250,248,243,.25) 100%),
    linear-gradient(0deg, rgba(250,248,243,.85), transparent 50%);
  --wallpaper-brightness: .96;
  --wallpaper-saturation: .85;
  /* light color-mode 底色向纸面校准（pack 块内覆盖，不污染其他包） */
  --bg-void: #f1ede4;
  --bg-deep: #f7f4ee;
  --bg: #f2eee6;
  --bg-base: var(--bg);
  --bg-surface: #fbf9f4;
  --bg-secondary: #ece7dc;
  --bg-elev: #fffefa;
  --bg-card: #fffdf8;
  --bg-hover: #e7e1d4;
  --bg-active: rgba(212, 41, 60, .12);
  --text-primary: #18150f;
  --text-secondary: #4a4438;
  --text-muted: #8a8272;
  --text-dim: #b0a894;
  --border: rgba(24, 21, 15, .14);
  --border-hover: rgba(24, 21, 15, .28);
  --border-glass: rgba(24, 21, 15, .10);
}
```
对比度自检：text-primary on bg-card ≈ 15:1（AA+）；accent on bg-card ≈ 5.6:1（AA）。
defaultColorMode: `light`；decoration: `light-particles`。

## 3. 构图描述（`scripts/theme-art/shift-editorial.mjs`）
- **wallpaper-1「横排大标题」**：#f7f4ee 纸底；右侧出血的超大墨色（#18150f）衬线感字母 "MOE"（系统字体 Georgia/serif 粗体，字高约 60vh）；左侧竖排 1px hairline 栏线三条；角落红色（#d4293c）8×8px 对齐标记与 10px 等宽小字（mono，灰色 #8a8272）。
- **wallpaper-2「网格与留白」**：纸底；左 2/3 大留白；右 1/3 密排水平 hairline（rgba(24,21,15,.18)，间距 18px）；一个 48×48px 红色实心方块作为唯一色点；底部一条贯穿 hairline。
- **wallpaper-3「墨色渐变」**：纸底到 #e4ded0 的垂直淡渐变；一条 6px 红色对角粗线自左下向右上（35°）；角落浅灰 halftone 圆点阵（radial-gradient 点，10% 透明度）。
- **preview（800×500）**：wallpaper-1 的缩小变体 + 底部白色信息条（hairline 上边框 + 黑色小标题「素纸编集」+ 红色小方块）。
- **mascot（512×512 透明 PNG）**：红色实心圆（直径 55%）居中，叠加墨色 2px hairline 十字（横竖各一，超出圆边缘），圆外一个 3px 墨色细方框（边长 88%）。扁平无渐变。

## 4. 验收清单
- token 文件色值与本表一致；文本对比度 AA（实测主要文本组合）。
- 资产 8 件：wallpaper-{1,2,3}.jpg（1920×1080 ≤600KB）、wallpaper-{1,2,3}-blur.jpg（32×18）、preview.jpg（800×500）、mascot.png（512×512 透明 ≤300KB）。
- 回读目检：无文字裁切、无偏色、blur 与主图同构图。
- `theme-pack-shift-editorial.test.ts` 断言：注册表含包、token 文件含 `--accent: #d4293c`、资产 existsSync。
