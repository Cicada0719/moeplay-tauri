import { beforeEach, describe, expect, it } from "vitest";
import { readHandheldPreference, resolveHandheld } from "./handheld";

describe("handheld 掌机判定", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("auto + 非触屏：720p/800p 矮横屏命中", () => {
    expect(resolveHandheld("auto", { width: 1280, height: 720 }, false)).toBe(true);
    expect(resolveHandheld("auto", { width: 1280, height: 800 }, false)).toBe(true);
    expect(resolveHandheld("auto", { width: 1366, height: 768 }, false)).toBe(true);
  });

  it("auto + 非触屏：1080p 不命中（防桌面显示器误判）", () => {
    expect(resolveHandheld("auto", { width: 1920, height: 1080 }, false)).toBe(false);
  });

  it("auto + 触屏：1080p 掌机命中", () => {
    expect(resolveHandheld("auto", { width: 1920, height: 1080 }, true)).toBe(true);
    expect(resolveHandheld("auto", { width: 1280, height: 800 }, true)).toBe(true);
  });

  it("auto：电视/竖屏/超限不命中", () => {
    expect(resolveHandheld("auto", { width: 3840, height: 2160 }, true)).toBe(false); // 4K
    expect(resolveHandheld("auto", { width: 2560, height: 1440 }, true)).toBe(false); // 宽超限
    expect(resolveHandheld("auto", { width: 720, height: 1280 }, true)).toBe(false); // 竖屏
    expect(resolveHandheld("auto", { width: 1920, height: 1200 }, true)).toBe(false); // 高超限
  });

  it("手动开/关覆盖自动判定", () => {
    expect(resolveHandheld("on", { width: 3840, height: 2160 }, false)).toBe(true);
    expect(resolveHandheld("off", { width: 1280, height: 800 }, true)).toBe(false);
  });

  it("偏好读取：缺失/非法值回退 auto", () => {
    expect(readHandheldPreference()).toBe("auto");
    localStorage.setItem("moeplay-handheld-mode-v1", "on");
    expect(readHandheldPreference()).toBe("on");
    localStorage.setItem("moeplay-handheld-mode-v1", "garbage");
    expect(readHandheldPreference()).toBe("auto");
  });
});
