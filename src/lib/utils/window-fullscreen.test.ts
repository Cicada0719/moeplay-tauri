import { describe, expect, it } from "vitest";
import { boundsMatchMonitor } from "./window-fullscreen";

describe("window fullscreen geometry", () => {
  const monitor = { position: { x: 0, y: 0 }, size: { width: 1920, height: 1080 } };
  it("accepts native fullscreen bounds with small Windows rounding", () => {
    expect(boundsMatchMonitor({ x: -1, y: 0 }, { width: 1921, height: 1080 }, monitor)).toBe(true);
  });
  it("rejects a decorated window even when the native fullscreen flag is stale", () => {
    expect(boundsMatchMonitor({ x: 120, y: 80 }, { width: 1200, height: 800 }, monitor)).toBe(false);
  });
});
