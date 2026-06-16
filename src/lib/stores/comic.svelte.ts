import { invoke } from "@tauri-apps/api/core";

// ── 类型 ──────────────────────────────────────────────────────────────────

export interface ComicCategory {
  id: string;
  title: string;
  description: string;
  thumb_url: string;
}

export interface ComicSummary {
  id: string;
  title: string;
  author: string;
  thumb_url: string;
  categories: string[];
  likes_count: number;
  total_views: number;
  eps_count: number;
  finished: boolean;
}

export interface ComicDetail {
  id: string;
  title: string;
  author: string;
  description: string;
  thumb_url: string;
  categories: string[];
  tags: string[];
  likes_count: number;
  total_views: number;
  eps_count: number;
  pages_count: number;
  finished: boolean;
  is_liked: boolean;
  is_favourite: boolean;
  chinese_team: string;
  comments_count: number;
  allow_comment: boolean;
  updated_at: string;
  created_at: string;
}

export interface ComicChapter {
  id: string;
  title: string;
  order: number;
  updated_at: string;
}

export interface ComicImage {
  id: string;
  url: string;
}

export interface ComicListPage {
  docs: ComicSummary[];
  total: number;
  pages: number;
  page: number;
}

export interface ComicUserProfile {
  id: string;
  name: string;
  email: string;
  avatar_url: string;
  title: string;
  slogan: string;
  level: number;
  exp: number;
  gender: string;
  is_punched: boolean;
  character: string;
  created_at: string;
}

export interface ComicCommentUser {
  id: string;
  name: string;
  avatar_url: string;
  level: number;
  title: string;
  role: string;
  character: string;
  slogan: string;
}

export interface ComicComment {
  id: string;
  content: string;
  user: ComicCommentUser;
  created_at: string;
  likes_count: number;
  is_liked: boolean;
  comments_count: number;
  is_top: boolean;
}

export interface CommentsPage {
  docs: ComicComment[];
  total: number;
  pages: number;
  page: number;
}

export type SortMode = "dd" | "da" | "ld" | "vt";
export const SORT_OPTIONS: { value: SortMode; label: string }[] = [
  { value: "dd", label: "新到旧" },
  { value: "da", label: "旧到新" },
  { value: "ld", label: "最多喜欢" },
  { value: "vt", label: "最多观看" },
];

// ── 阅读历史（本地持久化）─────────────────────────────────────────────────

export interface ReadRecord {
  id: string;
  title: string;
  thumb_url: string;
  author: string;
  last_order: number;
  last_title: string;
  ts: number;
}

const HISTORY_KEY = "picacg-history";
const MAX_HISTORY = 100;

function loadHistory(): ReadRecord[] {
  try {
    return JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]");
  } catch { return []; }
}
function saveHistory(h: ReadRecord[]) {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(h.slice(0, MAX_HISTORY)));
}

// ── 响应式状态 ────────────────────────────────────────────────────────────

const TOKEN_KEY = "picacg-token";
const EMAIL_KEY = "picacg-email";

let _token = $state(
  typeof localStorage !== "undefined" ? (localStorage.getItem(TOKEN_KEY) ?? "") : ""
);
let _savedEmail = $state(
  typeof localStorage !== "undefined" ? (localStorage.getItem(EMAIL_KEY) ?? "") : ""
);
let _loading = $state(false);
let _error = $state<string | null>(null);

// 导航层级
let _view = $state<"home" | "detail" | "reader">("home");
let _activeTab = $state<"explore" | "ranking" | "favorites" | "random" | "history">("explore");

// 用户资料
let _profile = $state<ComicUserProfile | null>(null);

// 分类
let _categories = $state<ComicCategory[]>([]);
let _selectedCategory = $state<string | null>(null);
let _categoriesLoaded = $state(false);

// 排序
let _sort = $state<SortMode>("dd");

// 漫画列表
let _comicList = $state<ComicSummary[]>([]);
let _comicPage = $state(1);
let _comicPages = $state(1);
let _comicTotal = $state(0);

// 排行榜
let _ranking = $state<ComicSummary[]>([]);
let _rankingType = $state<"H24" | "D7" | "D30">("H24");
let _rankingLoaded = $state(false);

// 随机本子
let _randomList = $state<ComicSummary[]>([]);

// 收藏
let _favorites = $state<ComicSummary[]>([]);
let _favPage = $state(1);
let _favPages = $state(1);

// 搜索
let _searchKeyword = $state("");
let _searchResults = $state<ComicSummary[]>([]);
let _searchPage = $state(1);
let _searchPages = $state(1);

