import { describe, expect, it } from "vitest";
import { defaultCapabilities, isViewSupportedOnPlatform } from "./runtime.svelte";

describe("runtime platform capabilities", () => {
  it("keeps desktop capabilities enabled", () => {
    const desktop = defaultCapabilities("windows");
    expect(desktop).toMatchObject({
      gameLaunch: true,
      steamIntegration: true,
      localGameScan: true,
      emulatorImport: true,
      desktopWindowControl: true,
      desktopUpdater: true,
    });
    expect(isViewSupportedOnPlatform("steam-import", desktop)).toBe(true);
  });

  it("exposes only companion-safe capabilities on Android", () => {
    const android = defaultCapabilities("android");
    expect(android).toMatchObject({
      orientationControl: true,
      steamIntegration: false,
      gameLaunch: false,
      localGameScan: false,
      emulatorImport: false,
      tray: false,
      autostart: false,
      desktopUpdater: false,
      externalPlayer: false,
    });
  });

  it("filters desktop-only routes on Android", () => {
    const android = defaultCapabilities("android");
    expect(isViewSupportedOnPlatform("anime", android)).toBe(true);
    expect(isViewSupportedOnPlatform("novel", android)).toBe(true);
    expect(isViewSupportedOnPlatform("settings", android)).toBe(true);
    expect(isViewSupportedOnPlatform("steam-import", android)).toBe(false);
    expect(isViewSupportedOnPlatform("emulator", android)).toBe(false);
    expect(isViewSupportedOnPlatform("diagnostics", android)).toBe(false);
  });
});
