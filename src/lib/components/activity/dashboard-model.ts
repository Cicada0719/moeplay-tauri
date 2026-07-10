import type { PlaySessionEntry, PlaytimeSummary } from "../../api";

export type DashboardMediaKind = "game" | "anime" | "comic";

export interface DashboardMediaActivity {
  id: string;
  kind: DashboardMediaKind;
  title: string;
  subtitle: string;
  timeLabel: string;
  timestamp: number;
  imageSrc: string | null;
  payload: unknown;
}

export interface DashboardStat {
  id: string;
  label: string;
  value: string | number;
  detail?: string;
  tone?: "default" | "accent" | "success" | "warning";
}

export interface DashboardChartPoint {
  key: string;
  label: string;
  value: number;
  valueLabel: string;
}

export type DashboardTopGame = PlaytimeSummary["top_games"][number] & {
  cover: string | null;
};

export interface DashboardSession extends PlaySessionEntry {
  imageSrc: string | null;
  formattedTime: string;
  formattedDuration: string;
}




