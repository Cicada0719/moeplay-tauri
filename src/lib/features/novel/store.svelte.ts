import { loadNovelDetail, readNovelChapter, searchNovels } from "./api";
import type {
  NovelBook,
  NovelChapter,
  NovelChapterContent,
  NovelDetail,
  NovelHistoryEntry,
  NovelSource,
} from "./types";

const HISTORY_KEY = "moeplay-novel-history-v1";
const MAX_HISTORY = 60;
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
    return Array.isArray(parsed) ? parsed.slice(0, MAX_HISTORY) : [];
  } catch {
    return [];
  }
}

function persistHistory(entries: NovelHistoryEntry[]) {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(HISTORY_KEY, JSON.stringify(entries.slice(0, MAX_HISTORY)));
}

function historyKey(book: NovelBook, chapterId: string) {
  return `${book.source}:${book.id}:${chapterId}`;
}

let _source = $state<NovelSource>("all");
let _query = $state("");
let _books = $state<NovelBook[]>([]);
let _detail = $state<NovelDetail | null>(null);
let _content = $state<NovelChapterContent | null>(null);
let _history = $state<NovelHistoryEntry[]>(loadHistory());
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
  get history() { return _history; },
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
    _loading = true;
    _error = "";
    _content = null;
    try {
      _detail = await loadNovelDetail(book.source, book.id);
      _view = "detail";
    } catch (error) {
      _error = String(error);
    } finally {
      _loading = false;
    }
  },

  async readChapter(chapter: NovelChapter) {
    const book = _detail?.book;
    if (!book) return;
    _loading = true;
    _error = "";
    try {
      _content = await readNovelChapter(book.source, book.id, chapter.id);
      _view = "reader";
      this.setProgress(0, false);
    } catch (error) {
      _error = String(error);
    } finally {
      _loading = false;
    }
  },

  setProgress(progress: number, overwrite = true) {
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
    _history = [entry, ..._history.filter((item) => item.key !== key)].slice(0, MAX_HISTORY);
    persistHistory(_history);
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
    if (_detail) _view = "detail";
  },

  showHome() {
    _view = "home";
    _content = null;
    _detail = null;
    _error = "";
  },
};
