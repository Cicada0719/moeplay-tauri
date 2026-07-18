# Theme Spec · borderless-lumen「无界流光」（stretch）

## 1. 设计参考
teamLab Borderless（teamlab.art）。无界光场：纯黑空间中晕开的有机渐变色彩、呼吸感光晕、沉浸式荧光粒子。
**禁忌**：不复制其装置照片与 logo；只取黑底+有机色场语言。

## 2. Token 表（`[data-theme-pack="borderless-lumen"]`）

```css
[data-theme-pack="borderless-lumen"] {
  --accent: #7c5cff;
  --accent-hi: #a68bff;
  --accent-lo: rgba(124, 92, 255, .18);
  --accent-ring: rgba(124, 92, 255, .55);
  --mascot-accent: #56e0d4;
  --theme-ambient: rgba(124, 92, 255, .22);
  --wallpaper-scrim:
    linear-gradient(90deg, rgba(4,4,8,.88) 0%, rgba(4,4,8,.58) 47%, rgba(4,4,8,.22) 100%),
    linear-gradient(0deg, rgba(4,4,8,.82), transparent 50%);
  --wallpaper-brightness: .80;
  --wallpaper-saturation: 1.10;
}
```
dark 色模式底色沿用 dark 块。对比度自检：accent on #0b0e14 ≈ 4.9:1（AA）。
defaultColorMode: `dark`；decoration: `petals`（发光浮瓣）。

## 3. 构图描述（`scripts/theme-art/borderless-lumen.mjs`）
- **wallpaper-1「花舞光场」**：#040408 黑底；三团柔光色晕（radial-gradient：紫 #7c5cff、青 #56e0d4、品红 #e056a8，直径 40-60vh，错位分布，边缘全透明）；散景光点 20+（小圆，白色/淡紫，opacity .15-.5，部分失焦放大）。
- **wallpaper-2「水镜」**：黑底；中部一条水平流光带（青 → 紫 → 品红 的 180° linear-gradient，高 30vh，上下大 blur 柔化）；其倒影（scaleY(-1)，opacity .3，向下渐隐）。
- **wallpaper-3「萤火之森」**：黑底；偏右一团大紫晕（70vh）；数十微小发光粒子（1-2px，青/白/紫，随机分布，少数带 8px 光晕）；底部一线极淡青色地平线辉光。
- **preview（800×500）**：单团紫青光晕 + 中央「无界流光」白色细体小标题。
- **mascot（512×512 透明 PNG）**：三枚交叠的软渐变圆（紫/青/品红，直径 45%，两两重叠区自然混色，整体外缘 12px 同色光晕）。有机、无硬边。

## 4. 验收清单
- token 色值一致；AA 对比度实测。
- 资产 8 件（规格同前）；回读目检：色晕过渡自然无 banding、粒子不过曝。
- `theme-pack-borderless-lumen.test.ts` 断言：注册表含包、token 文件含 `--accent: #7c5cff`、资产 existsSync。

> 本主题为 stretch：时间不足时整包跳过（Wave F 已交付占位版本，构建不受影响）。

---

## 5. Wave T2 · 全套换肤扩容（覆写 scheme-c）

**选择器纪律**：所有规则必须以 `html[data-theme-pack="borderless-lumen"]` 为作用域（同特异性 (0,1,1)，靠 main.ts 末位导入取胜）。参照 `src/lib/styles/themes/shift-editorial.css`（已通过 QA 的参考实现）。

**全量调色板**（写入 `src/lib/styles/themes/borderless-lumen.css`，替换原低特异性块）：

```css
html[data-theme-pack="borderless-lumen"] {
  --c-black: #040408; --c-void: #07070d; --c-surface-1: #0b0b14; --c-surface-2: #10101c; --c-surface-3: #151524;
  --c-paper: #f2f1ff; --c-muted: #a9a6cc; --c-dim: #615e85;
  --c-line: rgba(242,241,255,.13); --c-line-strong: rgba(242,241,255,.34);
  --c-accent: #7c5cff; --c-accent-hi: #a68bff;
  --accent: #7c5cff; --accent-hi: #a68bff; --accent-lo: rgba(124,92,255,.18); --accent-ring: rgba(124,92,255,.55);
  --bg-void: #040408; --bg-deep: #07070d; --bg: #0b0b14; --bg-base: #0b0b14; --bg-surface: #0e0e1a;
  --bg-secondary: #10101c; --bg-elev: #121222; --bg-card: #17172a; --bg-hover: #1c1c33; --bg-active: rgba(124,92,255,.16);
  --text-primary: #f2f1ff; --text-secondary: #c6c3e8; --text-muted: #a9a6cc; --text-dim: #615e85;
  --border: rgba(242,241,255,.12); --border-hover: rgba(242,241,255,.30); --border-glass: rgba(242,241,255,.10);
  --radius-sm: 6px; --radius-md: 10px; --radius-lg: 14px; --radius-xl: 18px;
  --focus-ring: 0 0 0 2px #040408, 0 0 0 4px rgba(124,92,255,.60);
  --glass-bg: rgba(4,4,8,.92); --glass-border: rgba(242,241,255,.13); --glass-highlight: none;
  --shadow-card: 0 2px 16px rgba(2,2,6,.6); --shadow-hover: 0 8px 28px rgba(2,2,6,.7);
  --mascot-accent: #56e0d4; --theme-ambient: rgba(124,92,255,.22);
  --wallpaper-scrim: <保留现有>; --wallpaper-brightness: .80; --wallpaper-saturation: 1.10;
}
```

**控件签名（有机流光）**：全圆角 pill + 柔光。
- `.ui-button`：`border-radius: 999px`；无边框；primary 用渐变底 `linear-gradient(120deg, #6f4ef0, #4f8fe8)` + 白字（#6f4ef0 处对比度 ≈5.2:1 ✓），hover 提亮（filter: brightness(1.12)，transform 不位移）。
- `.ui-button--secondary/.ui-button--ghost`：1px var(--border) 边框 + 全圆角。
- `.ui-card`：14px 圆角 + var(--shadow-card)。
- 输入控件：10px 圆角。
- reduced-motion 双写收尾（照 shift-editorial.css 尾部写法）。

**QA 驱动**：QA spec 断言 `--accent === #7c5cff`、body 底色 `rgb(4, 4, 8)`。
