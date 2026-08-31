import { describe, expect, it } from "vitest";
import {
  getReaderKeyboardCommand,
  moveReaderSpread,
  normalizeSpread,
  spreadStepSize,
  spreadWindowPages,
  spreadWindowStart,
} from "./reader";

describe("comic reader double-page helpers", () => {
  it("normalizeSpread only accepts single/double", () => {
    expect(normalizeSpread("double")).toBe("double");
    expect(normalizeSpread("single")).toBe("single");
    expect(normalizeSpread("weird" as unknown)).toBe("single");
    expect(normalizeSpread(undefined)).toBe("single");
  });

  it("single-page window is just the clamped page", () => {
    expect(spreadWindowPages(4, 6, "single")).toEqual([4]);
    expect(spreadWindowPages(-1, 6, "single")).toEqual([0]);
    expect(spreadWindowPages(9, 6, "single")).toEqual([5]);
  });

  it("double-page window groups by even start and degrades at odd tail", () => {
    expect(spreadWindowPages(0, 6, "double")).toEqual([0, 1]);
    expect(spreadWindowPages(1, 6, "double")).toEqual([0, 1]);
    expect(spreadWindowPages(3, 6, "double")).toEqual([2, 3]);
    expect(spreadWindowPages(5, 6, "double")).toEqual([4, 5]);
    // 奇数总页数：末组只剩单页
    expect(spreadWindowPages(4, 5, "double")).toEqual([4]);
    expect(spreadWindowPages(0, 5, "double")).toEqual([0, 1]);
  });

  it("spread step size is 2 in double mode, 1 otherwise", () => {
    expect(spreadStepSize("double")).toBe(2);
    expect(spreadStepSize("single")).toBe(1);
  });

  it("spreadWindowStart aligns to even", () => {
    expect(spreadWindowStart(0)).toBe(0);
    expect(spreadWindowStart(3)).toBe(2);
    expect(spreadWindowStart(4)).toBe(4);
  });

  it("moveReaderSpread steps by groups and keeps even start", () => {
    expect(moveReaderSpread(0, 1, 6, "double")).toBe(2);
    expect(moveReaderSpread(2, -1, 6, "double")).toBe(0);
    expect(moveReaderSpread(0, -1, 6, "double")).toBe(0);
    expect(moveReaderSpread(4, 1, 5, "double")).toBe(4);
    expect(moveReaderSpread(2, 1, 6, "single")).toBe(3);
  });

  it("keyboard exposes cycle_spread on S", () => {
    expect(getReaderKeyboardCommand({ key: "s", ctrlKey: false, metaKey: false, altKey: false, shiftKey: false }, "vertical")).toBe("cycle_spread");
    expect(getReaderKeyboardCommand({ key: "S", ctrlKey: false, metaKey: false, altKey: false, shiftKey: false }, "left-to-right")).toBe("cycle_spread");
  });
});
