import { describe, expect, it, vi } from "vitest";
import { focusComicRovingItem, nextComicRovingIndex } from "./a11y";

describe("comic roving tab helpers", () => {
  it("wraps horizontal tabs and supports Home/End", () => {
    expect(nextComicRovingIndex("ArrowRight", 3, 4)).toBe(0);
    expect(nextComicRovingIndex("ArrowLeft", 0, 4)).toBe(3);
    expect(nextComicRovingIndex("Home", 2, 4)).toBe(0);
    expect(nextComicRovingIndex("End", 1, 4)).toBe(3);
    expect(nextComicRovingIndex("ArrowDown", 1, 4)).toBeNull();
  });

  it("moves DOM focus after state settles", async () => {
    const focus = vi.fn();
    focusComicRovingItem([{ focus } as unknown as HTMLElement], 0);
    await Promise.resolve();
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
  });
});
