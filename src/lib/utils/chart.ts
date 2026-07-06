import type { ChartData, ChartOptions } from "chart.js";

export interface MonthlyHeatmapItem {
  month: string;
  hours: number;
}

export interface StatusDistributionItem {
  status: string;
  count: number;
}

const ACCENT = "#E8557F";
const ACCENT_SOFT = "rgba(232, 85, 127, 0.25)";
const BORDER = "rgba(255, 255, 255, 0.08)";
const TEXT = "#E6E9F0";
const TEXT_MUTED = "#8B92A8";

export function formatMonthLabel(month: string): string {
  const parts = month.split("-");
  const m = parts[1] ?? parts[0];
  return `${parseInt(m, 10)}月`;
}

export function buildMonthlyTrendData(items: MonthlyHeatmapItem[]): ChartData<"line"> {
  const slice = items.slice(-12);
  const labels = slice.map((i) => formatMonthLabel(i.month));
  const data = slice.map((i) => i.hours);

  return {
    labels,
    datasets: [
      {
        label: "游玩时长 (h)",
        data,
        borderColor: ACCENT,
        backgroundColor: (ctx) => {
          const canvas = ctx.chart.ctx;
          const gradient = canvas.createLinearGradient(0, 0, 0, 200);
          gradient.addColorStop(0, "rgba(232, 85, 127, 0.35)");
          gradient.addColorStop(1, "rgba(232, 85, 127, 0.0)");
          return gradient;
        },
        borderWidth: 2,
        pointBackgroundColor: ACCENT,
        pointBorderColor: "#0B0E14",
        pointBorderWidth: 2,
        pointRadius: 3,
        pointHoverRadius: 5,
        fill: true,
        tension: 0.35,
      },
    ],
  };
}

export function buildStatusDistributionData(
  distribution: StatusDistributionItem[],
  statusLabels: Record<string, string>
): ChartData<"bar"> {
  const items = distribution
    .filter((d) => d.count > 0)
    .sort((a, b) => b.count - a.count);

  return {
    labels: items.map((d) => statusLabels[d.status] ?? d.status),
    datasets: [
      {
        label: "数量",
        data: items.map((d) => d.count),
        backgroundColor: ACCENT_SOFT,
        borderColor: ACCENT,
        borderWidth: 1,
        borderRadius: 4,
        barThickness: 12,
      },
    ],
  };
}

export function buildCompletionDoughnutData(rate: number): ChartData<"doughnut"> {
  const r = Math.max(0, Math.min(100, rate));
  return {
    labels: ["已完成", "未完成"],
    datasets: [
      {
        data: [r, 100 - r],
        backgroundColor: [ACCENT, BORDER],
        borderColor: "transparent",
        borderWidth: 0,
        hoverOffset: 4,
      },
    ],
  };
}

export const commonChartOptions: ChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: "rgba(10, 13, 20, 0.95)",
      titleColor: TEXT,
      bodyColor: TEXT,
      borderColor: BORDER,
      borderWidth: 1,
      padding: 8,
      cornerRadius: 6,
      displayColors: false,
    },
  },
  scales: {
    x: {
      grid: { color: "rgba(255,255,255,0.04)" },
      ticks: { color: TEXT_MUTED, font: { size: 10 } },
      border: { display: false },
    },
    y: {
      grid: { color: "rgba(255,255,255,0.04)" },
      ticks: { color: TEXT_MUTED, font: { size: 10 } },
      border: { display: false },
      beginAtZero: true,
    },
  },
};

export const statusBarOptions: ChartOptions<"bar"> = {
  responsive: true,
  maintainAspectRatio: false,
  indexAxis: "y",
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: "rgba(10, 13, 20, 0.95)",
      titleColor: TEXT,
      bodyColor: TEXT,
      borderColor: BORDER,
      borderWidth: 1,
      padding: 8,
      cornerRadius: 6,
      displayColors: false,
    },
  },
  scales: {
    x: {
      grid: { color: "rgba(255,255,255,0.04)" },
      ticks: { color: TEXT_MUTED, font: { size: 10 } },
      border: { display: false },
      beginAtZero: true,
    },
    y: {
      grid: { display: false },
      ticks: { color: TEXT, font: { size: 11 } },
      border: { display: false },
    },
  },
};

export const doughnutOptions: ChartOptions<"doughnut"> = {
  responsive: true,
  maintainAspectRatio: false,
  cutout: "75%",
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: "rgba(10, 13, 20, 0.95)",
      bodyColor: TEXT,
      borderColor: BORDER,
      borderWidth: 1,
      padding: 8,
      cornerRadius: 6,
      displayColors: false,
      callbacks: {
        label: (ctx) => `${ctx.label}: ${ctx.raw}%`,
      },
    },
  },
};
