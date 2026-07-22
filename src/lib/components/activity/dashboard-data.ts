import type { Game, PlaySessionEntry, PlaytimeSummary } from "../../api";
import type { AnimeHistory } from "../../stores/anime.svelte";
import type { ReadRecord } from "../../stores/comic.svelte";
import type { NovelHistoryEntry } from "../../features/novel/types";
import { fileSrc } from "../../utils";
import { coverOf, gameLastPlayed, gameTotalSeconds } from "../../utils/game";
import type { DashboardChartPoint, DashboardMediaActivity, DashboardSession, DashboardTopGame } from "./dashboard-model";

export function buildLocalSummary(games: Game[]): PlaytimeSummary {
  const sessions: PlaySessionEntry[] = [];
  const daily = new Map<string, { seconds: number; sessions: number }>();
  const monthly = new Map<string, { seconds: number; sessions: number }>();
  let totalSeconds = 0;
  for (const game of games) {
    totalSeconds += gameTotalSeconds(game);
    for (const session of game.play_tracker?.sessions ?? []) {
      const seconds = Number(session.duration_seconds ?? 0);
      if (!seconds || seconds <= 0) continue;
      sessions.push({ game_id: game.id, game_name: game.name, session });
      const day = dateKey(session.start_time);
      const month = day.slice(0, 7);
      const dailyValue = daily.get(day) ?? { seconds: 0, sessions: 0 };
      dailyValue.seconds += seconds; dailyValue.sessions += 1; daily.set(day, dailyValue);
      const monthlyValue = monthly.get(month) ?? { seconds: 0, sessions: 0 };
      monthlyValue.seconds += seconds; monthlyValue.sessions += 1; monthly.set(month, monthlyValue);
    }
  }
  sessions.sort((a, b) => toTimestamp(b.session.start_time) - toTimestamp(a.session.start_time));
  const topGames = games.map((game) => ({ game_id: game.id, game_name: game.name, total_seconds: gameTotalSeconds(game), sessions: game.play_tracker?.sessions?.length ?? 0, last_played: gameLastPlayed(game) ?? undefined })).filter((item) => item.total_seconds > 0).sort((a, b) => b.total_seconds - a.total_seconds);
  const sessionCount = sessions.length;
  return {
    total_seconds: totalSeconds,
    session_count: sessionCount,
    play_days: [...daily.values()].filter((day) => day.seconds > 0).length,
    average_session_seconds: sessionCount ? Math.round(sessions.reduce((sum, item) => sum + item.session.duration_seconds, 0) / sessionCount) : 0,
    daily: [...daily.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([date, value]) => ({ date, ...value })),
    monthly: [...monthly.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([month, value]) => ({ month, ...value })),
    recent_sessions: sessions,
    top_games: topGames,
  };
}

export function fillDailyBars(days: PlaytimeSummary["daily"], count: number, now = new Date()): PlaytimeSummary["daily"] {
  const byDate = new Map(days.map((day) => [day.date, day]));
  const list: PlaytimeSummary["daily"] = [];
  for (let index = count - 1; index >= 0; index -= 1) {
    const date = new Date(now); date.setDate(now.getDate() - index);
    const key = dateKey(date.toISOString());
    list.push(byDate.get(key) ?? { date: key, seconds: 0, sessions: 0 });
  }
  return list;
}

export function buildMediaActivities(sessions: PlaySessionEntry[], animeHistory: AnimeHistory[], comicHistory: ReadRecord[], novelHistory: NovelHistoryEntry[], games: Game[]): DashboardMediaActivity[] {
  const gameById = new Map(games.map((game) => [game.id, game]));
  const gameItems: DashboardMediaActivity[] = sessions.map((entry) => ({
    id: `game:${entry.game_id}:${entry.session.id}`,
    kind: "game",
    title: entry.game_name,
    subtitle: `游玩 ${formatCompactSeconds(entry.session.duration_seconds)}`,
    timeLabel: formatDateTime(entry.session.start_time),
    timestamp: toTimestamp(entry.session.start_time),
    imageSrc: fileSrc(coverOf(gameById.get(entry.game_id))),
    payload: entry,
  }));
  const animeItems: DashboardMediaActivity[] = animeHistory.map((entry) => ({ id: `anime:${entry.key}`, kind: "anime", title: entry.name, subtitle: `看到 ${entry.lastEpisodeName || `第 ${entry.lastEpisode + 1} 集`}`, timeLabel: formatDateTime(entry.updatedAt), timestamp: toTimestamp(entry.updatedAt), imageSrc: entry.image || null, payload: entry }));
  const comicItems: DashboardMediaActivity[] = comicHistory.map((entry) => ({ id: `comic:${entry.id}`, kind: "comic", title: entry.title, subtitle: `读到 ${entry.last_title || `第 ${entry.last_order} 话`}`, timeLabel: formatDateTime(new Date(entry.ts).toISOString()), timestamp: entry.ts || 0, imageSrc: entry.thumb_url || null, payload: entry }));
  const novelItems: DashboardMediaActivity[] = novelHistory.map((entry) => ({ id: `novel:${entry.key}`, kind: "novel", title: entry.book.title, subtitle: `读到 ${entry.chapterTitle} · ${Math.round(entry.progress * 100)}%`, timeLabel: formatDateTime(new Date(entry.updatedAt).toISOString()), timestamp: entry.updatedAt || 0, imageSrc: entry.book.coverUrl ?? null, payload: entry }));
  return [...gameItems, ...animeItems, ...comicItems, ...novelItems].filter((item) => item.timestamp > 0).sort((a, b) => b.timestamp - a.timestamp);
}


