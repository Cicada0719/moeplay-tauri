// api 域模块：format（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";
import type { CompletionStatus, Game } from "./types";

export function formatPlayTime(seconds: number): string {
  if (seconds === 0) return "未游玩";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}


export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/** 获取有效的评分（优先用户评分 → VNDB → Bangumi → 旧字段） */

export function effectiveRating(game: Game): number | null {
  return (
    game.play_tracker?.user_rating ??
    game.metadata?.vndb_rating ??
    game.metadata?.bangumi_rating ??
    game.rating ??
    null
  );
}

/** 获取有效的发行年份 */

export function effectiveReleaseYear(game: Game): number | null {
  return game.metadata?.release_year ?? game.release_year ?? null;
}

/** 获取有效的最后游玩时间 */

export function effectiveLastPlayed(game: Game): string | null {
  return game.play_tracker?.last_played ?? game.last_played ?? null;
}

/** 获取完成状态的中文描述 */

export function completionStatusLabel(status: CompletionStatus): string {
  const labels: Record<CompletionStatus, string> = {
    not_started: "未开始",
    playing: "游玩中",
    completed: "已通关",
    dropped: "已弃坑",
    on_hold: "搁置中",
    plan_to_play: "计划玩",
    replaying: "重温中",
  };
  return labels[status] ?? status;
}

// ===== M6 Steam 集成 =====

