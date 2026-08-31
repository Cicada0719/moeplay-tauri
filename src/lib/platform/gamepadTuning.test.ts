import { beforeEach, describe, expect, it } from "vitest";
import {
  AXIS_PRESETS,
  gamepadTuning,
  getGamepadTuningRevision,
} from "./gamepadTuning.svelte";

describe("gamepadTuning 手柄灵敏度调参", () => {
  beforeEach(() => {
    localStorage.clear();
    // 重置模块状态到标准档
    gamepadTuning.sensitivity = "standard";
    gamepadTuning.repeatSpeed = "standard";
  });

  it("默认标准档与历史默认一致（0.55/0.35、100ms、320ms）", () => {
    expect(gamepadTuning.axisPress).toBe(0.55);
    expect(gamepadTuning.axisRelease).toBe(0.35);
    expect(gamepadTuning.repeatIntervalMs).toBe(100);
    expect(gamepadTuning.initialDelayMs).toBe(320);
  });

  it("切换灵敏度与连发速度并持久化", () => {
    gamepadTuning.sensitivity = "loose";
    expect(gamepadTuning.axisPress).toBe(AXIS_PRESETS.loose.press);
    gamepadTuning.sensitivity = "tight";
    expect(gamepadTuning.axisPress).toBe(AXIS_PRESETS.tight.press);
    gamepadTuning.repeatSpeed = "fast";
    expect(gamepadTuning.repeatIntervalMs).toBe(60);
    expect(gamepadTuning.initialDelayMs).toBe(192);
    expect(localStorage.getItem("moeplay-gamepad-sensitivity-v1")).toBe("tight");
    expect(localStorage.getItem("moeplay-gamepad-repeat-v1")).toBe("fast");
  });

  it("修改推进 revision（runtime 缓存失效信号）", () => {
    const before = getGamepadTuningRevision();
    gamepadTuning.repeatSpeed = "slow";
    expect(getGamepadTuningRevision()).toBeGreaterThan(before);
  });
});
