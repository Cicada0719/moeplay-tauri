import { describe, expect, it } from "vitest";
import {
  filterYearSessions,
  monthlyHeat,
  newGamesTimeline,
  summarizeYear,
  topAchievements,
  topCompletions,
  topPlayed,
  type ReplayGameLike,
} from "./aggregate";

/** 用无时区后缀的 ISO 串：按本地时区解析，跨 TZ 结果确定。 */
function makeGame(
  id: string,
  overrides: Partial<ReplayGameLike> & { tracker?: Partial<NonNullable<ReplayGameLike["play_tracker"]>> } = {},
): ReplayGameLike {
  const { tracker, ...rest } = overrides;
  return {
    id,
    name: id,
    ...rest,
    play_tracker: { sessions: [], ...tracker },
  };
}

function session(id: string, start_time: string, duration_seconds = 3600) {
  return { id, start_time, duration_seconds };
}

describe("filterYearSessions", () => {
  it("keeps only sessions whose start_time falls in the target year", () => {
    const games = [
      makeGame("a", {
        tracker: {
          sessions: [
            session("s1", "2024-01-15T10:00:00"),
            session("s2", "2024-12-31T23:30:00"),
            session("s3", "2023-06-01T12:00:00"),
            session("s4", "2025-01-01T00:10:00"),
          ],
        },
      }),
    ];
    const entries = filterYearSessions(games, 2024);
    expect(entries.map((e) => e.session.id)).toEqual(["s1", "s2"]);
    expect(entries[0].game.id).toBe("a");
    expect(entries[0].start.getFullYear()).toBe(2024);
  });

  it("attributes a cross-year session to the year of its start_time", () => {
    const games = [
      makeGame("a", {
        tracker: {
          sessions: [
            { id: "nye", start_time: "2023-12-31T23:00:00", end_time: "2024-01-01T01:00:00", duration_seconds: 7200 },
          ],
        },
      }),
    ];
    expect(filterYearSessions(games, 2023)).toHaveLength(1);
    expect(filterYearSessions(games, 2024)).toHaveLength(0);
  });

  it("skips sessions with unparseable start_time and tolerates missing tracker data", () => {
    const games = [
      makeGame("a", { tracker: { sessions: [session("bad", "not-a-date"), session("ok", "2024-05-01T08:00:00")] } }),
      makeGame("b"),
      { id: "c", name: "c", play_tracker: null },
      { id: "d", name: "d" },
    ];
    const entries = filterYearSessions(games, 2024);
    expect(entries.map((e) => e.session.id)).toEqual(["ok"]);
    expect(filterYearSessions([], 2024)).toEqual([]);
  });
});

describe("summarizeYear", () => {
  it("sums durations and counts distinct local days, games and sessions", () => {
    const games = [
      makeGame("a", {
        tracker: {
          sessions: [
            session("s1", "2024-03-05T10:00:00", 3600),
            session("s2", "2024-03-05T20:00:00", 1800),
            session("s3", "2024-03-07T09:00:00", 1800),
          ],
        },
      }),
      makeGame("b", { tracker: { sessions: [session("s4", "2024-03-07T21:00:00", 7200)] } }),
    ];
    const summary = summarizeYear(filterYearSessions(games, 2024));
    expect(summary.totalSeconds).toBe(14400);
    expect(summary.playDays).toBe(2);
    expect(summary.sessionCount).toBe(4);
    expect(summary.gameCount).toBe(2);
    expect(summary.hours).toBe(4);
  });

  it("clamps negative/NaN durations to zero", () => {
    const games = [
      makeGame("a", {
        tracker: {
          sessions: [
            session("neg", "2024-01-01T10:00:00", -300),
            { id: "nan", start_time: "2024-01-01T11:00:00", duration_seconds: Number.NaN },
            session("ok", "2024-01-01T12:00:00", 600),
          ],
        },
      }),
    ];
    expect(summarizeYear(filterYearSessions(games, 2024)).totalSeconds).toBe(600);
  });

  it("returns zeros for an empty year", () => {
    expect(summarizeYear([])).toEqual({
      totalSeconds: 0,
      playDays: 0,
      sessionCount: 0,
      gameCount: 0,
      hours: 0,
    });
  });
});

describe("topPlayed", () => {
  const games = [
    makeGame("alpha", { tracker: { sessions: [session("s1", "2024-02-01T10:00:00", 1000), session("s2", "2024-03-01T10:00:00", 2000)] } }),
    makeGame("beta", { tracker: { sessions: [session("s3", "2024-02-02T10:00:00", 5000)] } }),
    makeGame("gamma", { tracker: { sessions: [session("s4", "2024-02-03T10:00:00", 5000)] } }),
    makeGame("delta", { tracker: { sessions: [session("s5", "2024-02-04T10:00:00", 500)] } }),
    makeGame("epsilon", { tracker: { sessions: [session("s6", "2024-02-05T10:00:00", 100)] } }),
    makeGame("zeta", { tracker: { sessions: [session("s7", "2024-02-06T10:00:00", 50)] } }),
  ];

  it("aggregates per game, sorts by seconds desc, breaks ties by name", () => {
    const top = topPlayed(filterYearSessions(games, 2024), 10);
    expect(top.map((t) => t.game.id)).toEqual(["beta", "gamma", "alpha", "delta", "epsilon", "zeta"]);
    expect(top.find((t) => t.game.id === "alpha")).toMatchObject({ seconds: 3000, sessions: 2 });
  });

  it("truncates to the limit", () => {
    expect(topPlayed(filterYearSessions(games, 2024), 5)).toHaveLength(5);
    expect(topPlayed(filterYearSessions(games, 2024), 1)[0].game.id).toBe("beta");
    expect(topPlayed(filterYearSessions(games, 2024), 0)).toEqual([]);
  });
});