export function archiveActivityIdentity(item: DashboardMediaActivity): string {
  const payload = item.payload as Record<string, unknown> | null | undefined;
  if (item.kind === "game") {
    const gameId = String(payload?.game_id ?? "").trim();
    if (gameId) return `game:${gameId}`;
  }
  if (item.kind === "anime") {
    const key = String(payload?.key ?? payload?.sourceUrl ?? "").trim();
    if (key) return `anime:${key}`;
  }
  if (item.kind === "comic") {
    const id = String(payload?.id ?? "").trim();
    if (id) return `comic:${id}`;
  }
  if (item.kind === "novel") {
    const key = String(payload?.key ?? "").trim();
    if (key) return `novel:${key}`;
  }
  return `${item.kind}:${item.title.trim().replace(/\s+/g, " ").toLocaleLowerCase("zh-CN")}`;
}

/** Keeps the newest archive row for each software/title while preserving the full activity stream elsewhere. */
export function uniqueArchiveActivities(items: readonly DashboardMediaActivity[]): DashboardMediaActivity[] {
  const seen = new Set<string>();
  return [...items]
    .sort((a, b) => b.timestamp - a.timestamp || a.id.localeCompare(b.id))
    .filter((item) => {
      const identity = archiveActivityIdentity(item);
      if (seen.has(identity)) return false;
      seen.add(identity);
      return true;
    });
}

export function fillActivityBars(items: DashboardMediaActivity[], count: number, now = new Date()): Array<{ date: string; count: number }> {
  const byDate = new Map<string, number>();
  for (const item of items) {
    const key = dateKey(new Date(item.timestamp).toISOString());
    byDate.set(key, (byDate.get(key) ?? 0) + 1);
  }
  const list: Array<{ date: string; count: number }> = [];
  for (let index = count - 1; index >= 0; index -= 1) {
    const date = new Date(now); date.setDate(now.getDate() - index);
    const key = dateKey(date.toISOString());
    list.push({ date: key, count: byDate.get(key) ?? 0 });
  }
  return list;
}

export function countRecentActivities(items: DashboardMediaActivity[], days: number, now = Date.now()): number {
  const since = now - days * 86_400_000;
  return items.filter((item) => item.timestamp >= since).length;
}

export function countMediaKinds(items: DashboardMediaActivity[]) {
  return items.reduce((counts, item) => { counts[item.kind] += 1; return counts; }, { game: 0, anime: 0, comic: 0, novel: 0 });
}

export function toDashboardTopGames(summary: PlaytimeSummary, games: Game[]): DashboardTopGame[] {
  const byId = new Map(games.map((game) => [game.id, game]));
  return summary.top_games.slice(0, 5).map((item) => ({ ...item, cover: fileSrc(coverOf(byId.get(item.game_id))) }));
}

export function toDashboardSessions(summary: PlaytimeSummary, games: Game[]): DashboardSession[] {
  const byId = new Map(games.map((game) => [game.id, game]));
  return summary.recent_sessions.slice(0, 7).map((entry) => ({ ...entry, imageSrc: fileSrc(coverOf(byId.get(entry.game_id))), formattedTime: formatDateTime(entry.session.start_time), formattedDuration: formatCompactSeconds(entry.session.duration_seconds) }));
}

export function dailyChartPoints(days: PlaytimeSummary["daily"]): DashboardChartPoint[] {
  return days.map((day) => ({ key: day.date, label: dayLabel(day.date), value: day.seconds, valueLabel: formatCompactSeconds(day.seconds) }));
}
export function monthlyChartPoints(months: PlaytimeSummary["monthly"]): DashboardChartPoint[] {
  return months.map((month) => ({ key: month.month, label: monthLabel(month.month), value: month.seconds, valueLabel: formatCompactSeconds(month.seconds) }));
}
export function activityChartPoints(days: Array<{ date: string; count: number }>): DashboardChartPoint[] {
  return days.map((day) => ({ key: day.date, label: dayLabel(day.date), value: day.count, valueLabel: `${day.count} 次` }));
}

export function summarizeChart(points: DashboardChartPoint[], noun: string): string {
  if (points.length === 0) return `暂无${noun}数据。`;
  const total = points.reduce((sum, point) => sum + point.value, 0);
  const peak = points.reduce((best, point) => point.value > best.value ? point : best, points[0]);
  return `${points.length} 个时间点，合计 ${noun === "活动" ? `${total} 次` : formatCompactSeconds(total)}；峰值为 ${peak.label} 的 ${peak.valueLabel}。`;
}

export function dateKey(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? new Date().toISOString().slice(0, 10) : date.toISOString().slice(0, 10);
}
export function monthLabel(value: string): string { const [, month] = value.split("-"); return `${Number(month || 0)}月`; }
export function dayLabel(value: string): string { const date = new Date(`${value}T00:00:00`); return Number.isNaN(date.getTime()) ? value.slice(5) : `${date.getMonth() + 1}/${date.getDate()}`; }
export function formatDateTime(value: string | undefined): string { if (!value) return "暂无记录"; const date = new Date(value); return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }).format(date); }
export function formatCompactSeconds(seconds: number): string { if (!seconds || seconds <= 0) return "0m"; const hours = seconds / 3600; if (hours >= 10) return `${Math.round(hours)}h`; if (hours >= 1) return `${hours.toFixed(1)}h`; return `${Math.max(1, Math.round(seconds / 60))}m`; }
function toTimestamp(value: string | undefined): number { if (!value) return 0; const timestamp = new Date(value).getTime(); return Number.isNaN(timestamp) ? 0 : timestamp; }
