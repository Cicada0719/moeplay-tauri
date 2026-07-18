// platform 唯一出口：core（端探测 + capabilities）+ windows（桌面专属能力声明）+ android（屏幕方向）。
// 使用方一律从 "../platform"（或相应深度的 lib/platform 入口）导入，禁止跨端直接 import 对端目录。
export * from "./runtime.svelte";
export * from "./windows/capabilities";
export * from "./android/orientation.svelte";
