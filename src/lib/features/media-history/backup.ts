import { historyStore, type AnimeHistory } from "../anime-player/historyStore.svelte";
import {
  chapterKey,
  readingRepository,
  validPosition,
  type ReadingPosition,
} from "../reading-history/repository";

export const MEDIA_HISTORY_FORMAT = "moeplay-media-history" as const;
export const MEDIA_HISTORY_VERSION = 1 as const;

export interface AnimeBackupRecord {
  /** Stable source/work identity. */
  key: string;
  contentId: string;
  sourceId: string;
  sourceUrl: string;
  title: string;
  cover: string | null;
  roadIndex: number;
  roadName: string;
  episodeIndex: number;
  episodeId: string;
  episodeTitle: string;
  progressMs: number;
  updatedAt: number;
}

export interface MangaBackupRecord {
  sourceId: string;
  contentId: string;
  title: string;
  cover: string | null;
  chapterId: string;
  chapterTitle: string;
  pageIndex: number;
  pageId?: string;
  updatedAt: number;
}

export interface NovelBackupRecord {
  sourceId: string;
  contentId: string;
  title: string;
  cover: string | null;
  chapterId: string;
  chapterTitle: string;
  progress: number;
  updatedAt: number;
}

export interface MediaHistoryBackup {
  format: typeof MEDIA_HISTORY_FORMAT;
  version: typeof MEDIA_HISTORY_VERSION;
  exportedAt: number;
  coverage: ["anime", "manga", "novel"];
  anime: AnimeBackupRecord[];
  manga: MangaBackupRecord[];
  novel: NovelBackupRecord[];
  /** Compatibility mirror for clients that still consume reading-history v2 text backups. */
  positions: ReadingPosition[];
  /** Explicit marker retained for old UI/tests that identify the v2 text backup family. */
  legacyFormat: "moeplay-reading-history";
  errors: { anime: number; manga: number; novel: number; total: number };
  missing?: string[];
}

interface ParsedBackup {
  backup: MediaHistoryBackup;
  positions: ReadingPosition[];
  skipped: number;
}

export interface MediaHistoryImportPreview {
  added: number;
  updated: number;
  skipped: number;
  total: number;
  byType: {
    anime: { added: number; updated: number; skipped: number };
    manga: { added: number; updated: number; skipped: number };
    novel: { added: number; updated: number; skipped: number };
  };
  missing: string[];
}

export interface MediaHistoryImportResult extends MediaHistoryImportPreview {
  imported: number;
  failed: number;
  storage: {
    anime: { imported: number; failed: number };
    reading: { imported: number; failed: number };
  };
}

function asString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value : undefined;
}

function asNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function nested(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? value as Record<string, unknown> : {};
}

function timestamp(value: unknown): number | undefined {
  const number = asNumber(value);
  if (number !== undefined && number > 0) return Math.floor(number);
  if (typeof value === "string") {
    const parsed = Date.parse(value);
    if (Number.isFinite(parsed) && parsed > 0) return parsed;
  }
  return undefined;
}

/** Remove credentials commonly embedded in source URLs before serialisation. */
export function sanitizeBackupUrl(value: unknown): string {
  const raw = asString(value);
  if (!raw) return "";
  try {
    const url = new URL(raw);
    for (const key of [...url.searchParams.keys()]) {
      if (/(token|password|passwd|secret|auth|credential|api[-_]?key|access[-_]?key|download)/i.test(key)) {
        url.searchParams.set(key, "[redacted]");
      }
    }
    url.username = "";
    url.password = "";
    return url.toString();
  } catch {
    return raw.replace(/(token|password|passwd|secret|auth|credential|api[-_]?key)=([^&\s]+)/gi, "$1=[redacted]");
  }
}

