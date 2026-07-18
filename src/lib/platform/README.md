# lib/platform — 平台能力抽象层

桌面（Windows）与安卓（Android）两端的运行时探测、能力声明与端专属策略的统一入口。

## 目录结构与职责

| 路径 | 职责 |
| --- | --- |
| `src/lib/platform/`（本目录） | 核心层。`runtime.svelte.ts` 负责端探测（UA / query string / 后端 `get_platform_capabilities`）并暴露 `platformStore`、`PlatformCapabilities`、`defaultCapabilities`、`isViewSupportedOnPlatform` 等公共 API；`index.ts` 是唯一出口。 |
| `src/lib/platform/windows/` | Windows 桌面端专属。目前承载桌面能力声明（`capabilities.ts` 的 `desktopCapabilities`：桌面窗口控制、托盘、开机自启、签名自动更新、Steam 集成、本地扫描等）；后续桌面专属策略（窗口行为、更新器策略等）也放这里。 |
| `src/lib/platform/android/` | 安卓端专属。目前承载屏幕方向模块：`orientation.svelte.ts`（`orientationStore`，含能力守卫）与 `orientation-policy.ts`（纯函数策略：视频全屏自动横屏 / 恢复）。 |

## 导入约定

1. 使用方**一律从 `lib/platform` 入口导入**（按所在深度写 `../platform` / `../../platform` 等），不要直接深路径引用 `lib/platform/runtime.svelte`、`lib/platform/windows/*`、`lib/platform/android/*`。
2. **禁止跨端直接 import 对端目录**：Windows 侧代码不得 import `lib/platform/android/*`，安卓侧代码不得 import `lib/platform/windows/*`。端专属模块只能经由 `index.ts` re-export 后被消费，内部模块间的相对引用（如 `android/orientation.svelte.ts` 引用 `../runtime.svelte` 的 `platformStore`）除外。
3. 端专属能力必须先经过能力判定（`platformStore.capabilities.*`）再调用；安卓专属副作用 API（如 `orientationStore.setMode`）内部已做能力守卫，Windows 端误调为静默 no-op，不会抛错。

## 公开 API（re-export 自 `index.ts`）

- `runtime.svelte.ts`：`platformStore`、`RuntimePlatform`、`PlatformCapabilities`、`MOBILE_ALLOWED_VIEWS`、`isViewSupportedOnPlatform`、`defaultCapabilities`。
- `windows/capabilities.ts`：`desktopCapabilities`（Windows 桌面端能力表）。
- `android/orientation.svelte.ts`：`orientationStore`、`OrientationMode` 类型。
