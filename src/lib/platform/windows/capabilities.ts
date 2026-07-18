import type { PlatformCapabilities } from "../runtime.svelte";

// Windows 桌面端能力判定（桌面专属）：
// 桌面窗口控制、系统托盘、开机自启、签名自动更新、Steam 集成、本地游戏扫描等
// 仅 Windows 桌面端具备的能力在此集中声明；runtime.svelte.ts 探测到 windows 端时采用。
export const desktopCapabilities: PlatformCapabilities = {
  platform: "windows",
  orientationControl: false,
  steamIntegration: true,
  gameLaunch: true,
  localGameScan: true,
  emulatorImport: true,
  desktopWindowControl: true,
  tray: true,
  autostart: true,
  desktopUpdater: true,
  externalPlayer: true,
  fileSystemWatch: true,
};