function animeRecord(value: unknown): AnimeBackupRecord | undefined {
  if (!value || typeof value !== "object") return undefined;
  const raw = value as Record<string, unknown>;
  const source = nested(raw.source);
  const work = nested(raw.work);
  const episode = nested(raw.episode);
  const road = nested(raw.road);
  const key = asString(raw.key) ?? asString(raw.contentId) ?? asString(work.id);
  const title = asString(raw.title) ?? asString(raw.name) ?? asString(work.title);
  const sourceId = asString(raw.sourceId) ?? asString(raw.ruleName) ?? asString(raw.source) ?? asString(source.id) ?? asString(source.name);
  const progressMs = asNumber(raw.progressMs) ?? asNumber(episode.progressMs);
  const updatedAt = timestamp(raw.updatedAt);
  const roadIndex = asNumber(raw.roadIndex) ?? asNumber(raw.lastRoad) ?? asNumber(road.index);
  const episodeIndex = asNumber(raw.episodeIndex) ?? asNumber(raw.lastEpisode) ?? asNumber(episode.index);
  if (!key || !title || !sourceId || progressMs === undefined || progressMs < 0 || updatedAt === undefined
    || roadIndex === undefined || !Number.isInteger(roadIndex) || roadIndex < 0
    || episodeIndex === undefined || !Number.isInteger(episodeIndex) || episodeIndex < 0) return undefined;
  return {
    key, contentId: asString(raw.contentId) ?? key, sourceId, sourceUrl: sanitizeBackupUrl(raw.sourceUrl ?? source.url), title,
    cover: asString(raw.cover) ?? asString(raw.image) ?? null,
    roadIndex, roadName: asString(raw.roadName) ?? asString(road.name) ?? "",
    episodeIndex, episodeId: asString(raw.episodeId) ?? asString(episode.id) ?? String(episodeIndex),
    episodeTitle: asString(raw.episodeTitle) ?? asString(raw.lastEpisodeName) ?? asString(episode.title) ?? "",
    progressMs: Math.floor(progressMs), updatedAt,
  };
}

function animeToBackup(entry: AnimeHistory): AnimeBackupRecord | undefined {
  return animeRecord({
    key: entry.key, sourceId: entry.ruleName, sourceUrl: entry.sourceUrl, title: entry.name,
    image: entry.image, lastRoad: entry.lastRoad, lastEpisode: entry.lastEpisode,
    lastEpisodeName: entry.lastEpisodeName, progressMs: entry.progressMs, updatedAt: entry.updatedAt,
  });
}

function animeToStore(entry: AnimeBackupRecord): AnimeHistory {
  return { key: entry.key, name: entry.title, image: entry.cover ?? "", ruleName: entry.sourceId,
    sourceUrl: entry.sourceUrl, lastRoad: entry.roadIndex, lastEpisode: entry.episodeIndex,
    lastEpisodeName: entry.episodeTitle, progressMs: entry.progressMs,
    updatedAt: new Date(entry.updatedAt).toISOString() };
}

function mangaRecord(value: unknown): MangaBackupRecord | undefined {
  if (!value || typeof value !== "object") return undefined;
  const raw = value as Record<string, unknown>;
  const source = nested(raw.source);
  const work = nested(raw.work);
  const sourceId = asString(raw.sourceId) ?? asString(raw.source) ?? asString(source.id) ?? asString(source.name);
  const contentId = asString(raw.contentId) ?? asString(work.id);
  const title = asString(raw.title) ?? asString(work.title);
  const chapterId = asString(raw.chapterId);
  const updatedAt = timestamp(raw.updatedAt);
  const pageIndex = asNumber(raw.pageIndex);
  if (!sourceId || !contentId || !title || !chapterId || updatedAt === undefined
    || pageIndex === undefined || !Number.isInteger(pageIndex) || pageIndex < 0) return undefined;
  return { sourceId, contentId, title, cover: asString(raw.cover) ?? null, chapterId,
    chapterTitle: asString(raw.chapterTitle) ?? "", pageIndex, pageId: asString(raw.pageId), updatedAt };
}

