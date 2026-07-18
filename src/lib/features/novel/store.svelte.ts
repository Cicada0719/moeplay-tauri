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

  setSource(source: NovelSource) { _source = source; },

  async search(query = _query) {
    const normalized = query.trim();
    if (!normalized) return;
    _query = normalized;
    _loading = true;
    _error = "";
    try {
      _books = await searchNovels(_source, normalized);
      _view = "home";
    } catch (error) {
      _books = [];
      _error = String(error);
    } finally {
      _loading = false;
    }
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
