import { loadNovelDetail, readNovelChapter, searchNovels } from "./api";
import { offlineApi } from "../../api/offline";
import { findOfflineChapter } from "../offline/model";
import { bookKey, readingRepository, type ReadingPosition } from "../reading-history/repository";
import type {
  NovelBook,
  NovelChapter,
  NovelChapterContent,
  NovelDetail,
  NovelHistoryEntry,
  NovelSource,
} from "./types";

const HISTORY_KEY = "moeplay-novel-history-v1";
const ALL_SEARCH_SOURCES: ReadonlyArray<Exclude<NovelSource, "all">> = [
  "biquge",
  "x80",
  "internetarchive",
  "openlibrary",
  "standardebooks",
  "gutenberg",
  "wikisource",
];

function loadHistory(): NovelHistoryEntry[] {
  if (typeof localStorage === "undefined") return [];
  try {
    const parsed = JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]") as NovelHistoryEntry[];
    return Array.isArray(parsed) ? parsed.filter(e => e?.book?.id && e.chapterId && Number.isFinite(e.progress)) : [];
  } catch {
    return [];
  }
}

function historyKey(book: NovelBook, chapterId: string) {
  return JSON.stringify([book.source, book.id, chapterId]);
}

let _source = $state<NovelSource>("all");
let _query = $state("");
let _books = $state<NovelBook[]>([]);
let _detail = $state<NovelDetail | null>(null);
let _content = $state<NovelChapterContent | null>(null);
let _history = $state<NovelHistoryEntry[]>(loadHistory());
let _historyReady = $state(false);
let _historyError = $state("");
let _detailRequest = 0;
let _chapterRequest = 0;
readingRepository.subscribe(() => {
  _historyReady = readingRepository.ready;
  _historyError = readingRepository.error;
  if (!_historyReady) return;
  _history = readingRepository.positions.filter(p => p.kind === "novel" && p.metadata.book).map(p => ({
    key: historyKey(p.metadata.book as unknown as NovelBook, p.chapterId),
    book: p.metadata.book as unknown as NovelBook, chapterId: p.chapterId,
    chapterTitle: p.chapterTitle, progress: p.progress ?? 0, updatedAt: p.updatedAt,
  })).sort((a, b) => b.updatedAt - a.updatedAt);
});
if (typeof indexedDB !== "undefined") void readingRepository.init().catch(() => {});
let _view = $state<"home" | "detail" | "reader">("home");
let _loading = $state(false);
let _error = $state("");
let _searchRequest = 0;
let _sourcesTotal = $state(0);
let _sourcesDone = $state(0);