function novelRecord(value: unknown): NovelBackupRecord | undefined {
  if (!value || typeof value !== "object") return undefined;
  const raw = value as Record<string, unknown>;
  const source = nested(raw.source);
  const work = nested(raw.work);
  const sourceId = asString(raw.sourceId) ?? asString(raw.source) ?? asString(source.id) ?? asString(source.name);
  const contentId = asString(raw.contentId) ?? asString(work.id);
  const title = asString(raw.title) ?? asString(work.title);
  const chapterId = asString(raw.chapterId);
  const progress = asNumber(raw.progress);
  const updatedAt = timestamp(raw.updatedAt);
  if (!sourceId || !contentId || !title || !chapterId || progress === undefined || progress < 0 || progress > 1 || updatedAt === undefined) return undefined;
  return { sourceId, contentId, title, cover: asString(raw.cover) ?? null, chapterId,
    chapterTitle: asString(raw.chapterTitle) ?? "", progress, updatedAt };
}

function mangaToPosition(entry: MangaBackupRecord): ReadingPosition {
  return { kind: "comic", source: entry.sourceId, contentId: entry.contentId, title: entry.title,
    chapterId: entry.chapterId, chapterTitle: entry.chapterTitle, pageIndex: entry.pageIndex,
    pageId: entry.pageId, updatedAt: entry.updatedAt, metadata: { backup: true, cover: entry.cover } };
}

function novelToPosition(entry: NovelBackupRecord): ReadingPosition {
  const book = { id: entry.contentId, source: entry.sourceId, title: entry.title, subjects: [], publicDomain: false,
    sourceUrl: "", ...(entry.cover ? { coverUrl: entry.cover } : {}) };
  return { kind: "novel", source: entry.sourceId, contentId: entry.contentId, title: entry.title,
    chapterId: entry.chapterId, chapterTitle: entry.chapterTitle, progress: entry.progress,
    updatedAt: entry.updatedAt, metadata: { book } };
}

function emptyBackup(exportedAt = Date.now()): MediaHistoryBackup {
  return { format: MEDIA_HISTORY_FORMAT, version: MEDIA_HISTORY_VERSION, exportedAt,
    coverage: ["anime", "manga", "novel"], anime: [], manga: [], novel: [], positions: [],
    legacyFormat: "moeplay-reading-history",
    errors: { anime: 0, manga: 0, novel: 0, total: 0 } };
}

/** Keep the compatibility mirror useful without copying arbitrary metadata or credentials. */
function safeCompatibilityPosition(position: ReadingPosition): ReadingPosition {
  if (position.kind === "novel") {
    const book = position.metadata.book as Record<string, unknown> | undefined;
    return {
      ...position,
      metadata: {
        book: {
          id: position.contentId,
          source: position.source,
          title: position.title,
          subjects: Array.isArray(book?.subjects) ? book.subjects : [],
          publicDomain: book?.publicDomain === true,
          sourceUrl: sanitizeBackupUrl(book?.sourceUrl),
          ...(asString(book?.coverUrl) ? { coverUrl: sanitizeBackupUrl(book?.coverUrl) } : {}),
        },
      },
    };
  }
  return {
    ...position,
    metadata: {
      ...(asString(position.metadata.cover) ? { cover: sanitizeBackupUrl(position.metadata.cover) } : {}),
    },
  };
}

