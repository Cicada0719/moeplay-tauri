import { describe, expect, it } from "vitest";
import {
  adjacentIndex,
  adjacentItem,
  createWheelStepper,
  normalizeWheelDelta,
  shouldCaptureStageInput,
} from "./visualStepNavigation";

describe("Visual step navigation", () => {
  it("accumulates vertical movement before emitting one step", () => {
    const stepper = createWheelStepper({ threshold: 60 });

    expect(stepper.push({ deltaX: 0, deltaY: 20, time: 0 })).toBeNull();
    expect(stepper.push({ deltaX: 1, deltaY: 25, time: 20 })).toBeNull();
    expect(stepper.push({ deltaX: 0, deltaY: 16, time: 40 })).toBe(1);
  });

  it("emits at most one step for a continuous trackpad gesture", () => {
    const stepper = createWheelStepper({ threshold: 50, gestureIdleMs: 180 });

    expect(stepper.push({ deltaX: 0, deltaY: 55, time: 0 })).toBe(1);
    expect(stepper.push({ deltaX: 0, deltaY: 90, time: 100 })).toBeNull();
    expect(stepper.push({ deltaX: 0, deltaY: 90, time: 180 })).toBeNull();
  });

  it("requires an idle boundary and the cooldown before another step", () => {
    const stepper = createWheelStepper({ threshold: 40, cooldownMs: 450, gestureIdleMs: 150 });

    expect(stepper.push({ deltaX: 0, deltaY: 45, time: 0 })).toBe(1);
    expect(stepper.push({ deltaX: 0, deltaY: 45, time: 200 })).toBeNull();
    expect(stepper.push({ deltaX: 0, deltaY: 45, time: 700 })).toBe(1);
  });

  it("locks to vertical intent and resets accumulation on direction reversal", () => {
    const stepper = createWheelStepper({ threshold: 50, axisLockRatio: 1.15 });

    expect(stepper.push({ deltaX: 40, deltaY: 30, time: 0 })).toBeNull();
    expect(stepper.push({ deltaX: 0, deltaY: 35, time: 20 })).toBeNull();
    expect(stepper.push({ deltaX: 0, deltaY: -20, time: 40 })).toBeNull();
    expect(stepper.push({ deltaX: 0, deltaY: -31, time: 60 })).toBe(-1);
  });

  it("wraps adjacent selection at both ends", () => {
    const items = [{ id: "a" }, { id: "b" }, { id: "c" }];

    expect(adjacentIndex(3, 2, 1)).toBe(0);
    expect(adjacentIndex(3, 0, -1)).toBe(2);
    expect(adjacentItem(items, "c", 1)?.id).toBe("a");
    expect(adjacentItem(items, "a", -1)?.id).toBe("c");
    expect(adjacentItem(items, null, 1)?.id).toBe("b");
    expect(adjacentItem([], null, 1)).toBeNull();
  });

  it("only captures non-interactive, non-scrollable stage targets", () => {
    expect(shouldCaptureStageInput({ isInteractiveTarget: false, isScrollableSubregion: false })).toBe(true);
    expect(shouldCaptureStageInput({ isInteractiveTarget: true, isScrollableSubregion: false })).toBe(false);
    expect(shouldCaptureStageInput({ isInteractiveTarget: false, isScrollableSubregion: true })).toBe(false);
  });

  it("normalizes line and page wheel deltas", () => {
    expect(normalizeWheelDelta(2, 0, 900)).toBe(2);
    expect(normalizeWheelDelta(2, 1, 900)).toBe(32);
    expect(normalizeWheelDelta(2, 2, 900)).toBe(1800);
  });
});