describe("monthlyHeat", () => {
  it("buckets seconds per month and normalizes against the yearly peak", () => {
    const games = [
      makeGame("a", {
        tracker: {
          sessions: [
            session("jan", "2024-01-10T10:00:00", 3600),
            session("jun1", "2024-06-10T10:00:00", 3600),
            session("jun2", "2024-06-20T10:00:00", 3600),
          ],
        },
      }),
    ];
    const heat = monthlyHeat(filterYearSessions(games, 2024));
    expect(heat).toHaveLength(12);
    expect(heat[0]).toEqual({ month: 0, seconds: 3600, ratio: 0.5 });
    expect(heat[5]).toEqual({ month: 5, seconds: 7200, ratio: 1 });
    expect(heat[1]).toEqual({ month: 1, seconds: 0, ratio: 0 });
  });

  it("returns all-zero ratios when there is no data", () => {
    const heat = monthlyHeat([]);
    expect(heat).toHaveLength(12);
    expect(heat.every((h) => h.seconds === 0 && h.ratio === 0)).toBe(true);
  });
});

describe("topAchievements", () => {
  it("keeps only games with achievements_total > 0 and sorts by unlock ratio", () => {
    const games = [
      makeGame("none", { tracker: { achievements_total: 0, achievements_unlocked: 0 } }),
      makeGame("half", { tracker: { achievements_total: 10, achievements_unlocked: 5 } }),
      makeGame("full", { tracker: { achievements_total: 20, achievements_unlocked: 20 } }),
      makeGame("most", { tracker: { achievements_total: 40, achievements_unlocked: 30 } }),
    ];
    const top = topAchievements(games, 3);
    expect(top.map((t) => t.game.id)).toEqual(["full", "most", "half"]);
    expect(top[0].ratio).toBe(1);
    expect(top[1].ratio).toBe(0.75);
  });

  it("breaks ratio ties by unlocked count and clamps unlocked to [0, total]", () => {
    const games = [
      makeGame("small", { tracker: { achievements_total: 10, achievements_unlocked: 10 } }),
      makeGame("big", { tracker: { achievements_total: 30, achievements_unlocked: 30 } }),
      makeGame("over", { tracker: { achievements_total: 10, achievements_unlocked: 99 } }),
    ];
    const top = topAchievements(games);
    expect(top.map((t) => t.game.id)).toEqual(["big", "small", "over"]);
    expect(top[2].ratio).toBe(1);
    expect(topAchievements(games, 2)).toHaveLength(2);
  });
});

describe("topCompletions", () => {
  it("keeps only completion_count > 0, sorts desc, truncates", () => {
    const games = [
      makeGame("zero", { tracker: { completion_count: 0 } }),
      makeGame("once", { tracker: { completion_count: 1 } }),
      makeGame("thrice", { tracker: { completion_count: 3 } }),
      makeGame("twice", { tracker: { completion_count: 2 } }),
      makeGame("four", { tracker: { completion_count: 4 } }),
    ];
    const top = topCompletions(games, 3);
    expect(top.map((t) => t.game.id)).toEqual(["four", "thrice", "twice"]);
    expect(top[0].count).toBe(4);
  });
});

describe("newGamesTimeline", () => {
  it("buckets games by first_played month within the year", () => {
    const games = [
      makeGame("jan", { tracker: { first_played: "2024-01-20T10:00:00" } }),
      makeGame("jun-a", { tracker: { first_played: "2024-06-01T10:00:00" } }),
      makeGame("jun-b", { tracker: { first_played: "2024-06-15T10:00:00" } }),
      makeGame("old", { tracker: { first_played: "2022-03-01T10:00:00" } }),
    ];
    const timeline = newGamesTimeline(games, 2024);
    expect(timeline).toHaveLength(12);
    expect(timeline[0].games.map((g) => g.id)).toEqual(["jan"]);
    expect(timeline[5].games.map((g) => g.id)).toEqual(["jun-a", "jun-b"]);
    expect(timeline[11].games).toEqual([]);
  });

  it("falls back to add_date when first_played is missing, skips invalid dates", () => {
    const games = [
      makeGame("added", { add_date: "2024-09-10T12:00:00" }),
      makeGame("broken", { add_date: "garbage" }),
      makeGame("none"),
      makeGame("first-wins", { add_date: "2024-09-01T00:00:00", tracker: { first_played: "2024-02-01T00:00:00" } }),
    ];
    const timeline = newGamesTimeline(games, 2024);
    expect(timeline[8].games.map((g) => g.id)).toEqual(["added"]);
    expect(timeline[1].games.map((g) => g.id)).toEqual(["first-wins"]);
    expect(timeline.flatMap((m) => m.games.map((g) => g.id))).not.toContain("broken");
    expect(timeline.flatMap((m) => m.games.map((g) => g.id))).not.toContain("none");
  });
});
