import { describe, expect, it, vi } from "vitest";
import { focusRovingItem, nextRovingIndex } from "./a11y";

describe("anime roving tabindex helpers", () => {
  it("wraps horizontal tabs and supports Home/End", () => {
    expect(nextRovingIndex("ArrowRight", 3, 4)).toBe(0);
    expect(nextRovingIndex("ArrowLeft", 0, 4)).toBe(3);
    expect(nextRovingIndex("Home", 2, 4)).toBe(0);
    expect(nextRovingIndex("End", 1, 4)).toBe(3);
  });

  it("ignores keys outside the configured orientation", () => {
    expect(nextRovingIndex("ArrowDown", 0, 4, "horizontal")).toBeNull();
    expect(nextRovingIndex("ArrowRight", 0, 4, "vertical")).toBeNull();
    expect(nextRovingIndex("Enter", 0, 4)).toBeNull();
  });

  it("moves DOM focus after the current update", async () => {
    const focus = vi.fn();
    focusRovingItem([{ focus } as unknown as HTMLElement], 0);
    await Promise.resolve();
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
  });
});
