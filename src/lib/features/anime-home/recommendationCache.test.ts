import { describe, expect, it, vi } from "vitest";
import {
  isRecommendationSnapshotFresh,
  parseRecommendationSnapshot,
  readRecommendationSnapshot,
  writeRecommendationSnapshot,
} from "./recommendationCache";

describe("anime recommendation cache", () => {
  it("normalizes totals and rejects malformed payloads", () => {
    expect(parseRecommendationSnapshot(null)).toBeNull();
    expect(parseRecommendationSnapshot({ version: 1, storedAt: 1, seasonal: [], trending: [] })).toBeNull();
    expect(parseRecommendationSnapshot({
      version: 1,
      storedAt: 10,
      seasonal: [{ id: 1 }],
      seasonalTotal: -1,
      trending: [],
      topRated: [],
    })).toMatchObject({ seasonalTotal: 1, trendingTotal: 0, topRatedTotal: 0 });
  });

  it("reads and writes without making storage failures fatal", () => {
    const setItem = vi.fn();
    const snapshot = { version: 1 as const, storedAt: 20, seasonal: [], seasonalTotal: 0, trending: [], trendingTotal: 0, topRated: [], topRatedTotal: 0 };
    writeRecommendationSnapshot({ setItem }, "key", snapshot);
    expect(setItem).toHaveBeenCalledWith("key", JSON.stringify(snapshot));
    expect(readRecommendationSnapshot({ getItem: () => JSON.stringify(snapshot) }, "key")).toEqual(snapshot);
    expect(readRecommendationSnapshot({ getItem: () => "{" }, "key")).toBeNull();
  });

  it("uses a six-hour stale-while-revalidate window by default", () => {
    const now = 10_000_000;
    expect(isRecommendationSnapshotFresh(now - 1_000, now)).toBe(true);
    expect(isRecommendationSnapshotFresh(now - 6 * 60 * 60 * 1000 - 1, now)).toBe(false);
  });
});
