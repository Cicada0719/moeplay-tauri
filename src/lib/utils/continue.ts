import { startOfDay, isSameDay, isWithinInterval, subDays } from "date-fns";
import {
  coverOf,
  developerOf,
  gameCompletionStatus,
  gameTotalSeconds,
  platformOf,
  type GameLike,
} from "./game";

// ── 类型 ──────────────────────────────────────────────────────────────────

export interface ContinueItem {
  id: string;
  type: "game" | "anime" | "comic";
  title: string;
  cover: string | null;
  progress: number; // 0-100
  progressLabel: string;
  lastActivity: number;
  subtitle?: string;
  actionLabel?: string;
}

export interface ContinueStats {
  totalCount: number;
  gameCount: number;
  animeCount: number;
  comicCount: number;
  todayMinutes: number;
  weekMinutes: number;
  streakDays: number;
}

export interface PlaySession {
  id?: string;
  start_time?: string;
  end_time?: string;
  duration_seconds: number;
  notes?: string;
}

export interface AnimeHistoryLike {
  key: string;
  name: string;
  image: string;
  ruleName: string;
  sourceUrl: string;
  lastRoad: number;
  lastEpisode: number;
  lastEpisodeName: string;
  progressMs: number;
  updatedAt: string;
}

export interface ComicHistoryLike {
  id: string;
  title: string;
  thumb_url: string;
  author: string;
  last_order: number;
  last_title: string;
  ts: number;
}

// ── 游戏进度与文案 ────────────────────────────────────────────────────────

export function computeGameProgress(game: GameLike): number {
  const total = game?.play_tracker?.achievements_total ?? 0;
  const unlocked = game?.play_tracker?.achievements_unlocked ?? 0;
  if (total > 0) {
    return Math.min(100, Math.max(0, Math.round((unlocked / total) * 100)));
  }
  const status = gameCompletionStatus(game);
  switch (status) {
    case "not_started":
    case "plan_to_play":
      return 0;
    case "playing":
      return 15;
    case "replaying":
      return 50;
    case "on_hold":
      return 30;
    case "completed":
    case "dropped":
      return 100;
    default:
      return 0;
  }
}

export function gameSubtitle(game: GameLike): string {
  return developerOf(game) || platformOf(game) || "";
}

export function gameActionLabel(status: string): string {
  switch (status) {
    case "not_started":
    case "plan_to_play":
      return "开始游玩";
    case "completed":
      return "已通关";
    case "dropped":
      return "已弃坑";
    case "on_hold":
    case "playing":
    case "replaying":
      return "继续游玩";
    default:
      return "打开";
  }
}

// ── 时长聚合 ──────────────────────────────────────────────────────────────

function sessionDate(session: PlaySession): Date | null {
  const raw = session.start_time;
  if (!raw) return null;
  const d = new Date(raw);
  return isNaN(d.getTime()) ? null : d;
}

export function aggregateTodaySeconds(sessions: PlaySession[], now: Date = new Date()): number {
  const start = startOfDay(now);
  return sessions.reduce((sum, s) => {
    if (!s.duration_seconds || s.duration_seconds <= 0) return sum;
    const d = sessionDate(s);
    if (!d) return sum;
    if (d >= start && d <= now) return sum + s.duration_seconds;
    return sum;
  }, 0);
}

export function aggregateWeekSeconds(sessions: PlaySession[], now: Date = new Date()): number {
  const start = startOfDay(subDays(now, 6));
  return sessions.reduce((sum, s) => {
    if (!s.duration_seconds || s.duration_seconds <= 0) return sum;
    const d = sessionDate(s);
    if (!d) return sum;
    if (isWithinInterval(d, { start, end: now })) return sum + s.duration_seconds;
    return sum;
  }, 0);
}

// ── 连续活跃天数 ──────────────────────────────────────────────────────────

function startOfDayUTC(d: Date): number {
  return Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate());
}

export function computeStreakDays(activityTimestamps: number[], now: Date = new Date()): number {
  const days = new Set<number>();
  for (const ts of activityTimestamps) {
    const d = new Date(ts);
    if (isNaN(d.getTime())) continue;
    days.add(startOfDayUTC(d));
  }
  if (days.size === 0) return 0;

  let streak = 0;
  const today = startOfDayUTC(now);
  const hasToday = days.has(today);
  let checkDay = hasToday ? today : startOfDayUTC(new Date(now.getTime() - 86_400_000));

  while (days.has(checkDay)) {
    streak++;
    checkDay -= 86_400_000;
  }
  return streak;
}

// ── Item 构建 ─────────────────────────────────────────────────────────────

export function gameLastActivityTimestamp(game: GameLike): number | null {
  const sessions = game?.play_tracker?.sessions ?? [];
  let latest = 0;
  for (const s of sessions) {
    if (!s.start_time) continue;
    const t = new Date(s.start_time).getTime();
    if (!isNaN(t) && t > latest) latest = t;
  }
  const lastPlayed = game?.play_tracker?.last_played ?? game?.last_played;
  if (lastPlayed) {
    const t = new Date(lastPlayed).getTime();
    if (!isNaN(t) && t > latest) latest = t;
  }
  return latest > 0 ? latest : null;
}

