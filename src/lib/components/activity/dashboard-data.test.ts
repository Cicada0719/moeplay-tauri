import { describe, expect, it } from "vitest";
import type { Game, PlaytimeSummary } from "../../api";
import { activityChartPoints, buildLocalSummary, buildMediaActivities, dailyChartPoints, fillActivityBars, fillDailyBars, summarizeChart, uniqueArchiveActivities } from "./dashboard-data";

function game(): Game {
  return {
    id: "game-1",
    name: "星海回声",
    exe_path: "C:/Games/echo.exe",
    created_at: "2026-07-01T00:00:00Z",
    updated_at: "2026-07-10T00:00:00Z",
    screenshots: [],
    favorite: false,
    hidden: false,
    tags: [],
    aliases: [],
    tag_entries: [],
    metadata: {},
    save_data: { paths: [], backups: [] },
    play_tracker: {
      total_seconds: 5400,
      sessions: [{ id: "session-1", start_time: "2026-07-10T10:00:00Z", end_time: "2026-07-10T11:30:00Z", duration_seconds: 5400, notes: "" }],
      completion_status: "playing",
      last_played: "2026-07-10T10:00:00Z",
      user_rating: 0,
    },
    play_time_seconds: 5400,
  } as unknown as Game;
}

describe("records dashboard model", () => {
  it("preserves the legacy summary fallback from local game sessions", () => {
    const summary = buildLocalSummary([game()]);
    expect(summary.total_seconds).toBe(5400);
    expect(summary.session_count).toBe(1);
    expect(summary.play_days).toBe(1);
    expect(summary.top_games[0]).toMatchObject({ game_id: "game-1", total_seconds: 5400 });
  });

  it("builds deterministic chart points and an equivalent text summary", () => {
    const summary: PlaytimeSummary = { total_seconds: 5400, session_count: 1, play_days: 1, average_session_seconds: 5400, daily: [{ date: "2026-07-10", seconds: 5400, sessions: 1 }], monthly: [], recent_sessions: [], top_games: [] };
    const bars = fillDailyBars(summary.daily, 2, new Date("2026-07-10T12:00:00Z"));
    const points = dailyChartPoints(bars);
    expect(points.map((point) => point.value)).toEqual([0, 5400]);
    expect(summarizeChart(points, "游玩时长")).toContain("峰值为 7/10 的 1.5h");
  });

  it("keeps only the newest archive row for repeated sessions of the same software", () => {
    const source = game();
    source.play_tracker.sessions = [
      { id: "session-old", start_time: "2026-07-09T10:00:00Z", end_time: "2026-07-09T11:00:00Z", duration_seconds: 3600, notes: "" },
      { id: "session-new", start_time: "2026-07-10T10:00:00Z", end_time: "2026-07-10T11:30:00Z", duration_seconds: 5400, notes: "" },
    ];
    const activities = buildMediaActivities(buildLocalSummary([source]).recent_sessions, [], [], [source]);
    const archive = uniqueArchiveActivities(activities);
    expect(activities).toHaveLength(2);
    expect(archive).toHaveLength(1);
    expect(archive[0].id).toContain("session-new");
  });

  it("keeps game/anime/comic activities in one ordered legacy fallback timeline", () => {
    const activities = buildMediaActivities(
      buildLocalSummary([game()]).recent_sessions,
      [{ key: "anime-1", name: "夏日动画", image: "https://example.test/anime.jpg", ruleName: "fixture", sourceUrl: "", lastRoad: 0, lastEpisode: 3, lastEpisodeName: "第 3 集", progressMs: 0, updatedAt: "2026-07-10T11:00:00Z" }],
      [{ id: "comic-1", title: "海边漫画", thumb_url: "https://example.test/comic.jpg", author: "作者", last_order: 8, last_title: "第 8 话", ts: new Date("2026-07-10T12:00:00Z").getTime() }],
      [game()],
    );
    expect(activities.map((item) => item.kind)).toEqual(["comic", "anime", "game"]);
    const activityBars = fillActivityBars(activities, 1, new Date("2026-07-10T13:00:00Z"));
    expect(activityChartPoints(activityBars)[0]).toMatchObject({ value: 3, valueLabel: "3 次" });
  });
});
