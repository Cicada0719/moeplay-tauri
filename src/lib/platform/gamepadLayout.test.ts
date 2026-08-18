import { beforeEach, describe, expect, it } from "vitest";
import {
  detectGamepadLayout,
  getGamepadLayoutRevision,
  mapFaceButton,
  readDeviceLayoutFor,
  readDeviceLayoutMap,
  readGamepadLayoutPreference,
  resolveConnectedPadLayouts,
  resolveGamepadLayout,
  writeDeviceLayoutPreference,
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

describe("gamepadLayout 逐手柄覆盖（UU远程 等串流虚拟手柄场景）", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  // UU远程 虚拟手柄上报的 id 与真实 Xbox 完全相同，自动检测永远按 Xbox 语义，
  // 用户需要能为它单独指定任天堂布局。
  const UU_PAD_ID = "Xbox 360 Controller (XInput STANDARD GAMEPAD)";

  it("逐手柄覆盖优先于自动检测与全局偏好", () => {
    writeDeviceLayoutPreference(UU_PAD_ID, "nintendo", 0);
    expect(resolveGamepadLayout(UU_PAD_ID, "auto", 0)).toBe("nintendo");
    // 全局强制 xbox 也不能压过逐手柄的显式指定
    expect(resolveGamepadLayout(UU_PAD_ID, "xbox", 0)).toBe("nintendo");
    // 同一 id 的另一个槽位不受影响（真实 Xbox 手柄）
    expect(resolveGamepadLayout(UU_PAD_ID, "auto", 1)).toBe("xbox");
  });

  it("无槽位覆盖时退回 id 级覆盖，再退回全局偏好", () => {
    writeDeviceLayoutPreference(UU_PAD_ID, "nintendo");
    expect(resolveGamepadLayout(UU_PAD_ID, "auto", 0)).toBe("nintendo");
    expect(resolveGamepadLayout(UU_PAD_ID, "auto", 3)).toBe("nintendo");
    // 其他手柄不受影响
    expect(resolveGamepadLayout("DualSense Wireless Controller", "auto", 0)).toBe("xbox");
  });

  it("写入 null 清除覆盖，回到自动检测", () => {
    writeDeviceLayoutPreference(UU_PAD_ID, "nintendo", 0);
    writeDeviceLayoutPreference(UU_PAD_ID, null, 0);
    expect(readDeviceLayoutFor(UU_PAD_ID, 0)).toBeNull();
    expect(readDeviceLayoutMap()).toEqual({});
    expect(resolveGamepadLayout(UU_PAD_ID, "auto", 0)).toBe("xbox");
  });

  it("槽位覆盖优先于 id 级覆盖", () => {
    writeDeviceLayoutPreference(UU_PAD_ID, "nintendo");
    writeDeviceLayoutPreference(UU_PAD_ID, "xbox", 2);
    expect(resolveGamepadLayout(UU_PAD_ID, "auto", 2)).toBe("xbox");
    expect(resolveGamepadLayout(UU_PAD_ID, "auto", 0)).toBe("nintendo");
  });

  it("非法存储内容回退为空表，不抛错", () => {
    localStorage.setItem("moeplay-gamepad-layout-devices-v1", "{broken json");
    expect(readDeviceLayoutMap()).toEqual({});
    localStorage.setItem("moeplay-gamepad-layout-devices-v1", "[1,2,3]");
    expect(readDeviceLayoutMap()).toEqual({});
    localStorage.setItem("moeplay-gamepad-layout-devices-v1", JSON.stringify({ x: "garbage", y: "nintendo" }));
    expect(readDeviceLayoutMap()).toEqual({ y: "nintendo" });
  });

  it("写入全局偏好或逐手柄覆盖都会推进 revision（runtime 缓存失效信号）", () => {
    const before = getGamepadLayoutRevision();
    writeDeviceLayoutPreference(UU_PAD_ID, "nintendo", 0);
    expect(getGamepadLayoutRevision()).toBeGreaterThan(before);
    const mid = getGamepadLayoutRevision();
    writeGamepadLayoutPreference("xbox");
    expect(getGamepadLayoutRevision()).toBeGreaterThan(mid);
  });

  it("resolveConnectedPadLayouts 返回每个手柄的最终/自动检测/覆盖来源", () => {
    writeDeviceLayoutPreference(UU_PAD_ID, "nintendo", 0);
    const pads = [
      { id: UU_PAD_ID, index: 0, connected: true },
      { id: "Nintendo Switch Pro Controller", index: 1, connected: true },
      { id: "Virtual HID Device", index: 2, connected: false },
      null,
    ];
    const resolved = resolveConnectedPadLayouts(pads, "auto");
    expect(resolved).toHaveLength(2);
    expect(resolved[0]).toMatchObject({
      id: UU_PAD_ID, index: 0, layout: "nintendo", detected: "xbox", deviceOverride: "nintendo",
    });
    expect(resolved[1]).toMatchObject({
      id: "Nintendo Switch Pro Controller", index: 1, layout: "nintendo", detected: "nintendo", deviceOverride: null,
    });
  });
});
