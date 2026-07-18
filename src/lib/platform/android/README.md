# lib/platform/android — 安卓端专属

路径：`src/lib/platform/android/`

## 职责

承载仅安卓端（Tauri Android 构建）适用的能力与策略：

- `orientation.svelte.ts`：`orientationStore` —— 屏幕方向偏好（自动 / 竖屏 / 横屏）、视频全屏自动横屏、原生方向插件桥接。所有会触发原生副作用的 setter（`setMode`、`setVideoAutoLandscape`）都带能力守卫：`platformStore.capabilities.orientationControl` 为 `false` 时静默 no-op，Windows 端误调也不会抛错。
- `orientation-policy.ts`：纯函数策略（进入 / 退出视频全屏时的方向推导），无运行时依赖，可在 vitest 下直接单测（`orientation-policy.test.ts`）。

## 导入约定

- 本目录模块只被 `src/lib/platform/index.ts` re-export；使用方从 `lib/platform` 入口导入（如 `import { orientationStore } from "../platform"`），禁止直接深路径 import 本目录。
- Windows 侧代码禁止 import 本目录；桌面端通过 `platformStore.capabilities.orientationControl === false` 自然隐藏相关 UI（如设置页屏幕方向区块）。
- 本目录内部引用核心层使用相对路径（如 `../runtime.svelte`），这是唯一允许的跨层引用方向。
