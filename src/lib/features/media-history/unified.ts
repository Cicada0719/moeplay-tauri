import type { AnimeHistory } from "../anime-player/historyStore.svelte";
import type { NovelHistoryEntry } from "../novel/types";
import type { ReadRecord } from "../../stores/comic.svelte";

export type UnifiedMediaHistoryKind = "anime" | "comic" | "novel";
export type UnifiedMediaHistoryPayload = AnimeHistory | ReadRecord | NovelHistoryEntry;

export interface UnifiedMediaHistoryItem {
  /** 带媒体类型前缀的稳定 id，避免不同来源的相同 key 冲突。 */
  id: string;
  kind: UnifiedMediaHistoryKind;
  title: string;
  cover: string | null;
  sourceLabel: string;
  positionLabel: string;
  progress: number | null;
  updatedAt: number;
  payload: UnifiedMediaHistoryPayload;
}

export interface UnifiedMediaHistoryInput {
  anime: readonly AnimeHistory[];
  comic: readonly ReadRecord[];
  novel: readonly NovelHistoryEntry[];
}

const KIND_ORDER: Record<UnifiedMediaHistoryKind, number> = { anime: 0, comic: 1, novel: 2 };

function safeTimestamp(value: string | number | undefined): number {
  const timestamp = typeof value === "number" ? value : Date.parse(value ?? "");
  return Number.isFinite(timestamp) && timestamp > 0 ? timestamp : 0;
}

function clampProgress(value: number): number {
  return Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));
}

function animePosition(entry: AnimeHistory): string {
  return entry.lastEpisodeName?.trim() || (entry.lastEpisode >= 0 ? `第 ${entry.lastEpisode + 1} 集` : "已开始");
}

function comicPosition(entry: ReadRecord): string {
  return entry.last_title?.trim() || (entry.last_order > 0 ? `第 ${entry.last_order} 话` : "已开始");
}

function novelPosition(entry: NovelHistoryEntry): string {
  const chapter = entry.chapterTitle?.trim() || "已开始";
  const progress = clampProgress(entry.progress);
  return progress > 0 ? `${chapter} · ${Math.round(progress * 100)}%` : chapter;
}

/**
 * 将三个媒体 store 的本地历史转换成一个稳定、可排序、可恢复的时间线。
 * 这里只做纯数据归一化，不写入新的存储，也不改变旧历史字段语义。
 */
export function buildUnifiedMediaHistory(input: UnifiedMediaHistoryInput, limit = 100): UnifiedMediaHistoryItem[] {
  const items: UnifiedMediaHistoryItem[] = [];

  for (const entry of input.anime) {
    const key = entry.key || `${entry.ruleName}:${entry.name}`;
    if (!key || !entry.name?.trim()) continue;
    items.push({
      id: `anime:${key}`,
      kind: "anime",
      title: entry.name,
      cover: entry.image || null,
      sourceLabel: entry.ruleName || "番剧源",
      positionLabel: animePosition(entry),
      progress: null,
      updatedAt: safeTimestamp(entry.updatedAt),
      payload: entry,
    });
  }

  for (const entry of input.comic) {
    if (!entry.id || !entry.title?.trim()) continue;
    items.push({
      id: `comic:${entry.id}`,
      kind: "comic",
      title: entry.title,
      cover: entry.thumb_url || null,
      sourceLabel: entry.author?.trim() || "漫画来源",
      positionLabel: comicPosition(entry),
      progress: null,
      updatedAt: safeTimestamp(entry.ts),
      payload: entry,
    });
  }

  for (const entry of input.novel) {
    if (!entry.key || !entry.book.title?.trim()) continue;
    const progress = clampProgress(entry.progress);
    items.push({
      id: `novel:${entry.key}`,
      kind: "novel",
      title: entry.book.title,
      cover: entry.book.coverUrl || null,
      sourceLabel: entry.book.source,
      positionLabel: novelPosition(entry),
      progress,
      updatedAt: safeTimestamp(entry.updatedAt),
      payload: entry,
    });
  }

  return items
    .sort((a, b) => b.updatedAt - a.updatedAt || KIND_ORDER[a.kind] - KIND_ORDER[b.kind] || a.id.localeCompare(b.id))
    .slice(0, Math.max(0, limit));
}

export function mediaHistoryKindLabel(kind: UnifiedMediaHistoryKind): string {
  return kind === "anime" ? "番剧" : kind === "comic" ? "漫画" : "小说";
}

export function mediaHistoryActionLabel(kind: UnifiedMediaHistoryKind): string {
  return kind === "anime" ? "继续观看" : "继续阅读";
}