/** Parse supported backup formats while isolating malformed records. */
export function parseMediaHistoryBackup(input: string | unknown): ParsedBackup {
  let value: unknown = input;
  if (typeof input === "string") {
    try { value = JSON.parse(input); } catch { return { backup: emptyBackup(), positions: [], skipped: 1 }; }
  }
  const backup = emptyBackup();
  let skipped = 0;
  if (Array.isArray(value)) {
    // Legacy arrays are accepted only when each row has an unmistakable shape.
    for (const item of value) {
      const anime = animeRecord(item);
      if (anime) { backup.anime.push(anime); continue; }
      const raw = item as Record<string, unknown> | null;
      const book = raw && typeof raw === "object" ? raw.book as Record<string, unknown> | undefined : undefined;
      if (book && asString(book.id) && asString(book.source) && asString(book.title) && asString(raw?.chapterId)) {
        const novel = novelRecord({ sourceId: book.source, contentId: book.id, title: book.title,
          cover: book.coverUrl, chapterId: raw?.chapterId, chapterTitle: raw?.chapterTitle,
          progress: raw?.progress, updatedAt: raw?.updatedAt });
        if (novel) { backup.novel.push(novel); continue; }
      }
      // Recognise the old comic localStorage row by its explicit id/order/timestamp.
      if (raw && typeof raw.id === "string" && asString(raw.title) && asNumber(raw.last_order) !== undefined && timestamp(raw.ts) !== undefined) {
        const idMatch = /^(mangadex|baozi|dm5|ikkk):(.+)$/.exec(raw.id);
        const manga = mangaRecord({ sourceId: idMatch?.[1] ?? "picacg", contentId: idMatch?.[2] ?? raw.id,
          title: raw.title, chapterId: String(raw.last_order), chapterTitle: raw.last_title,
          pageIndex: raw.pageIndex ?? 0, pageId: raw.pageId, updatedAt: raw.ts });
        if (manga) { backup.manga.push(manga); continue; }
      }
      skipped += 1;
    }
  } else if (value && typeof value === "object") {
    const raw = value as Record<string, unknown>;
    if (raw.format === "moeplay-reading-history" && raw.version === 2 && Array.isArray(raw.positions)) {
      for (const item of raw.positions) {
        if (validPosition(item)) {
          const position = item;
          if (position.kind === "comic") backup.manga.push({ sourceId: position.source, contentId: position.contentId,
            title: position.title, cover: asString(position.metadata.cover) ?? null, chapterId: position.chapterId,
            chapterTitle: position.chapterTitle, pageIndex: position.pageIndex ?? 0, pageId: position.pageId, updatedAt: position.updatedAt });
          else backup.novel.push({ sourceId: position.source, contentId: position.contentId, title: position.title,
            cover: asString((position.metadata.book as Record<string, unknown>).coverUrl) ?? null,
            chapterId: position.chapterId, chapterTitle: position.chapterTitle, progress: position.progress ?? 0, updatedAt: position.updatedAt });
        } else skipped += 1;
      }
    } else if (raw.format === MEDIA_HISTORY_FORMAT && raw.version === MEDIA_HISTORY_VERSION) {
      for (const item of Array.isArray(raw.anime) ? raw.anime : []) { const row = animeRecord(item); row ? backup.anime.push(row) : skipped += 1; }
      for (const item of Array.isArray(raw.manga) ? raw.manga : []) { const row = mangaRecord(item); row ? backup.manga.push(row) : skipped += 1; }
      for (const item of Array.isArray(raw.novel) ? raw.novel : []) { const row = novelRecord(item); row ? backup.novel.push(row) : skipped += 1; }
      // A few pre-v0.24 clients only understand the v2 positions mirror. Use it
      // as a fallback when the typed arrays are absent, never in addition to them.
      if (!backup.manga.length && !backup.novel.length && Array.isArray(raw.positions)) {
        for (const item of raw.positions) {
          if (!validPosition(item)) { skipped += 1; continue; }
          if (item.kind === "comic") {
            backup.manga.push({ sourceId: item.source, contentId: item.contentId, title: item.title,
              cover: asString(item.metadata.cover) ?? null, chapterId: item.chapterId,
              chapterTitle: item.chapterTitle, pageIndex: item.pageIndex ?? 0, pageId: item.pageId, updatedAt: item.updatedAt });
          } else {
            const book = item.metadata.book as Record<string, unknown>;
            backup.novel.push({ sourceId: item.source, contentId: item.contentId, title: item.title,
              cover: asString(book.coverUrl) ?? null, chapterId: item.chapterId,
              chapterTitle: item.chapterTitle, progress: item.progress ?? 0, updatedAt: item.updatedAt });
          }
        }
      }
    } else skipped += 1;
  } else skipped += 1;
  backup.errors = { anime: 0, manga: 0, novel: 0, total: skipped };
  return { backup, positions: [...backup.manga.map(mangaToPosition), ...backup.novel.map(novelToPosition)], skipped };
}