export const novelStore = {
  get source() { return _source; },
  get query() { return _query; },
  get books() { return _books; },
  get detail() { return _detail; },
  get content() { return _content; },
  get history() {
    const seen = new Set<string>();
    return _history.filter(entry => {
      const key = JSON.stringify([entry.book.source, entry.book.id]);
      if (seen.has(key)) return false;
      seen.add(key); return true;
    });
  },
  get historyReady() { return _historyReady; },
  get historyError() { return _historyError; },
  get view() { return _view; },
  get loading() { return _loading; },
  get error() { return _error; },
  get sourcesTotal() { return _sourcesTotal; },
  get sourcesDone() { return _sourcesDone; },

  setSource(source: NovelSource) {
    if (_source === source) return;
    _source = source;
    _searchRequest += 1;
    _books = [];
    _error = "";
    _loading = false;
    _sourcesTotal = 0;
    _sourcesDone = 0;
  },

  async search(query = _query) {
    const normalized = query.trim();
    if (!normalized) return;
    const source = _source;
    const request = ++_searchRequest;
    _query = normalized;
    _loading = true;
    _error = "";
    if (source !== "all") {
      _sourcesTotal = 0;
      _sourcesDone = 0;
      try {
        const books = await searchNovels(source, normalized);
        if (request !== _searchRequest || source !== _source) return;
        _books = books;
        _view = "home";
      } catch (error) {
        if (request !== _searchRequest || source !== _source) return;
        _books = [];
        _error = String(error);
      } finally {
        if (request === _searchRequest) _loading = false;
      }
      return;
    }
    // “全部”源改为逐源并发：哪个源先返回就先展示，慢源超时不再拖垮整体（0.19.5）。
    _books = [];
    _sourcesTotal = ALL_SEARCH_SOURCES.length;
    _sourcesDone = 0;
    _view = "home";
    const seen = new Set<string>();
    const errors: string[] = [];
    await Promise.all(ALL_SEARCH_SOURCES.map(async (item) => {
      try {
        const books = await searchNovels(item, normalized);
        if (request !== _searchRequest || _source !== "all") return;
        const fresh = books.filter((book) => {
          const key = `${book.source}:${book.id}`;
          if (seen.has(key)) return false;
          seen.add(key);
          return true;
        });
        if (fresh.length > 0) _books = [..._books, ...fresh];
      } catch (error) {
        errors.push(String(error));
      } finally {
        if (request === _searchRequest) _sourcesDone += 1;
      }
    }));
    if (request !== _searchRequest || _source !== "all") return;
    if (_books.length === 0 && errors.length > 0) _error = errors.join("；");
    _loading = false;
  },

  cancel() {
    if (!_loading) return;
    _searchRequest += 1;
    _loading = false;
    _sourcesDone = _sourcesTotal;
  },

  async openBook(book: NovelBook) {
    const request = ++_detailRequest;
    ++_chapterRequest;
    _loading = true;
    _error = "";
    _content = null;
    _detail = null;
    try {
      const detail = await loadNovelDetail(book.source, book.id);
      if (request !== _detailRequest) return;
      _detail = detail;
      _view = "detail";
    } catch (error) {
      if (request !== _detailRequest) return;
      _error = String(error);
    } finally {
      if (request === _detailRequest) _loading = false;
    }
  },

  async readChapter(chapter: NovelChapter) {
    const request = ++_chapterRequest;
    const book = _detail?.book;
    if (!book) {
      _error = "请先打开作品详情，再选择章节";
      return;
    }
    _loading = true;
    _error = "";
    try {
      await readingRepository.init();
      let content: NovelChapterContent | undefined;
      try {
        const offline = findOfflineChapter(await offlineApi.list(), {
          contentType: "novel", sourceId: book.source, contentId: book.id, chapterId: chapter.id,
        });
        if (offline?.readable) {
          const cached = await offlineApi.getChapter(offline.offlineChapterKey);
          if (!cached.body) throw new Error("离线章节正文为空");
          content = { bookId: book.id, source: book.source, chapter, content: cached.body };
        }
      } catch { /* absent/corrupt cache falls through to the source API */ }
      if (!content) content = await readNovelChapter(book.source, book.id, chapter.id);
      if (request !== _chapterRequest) return;
      _content = content;
      _view = "reader";
      this.setProgress(0, false);
    } catch (error) {
      if (request !== _chapterRequest) return;
      _error = String(error);
    } finally {
      if (request === _chapterRequest) _loading = false;
    }
  },

  /** Read chapter content for offline storage without changing reader state or history. */
  async resolveChapterForOffline(chapter: NovelChapter): Promise<{ body: string }> {
    const book = _detail?.book;
    if (!book) throw new Error("请先打开作品详情，再选择章节");
    return { body: (await readNovelChapter(book.source, book.id, chapter.id)).content };
  },

  setProgress(progress: number, overwrite = true) {
    if (!_historyReady || !Number.isFinite(progress)) return;
    const book = _detail?.book;
    const chapter = _content?.chapter;
    if (!book || !chapter) return;
    const key = historyKey(book, chapter.id);
    const previous = _history.find((entry) => entry.key === key);
    const value = overwrite ? progress : (previous?.progress ?? progress);
    const entry: NovelHistoryEntry = {
      key,
      book,
      chapterId: chapter.id,
      chapterTitle: chapter.title,
      progress: Math.max(0, Math.min(1, value)),
      updatedAt: Date.now(),
    };
    _history = [entry, ..._history.filter((item) => item.key !== key)];
    const position: ReadingPosition = { kind: "novel", source: book.source, contentId: book.id,
      title: book.title, chapterId: chapter.id, chapterTitle: chapter.title,
      progress: entry.progress, updatedAt: entry.updatedAt,
      metadata: { book: JSON.parse(JSON.stringify(book)) } };
    void readingRepository.save(position).catch(() => {});
  },

  async removeHistory(entry: NovelHistoryEntry) {
    await readingRepository.remove(bookKey({ kind: "novel", source: entry.book.source, contentId: entry.book.id }));
  },

  progressFor(book: NovelBook, chapterId: string) {
    return _history.find((entry) => entry.key === historyKey(book, chapterId))?.progress ?? 0;
  },

  async resume(entry: NovelHistoryEntry) {
    await this.openBook(entry.book);
    const chapter = _detail?.chapters.find((item) => item.id === entry.chapterId) ?? _detail?.chapters[0];
    if (chapter) await this.readChapter(chapter);
  },

  showDetail() {
    ++_chapterRequest;
    _loading = false;
    if (_detail) _view = "detail";
  },

  showHome() {
    ++_detailRequest;
    ++_chapterRequest;
    _loading = false;
    _view = "home";
    _content = null;
    _detail = null;
    _error = "";
  },
};
