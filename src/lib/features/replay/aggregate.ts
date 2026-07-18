import type { PlaySession } from "../../api";

/**
 * 年度回顾「我的年度游戏档案」聚合层：纯函数、零副作用、可单测。
 * 输入只依赖 Game 的少数字段（结构子集），便于测试用字面量构造。
 * 日期一律按本地时区解析（与游玩记录展示口径一致）。
 */

export interface ReplayGameLike {
  id: string;
  name: string;
  /** 入库时间，first_played 缺失时作为「年度新增」的回退口径。 */
  add_date?: string;
  play_tracker?: {
    sessions?: PlaySession[];
    first_played?: string;
    achievements_total?: number;
    achievements_unlocked?: number;
    completion_count?: number;
  } | null;
}

export interface YearSessionEntry<T extends ReplayGameLike = ReplayGameLike> {
  game: T;
  session: PlaySession;
  start: Date;
}

export interface YearSummary {
  totalSeconds: number;
  playDays: number;
  sessionCount: number;
  gameCount: number;
  /** totalSeconds / 3600，保留 1 位小数。 */
  hours: number;
}

export interface TopPlayedEntry<T extends ReplayGameLike = ReplayGameLike> {
  game: T;
  seconds: number;
  sessions: number;
}

export interface MonthHeat {
  /** 0-11 */
  month: number;
  seconds: number;
  /** 相对全年峰值月的归一值，0-1；全年无数据时全为 0。 */
  ratio: number;
}

export interface AchievementEntry<T extends ReplayGameLike = ReplayGameLike> {
  game: T;
  total: number;
  unlocked: number;
  /** unlocked / total，0-1。 */
  ratio: number;
}

export interface CompletionEntry<T extends ReplayGameLike = ReplayGameLike> {
  game: T;
  count: number;
}

export interface NewGamesMonth<T extends ReplayGameLike = ReplayGameLike> {
  /** 0-11 */
  month: number;
  games: T[];
}

function parseLocalDate(value: string | null | undefined): Date | null {
  if (!value || typeof value !== "string") return null;
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** 本地日期键，用于「游玩天数」按日去重。 */
function localDayKey(d: Date): string {
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

/** 非负时长：负值与 NaN 一律记 0，避免脏数据污染聚合。 */
function safeDuration(session: PlaySession): number {
  const d = Number(session?.duration_seconds);
  return Number.isFinite(d) && d > 0 ? d : 0;
}

/** 过滤出 start_time 落在指定年份（本地时区）的会话。
 *  跨年会话按 start_time 归属到开始那一年；无法解析的 start_time 直接跳过。 */
export function filterYearSessions<T extends ReplayGameLike>(
  games: readonly T[],
  year: number,
): YearSessionEntry<T>[] {
  const out: YearSessionEntry<T>[] = [];
  for (const game of games) {
    for (const session of game?.play_tracker?.sessions ?? []) {
      const start = parseLocalDate(session?.start_time);
      if (!start || start.getFullYear() !== year) continue;
      out.push({ game, session, start });
    }
  }
  return out;
}

/** 年度总量：总秒数、游玩天数（不同本地日期数）、会话数、涉及游戏数、小时数。 */
export function summarizeYear(entries: readonly YearSessionEntry<ReplayGameLike>[]): YearSummary {
  const days = new Set<string>();
  const gameIds = new Set<string>();
  let totalSeconds = 0;
  for (const entry of entries) {
    totalSeconds += safeDuration(entry.session);
    days.add(localDayKey(entry.start));
    gameIds.add(entry.game.id);
  }
  return {
    totalSeconds,
    playDays: days.size,
    sessionCount: entries.length,
    gameCount: gameIds.size,
    hours: Math.round(totalSeconds / 360) / 10,
  };
}

/** 最玩榜：按年度会话累计时长降序，平手按游戏名升序，截断 limit。 */
export function topPlayed<T extends ReplayGameLike>(
  entries: readonly YearSessionEntry<T>[],
  limit = 5,
): TopPlayedEntry<T>[] {
  const byGame = new Map<string, TopPlayedEntry<T>>();
  for (const entry of entries) {
    const prev = byGame.get(entry.game.id);
    if (prev) {
      prev.seconds += safeDuration(entry.session);
      prev.sessions += 1;
    } else {
      byGame.set(entry.game.id, {
        game: entry.game,
        seconds: safeDuration(entry.session),
        sessions: 1,
      });
    }
  }
  return [...byGame.values()]
    .sort((a, b) => b.seconds - a.seconds || a.game.name.localeCompare(b.game.name))
    .slice(0, Math.max(0, limit));
}

/** 月度热力：12 格，seconds 为当月总会话秒数，ratio 为相对全年峰值月的归一值。 */
export function monthlyHeat(entries: readonly YearSessionEntry<ReplayGameLike>[]): MonthHeat[] {
  const secondsByMonth = new Array<number>(12).fill(0);
  for (const entry of entries) {
    secondsByMonth[entry.start.getMonth()] += safeDuration(entry.session);
  }
  const peak = Math.max(0, ...secondsByMonth);
  return secondsByMonth.map((seconds, month) => ({
    month,
    seconds,
    ratio: peak > 0 ? seconds / peak : 0,
  }));
}

/** 成就解锁率榜：仅统计 achievements_total > 0 的游戏；
 *  按解锁率降序（平手按已解锁数降序），截断 limit。 */
export function topAchievements<T extends ReplayGameLike>(
  games: readonly T[],
  limit = 3,
): AchievementEntry<T>[] {
  const out: AchievementEntry<T>[] = [];
  for (const game of games) {
    const total = Number(game?.play_tracker?.achievements_total) || 0;
    if (total <= 0) continue;
    const unlocked = Math.min(
      total,
      Math.max(0, Number(game?.play_tracker?.achievements_unlocked) || 0),
    );
    out.push({ game, total, unlocked, ratio: unlocked / total });
  }
  return out
    .sort((a, b) => b.ratio - a.ratio || b.unlocked - a.unlocked)
    .slice(0, Math.max(0, limit));
}

/** 完成次数榜：仅统计 completion_count > 0 的游戏，降序截断。 */
export function topCompletions<T extends ReplayGameLike>(
  games: readonly T[],
  limit = 3,
): CompletionEntry<T>[] {
  const out: CompletionEntry<T>[] = [];
  for (const game of games) {
    const count = Number(game?.play_tracker?.completion_count) || 0;
    if (count <= 0) continue;
    out.push({ game, count });
  }
  return out
    .sort((a, b) => b.count - a.count || a.game.name.localeCompare(b.game.name))
    .slice(0, Math.max(0, limit));
}

/** 年度新增时间线：first_played（缺省回退 add_date）落在该年的游戏，按月份 0-11 分桶。 */
export function newGamesTimeline<T extends ReplayGameLike>(
  games: readonly T[],
  year: number,
): NewGamesMonth<T>[] {
  const buckets: T[][] = Array.from({ length: 12 }, () => []);
  for (const game of games) {
    const date = parseLocalDate(game?.play_tracker?.first_played ?? game?.add_date);
    if (!date || date.getFullYear() !== year) continue;
    buckets[date.getMonth()].push(game);
  }
  return buckets.map((gamesInMonth, month) => ({ month, games: gamesInMonth }));
}
