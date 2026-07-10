import { afterEach, describe, expect, it, vi } from "vitest";
import { motionStore } from "./motion.svelte";
import { motionDuration } from "../utils/motion";

const originalMatchMedia = window.matchMedia;

afterEach(() => {
  motionStore.setPreference("system");
  delete document.documentElement.dataset.motion;
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: originalMatchMedia,
  });
});

describe("motionStore", () => {
  it("honors an explicit reduced-motion choice", () => {
    motionStore.setPreference("reduce");

    expect(motionStore.reduced).toBe(true);
    expect(motionDuration(0.2)).toBe(0);
    expect(document.documentElement.dataset.motion).toBe("reduce");
  });

  it("tracks system preference and removes its listener after cleanup", () => {
    let listener: ((event: MediaQueryListEvent) => void) | undefined;
    const addEventListener = vi.fn((_type: string, callback: (event: MediaQueryListEvent) => void) => {
      listener = callback;
    });
    const removeEventListener = vi.fn();

    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: vi.fn(() => ({
        matches: false,
        addEventListener,
        removeEventListener,
      })),
    });

    const cleanup = motionStore.initialize();
    expect(motionStore.reduced).toBe(false);

    listener?.({ matches: true } as MediaQueryListEvent);
    expect(motionStore.reduced).toBe(true);
    expect(document.documentElement.dataset.motion).toBe("reduce");

    cleanup();
    expect(removeEventListener).toHaveBeenCalledWith("change", listener);
  });
});
