import { invokeCmd } from "./core";
import type {
  Collection,
  DashboardData,
  MonthActivity,
} from "./types";

export type { DashboardData } from "./types";

export interface CountItem {
  name: string;
  count: number;
}

export interface StatusCountItem {
  status: string;
  count: number;
}

export interface CollectionCountItem {
  id: string;
  name: string;
  count: number;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function numberField(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`DashboardData.${key} 必须是有限数字`);
  }
  return value;
}

function stringArrayField(record: Record<string, unknown>, key: string): string[] {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new TypeError(`DashboardData.${key} 必须是字符串数组`);
  }
  return [...value];
}

function countTuplesField(record: Record<string, unknown>, key: string): [string, number][] {
  const value = record[key];
  if (
    !Array.isArray(value) ||
    value.some(
      (item) =>
        !Array.isArray(item) ||
        item.length !== 2 ||
        typeof item[0] !== "string" ||
        typeof item[1] !== "number" ||
        !Number.isFinite(item[1])
    )
  ) {
    throw new TypeError(`DashboardData.${key} 必须是 [string, number][]`);
  }
  return value.map((item) => [item[0] as string, item[1] as number]);
}

function monthActivitiesField(record: Record<string, unknown>): MonthActivity[] {
  const value = record.monthly_heatmap;
  if (!Array.isArray(value)) {
    throw new TypeError("DashboardData.monthly_heatmap 必须是数组");
  }
  return value.map((item) => {
    if (!isRecord(item) || typeof item.month !== "string") {
      throw new TypeError("DashboardData.monthly_heatmap 项格式错误");
    }
    return {
      month: item.month,
      sessions: numberField(item, "sessions"),
      hours: numberField(item, "hours"),
    };
  });
}

function collectionsField(record: Record<string, unknown>): Collection[] {
  const value = record.collections;
  if (!Array.isArray(value)) {
    throw new TypeError("DashboardData.collections 必须是数组");
  }
  return value.map((item) => {
    if (
      !isRecord(item) ||
      typeof item.id !== "string" ||
      typeof item.name !== "string" ||
      typeof item.description !== "string" ||
      typeof item.icon !== "string"
    ) {
      throw new TypeError("DashboardData.collections 项格式错误");
    }
    return {
      id: item.id,
      name: item.name,
      description: item.description,
      game_count: numberField(item, "game_count"),
      icon: item.icon,
    };
  });
}

/** 将 Tauri 返回值收敛到 Rust stats::DashboardData 的唯一前端契约。 */
export function parseDashboardData(value: unknown): DashboardData {
  if (!isRecord(value)) {
    throw new TypeError("DashboardData 必须是对象");
  }

  return {
    total_games: numberField(value, "total_games"),
    installed_games: numberField(value, "installed_games"),
    completed_games: numberField(value, "completed_games"),
    playtime_hours: numberField(value, "playtime_hours"),
    completion_rate: numberField(value, "completion_rate"),
    scrape_coverage: numberField(value, "scrape_coverage"),
    disk_usage_gb: numberField(value, "disk_usage_gb"),
    recent_games: stringArrayField(value, "recent_games"),
    top_tags: countTuplesField(value, "top_tags"),
    completion_distribution: countTuplesField(value, "completion_distribution"),
    monthly_heatmap: monthActivitiesField(value),
    collections: collectionsField(value),
  };
}

export async function getDashboardData(): Promise<DashboardData> {
  return parseDashboardData(await invokeCmd<unknown>("get_dashboard_data"));
}

export function toCountItems(entries: [string, number][]): CountItem[] {
  return entries.map(([name, count]) => ({ name, count }));
}

export function toStatusCountItems(entries: [string, number][]): StatusCountItem[] {
  return entries.map(([status, count]) => ({
    status: status
      .trim()
      .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
      .replace(/[\s-]+/g, "_")
      .toLowerCase(),
    count,
  }));
}

export function toCollectionCountItems(collections: Collection[]): CollectionCountItem[] {
  return collections.map((collection) => ({
    id: collection.id,
    name: collection.name,
    count: collection.game_count,
  }));
}