export async function buildMediaHistoryBackup(): Promise<MediaHistoryBackup> {
  const backup = emptyBackup();
  for (const entry of historyStore.items) {
    const row = animeToBackup(entry);
    if (row) backup.anime.push(row); else backup.errors.anime += 1;
  }
  const snapshot = await readingRepository.positionsForBackup();
  backup.positions = snapshot.positions.map(safeCompatibilityPosition);
  for (const position of snapshot.positions) {
    if (position.kind === "comic") {
      const row = mangaRecord({ sourceId: position.source, contentId: position.contentId, title: position.title,
        cover: (position.metadata.cover as string | undefined), chapterId: position.chapterId,
        chapterTitle: position.chapterTitle, pageIndex: position.pageIndex ?? 0, pageId: position.pageId, updatedAt: position.updatedAt });
      if (row) backup.manga.push(row); else backup.errors.manga += 1;
    } else {
      const book = position.metadata.book as Record<string, unknown> | undefined;
      const row = novelRecord({ sourceId: position.source, contentId: position.contentId, title: position.title,
        cover: book?.coverUrl, chapterId: position.chapterId, chapterTitle: position.chapterTitle,
        progress: position.progress ?? 0, updatedAt: position.updatedAt });
      if (row) backup.novel.push(row); else backup.errors.novel += 1;
    }
  }
  if (!snapshot.storageAvailable) backup.missing = snapshot.missing;
  backup.errors.total = backup.errors.anime + backup.errors.manga + backup.errors.novel;
  return backup;
}

export async function exportMediaHistory(): Promise<string> {
  return JSON.stringify(await buildMediaHistoryBackup(), null, 2);
}

export async function previewMediaHistoryImport(input: string | unknown): Promise<MediaHistoryImportPreview> {
  const parsed = parseMediaHistoryBackup(input);
  const anime = historyStore.previewImport(parsed.backup.anime.map(animeToStore));
  const reading = await readingRepository.previewPositions(parsed.positions);
  const snapshot = await readingRepository.positionsForBackup();
  const mangaCount = parsed.backup.manga.length;
  const novelCount = parsed.backup.novel.length;
  const byType = {
    anime,
    manga: { added: reading.added && mangaCount ? parsed.positions.filter(p => p.kind === "comic").length : 0, updated: 0, skipped: 0 },
    novel: { added: 0, updated: 0, skipped: 0 },
  };
  // Per-type counts are calculated against the stable chapter key to keep manga and novel identities separate.
  const current = snapshot.positions;
  const currentMap = new Map(current.map(p => [chapterKey(p), p]));
  byType.manga = { added: 0, updated: 0, skipped: 0 }; byType.novel = { added: 0, updated: 0, skipped: 0 };
  for (const p of parsed.positions) {
    const type = p.kind === "comic" ? byType.manga : byType.novel;
    const old = currentMap.get(chapterKey(p));
    if (!old) type.added += 1; else if (p.updatedAt > old.updatedAt) type.updated += 1; else type.skipped += 1;
  }
  const skipped = parsed.skipped + anime.skipped + byType.manga.skipped + byType.novel.skipped;
  return { added: anime.added + reading.added, updated: anime.updated + reading.updated,
    skipped, total: parsed.backup.anime.length + parsed.positions.length + skipped, byType,
    missing: snapshot.storageAvailable ? [] : snapshot.missing };
}

export async function importMediaHistory(input: string | unknown): Promise<MediaHistoryImportResult> {
  const parsed = parseMediaHistoryBackup(input);
  const preview = await previewMediaHistoryImport(input);
  const anime = historyStore.importEntries(parsed.backup.anime.map(animeToStore));
  const reading = await readingRepository.importPositions(parsed.positions);
  const imported = anime.imported + reading.imported;
  const failed = anime.failed + reading.failed;
  return { ...preview, imported, failed,
    storage: { anime: { imported: anime.imported, failed: anime.failed }, reading: { imported: reading.imported, failed: reading.failed } } };
}

// Explicit aliases make the module convenient for UI callers and future adapters.
export const createMediaHistoryBackup = buildMediaHistoryBackup;
export const createBackup = buildMediaHistoryBackup;
export const exportBackup = exportMediaHistory;
export const parseBackup = parseMediaHistoryBackup;
export const previewBackupImport = previewMediaHistoryImport;
export const importBackup = importMediaHistory;
