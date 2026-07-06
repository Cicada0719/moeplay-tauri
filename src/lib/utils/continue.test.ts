import { describe, expect, it } from "vitest";
import {
  aggregateTodaySeconds,
  aggregateWeekSeconds,
  buildAnimeContinueItem,
  buildComicContinueItem,
  buildContinueItems,
  buildContinueStats,
  buildGameContinueItem,
  computeGameProgress,
  computeStreakDays,
  gameActionLabel,
  gameSubtitle,
  priorityScore,
  type AnimeHistoryLike,
  type ComicHistoryLike,
  type PlaySession,
} from "./continue";
import type { GameLike } from "./game";

const now = new Date("2026-07-05T14:30:00.000Z");

function session(opts: Partial<PlaySession> & { duration_seconds: number; start_time: string }): PlaySession {
  return opts;
}

describe("continue utils", () => {
  describe("computeGameProgress", () => {
    it("uses achievements when available", () => {
      expect(
        computeGameProgress({
          play_tracker: { achievements_total: 10, achievements_unlocked: 7 },
        } as GameLike)
      ).toBe(70);
    });

    it("maps completion status when no achievements", () => {
      expect(computeGameProgress({ play_tracker: { completion_status: "not_started" } } as GameLike)).toBe(0);
      expect(computeGameProgress({ play_tracker: { completion_status: "playing" } } as GameLike)).toBe(15);
      expect(computeGameProgress({ play_tracker: { completion_status: "on_hold" } } as GameLike)).toBe(30);
      expect(computeGameProgress({ play_tracker: { completion_status: "replaying" } } as GameLike)).toBe(50);
      expect(computeGameProgress({ play_tracker: { completion_status: "completed" } } as GameLike)).toBe(100);
      expect(computeGameProgress({ play_tracker: { completion_status: "dropped" } } as GameLike)).toBe(100);
    });

    it("clamps achievement progress to 0-100", () => {
      expect(
        computeGameProgress({ play_tracker: { achievements_total: 10, achievements_unlocked: 12 } } as GameLike)
      ).toBe(100);
      expect(
        computeGameProgress({ play_tracker: { achievements_total: 10, achievements_unlocked: -1 } } as GameLike)
      ).toBe(0);
    });
  });

  describe("gameSubtitle and gameActionLabel", () => {
    it("prefers developer as subtitle", () => {
      expect(gameSubtitle({ developer: "Key", platform: "pc" } as GameLike)).toBe("Key");
    });

    it("falls back to platform", () => {
      expect(gameSubtitle({ platform: "Steam" } as GameLike)).toBe("Steam");
    });

    it("returns action label by status", () => {
      expect(gameActionLabel("not_started")).toBe("开始游玩");
      expect(gameActionLabel("playing")).toBe("继续游玩");
      expect(gameActionLabel("completed")).toBe("已通关");
      expect(gameActionLabel("dropped")).toBe("已弃坑");
      expect(gameActionLabel("unknown")).toBe("打开");
    });
  });

  describe("aggregateTodaySeconds", () => {
    it("sums sessions from today", () => {
      const sessions = [
        session({ duration_seconds: 3600, start_time: "2026-07-05T10:00:00.000Z" }),
        session({ duration_seconds: 1800, start_time: "2026-07-05T12:00:00.000Z" }),
      ];
      expect(aggregateTodaySeconds(sessions, now)).toBe(5400);
    });

    it("ignores sessions from other days", () => {
      const sessions = [
        session({ duration_seconds: 3600, start_time: "2026-07-04T10:00:00.000Z" }),
        session({ duration_seconds: 1800, start_time: "2026-07-05T12:00:00.000Z" }),
      ];
      expect(aggregateTodaySeconds(sessions, now)).toBe(1800);
    });

    it("ignores invalid durations", () => {
      const sessions = [
        session({ duration_seconds: 0, start_time: "2026-07-05T10:00:00.000Z" }),
        session({ duration_seconds: -10, start_time: "2026-07-05T11:00:00.000Z" }),
        session({ duration_seconds: 1200, start_time: "2026-07-05T12:00:00.000Z" }),
      ];
      expect(aggregateTodaySeconds(sessions, now)).toBe(1200);
    });
  });

  describe("aggregateWeekSeconds", () => {
    it("sums sessions within the last 7 days", () => {
      const sessions = [
        session({ duration_seconds: 3600, start_time: "2026-07-05T10:00:00.000Z" }),
        session({ duration_seconds: 1800, start_time: "2026-06-30T10:00:00.000Z" }),
        session({ duration_seconds: 600, start_time: "2026-06-28T10:00:00.000Z" }),
      ];
      expect(aggregateWeekSeconds(sessions, now)).toBe(5400);
    });
  });

  describe("computeStreakDays", () => {
    it("counts consecutive days including today", () => {
      const ts = [
        new Date("2026-07-05T10:00:00.000Z").getTime(),
        new Date("2026-07-04T22:00:00.000Z").getTime(),
        new Date("2026-07-03T08:00:00.000Z").getTime(),
      ];
      expect(computeStreakDays(ts, now)).toBe(3);
    });

    it("continues from yesterday when today has no activity", () => {
      const ts = [
        new Date("2026-07-04T22:00:00.000Z").getTime(),
        new Date("2026-07-03T08:00:00.000Z").getTime(),
      ];
      expect(computeStreakDays(ts, now)).toBe(2);
    });

    it("breaks streak on gap", () => {
      const ts = [
        new Date("2026-07-05T10:00:00.000Z").getTime(),
        new Date("2026-07-03T08:00:00.000Z").getTime(),
      ];
      expect(computeStreakDays(ts, now)).toBe(1);
    });

    it("returns 0 for empty input", () => {
      expect(computeStreakDays([], now)).toBe(0);
    });
  });

  describe("item builders", () => {
    it("builds game item with progress and labels", () => {
      const item = buildGameContinueItem({
        id: "g1",
        name: "Test Game",
        cover: "cover.jpg",
        developer: "DevCo",
        play_tracker: {
          total_seconds: 7200,
          completion_status: "playing",
          achievements_total: 10,
          achievements_unlocked: 3,
          last_played: "2026-07-05T08:00:00.000Z",
        },
      } as GameLike);
      expect(item.id).toBe("game-g1");
      expect(item.type).toBe("game");
      expect(item.title).toBe("Test Game");
      expect(item.progress).toBe(30);
      expect(item.progressLabel).toBe("2.0h");
      expect(item.subtitle).toBe("DevCo");
      expect(item.actionLabel).toBe("继续游玩");
    });

    it("builds anime item", () => {
      const h: AnimeHistoryLike = {
        key: "k1",
        name: "Anime A",
        image: "img.jpg",
        ruleName: "规则A",
        sourceUrl: "http://a",
        lastRoad: 0,
        lastEpisode: 5,
        lastEpisodeName: "ep5",
        progressMs: 120000,
        updatedAt: "2026-07-05T10:00:00.000Z",
      };
      const item = buildAnimeContinueItem(h);
      expect(item.id).toBe("anime-k1");
      expect(item.progressLabel).toBe("第5话");
      expect(item.actionLabel).toBe("继续观看");
    });

    it("builds comic item", () => {
      const h: ComicHistoryLike = {
        id: "c1",
        title: "Comic B",
        thumb_url: "thumb.jpg",
        author: "Author",
        last_order: 12,
        last_title: "第12话",
        ts: new Date("2026-07-05T10:00:00.000Z").getTime(),
      };
      const item = buildComicContinueItem(h);
      expect(item.id).toBe("comic-c1");
      expect(item.progressLabel).toBe("第12话");
      expect(item.actionLabel).toBe("继续阅读");
    });
  });

  describe("buildContinueItems", () => {
    it("filters completed and dropped games", () => {
      const items = buildContinueItems(
        [
          { id: "1", name: "A", play_tracker: { completion_status: "completed", last_played: "2026-07-05T10:00:00.000Z" } } as GameLike,
          { id: "2", name: "B", play_tracker: { completion_status: "dropped", last_played: "2026-07-05T10:00:00.000Z" } } as GameLike,
          { id: "3", name: "C", play_tracker: { completion_status: "playing", last_played: "2026-07-05T10:00:00.000Z" } } as GameLike,
        ],
        [],
        []
      );
      expect(items).toHaveLength(1);
      expect(items[0].title).toBe("C");
    });

    it("filters games without last_played", () => {
      const items = buildContinueItems(
        [{ id: "1", name: "A", play_tracker: { completion_status: "playing" } } as GameLike],
        [],
        []
      );
      expect(items).toHaveLength(0);
    });

    it("filters anime without episodes", () => {
      const items = buildContinueItems(
        [],
        [
          { key: "a1", name: "A", lastEpisode: 0, updatedAt: "2026-07-05T10:00:00.000Z" } as AnimeHistoryLike,
          { key: "a2", name: "B", lastEpisode: 3, updatedAt: "2026-07-05T10:00:00.000Z" } as AnimeHistoryLike,
        ],
        []
      );
      expect(items).toHaveLength(1);
      expect(items[0].title).toBe("B");
    });

    it("limits to 30 items", () => {
      const games = Array.from({ length: 40 }, (_, i) => ({
        id: `g${i}`,
        name: `Game ${i}`,
        play_tracker: {
          completion_status: "playing",
          last_played: `2026-07-0${(i % 9) + 1}T10:00:00.000Z`,
        },
      } as GameLike));
      const items = buildContinueItems(games, [], []);
      expect(items).toHaveLength(30);
    });
  });

  describe("priorityScore", () => {
    it("ranks mid-progress items higher", () => {
      const base = { id: "x", type: "game" as const, title: "X", cover: null, lastActivity: 1e12, progressLabel: "" };
      const mid = { ...base, progress: 50 };
      const full = { ...base, progress: 100 };
      expect(priorityScore(mid)).toBeGreaterThan(priorityScore(full));
    });

    it("prefers newer activity", () => {
      const a = { id: "a", type: "game" as const, title: "A", cover: null, lastActivity: 1e12, progress: 0, progressLabel: "" };
      const b = { id: "b", type: "game" as const, title: "B", cover: null, lastActivity: 1e12 + 86400000, progress: 0, progressLabel: "" };
      expect(priorityScore(b)).toBeGreaterThan(priorityScore(a));
    });
  });

  describe("buildContinueStats", () => {
    it("aggregates counts and minutes", () => {
      const games = [
        {
          id: "g1",
          play_tracker: {
            completion_status: "playing",
            sessions: [session({ duration_seconds: 3600, start_time: "2026-07-05T10:00:00.000Z" })],
          },
        } as GameLike,
      ];
      const anime: AnimeHistoryLike[] = [
        {
          key: "a1",
          name: "Anime",
          lastEpisode: 2,
          updatedAt: "2026-07-05T10:00:00.000Z",
        } as AnimeHistoryLike,
      ];
      const comics: ComicHistoryLike[] = [
        {
          id: "c1",
          title: "Comic",
          last_order: 3,
          ts: new Date("2026-07-05T10:00:00.000Z").getTime(),
        } as ComicHistoryLike,
      ];
      const items = buildContinueItems(games, anime, comics);
      const stats = buildContinueStats(items, games, anime, comics, now);
      expect(stats.totalCount).toBe(3);
      expect(stats.gameCount).toBe(1);
      expect(stats.animeCount).toBe(1);
      expect(stats.comicCount).toBe(1);
      expect(stats.todayMinutes).toBeGreaterThan(0);
      expect(stats.weekMinutes).toBeGreaterThan(0);
      expect(stats.streakDays).toBeGreaterThan(0);
    });
  });
});
