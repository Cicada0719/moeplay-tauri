import { describe, expect, it } from "vitest";
import {
  coverOf,
  developerOf,
  gameCompletionStatus,
  gameLastPlayed,
  gameRating,
  gameTotalSeconds,
  heroImageOf,
  normalizeCompletionStatus,
  normalizeGame,
  originalNameOf,
  screenshotsOf,
  tagsOf,
  userFacingErrorMessage,
} from "./game";

describe("game utils", () => {
  it("normalizes backend completion status variants", () => {
    expect(normalizeCompletionStatus("NotStarted")).toBe("not_started");
    expect(normalizeCompletionStatus("PlanToPlay")).toBe("plan_to_play");
    expect(normalizeCompletionStatus("on-hold")).toBe("on_hold");
    expect(normalizeCompletionStatus(" playing ")).toBe("playing");
    expect(normalizeCompletionStatus(null)).toBe("not_started");
  });

  it("reads completion status from a game-like object", () => {
    expect(gameCompletionStatus({ play_tracker: { completion_status: "Completed" as never } })).toBe("completed");
    expect(gameCompletionStatus({})).toBe("not_started");
  });

  it("prefers tracker fields while preserving legacy fallbacks", () => {
    expect(gameTotalSeconds({ play_tracker: { total_seconds: 3600 }, play_time_seconds: 20 })).toBe(3600);
    expect(gameTotalSeconds({ play_time_seconds: 20 })).toBe(20);
    expect(gameLastPlayed({ play_tracker: { last_played: "2026-01-02" }, last_played: "2025-01-01" })).toBe("2026-01-02");
    expect(gameLastPlayed({ last_played: "2025-01-01" })).toBe("2025-01-01");
  });

  it("projects legacy fields into canonical metadata and play tracker", () => {
    const normalized = normalizeGame({
      developer: "Legacy Dev",
      publisher: "Legacy Pub",
      platform: "steam",
      genres: ["Adventure"],
      cover: "legacy-cover",
      background: "legacy-bg",
      rating: 8.1,
      release_year: 2024,
      last_played: "2026-01-02",
      play_time_seconds: 7200,
      metadata: {
        developer: "Canonical Dev",
        genres: [],
      },
      play_tracker: {
        completion_status: "PlanToPlay" as never,
      },
    });

    expect(normalized.metadata.developer).toBe("Canonical Dev");
    expect(normalized.metadata.publisher).toBe("Legacy Pub");
    expect(normalized.metadata.platform).toBe("pc");
    expect(normalized.metadata.genres).toEqual(["Adventure"]);
    expect(normalized.metadata.cover).toBe("legacy-cover");
    expect(normalized.metadata.background).toBe("legacy-bg");
    expect(normalized.metadata.release_year).toBe(2024);
    expect(normalized.metadata.vndb_rating).toBe(8.1);
    expect(normalized.play_tracker.total_seconds).toBe(7200);
    expect(normalized.play_tracker.last_played).toBe("2026-01-02");
    expect(normalized.play_tracker.completion_status).toBe("plan_to_play");
    expect(normalized.play_tracker.achievements_total).toBe(0);
  });

  it("prefers user rating before metadata ratings", () => {
    expect(gameRating({ play_tracker: { user_rating: 8.5 }, metadata: { vndb_rating: 7 } })).toBe(8.5);
    expect(gameRating({ metadata: { bangumi_rating: 6.7 }, rating: 5 })).toBe(6.7);
    expect(gameRating({ rating: 5 })).toBe(5);
  });

  it("resolves art and text from metadata with legacy fallbacks", () => {
    expect(coverOf({ metadata: { cover: "metadata-cover" }, cover: "legacy-cover" })).toBe("metadata-cover");
    expect(coverOf({ cover: "legacy-cover" })).toBe("legacy-cover");
    expect(heroImageOf({ metadata: { background: "metadata-bg" }, cover: "cover" })).toBe("metadata-bg");
    expect(screenshotsOf({ background: "bg", metadata: { cover: "cover" } })).toEqual(["bg", "cover"]);
    expect(developerOf({ metadata: { publisher: "Publisher" } })).toBe("Publisher");
    expect(originalNameOf({ aliases: [{ name: "原名", language: "ja" }] })).toBe("原名");
  });

  it("merges top-level tags with metadata genres", () => {
    expect(tagsOf({ tags: ["Story"], metadata: { genres: ["Story", "Mystery"] } })).toEqual(["Story", "Mystery"]);
  });

  it("hides raw TypeError details from user-facing messages", () => {
    expect(userFacingErrorMessage(new Error("Steam API Key 无效"))).toBe("Steam API Key 无效");
    expect(userFacingErrorMessage("导入失败")).toBe("导入失败");
    expect(userFacingErrorMessage(new TypeError("Cannot read properties of undefined"))).toBe("操作失败，请稍后重试。");
  });
});