export function buildGameContinueItem(game: GameLike): ContinueItem {
  const normalized = game as GameLike;
  const status = gameCompletionStatus(normalized);
  const totalSec = gameTotalSeconds(normalized);
  const ts = gameLastActivityTimestamp(normalized) ?? Date.now();

  return {
    id: `game-${(normalized as { id?: string }).id ?? "unknown"}`,
    type: "game",
    title: (normalized as { name?: string }).name ?? "未知游戏",
    cover: coverOf(normalized),
    progress: computeGameProgress(normalized),
    progressLabel: totalSec > 0 ? `${(totalSec / 3600).toFixed(1)}h` : "未玩",
    lastActivity: isNaN(ts) ? Date.now() : ts,
    subtitle: gameSubtitle(normalized),
    actionLabel: gameActionLabel(status),
  };
}

export function buildAnimeContinueItem(h: AnimeHistoryLike): ContinueItem {
  const ts = h.updatedAt ? new Date(h.updatedAt).getTime() : Date.now();
  return {
    id: `anime-${h.key}`,
    type: "anime",
    title: h.name,
    cover: h.image || null,
    progress: 0,
    progressLabel: h.lastEpisode > 0 ? `第${h.lastEpisode}话` : "",
    lastActivity: isNaN(ts) ? Date.now() : ts,
    subtitle: h.ruleName || "",
    actionLabel: "继续观看",
  };
}

export function buildComicContinueItem(h: ComicHistoryLike): ContinueItem {
  const ts = h.ts || Date.now();
  return {
    id: `comic-${h.id}`,
    type: "comic",
    title: h.title,
    cover: h.thumb_url || null,
    progress: 0,
    progressLabel: h.last_title || (h.last_order > 0 ? `第${h.last_order}话` : ""),
    lastActivity: ts,
    subtitle: h.author || "",
    actionLabel: "继续阅读",
  };
}

export function buildContinueItems(
  games: GameLike[],
  animeHistory: AnimeHistoryLike[],
  comicHistory: ComicHistoryLike[]
): ContinueItem[] {
  const items: ContinueItem[] = [];

  for (const g of games) {
    const status = gameCompletionStatus(g);
    if (status === "completed" || status === "dropped") continue;
    if (gameLastActivityTimestamp(g) === null) continue;
    items.push(buildGameContinueItem(g));
  }

  for (const h of animeHistory) {
    if (h.lastEpisode <= 0) continue;
    items.push(buildAnimeContinueItem(h));
  }

  for (const h of comicHistory) {
    items.push(buildComicContinueItem(h));
  }

  return items
    .sort((a, b) => priorityScore(b) - priorityScore(a))
    .slice(0, 30);
}

// ── 优先级评分 ─────────────────────────────────────────────────────────────

export function priorityScore(item: ContinueItem): number {
  let score = item.lastActivity * 1e-7;
  if (item.type === "game") score += 30;
  else if (item.type === "anime") score += 20;
  else if (item.type === "comic") score += 20;

  if (item.progress > 0 && item.progress < 100) score += 100;
  if (item.progress >= 100) score -= 50;
  return score;
}

// ── Stats 构建 ────────────────────────────────────────────────────────────

function activityTimestampsFromSources(
  items: ContinueItem[],
  games: GameLike[],
  animeHistory: AnimeHistoryLike[],
  comicHistory: ComicHistoryLike[]
): number[] {
  const timestamps = items.map((i) => i.lastActivity);
  for (const g of games) {
    for (const s of g?.play_tracker?.sessions ?? []) {
      if (!s.start_time) continue;
      const t = new Date(s.start_time).getTime();
      if (!isNaN(t)) timestamps.push(t);
    }
  }
  for (const h of animeHistory) {
    if (h.updatedAt) {
      const t = new Date(h.updatedAt).getTime();
      if (!isNaN(t)) timestamps.push(t);
    }
  }
  for (const h of comicHistory) {
    if (h.ts) timestamps.push(h.ts);
  }
  return timestamps;
}

export function buildContinueStats(
  items: ContinueItem[],
  games: GameLike[],
  animeHistory: AnimeHistoryLike[],
  comicHistory: ComicHistoryLike[],
  now: Date = new Date()
): ContinueStats {
  const gameCount = items.filter((i) => i.type === "game").length;
  const animeCount = items.filter((i) => i.type === "anime").length;
  const comicCount = items.filter((i) => i.type === "comic").length;

  let todaySeconds = 0;
  let weekSeconds = 0;

  for (const g of games) {
    const sessions = g?.play_tracker?.sessions ?? [];
    todaySeconds += aggregateTodaySeconds(sessions, now);
    weekSeconds += aggregateWeekSeconds(sessions, now);
  }

  const todayStart = startOfDay(now).getTime();
  const weekStart = startOfDay(subDays(now, 6)).getTime();

  for (const h of animeHistory) {
    const ts = h.updatedAt ? new Date(h.updatedAt).getTime() : 0;
    if (ts >= todayStart) todaySeconds += (h.lastEpisode || 1) * 24 * 60;
    if (ts >= weekStart) weekSeconds += (h.lastEpisode || 1) * 24 * 60;
  }

  for (const h of comicHistory) {
    const ts = h.ts || 0;
    if (ts >= todayStart) todaySeconds += (h.last_order || 1) * 2 * 60;
    if (ts >= weekStart) weekSeconds += (h.last_order || 1) * 2 * 60;
  }

  const timestamps = activityTimestampsFromSources(items, games, animeHistory, comicHistory);

  return {
    totalCount: items.length,
    gameCount,
    animeCount,
    comicCount,
    todayMinutes: Math.round(todaySeconds / 60),
    weekMinutes: Math.round(weekSeconds / 60),
    streakDays: computeStreakDays(timestamps, now),
  };
}
