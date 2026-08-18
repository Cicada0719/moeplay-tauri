import { beforeEach, describe, expect, it } from "vitest";
import {
  gamepadGlyphFor,
  physicalButtonFor,
  readGamepadRemap,
  resetGamepadRemap,
  writeGamepadRemap,
} from "./gamepadRemap";

describe("gamepadRemap 按键绑定", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("默认绑定 = 语义索引（Xbox），任天堂布局自动换位", () => {
    expect(physicalButtonFor(0, "xbox")).toBe(0);
    expect(physicalButtonFor(1, "xbox")).toBe(1);
    expect(physicalButtonFor(2, "xbox")).toBe(2);
    expect(physicalButtonFor(0, "nintendo")).toBe(1); // 确认 → 右键 A
    expect(physicalButtonFor(1, "nintendo")).toBe(0); // 取消 → 下键 B
    expect(physicalButtonFor(4, "nintendo")).toBe(4); // LB 不换
    expect(physicalButtonFor(12, "xbox")).toBe(12); // 方向键不参与
  });

  it("显式重绑为物理索引，不受布局换位影响", () => {
    writeGamepadRemap({ launch: 3 });
    expect(physicalButtonFor(0, "xbox")).toBe(3);
    expect(physicalButtonFor(0, "nintendo")).toBe(3);
  });

  it("读写与非法值过滤", () => {
    expect(readGamepadRemap()).toEqual({});
    const bad: Record<string, unknown> = { back: 2, filter: 99, garbage: 1 };
    writeGamepadRemap(bad as never);
    const map = readGamepadRemap();
    expect(map).toEqual({ back: 2 });
    expect(map.filter).toBeUndefined();
    localStorage.setItem("moeplay-gamepad-remap-v1", "{broken");
    expect(readGamepadRemap()).toEqual({});
  });

  it("恢复默认清除绑定", () => {
    writeGamepadRemap({ launch: 3 });
    resetGamepadRemap();
    expect(readGamepadRemap()).toEqual({});
    expect(physicalButtonFor(0, "xbox")).toBe(0);
  });

  it("glyph 标签随布局与重绑变化", () => {
    expect(gamepadGlyphFor("launch", "xbox")).toBe("A");
    expect(gamepadGlyphFor("launch", "nintendo")).toBe("A"); // 任天堂 A 在右
    expect(gamepadGlyphFor("back", "nintendo")).toBe("B");
    expect(gamepadGlyphFor("pageLeft", "xbox")).toBe("LB");
    expect(gamepadGlyphFor("start", "xbox")).toBe("START");
    writeGamepadRemap({ launch: 3 });
    expect(gamepadGlyphFor("launch", "xbox")).toBe("Y");
    expect(gamepadGlyphFor("launch", "nintendo")).toBe("X"); // 物理 3 = 任天堂 X（上键）
  });

  it("PlayStation 布局显示 ○×□△ 符号键帽", () => {
    expect(gamepadGlyphFor("launch", "playstation")).toBe("✕");
    expect(gamepadGlyphFor("back", "playstation")).toBe("○");
    expect(gamepadGlyphFor("favorite", "playstation")).toBe("□");
    expect(gamepadGlyphFor("activate", "playstation")).toBe("△");
    expect(gamepadGlyphFor("pageLeft", "playstation")).toBe("L1");
    expect(gamepadGlyphFor("start", "playstation")).toBe("OPTIONS");
    expect(physicalButtonFor(0, "playstation")).toBe(0); // 语义顺序一致
  });
});
