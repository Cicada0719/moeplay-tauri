import { describe, expect, it } from "vitest";
import {
  buildCompletionDoughnutData,
  buildMonthlyTrendData,
  buildStatusDistributionData,
  formatMonthLabel,
} from "./chart";

describe("chart utils", () => {
  describe("formatMonthLabel", () => {
    it("formats YYYY-MM to M月", () => {
      expect(formatMonthLabel("2026-07")).toBe("7月");
      expect(formatMonthLabel("2026-12")).toBe("12月");
      expect(formatMonthLabel("01")).toBe("1月");
    });
  });

  describe("buildMonthlyTrendData", () => {
    it("uses last 12 items and maps hours", () => {
      const items = Array.from({ length: 15 }, (_, i) => ({
        month: `2026-${String((i % 12) + 1).padStart(2, "0")}`,
        hours: i + 1,
      }));
      const data = buildMonthlyTrendData(items);
      expect(data.labels).toHaveLength(12);
      expect(data.datasets[0].data).toEqual(items.slice(-12).map((i) => i.hours));
    });

    it("handles empty input", () => {
      const data = buildMonthlyTrendData([]);
      expect(data.labels).toHaveLength(0);
      expect(data.datasets[0].data).toHaveLength(0);
    });
  });

  describe("buildStatusDistributionData", () => {
    it("filters zero counts and sorts descending", () => {
      const distribution = [
        { status: "playing", count: 3 },
        { status: "completed", count: 10 },
        { status: "dropped", count: 0 },
      ];
      const labels: Record<string, string> = {
        playing: "游玩中",
        completed: "已通关",
        dropped: "已弃坑",
      };
      const data = buildStatusDistributionData(distribution, labels);
      expect(data.labels).toEqual(["已通关", "游玩中"]);
      expect(data.datasets[0].data).toEqual([10, 3]);
    });
  });

  describe("buildCompletionDoughnutData", () => {
    it("clamps rate to 0-100", () => {
      expect(buildCompletionDoughnutData(-10).datasets[0].data).toEqual([0, 100]);
      expect(buildCompletionDoughnutData(50).datasets[0].data).toEqual([50, 50]);
      expect(buildCompletionDoughnutData(110).datasets[0].data).toEqual([100, 0]);
    });
  });
});
