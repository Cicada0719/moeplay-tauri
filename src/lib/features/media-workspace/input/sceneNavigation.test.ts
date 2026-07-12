import { describe, expect, it } from "vitest";
import {
  adjacentGameIndex,
  adjacentMediaIndex,
  createWheelGestureTracker,
  dragSceneStep,
  wrapSceneIndex,
} from "./sceneNavigation";

const entries = [
  { ownerItemId: "game-a" },
  { ownerItemId: "game-a" },
  { ownerItemId: "game-a" },
  { ownerItemId: "game-b" },
  { ownerItemId: "game-c" },
  { ownerItemId: "game-c" },
];

describe("scene navigation", () => {
  it("turns a trackpad gesture into exactly one step until the quiet window passes", () => {
    const tracker = createWheelGestureTracker(20, 180);
    expect(tracker.push(0, 8, 0)).toBe(0);
    expect(tracker.push(0, 13, 20)).toBe(1);
    expect(tracker.push(0, 90, 40)).toBe(0);
    expect(tracker.push(0, -30, 250)).toBe(-1);
  });

  it("moves horizontally within an owner and vertically between owners", () => {
    expect(adjacentMediaIndex(entries, 1, 1)).toBe(2);
    expect(adjacentMediaIndex(entries, 2, 1)).toBe(0);
    expect(adjacentGameIndex(entries, 1, 1)).toBe(3);
    expect(adjacentGameIndex(entries, 3, -1)).toBe(0);
    expect(adjacentGameIndex(entries, 4, 1)).toBe(0);
  });

  it("wraps the continuous stream and resolves drag intent with distance or velocity", () => {
    expect(wrapSceneIndex(-1, entries.length)).toBe(5);
    expect(wrapSceneIndex(6, entries.length)).toBe(0);
    expect(dragSceneStep(-90, -0.2, 1000)).toBe(1);
    expect(dragSceneStep(10, 0.6, 1000)).toBe(-1);
    expect(dragSceneStep(20, 0.1, 1000)).toBe(0);
  });
});