// 详情
let _currentComic = $state<ComicDetail | null>(null);
let _chapters = $state<ComicChapter[]>([]);
let _recommendations = $state<ComicSummary[]>([]);

// 评论
let _comments = $state<ComicComment[]>([]);
let _commentsPage = $state(1);
let _commentsPages = $state(1);
let _commentsTotal = $state(0);
let _commentsLoading = $state(false);

// 阅读器
let _readerImages = $state<ComicImage[]>([]);
let _readerChapterOrder = $state(1);
let _readerChapterTitle = $state("");
let _readerLoading = $state(false);

// 阅读历史
let _readHistory = $state<ReadRecord[]>(
  typeof localStorage !== "undefined" ? loadHistory() : []
);

export const comicStore = {
  // ── 状态访问 ────────────────────────────────────────────────────────────
  get token() { return _token; },
  get savedEmail() { return _savedEmail; },
  get isLoggedIn() { return _token.length > 0; },
  get loading() { return _loading; },
  get error() { return _error; },
  get view() { return _view; },
  get activeTab() { return _activeTab; },
  get profile() { return _profile; },
  get categories() { return _categories; },
  get selectedCategory() { return _selectedCategory; },
  get sort() { return _sort; },
  get comicList() { return _comicList; },
  get comicPage() { return _comicPage; },
  get comicPages() { return _comicPages; },
  get comicTotal() { return _comicTotal; },
  get ranking() { return _ranking; },
  get rankingType() { return _rankingType; },
  get randomList() { return _randomList; },
  get favorites() { return _favorites; },
  get favPage() { return _favPage; },
  get favPages() { return _favPages; },
  get searchKeyword() { return _searchKeyword; },
  set searchKeyword(v: string) { _searchKeyword = v; },
  get searchResults() { return _searchResults; },
  get searchPage() { return _searchPage; },
  get searchPages() { return _searchPages; },
  get currentComic() { return _currentComic; },
  get chapters() { return _chapters; },
  get recommendations() { return _recommendations; },
  get comments() { return _comments; },
  get commentsPage() { return _commentsPage; },
  get commentsPages() { return _commentsPages; },
  get commentsTotal() { return _commentsTotal; },
  get commentsLoading() { return _commentsLoading; },
  get readerImages() { return _readerImages; },
  get readerChapterOrder() { return _readerChapterOrder; },
  get readerChapterTitle() { return _readerChapterTitle; },
  get readerLoading() { return _readerLoading; },
  get readHistory() { return _readHistory; },

  clearError() { _error = null; },

  // ── 认证 ────────────────────────────────────────────────────────────────

  async rehydrate() {
    if (_token) {
      await invoke("comic_set_token", { token: _token }).catch(() => {});
    }
  },

  async login(email: string, password: string): Promise<void> {
    _loading = true;
    _error = null;
    try {
      const token = await invoke<string>("comic_login", { email, password });
      _token = token;
      _savedEmail = email;
      localStorage.setItem(TOKEN_KEY, token);
      localStorage.setItem(EMAIL_KEY, email);
    } catch (e) {
      _error = String(e);
      throw e;
    } finally {
      _loading = false;
    }
  },

  logout() {
    _token = "";
    localStorage.removeItem(TOKEN_KEY);
    invoke("comic_set_token", { token: "" }).catch(() => {});
    _view = "home";
    _activeTab = "explore";
    _profile = null;
    _categories = [];
    _categoriesLoaded = false;
    _comicList = [];
    _currentComic = null;
    _chapters = [];
    _readerImages = [];
    _comments = [];
    _recommendations = [];
    _randomList = [];
    _ranking = [];
    _favorites = [];
    _searchResults = [];
  },

  // ── 用户资料 ────────────────────────────────────────────────────────────

  async loadProfile() {
    try {
      _profile = await invoke<ComicUserProfile>("comic_profile");
    } catch { /* silent — profile is optional */ }
  },

  async punchIn(): Promise<boolean> {
    try {
      await invoke("comic_punch_in");
      if (_profile) _profile = { ..._profile, is_punched: true };
      return true;
    } catch (e) {
      _error = String(e);
      return false;
    }
  },

  // ── 分类 & 列表 ──────────────────────────────────────────────────────────

  async loadCategories() {
    if (_categoriesLoaded) return;
    _loading = true;
    _error = null;
    try {
      _categories = await invoke<ComicCategory[]>("comic_categories");
      _categoriesLoaded = true;
      if (_categories.length > 0 && !_selectedCategory) {
        await this.selectCategory(null);
      }
    } catch (e) {
      _error = String(e);
    } finally {
      _loading = false;
    }
  },

  async selectCategory(category: string | null) {
    _selectedCategory = category;
    _comicPage = 1;
    _comicList = [];
    await this._loadComicList();
  },

  setSort(sort: SortMode) {
    if (_sort === sort) return;
    _sort = sort;
    _comicPage = 1;
    _comicList = [];
    this._loadComicList();
  },

  async _loadComicList(page = 1) {
    _loading = true;
    _error = null;
    try {
      const result = await invoke<ComicListPage>("comic_list", {
        page,
        category: _selectedCategory,
        sort: _sort,
      });
      _comicList = page === 1 ? result.docs : [..._comicList, ...result.docs];
      _comicPage = result.page;
      _comicPages = result.pages;
      _comicTotal = result.total;
    } catch (e) {
      _error = String(e);
    } finally {
      _loading = false;
    }
  },

  async loadMoreComics() {
    if (_comicPage >= _comicPages || _loading) return;
    await this._loadComicList(_comicPage + 1);
  },

  // ── 搜索 ────────────────────────────────────────────────────────────────

  async search(keyword: string) {
    _searchKeyword = keyword;
    _searchPage = 1;
    _searchResults = [];
    _loading = true;
    _error = null;
    try {
      const result = await invoke<ComicListPage>("comic_search", {
        keyword, page: 1, sort: _sort,
      });
      _searchResults = result.docs;
      _searchPage = result.page;
      _searchPages = result.pages;
    } catch (e) {
      _error = String(e);
    } finally {
      _loading = false;
    }
  },

  async searchNextPage() {
    if (_searchPage >= _searchPages || _loading) return;
    _loading = true;
    try {
      const result = await invoke<ComicListPage>("comic_search", {
        keyword: _searchKeyword,
        page: _searchPage + 1,
        sort: _sort,
      });
      _searchResults = [..._searchResults, ...result.docs];
      _searchPage = result.page;
    } catch (e) {
      _error = String(e);
    } finally {
      _loading = false;
    }
  },

  // ── 排行榜 ───────────────────────────────────────────────────────────────

  async loadRanking(type: "H24" | "D7" | "D30" = "H24") {
    _rankingType = type;
    _rankingLoaded = false;
    _loading = true;
    _error = null;
    try {
      _ranking = await invoke<ComicSummary[]>("comic_ranking", { timeType: type });
      _rankingLoaded = true;
    } catch (e) {
      _error = String(e);
    } finally {
      _loading = false;
    }
  },

  // ── 随机本子 ─────────────────────────────────────────────────────────────

  async loadRandom() {
    _loading = true;
    _error = null;
    try {
      _randomList = await invoke<ComicSummary[]>("comic_random");
    } catch (e) {
      _error = String(e);
    } finally {
      _loading = false;
    }
  },

  // ── 收藏 ────────────────────────────────────────────────────────────────

  async loadFavorites(page = 1) {
    _loading = true;
    _error = null;
    try {
      const result = await invoke<ComicListPage>("comic_favorites", { page, sort: _sort });
      _favorites = page === 1 ? result.docs : [..._favorites, ...result.docs];
      _favPage = result.page;
      _favPages = result.pages;
    } catch (e) {
      _error = String(e);
    } finally {
      _loading = false;
    }
  },

  async toggleFavourite(id: string) {
    try {
      const action = await invoke<string>("comic_toggle_favourite", { id });
      if (_currentComic && _currentComic.id === id) {
        _currentComic = { ..._currentComic, is_favourite: action === "favourite" };
      }
      return action;
    } catch (e) {
      _error = String(e);
    }
  },

  // ── 点赞 ────────────────────────────────────────────────────────────────

  async toggleLike(id: string) {
    try {
      const action = await invoke<string>("comic_like", { id });
      if (_currentComic && _currentComic.id === id) {
        const liked = action === "like";
        _currentComic = {
          ..._currentComic,
          is_liked: liked,
          likes_count: _currentComic.likes_count + (liked ? 1 : -1),
        };
      }
      return action;
    } catch (e) {
      _error = String(e);
    }
  },

  // ── 详情 ────────────────────────────────────────────────────────────────

  async openComic(id: string) {
    _loading = true;
    _error = null;
    _view = "detail";
    _comments = [];
    _commentsPage = 1;
    _recommendations = [];
    try {
      const [detail, chapters] = await Promise.all([
        invoke<ComicDetail>("comic_detail", { id }),
        invoke<ComicChapter[]>("comic_chapters", { id }),
      ]);
      _currentComic = detail;
      _chapters = chapters;
      // 异步加载评论和推荐
      this.loadComments(id, 1);
      this.loadRecommendations(id);
    } catch (e) {
      _error = String(e);
      _view = "home";
    } finally {
      _loading = false;
    }
  },

  closeComic() {
    _view = "home";
    _currentComic = null;
    _chapters = [];
    _comments = [];
    _recommendations = [];
  },

  // ── 评论 ────────────────────────────────────────────────────────────────

  async loadComments(comicId: string, page: number) {
    _commentsLoading = true;
    try {
      const result = await invoke<CommentsPage>("comic_comments", { id: comicId, page });
      if (page === 1) {
        _comments = result.docs;
      } else {
        _comments = [..._comments, ...result.docs];
      }
      _commentsPage = result.page;
      _commentsPages = result.pages;
      _commentsTotal = result.total;
    } catch { /* silent */ }
    finally { _commentsLoading = false; }
  },

  async postComment(comicId: string, content: string): Promise<boolean> {
    try {
      await invoke("comic_post_comment", { id: comicId, content });
      await this.loadComments(comicId, 1);
      return true;
    } catch (e) {
      _error = String(e);
      return false;
    }
  },

  async likeComment(commentId: string) {
    try {
      const action = await invoke<string>("comic_comment_like", { commentId });
      const liked = action === "like";
      _comments = _comments.map(c =>
        c.id === commentId
          ? { ...c, is_liked: liked, likes_count: c.likes_count + (liked ? 1 : -1) }
          : c
      );
    } catch { /* silent */ }
  },

  async loadMoreComments() {
    if (!_currentComic || _commentsPage >= _commentsPages || _commentsLoading) return;
    await this.loadComments(_currentComic.id, _commentsPage + 1);
  },

  // ── 推荐 ────────────────────────────────────────────────────────────────

  async loadRecommendations(comicId: string) {
    try {
      _recommendations = await invoke<ComicSummary[]>("comic_recommendation", { id: comicId });
    } catch { /* silent */ }
  },

  // ── 阅读器 ───────────────────────────────────────────────────────────────

  async openChapter(order: number, title: string) {
    if (!_currentComic) return;
    _readerLoading = true;
    _readerChapterOrder = order;
    _readerChapterTitle = title;
    _readerImages = [];
    _view = "reader";
    try {
      _readerImages = await invoke<ComicImage[]>("comic_chapter_images", {
        id: _currentComic.id,
        order,
      });
      // 记录阅读历史
      this._recordHistory(order, title);
    } catch (e) {
      _error = String(e);
      _view = "detail";
    } finally {
      _readerLoading = false;
    }
  },

  closeReader() {
    _view = "detail";
    _readerImages = [];
  },

  async prevChapter() {
    const idx = _chapters.findIndex((c) => c.order === _readerChapterOrder);
    if (idx > 0) {
      const c = _chapters[idx - 1];
      await this.openChapter(c.order, c.title);
    }
  },

  async nextChapter() {
    const idx = _chapters.findIndex((c) => c.order === _readerChapterOrder);
    if (idx >= 0 && idx < _chapters.length - 1) {
      const c = _chapters[idx + 1];
      await this.openChapter(c.order, c.title);
    }
  },

  // ── 阅读历史 ─────────────────────────────────────────────────────────────

  _recordHistory(order: number, chapterTitle: string) {
    if (!_currentComic) return;
    const comic = _currentComic;
    const existing = _readHistory.filter(r => r.id !== comic.id);
    const record: ReadRecord = {
      id: comic.id,
      title: comic.title,
      thumb_url: comic.thumb_url,
      author: comic.author,
      last_order: order,
      last_title: chapterTitle,
      ts: Date.now(),
    };
    _readHistory = [record, ...existing].slice(0, MAX_HISTORY);
    saveHistory(_readHistory);
  },

  removeHistory(id: string) {
    _readHistory = _readHistory.filter(r => r.id !== id);
    saveHistory(_readHistory);
  },

  clearHistory() {
    _readHistory = [];
    saveHistory([]);
  },

  // ── Tab 切换 ────────────────────────────────────────────────────────────

  setTab(tab: "explore" | "ranking" | "favorites" | "random" | "history") {
    _activeTab = tab;
    _error = null;
    if (tab === "ranking" && !_rankingLoaded) {
      this.loadRanking("H24");
    }
    if (tab === "favorites") {
      _favorites = [];
      this.loadFavorites(1);
    }
    if (tab === "random") {
      this.loadRandom();
    }
  },
};
