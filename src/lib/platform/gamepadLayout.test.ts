import { beforeEach, describe, expect, it } from "vitest";
import {
  detectGamepadLayout,
  mapFaceButton,
  readGamepadLayoutPreference,
  resolveGamepadLayout,
  writeGamepadLayoutPreference,
} from "./gamepadLayout";

describe("gamepadLayout", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("按 id 检测任天堂手柄", () => {
    expect(detectGamepadLayout("Nintendo Switch Pro Controller")).toBe("nintendo");
    expect(detectGamepadLayout("Pro Controller (057e:2009)")).toBe("nintendo");
    expect(detectGamepadLayout("Joy-Con (L)")).toBe("nintendo");
    expect(detectGamepadLayout("Joy-Con (R)")).toBe("nintendo");
    expect(detectGamepadLayout("Wireless Gamepad (STANDARD GAMEPAD Vendor: 057e Product: 2009)")).toBe("nintendo");
  });

  it("非任天堂特征一律按 Xbox 语义", () => {
    expect(detectGamepadLayout("Xbox 360 Controller (XInput STANDARD GAMEPAD)")).toBe("xbox");
    expect(detectGamepadLayout("DualSense Wireless Controller")).toBe("xbox");
    expect(detectGamepadLayout("")).toBe("xbox");
  });

  it("override 优先于自动检测", () => {
    expect(resolveGamepadLayout("Xbox 360 Controller", "nintendo")).toBe("nintendo");
    expect(resolveGamepadLayout("Nintendo Switch Pro Controller", "xbox")).toBe("xbox");
    expect(resolveGamepadLayout("Nintendo Switch Pro Controller", "auto")).toBe("nintendo");
    expect(resolveGamepadLayout("Xbox 360 Controller")).toBe("xbox"); // 默认 auto
  });

  it("任天堂布局交换 0↔1、2↔3，其余索引不变", () => {
    expect(mapFaceButton("nintendo", 0)).toBe(1); // 确认 → 右键(A)
    expect(mapFaceButton("nintendo", 1)).toBe(0); // 取消 → 下键(B)
    expect(mapFaceButton("nintendo", 2)).toBe(3); // X → 上键
    expect(mapFaceButton("nintendo", 3)).toBe(2); // Y → 左键
    expect(mapFaceButton("nintendo", 4)).toBe(4); // LB 不换
    expect(mapFaceButton("nintendo", 9)).toBe(9); // START 不换
  });

  it("xbox 布局原样返回", () => {
    for (const i of [0, 1, 2, 3, 4, 5, 8, 9]) {
      expect(mapFaceButton("xbox", i)).toBe(i);
    }
  });

  it("偏好读写与非法值回退", () => {
    expect(readGamepadLayoutPreference()).toBe("auto");
    writeGamepadLayoutPreference("nintendo");
    expect(readGamepadLayoutPreference()).toBe("nintendo");
    localStorage.setItem("moeplay-gamepad-layout-v1", "garbage");
    expect(readGamepadLayoutPreference()).toBe("auto");
  });
});
