import { afterEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "./core";
import {
  getDashboardData,
  parseDashboardData,
  toCollectionCountItems,
  toCountItems,
  toStatusCountItems,
  type DashboardData,
} from "./dashboard";

const dashboardFixture = {
  total_games: 3,
  installed_games: 2,
  completed_games: 1,
  playtime_hours: 12.5,
  completion_rate: 33.3,
  scrape_coverage: 66.7,
  disk_usage_gb: 4.2,
  recent_games: ["月影", "星海"],
  top_tags: [
    ["剧情", 2],
    ["科幻", 1],
  ],
  completion_distribution: [
    ["Completed", 1],
    ["on-hold", 2],
  ],
  monthly_heatmap: [
    { month: "2026-06", sessions: 1, hours: 0.5 },
    { month: "2026-07", sessions: 2, hours: 1.5 },
  ],
  collections: [
    {
      id: "completed",
      name: "已通关",
      description: "已通关的游戏",
      game_count: 1,
      icon: "check",
    },
  ],
} satisfies DashboardData;

afterEach(() => clearMockInvokeHandler());

describe("dashboard DTO contract", () => {
  it("parses the Rust JSON shape and preserves the real field names", () => {
    const parsed = parseDashboardData(JSON.parse(JSON.stringify(dashboardFixture)));

    expect(parsed).toEqual(dashboardFixture);
    expect(parsed.playtime_hours).toBe(12.5);
    expect("total_playtime_hours" in parsed).toBe(false);
    expect(parsed.completion_distribution).toEqual(dashboardFixture.completion_distribution);
  });

  it("rejects the drifted StatsPage shape", () => {
    expect(() =>
      parseDashboardData({
        ...dashboardFixture,
        playtime_hours: undefined,
        total_playtime_hours: 12.5,
      })
    ).toThrow("DashboardData.playtime_hours");
  });

  it("loads through the dashboard API boundary", async () => {
    setMockInvokeHandler((command) => {
      expect(command).toBe("get_dashboard_data");
      return dashboardFixture;
    });

    await expect(getDashboardData()).resolves.toEqual(dashboardFixture);
  });

  it("transforms tuple and collection fields for existing charts and cards", () => {
    expect(toCountItems(dashboardFixture.top_tags)).toEqual([
      { name: "剧情", count: 2 },
      { name: "科幻", count: 1 },
    ]);
    expect(toStatusCountItems(dashboardFixture.completion_distribution)).toEqual([
      { status: "completed", count: 1 },
      { status: "on_hold", count: 2 },
    ]);
    expect(toCollectionCountItems(dashboardFixture.collections)).toEqual([
      { id: "completed", name: "已通关", count: 1 },
    ]);
  });
});
