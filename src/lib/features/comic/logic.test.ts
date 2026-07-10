import { describe, expect, it } from "vitest";
import { RequestGeneration, computePrefetchWindow, pageRetryDelayMs, planPageRetry } from "./logic";

describe("comic provider pure scheduling logic", () => {
  it("ignores stale generations", () => {
    const generation = new RequestGeneration();
    const first = generation.bump();
    const second = generation.bump();
    expect(generation.isCurrent(first)).toBe(false);
    expect(generation.isCurrent(second)).toBe(true);
  });

  it("plans bounded page retries without retrying non-retryable failures", () => {
    expect(planPageRetry(4, 0, 3, true)).toEqual({ page: 4, attempt: 1, maxAttempts: 3, retryable: true });
    expect(planPageRetry(4, 2, 3, true)).toBeUndefined();
    expect(planPageRetry(4, 0, 3, false)).toBeUndefined();
    expect(pageRetryDelayMs(3, 250, 1_000)).toBe(1_000);
  });

  it("computes a clamped neighbor-only prefetch window", () => {
    expect(computePrefetchWindow(0, 5, 2)).toEqual([1, 2]);
    expect(computePrefetchWindow(2, 5, 2)).toEqual([0, 1, 3, 4]);
    expect(computePrefetchWindow(99, 3, 2)).toEqual([0, 1]);
    expect(computePrefetchWindow(0, 0, 2)).toEqual([]);
  });
});
