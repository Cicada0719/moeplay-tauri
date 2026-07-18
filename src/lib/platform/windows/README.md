# lib/platform/windows — Windows 桌面端专属

路径：`src/lib/platform/windows/`

## 职责

承载仅 Windows 桌面端（Tauri 桌面构建）适用的能力与策略：

- `capabilities.ts`：`desktopCapabilities` —— 桌面端能力表（桌面窗口控制、系统托盘、开机自启、签名自动更新、Steam 集成、本地游戏扫描、外部播放器、文件监视等）。`runtime.svelte.ts` 探测到 Windows 端时采用此表。
- 后续新增的桌面专属策略（窗口行为、托盘、更新器、自启动等）也放在本目录。

## 导入约定

- 本目录模块只被 `src/lib/platform/index.ts` re-export；使用方从 `lib/platform` 入口导入，禁止直接深路径 import 本目录。
- 安卓侧代码禁止 import 本目录（跨端引用会导致安卓构建/运行报错）。
- 类型上允许 `import type` 引用 `../runtime.svelte` 的 `PlatformCapabilities`（仅类型，编译期擦除，无运行时循环依赖）。
