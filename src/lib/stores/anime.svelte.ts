import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ── 类型 ──────────────────────────────────────────────────────────────────

export interface AnimeRule {
  name: string;
  version: string;
  baseUrl: string;
  searchURL: string;
  searchList: string;
  searchName: string;
  searchResult: string;
  chapterRoads: string;
  chapterResult: string;
  muliSources: boolean;
  useWebview: boolean;
  useNativePlayer: boolean;
  usePost: boolean;
  useLegacyParser: boolean;
  adBlocker: boolean;
  userAgent: string;
  referer: string;
  api: string;
  type: string;
}

export interface SearchItem {
  name: string;
  url: string;
}

export interface Episode {
  name: string;
  url: string;
}

export interface Road {
  name: string;
  episodes: Episode[];
}

export interface AnimeCollect {
  key: string;
  name: string;
  image: string;
  collectType: number; // 1=在看 2=想看 3=搁置 4=看过 5=抛弃
  ruleSource?: string;
  sourceUrl?: string;
  updatedAt: string;
}

export interface AnimeHistory {
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

export const COLLECT_TYPES: Record<number, string> = {
  0: "未收藏",
  1: "在看",
  2: "想看",
  3: "搁置",
  4: "看过",
  5: "抛弃",
};

// ── GitHub 规则仓库类型 ─────────────────────────────────────────────────

export interface RuleCatalogItem {
  name: string;
  version: string;
  useNativePlayer: boolean;
  antiCrawlerEnabled: boolean;
  author: string;
  lastUpdate: number;
}

// ── Bangumi 类型 ────────────────────────────────────────────────────────

export interface BangumiSubject {
  id: number;
  name: string;
  name_cn: string;
  image: string;
  summary: string;
  air_date: string;
  air_weekday: number;
  rating: number;
  rank: number;
  eps_count: number;
}

export interface BangumiCalendarDay {
  weekday: number;
  weekday_cn: string;
  items: BangumiSubject[];
}

export interface BangumiSubjectDetail {
  id: number; name: string; name_cn: string; summary: string;
  date: string; image: string; rating_score: number; rating_total: number;
  rank: number; tags: BangumiTag[];
}
export interface BangumiTag { name: string; count: number; }
export interface BangumiRatingDetail {
  score: number; total: number; count: number[]; // 0 unused, 1-10
}
export interface BangumiCharacter {
  id: number; name: string; name_cn: string; image: string;
  actors: { id: number; name: string; name_cn: string; }[];
}
export interface BangumiPerson {
  id: number; name: string; name_cn: string; image: string; jobs: string[];
}
export interface BangumiComment {
  user: string; avatar: string; rate: number; comment: string; date: string;
}

export interface BangumiCollectionEntry {
  subject_id: number;
  subject_name: string;
  subject_name_cn: string;
  subject_image: string;
  collection_type: number; // 1=在看 2=想看 3=搁置 4=看过 5=抛弃 (local types)
  updated_at: string;
}

// ── DanDanPlay 弹幕类型 ─────────────────────────────────────────────────

export interface DanmakuComment {
  time: number;
  mode: number; // 1=scroll, 4=bottom, 5=top
  color: number;
  text: string;
}

export interface DanmakuEpisode {
  episode_id: number;
  episode_title: string;
}

export interface DanmakuAnime {
  anime_id: number;
  anime_title: string;
  episodes: DanmakuEpisode[];
}

// ── trace.moe 图片搜番类型 ──────────────────────────────────────────────

export interface TraceMoeResult {
  anilist_id: number;
  filename: string;
  episode: string;
  from: number;
  to: number;
  similarity: number;
  video: string;
  image: string;
  title_native: string;
  title_chinese: string;
  title_english: string;
}

// ── Bangumi 章节评论类型 ────────────────────────────────────────────────

export interface BangumiEpisodeComment {
  user: string;
  avatar: string;
  comment: string;
  date: string;
}

// ── localStorage 键 ──────────────────────────────────────────────────────

const RULES_KEY = "anime-rules";
const COLLECT_KEY = "anime-collect";
const HISTORY_KEY = "anime-history";
const BANGUMI_TOKEN_KEY = "bangumi-token";
const BANGUMI_USERNAME_KEY = "bangumi-username";
const BANGUMI_SYNC_PRIORITY_KEY = "bangumi-sync-priority"; // 0=localFirst, 1=bangumiFirst

function loadJson<T>(key: string, fallback: T): T {
  if (typeof localStorage === "undefined") return fallback;
  try {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : fallback;
  } catch { return fallback; }
}
function saveJson(key: string, data: unknown) {
  if (typeof localStorage !== "undefined") localStorage.setItem(key, JSON.stringify(data));
}

// ── 响应式状态 ────────────────────────────────────────────────────────────

let _rules = $state<AnimeRule[]>(loadJson(RULES_KEY, []));
let _loading = $state(false);
let _error = $state<string | null>(null);

// 导航 — Kazumi 风格: 推荐 | 时间表 | 我的 | 规则
let _view = $state<"home" | "search" | "detail" | "player">("home");
let _activeTab = $state<"recommend" | "calendar" | "my" | "rules">("recommend");

// 搜索
let _searchKeyword = $state("");
let _searchResults = $state<[string, SearchItem[]][]>([]);
let _selectedRule = $state<string | null>(null);
let _searchToken = 0; // 防止旧的流式监听污染新一次搜索
let _playGeneration = 0; // playEpisode 代际计数器，防止旧提取事件污染状态

// 详情 (选中番剧的线路/集)
let _detailName = $state("");
let _detailUrl = $state("");
let _detailRuleName = $state("");
let _detailImage = $state("");
let _roads = $state<Road[]>([]);

// Bangumi 详情
let _detailSubject = $state<BangumiSubjectDetail | null>(null);
let _detailRating = $state<BangumiRatingDetail | null>(null);
let _detailCharacters = $state<BangumiCharacter[]>([]);
let _detailPersons = $state<BangumiPerson[]>([]);
let _detailComments = $state<BangumiComment[]>([]);
let _detailTab = $state<'overview' | 'comments' | 'characters' | 'staff'>('overview');
let _drawerOpen = $state(false);
let _playerExtractStatus = $state<'idle' | 'extracting' | 'found' | 'timeout' | 'error'>('idle');
let _playerVideoSrc = $state('');
let _playerIsM3u8 = $state(false);
let _sourceSheetOpen = $state(false);
// 单调递增的"打开"序号。每次打开播放源面板 +1，SourceSheet 据此触发一次搜索。
// 取代旧的 prevOpen 布尔边沿检测 —— 布尔会在反复进出后与真实状态错位，导致
// 「开始观看没反应」；序号每次必变，永不错位。
let _sourceSheetNonce = $state(0);

// 播放器
let _playerUrl = $state("");
let _playerRuleName = $state("");
let _playerEpisodeName = $state("");
let _playerRoadIdx = $state(0);
let _playerEpisodeIdx = $state(0);

// 收藏 & 历史
let _collection = $state<AnimeCollect[]>(loadJson(COLLECT_KEY, []));
let _history = $state<AnimeHistory[]>(loadJson(HISTORY_KEY, []));

// GitHub 规则仓库
let _catalog = $state<RuleCatalogItem[]>([]);
let _catalogLoading = $state(false);
let _catalogError = $state<string | null>(null);
let _installingRules = $state<Set<string>>(new Set());

// Bangumi 时间表
let _calendar = $state<BangumiCalendarDay[]>([]);
let _calendarLoading = $state(false);
let _calendarDay = $state(new Date().getDay() || 7); // 1=Mon..7=Sun

// 推荐页 — 多板块
let _recTrending = $state<BangumiSubject[]>([]);
let _recTrendingTotal = $state(0);
let _recTrendingLoading = $state(false);
let _recTrendingOffset = $state(0);

let _recSeasonal = $state<BangumiSubject[]>([]);
let _recSeasonalTotal = $state(0);
let _recSeasonalLoading = $state(false);
let _recSeasonalOffset = $state(0);

let _recTopRated = $state<BangumiSubject[]>([]);
let _recTopRatedTotal = $state(0);
let _recTopRatedLoading = $state(false);
let _recTopRatedOffset = $state(0);

let _recInitialized = $state(false);

// 我的 — 子 tab
let _mySubTab = $state<"collection" | "history" | "stats">("collection");
let _collectFilter = $state(0); // 0=全部, 1-5=对应类型

// Bangumi 收藏同步
let _bangumiToken = $state(loadJson<string>(BANGUMI_TOKEN_KEY, ""));
let _bangumiUsername = $state(loadJson<string>(BANGUMI_USERNAME_KEY, ""));
let _bangumiCollections = $state<BangumiCollectionEntry[]>([]);
let _bangumiSyncLoading = $state(false);
let _bangumiSyncError = $state<string | null>(null);
let _bangumiSyncProgress = $state("");
let _bangumiSyncPriority = $state(loadJson<number>(BANGUMI_SYNC_PRIORITY_KEY, 0)); // 0=localFirst

// 图片代理缓存 (原始URL → asset URL)
let _imgCache = $state<Record<string, string>>({});

// 播放器设置
let _pendingSeekMs = $state(0); // 续播目标进度（毫秒）
let _autoNext = $state(loadJson<boolean>('player-auto-next', true)); // 自动连播
let _playbackRate = $state(loadJson<number>('player-playback-rate', 1)); // 默认倍速
let _longPressRate = $state(loadJson<number>('player-long-press-rate', 3)); // 长按倍速
let _skipOpening = $state(loadJson<number>('player-skip-opening', 0)); // 跳片头（秒）
let _skipEnding = $state(loadJson<number>('player-skip-ending', 0)); // 跳片尾（秒）

// 弹幕设置
let _danmakuEnabled = $state(loadJson<boolean>('danmaku-enabled', true));
let _danmakuOpacity = $state(loadJson<number>('danmaku-opacity', 1));
let _danmakuSpeed = $state(loadJson<number>('danmaku-speed', 1));
let _danmakuFontSize = $state(loadJson<number>('danmaku-font-size', 24));
let _danmakuArea = $state(loadJson<number>('danmaku-area', 1)); // 0=1/4 1=1/2 2=全屏
let _danmakuBlockScroll = $state(loadJson<boolean>('danmaku-block-scroll', false));
let _danmakuBlockTop = $state(loadJson<boolean>('danmaku-block-top', false));
let _danmakuBlockBottom = $state(loadJson<boolean>('danmaku-block-bottom', false));
let _danmakuBlockWords = $state<string[]>(loadJson('danmaku-block-words', []));

// 搜索历史（旧版 SearchDrawer 存的是 {keyword,timestamp}[]，自动迁移为 string[]）
function loadSearchHistory(): string[] {
  const raw = loadJson<unknown[]>('anime-search-history', []);
  return raw.map(item =>
    typeof item === 'string' ? item : (item as any)?.keyword ?? ''
  ).filter(Boolean);
}
let _searchHistory = $state<string[]>(loadSearchHistory());
let _danmakuComments = $state<DanmakuComment[]>([]);
let _danmakuLoading = $state(false);
let _danmakuAnimeId = $state(0);
let _danmakuEpisodeId = $state(0);

// 图片搜番状态
let _imageSearchResults = $state<TraceMoeResult[]>([]);
let _imageSearchLoading = $state(false);
let _imageSearchError = $state<string | null>(null);

// 章节评论状态
let _episodeComments = $state<BangumiEpisodeComment[]>([]);
let _episodeCommentsLoading = $state(false);

function currentSeason(): { gte: string; lte: string } {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth() + 1;
  let q: number;
  if (m <= 3) q = 1;
  else if (m <= 6) q = 4;
  else if (m <= 9) q = 7;
  else q = 10;
  const qEnd = q + 2;
  return {
    gte: `${y}-${String(q).padStart(2, "0")}-01`,
    lte: `${y}-${String(qEnd).padStart(2, "0")}-${qEnd === 12 ? 31 : qEnd === 2 ? 28 : qEnd === 3 ? 31 : 30}`,
  };
}

export const animeStore = {
  // ── 访问器 ────────────────────────────────────────────────────────────
  get rules() { return _rules; },
  get loading() { return _loading; },
  get error() { return _error; },
  get view() { return _view; },
  get activeTab() { return _activeTab; },
  get searchKeyword() { return _searchKeyword; },
  get searchResults() { return _searchResults; },
  get selectedRule() { return _selectedRule; },
  get detailName() { return _detailName; },
  get detailUrl() { return _detailUrl; },
  get detailRuleName() { return _detailRuleName; },
  get detailImage() { return _detailImage; },
  get roads() { return _roads; },
  get playerUrl() { return _playerUrl; },
  get playerRuleName() { return _playerRuleName; },
  get playerEpisodeName() { return _playerEpisodeName; },
  get playerRoadIdx() { return _playerRoadIdx; },
  get playerEpisodeIdx() { return _playerEpisodeIdx; },
  get collection() { return _collection; },
  get history() { return _history; },
  get catalog() { return _catalog; },
  get catalogLoading() { return _catalogLoading; },
  get catalogError() { return _catalogError; },
  get installingRules() { return _installingRules; },
  get calendar() { return _calendar; },
  get calendarLoading() { return _calendarLoading; },
  get calendarDay() { return _calendarDay; },
  set calendarDay(v: number) { _calendarDay = v; },
  get imgCache() { return _imgCache; },

  // 推荐页
  get recTrending() { return _recTrending; },
  get recTrendingLoading() { return _recTrendingLoading; },
  get recTrendingTotal() { return _recTrendingTotal; },
  get recSeasonal() { return _recSeasonal; },
  get recSeasonalLoading() { return _recSeasonalLoading; },
  get recSeasonalTotal() { return _recSeasonalTotal; },
  get recTopRated() { return _recTopRated; },
  get recTopRatedLoading() { return _recTopRatedLoading; },
  get recTopRatedTotal() { return _recTopRatedTotal; },
  get recInitialized() { return _recInitialized; },

  // 我的
  get mySubTab() { return _mySubTab; },
  set mySubTab(v: "collection" | "history" | "stats") { _mySubTab = v; },
  get collectFilter() { return _collectFilter; },
  set collectFilter(v: number) { _collectFilter = v; },

  // Bangumi 收藏同步
  get bangumiToken() { return _bangumiToken; },
  get bangumiUsername() { return _bangumiUsername; },
  get bangumiCollections() { return _bangumiCollections; },
  get bangumiSyncLoading() { return _bangumiSyncLoading; },
  get bangumiSyncError() { return _bangumiSyncError; },
  get bangumiSyncProgress() { return _bangumiSyncProgress; },
  get bangumiSyncPriority() { return _bangumiSyncPriority; },
  set bangumiSyncPriority(v: number) { _bangumiSyncPriority = v; saveJson(BANGUMI_SYNC_PRIORITY_KEY, v); },
  get bangumiConnected() { return !!_bangumiToken && !!_bangumiUsername; },

  // Bangumi 详情
  get detailSubject() { return _detailSubject; },
  get detailRating() { return _detailRating; },
  get detailCharacters() { return _detailCharacters; },
  get detailPersons() { return _detailPersons; },
  get detailComments() { return _detailComments; },
  get detailTab() { return _detailTab; },
  set detailTab(v) { _detailTab = v; },
  get drawerOpen() { return _drawerOpen; },
  set drawerOpen(v) { _drawerOpen = v; },
  get playerExtractStatus() { return _playerExtractStatus; },
  set playerExtractStatus(v) { _playerExtractStatus = v; },
  get playerVideoSrc() { return _playerVideoSrc; },
  get playerIsM3u8() { return _playerIsM3u8; },
  get sourceSheetOpen() { return _sourceSheetOpen; },
  set sourceSheetOpen(v: boolean) { _sourceSheetOpen = v; },
  get sourceSheetNonce() { return _sourceSheetNonce; },
  /** 打开播放源面板。每次都 bump nonce，保证 SourceSheet 重新搜索（修复反复进出后无反应）。 */
  openSourceSheet() {
    _sourceSheetNonce++;
    _sourceSheetOpen = true;
  },

  // 弹幕
  get danmakuEnabled() { return _danmakuEnabled; },
  set danmakuEnabled(v: boolean) { _danmakuEnabled = v; saveJson('danmaku-enabled', v); },
  get danmakuComments() { return _danmakuComments; },
  get danmakuLoading() { return _danmakuLoading; },
  get danmakuAnimeId() { return _danmakuAnimeId; },
  get danmakuEpisodeId() { return _danmakuEpisodeId; },

  // 播放器设置
  get pendingSeekMs() { return _pendingSeekMs; },
  set pendingSeekMs(v: number) { _pendingSeekMs = v; },
  get autoNext() { return _autoNext; },
  set autoNext(v: boolean) { _autoNext = v; saveJson('player-auto-next', v); },
  get playbackRate() { return _playbackRate; },
  set playbackRate(v: number) { _playbackRate = v; saveJson('player-playback-rate', v); },
  get longPressRate() { return _longPressRate; },
  set longPressRate(v: number) { _longPressRate = v; saveJson('player-long-press-rate', v); },
  get skipOpening() { return _skipOpening; },
  set skipOpening(v: number) { _skipOpening = v; saveJson('player-skip-opening', v); },
  get skipEnding() { return _skipEnding; },
  set skipEnding(v: number) { _skipEnding = v; saveJson('player-skip-ending', v); },

  // 弹幕设置
  get danmakuOpacity() { return _danmakuOpacity; },
  set danmakuOpacity(v: number) { _danmakuOpacity = v; saveJson('danmaku-opacity', v); },
  get danmakuSpeed() { return _danmakuSpeed; },
  set danmakuSpeed(v: number) { _danmakuSpeed = v; saveJson('danmaku-speed', v); },
  get danmakuFontSize() { return _danmakuFontSize; },
  set danmakuFontSize(v: number) { _danmakuFontSize = v; saveJson('danmaku-font-size', v); },
  get danmakuArea() { return _danmakuArea; },
  set danmakuArea(v: number) { _danmakuArea = v; saveJson('danmaku-area', v); },
  get danmakuBlockScroll() { return _danmakuBlockScroll; },
  set danmakuBlockScroll(v: boolean) { _danmakuBlockScroll = v; saveJson('danmaku-block-scroll', v); },
  get danmakuBlockTop() { return _danmakuBlockTop; },
  set danmakuBlockTop(v: boolean) { _danmakuBlockTop = v; saveJson('danmaku-block-top', v); },
  get danmakuBlockBottom() { return _danmakuBlockBottom; },
  set danmakuBlockBottom(v: boolean) { _danmakuBlockBottom = v; saveJson('danmaku-block-bottom', v); },
  get danmakuBlockWords() { return _danmakuBlockWords; },
  set danmakuBlockWords(v: string[]) { _danmakuBlockWords = v; saveJson('danmaku-block-words', v); },

  // 搜索历史
  get searchHistory() { return _searchHistory; },
  addSearchHistory(keyword: string) {
    const trimmed = keyword.trim();
    if (!trimmed) return;
    _searchHistory = [trimmed, ..._searchHistory.filter(k => k !== trimmed)].slice(0, 20);
    saveJson('anime-search-history', _searchHistory);
  },
  removeSearchHistory(keyword: string) {
    _searchHistory = _searchHistory.filter(k => k !== keyword);
    saveJson('anime-search-history', _searchHistory);
  },
  clearSearchHistory() {
    _searchHistory = [];
    saveJson('anime-search-history', _searchHistory);
  },

  // 图片搜番
  get imageSearchResults() { return _imageSearchResults; },
  get imageSearchLoading() { return _imageSearchLoading; },
  get imageSearchError() { return _imageSearchError; },

  // 章节评论
  get episodeComments() { return _episodeComments; },
  get episodeCommentsLoading() { return _episodeCommentsLoading; },

  get filteredCollection(): AnimeCollect[] {
    if (_collectFilter === 0) return _collection;
    return _collection.filter(c => c.collectType === _collectFilter);
  },

  get stats() {
    const total = _collection.length;
    const watching = _collection.filter(c => c.collectType === 1).length;
    const planned = _collection.filter(c => c.collectType === 2).length;
    const onHold = _collection.filter(c => c.collectType === 3).length;
    const watched = _collection.filter(c => c.collectType === 4).length;
    const dropped = _collection.filter(c => c.collectType === 5).length;
    const historyCount = _history.length;
    const rulesCount = _rules.length;
    return { total, watching, planned, onHold, watched, dropped, historyCount, rulesCount };
  },

  // ── 初始化 ────────────────────────────────────────────────────────────

  async init() {
    if (_rules.length > 0) {
      await invoke("anime_set_rules", { rules: _rules }).catch(() => {});
    }
    // video-url-found/timeout 不需要事件监听 — anime_extract_video_url 命令直接返回结果
  },

  // ── 规则管理 ──────────────────────────────────────────────────────────

  async addRule(rule: AnimeRule) {
    await invoke("anime_add_rule", { rule });
    const idx = _rules.findIndex((r) => r.name === rule.name);
    if (idx >= 0) _rules[idx] = rule; else _rules = [..._rules, rule];
    saveJson(RULES_KEY, _rules);
  },

  async removeRule(name: string) {
    await invoke("anime_remove_rule", { name });
    _rules = _rules.filter((r) => r.name !== name);
    saveJson(RULES_KEY, _rules);
  },

  async importRules(json: string): Promise<number> {
    const count = await invoke<number>("anime_import_rules", { json });
    _rules = await invoke<AnimeRule[]>("anime_get_rules");
    saveJson(RULES_KEY, _rules);
    return count;
  },

  // ── GitHub 规则仓库 ──────────────────────────────────────────────────

  async loadCatalog() {
    _catalogLoading = true;
    _catalogError = null;
    try {
      _catalog = await invoke<RuleCatalogItem[]>("anime_github_rules_index");
    } catch (e) {
      _catalogError = String(e);
    } finally {
      _catalogLoading = false;
    }
  },

  isRuleInstalled(name: string): boolean {
    return _rules.some((r) => r.name === name);
  },

  getRuleVersion(name: string): string | null {
    return _rules.find((r) => r.name === name)?.version ?? null;
  },

  isRuleInstalling(name: string): boolean {
    return _installingRules.has(name);
  },

  async installRule(name: string) {
    _installingRules = new Set([..._installingRules, name]);
    try {
      const rule = await invoke<AnimeRule>("anime_install_github_rule", { name });
      const idx = _rules.findIndex((r) => r.name === rule.name);
      if (idx >= 0) _rules[idx] = rule; else _rules = [..._rules, rule];
      saveJson(RULES_KEY, _rules);
    } catch (e) {
      _error = `安装规则 ${name} 失败: ${e}`;
    } finally {
      const next = new Set(_installingRules);
      next.delete(name);
      _installingRules = next;
    }
  },

  async installAllRules() {
    if (_catalog.length === 0) return;
    const names = _catalog.map((c) => c.name);
    _catalogLoading = true;
    try {
      const count = await invoke<number>("anime_install_all_github_rules", { names });
      _rules = await invoke<AnimeRule[]>("anime_get_rules");
      saveJson(RULES_KEY, _rules);
      _error = null;
      return count;
    } catch (e) {
      _error = String(e);
    } finally {
      _catalogLoading = false;
    }
  },

  // ── Bangumi 时间表 ──────────────────────────────────────────────────

  async loadCalendar() {
    if (_calendar.length > 0) return;
    _calendarLoading = true;
    try {
      _calendar = await invoke<BangumiCalendarDay[]>("anime_bangumi_calendar");
      const urls: string[] = [];
      for (const day of _calendar) {
        for (const item of day.items) {
          if (item.image) urls.push(item.image);
        }
      }
      this._proxyImages(urls);
    } catch (e) {
      _error = String(e);
    } finally {
      _calendarLoading = false;
    }
  },

  _proxyImages(urls: string[]) {
    const unique = [...new Set(urls)].filter(u => !_imgCache[u]);
    if (unique.length === 0) return;
    for (const url of unique) {
      invoke<string>("anime_proxy_image", { url }).then(localPath => {
        _imgCache = { ..._imgCache, [url]: convertFileSrc(localPath) };
      }).catch(() => {});
    }
  },

  getImg(url: string): string {
    return _imgCache[url] || "";
  },

  // ── 推荐页 ─────────────────────────────────────────────────────────

  async loadRecommendations() {
    if (_recInitialized) return;
    _recInitialized = true;
    this._loadTrending(false);
    this._loadSeasonal(false);
    this._loadTopRated(false);
  },

  async _loadTrending(append: boolean) {
    _recTrendingLoading = true;
    try {
      const [items, total] = await invoke<[BangumiSubject[], number]>("anime_bangumi_search", {
        keyword: "", offset: _recTrendingOffset, sort: "heat",
      });
      _recTrending = append ? [..._recTrending, ...items] : items;
      _recTrendingTotal = total;
      _recTrendingOffset = _recTrendingOffset + items.length;
      this._proxyImages(items.filter(i => i.image).map(i => i.image));
    } catch { /* silent */ }
    _recTrendingLoading = false;
  },

  async _loadSeasonal(append: boolean) {
    _recSeasonalLoading = true;
    try {
      const season = currentSeason();
      const [items, total] = await invoke<[BangumiSubject[], number]>("anime_bangumi_search", {
        keyword: "", offset: _recSeasonalOffset, sort: "heat",
        airDateGte: season.gte, airDateLte: season.lte,
      });
      _recSeasonal = append ? [..._recSeasonal, ...items] : items;
      _recSeasonalTotal = total;
      _recSeasonalOffset = _recSeasonalOffset + items.length;
      this._proxyImages(items.filter(i => i.image).map(i => i.image));
    } catch { /* silent */ }
    _recSeasonalLoading = false;
  },

  async _loadTopRated(append: boolean) {
    _recTopRatedLoading = true;
    try {
      const [items, total] = await invoke<[BangumiSubject[], number]>("anime_bangumi_search", {
        keyword: "", offset: _recTopRatedOffset, sort: "rank",
      });
      _recTopRated = append ? [..._recTopRated, ...items] : items;
      _recTopRatedTotal = total;
      _recTopRatedOffset = _recTopRatedOffset + items.length;
      this._proxyImages(items.filter(i => i.image).map(i => i.image));
    } catch { /* silent */ }
    _recTopRatedLoading = false;
  },

  loadMoreTrending() { this._loadTrending(true); },
  loadMoreSeasonal() { this._loadSeasonal(true); },
  loadMoreTopRated() { this._loadTopRated(true); },

  async searchBangumi(keyword: string): Promise<BangumiSubject[]> {
    try {
      const [items] = await invoke<[BangumiSubject[], number]>("anime_bangumi_search", {
        keyword, offset: 0,
      });
      return items;
    } catch {
      return [];
    }
  },

  // ── Bangumi 详情 ──────────────────────────────────────────────────────

  async loadBangumiDetail(subjectId: number) {
    _detailSubject = null;
    _detailRating = null;
    _detailCharacters = [];
    _detailPersons = [];
    _detailComments = [];
    try {
      const [detail, rating] = await Promise.all([
        invoke<BangumiSubjectDetail>('anime_bangumi_detail', { subjectId }),
        invoke<BangumiRatingDetail>('anime_bangumi_rating', { subjectId }),
      ]);
      _detailSubject = detail;
      _detailRating = rating;
      if (detail.image) this._proxyImages([detail.image]);
    } catch (e) {
      console.warn('Failed to load bangumi detail:', e);
    }
  },

  async loadBangumiDetailByName(name: string) {
    try {
      const [items] = await invoke<[BangumiSubject[], number]>('anime_bangumi_search', {
        keyword: name, offset: 0, sort: 'match',
      });
      if (items.length > 0 && items[0].id) {
        await this.loadBangumiDetail(items[0].id);
      }
    } catch (e) {
      console.warn('Failed to load bangumi detail by name:', e);
    }
  },

  async loadBangumiCharacters(subjectId: number) {
    try {
      _detailCharacters = await invoke<BangumiCharacter[]>('anime_bangumi_characters', { subjectId });
    } catch { /* silent */ }
  },

  async loadBangumiPersons(subjectId: number) {
    try {
      _detailPersons = await invoke<BangumiPerson[]>('anime_bangumi_persons', { subjectId });
    } catch { /* silent */ }
  },

  async loadBangumiComments(subjectId: number) {
    try {
      _detailComments = await invoke<BangumiComment[]>('anime_bangumi_comments', { subjectId });
    } catch { /* silent */ }
  },

  async loadBangumiTab(tab: 'comments' | 'characters' | 'staff', subjectId: number) {
    try {
      if (tab === 'characters') {
        _detailCharacters = await invoke('anime_bangumi_characters', { subjectId });
      } else if (tab === 'staff') {
        _detailPersons = await invoke('anime_bangumi_persons', { subjectId });
      } else if (tab === 'comments') {
        _detailComments = await invoke('anime_bangumi_comments', { subjectId, offset: 0, limit: 20 });
      }
    } catch (e) {
      console.warn(`Failed to load ${tab}:`, e);
    }
  },

  // ── 搜索 ──────────────────────────────────────────────────────────────

  async search(keyword: string) {
    if (!keyword.trim()) return;
    _searchKeyword = keyword;
    _loading = true;
    _error = null;
    _searchResults = [];
    _view = "search";
    const token = ++_searchToken;

    // 单一来源：直接搜
    if (_selectedRule) {
      try {
        const items = await invoke<SearchItem[]>("anime_search", { ruleName: _selectedRule, keyword });
        if (token !== _searchToken) return;
        _searchResults = items.length > 0 ? [[_selectedRule, items]] : [];
        if (_searchResults.length === 0) _error = "未找到结果";
      } catch (e) {
        if (token === _searchToken) _error = String(e);
      } finally {
        if (token === _searchToken) _loading = false;
      }
      return;
    }

    // 全部来源：流式 —— 每条规则一出结果就追加，首批结果即隐藏 spinner（不再干等全部完成）
    const seen = new Set<string>();
    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<[string, SearchItem[]]>("anime-search-result", (ev) => {
        if (token !== _searchToken) return;
        const [source, items] = ev.payload;
        if (!source || seen.has(source)) return;
        seen.add(source);
        _searchResults = [..._searchResults, [source, items]];
        _loading = false;
      });
      await invoke("anime_search_all", { keyword });
      if (token !== _searchToken) return;
      if (_searchResults.length === 0) _error = "未找到结果";
    } catch (e) {
      if (token === _searchToken) _error = String(e);
    } finally {
      if (token === _searchToken) _loading = false;
      unlisten?.();
    }
  },

  setSelectedRule(name: string | null) {
    _selectedRule = name;
  },

  // ── 详情（线路/集）─────────────────────────────────────────────────────

  /// 从 Bangumi 封面进入详情页：用 subject.id 直接加载详情，**不触发插件搜索**。
  /// 插件搜索只在用户点「开始观看」打开 SourceSheet 时才发生（与 Kazumi 一致）。
  async openInfo(subject: BangumiSubject) {
    _error = null;
    _detailName = subject.name_cn || subject.name;
    _detailUrl = "";
    _detailRuleName = "";
    _detailImage = subject.image ?? "";
    _roads = [];
    _detailSubject = null;
    _detailRating = null;
    _detailCharacters = [];
    _detailPersons = [];
    _detailComments = [];
    _detailTab = 'overview';
    _sourceSheetOpen = false;
    _view = "detail";
    if (subject.image) this._proxyImages([subject.image]);
    if (subject.id) {
      await this.loadBangumiDetail(subject.id);
    } else {
      await this.loadBangumiDetailByName(_detailName);
    }
  },

  async openDetail(ruleName: string, item: SearchItem, image?: string) {
    _error = null;
    _detailName = item.name;
    _detailUrl = item.url;
    _detailRuleName = ruleName;
    _detailImage = image ?? "";
    _roads = [];
    _detailSubject = null;
    _detailRating = null;
    _detailTab = 'overview';
    _sourceSheetOpen = false;
    _view = "detail";

    // 只加载 Bangumi 详情，线路在 SourceSheet 中按需加载
    this.loadBangumiDetailByName(item.name);
  },

  closeDetail() {
    _view = _searchKeyword ? "search" : "home";
    _roads = [];
    _detailSubject = null;
    _detailRating = null;
    _detailCharacters = [];
    _detailPersons = [];
    _detailComments = [];
    _detailTab = 'overview';
    _sourceSheetOpen = false;
  },

  // ── 播放器 ─────────────────────────────────────────────────────────────

  /// SourceSheet 调用：设置线路数据供播放器使用
  setRoadsForPlayback(roads: Road[], ruleName: string, sourceUrl: string) {
    _roads = roads;
    _detailRuleName = ruleName;
    _detailUrl = sourceUrl;
  },

  async playEpisode(roadIdx: number, episodeIdx: number, seekMs?: number) {
    const road = _roads[roadIdx];
    if (!road) return;
    const ep = road.episodes[episodeIdx];
    if (!ep) return;

    _playerRoadIdx = roadIdx;
    _playerEpisodeIdx = episodeIdx;
    _playerEpisodeName = ep.name;
    _playerRuleName = _detailRuleName;
    _playerExtractStatus = 'extracting';
    _playerVideoSrc = '';
    _playerIsM3u8 = false;
    _playerUrl = '';
    _sourceSheetOpen = false; // 进播放器必关播放源面板，杜绝面板盖在播放器上 / 串台
    _view = "player";

    // 续播逻辑：优先用传入的 seekMs，否则查历史记录
    if (seekMs !== undefined && seekMs > 0) {
      _pendingSeekMs = seekMs;
    } else {
      const historyKey = `${_detailRuleName}:${_detailName}`;
      const history = _history.find(h => h.key === historyKey);
      if (history && history.lastRoad === roadIdx && history.lastEpisode === episodeIdx && history.progressMs > 3000) {
        // 超过 3 秒才续播，避免开头误触
        _pendingSeekMs = history.progressMs;
      } else {
        _pendingSeekMs = 0;
      }
    }

    console.log("[播放] playEpisode", { roadIdx, episodeIdx, rule: _detailRuleName });

    try {
      _playerUrl = await invoke<string>("anime_build_url", {
        ruleName: _detailRuleName, url: ep.url,
      });
    } catch {
      _playerUrl = ep.url;
    }

    // Also try to extract the real video URL (Rust command returns result directly via oneshot)
    const gen = ++_playGeneration;
    try {
      const rule = _rules.find(r => r.name === _detailRuleName);
      console.log("[播放] 开始提取视频 URL:", _playerUrl);
      const result = await invoke<{ url: string; tab_url?: string }>('anime_extract_video_url', {
        episodeUrl: _playerUrl,
        useLegacyParser: rule?.useLegacyParser ?? false,
      });
      if (gen !== _playGeneration) return; // 用户切了集数，丢弃旧结果

      console.log("[播放] 提取成功:", result.url);
      // 通过本地代理播放（解决 CORS / 防盗链 Referer）
      // 用播放器页地址做 Referer（CDN 防盗链认的是播放器域名，不是规则 baseUrl）
      const playerPageUrl = result.tab_url || rule?.baseUrl || '';
      console.log("[播放] Referer:", playerPageUrl);
      const proxyUrl = await invoke<string>('anime_get_proxy_url', {
        url: result.url,
        referer: playerPageUrl || null,
      });
      console.log("[播放] 代理 URL:", proxyUrl);
      _playerVideoSrc = proxyUrl;
      // isM3u8 的实际语义是"是否优先用 hls.js"。URL 含 m3u8 必然是；否则只要不是明显的直链媒体
      // 文件(mp4/mkv/...)，也默认走 hls.js —— 国产番源绝大多数是 HLS，且流地址常是无扩展名的
      // token/playlist，仅靠扩展名判断会漏判 → 被塞进原生 <video> 黑屏。万一猜错，播放器有原生↔hls 自动兜底。
      const realUrl = result.url.toLowerCase();
      const directFile = /\.(mp4|mkv|webm|flv|avi|mov|m4v|mp3|m4a|wmv|3gp)(\?|#|$)/.test(realUrl);
      _playerIsM3u8 = realUrl.includes('m3u8') || !directFile;
      _playerExtractStatus = 'found';
      console.log("[播放] 状态设为 found, isM3u8(优先hls):", _playerIsM3u8, "directFile:", directFile);
    } catch (e) {
      console.error("[播放] 提取失败:", e);
      if (gen !== _playGeneration) return;
      if (String(e).includes('timeout')) {
        _playerExtractStatus = 'timeout';
      } else {
        _playerExtractStatus = 'error';
      }
    }

    this._updateHistory(roadIdx, episodeIdx, ep.name, 0);

    // 自动搜索弹幕
    this.searchDanmakuForAnime(_detailName, episodeIdx);
  },

  /** 按番名搜索弹幕库，找到后加载对应集数的弹幕 */
  async searchDanmakuForAnime(animeName: string, episodeIdx?: number) {
    if (!animeName.trim()) return;
    _danmakuLoading = true;
    _danmakuComments = [];
    _danmakuAnimeId = 0;
    _danmakuEpisodeId = 0;
    try {
      const animes = await invoke<DanmakuAnime[]>('anime_danmaku_search', { keyword: animeName });
      if (animes.length === 0) {
        _danmakuLoading = false;
        return;
      }
      // 选最佳匹配（第一个结果，DanDanPlay 按相关度排序）
      const best = animes[0];
      _danmakuAnimeId = best.anime_id;
      if (best.episodes.length > 0) {
        // 尝试用集数索引匹配（DanDanPlay 分集从 1 开始）
        const epNum = episodeIdx !== undefined ? episodeIdx + 1 : 1;
        const matchedEp = best.episodes.find(ep => {
          // 尝试从标题中提取集数
          const match = ep.episode_title.match(/(\d+)/);
          return match ? parseInt(match[1]) === epNum : false;
        }) || best.episodes[Math.min(episodeIdx ?? 0, best.episodes.length - 1)];

        if (matchedEp) {
          _danmakuEpisodeId = matchedEp.episode_id;
          await this.loadDanmaku(matchedEp.episode_id);
        }
      }
    } catch (e) {
      console.warn('弹幕搜索失败:', e);
    } finally {
      _danmakuLoading = false;
    }
  },

  /** 加载指定分集的弹幕评论 */
  async loadDanmaku(episodeId: number) {
    _danmakuLoading = true;
    try {
      _danmakuComments = await invoke<DanmakuComment[]>('anime_danmaku_get_comments', { episodeId });
      _danmakuEpisodeId = episodeId;
    } catch (e) {
      console.warn('弹幕加载失败:', e);
      _danmakuComments = [];
    } finally {
      _danmakuLoading = false;
    }
  },

  closePlayer() {
    _view = "detail";
    _playerUrl = "";
    _playerVideoSrc = '';
    _playerExtractStatus = 'idle';
    _playerIsM3u8 = false;
    _sourceSheetOpen = false; // 回详情时确保面板是关的，避免残留状态串台
  },

  async prevEpisode() {
    if (_playerEpisodeIdx > 0) {
      await this.playEpisode(_playerRoadIdx, _playerEpisodeIdx - 1);
    }
  },

  async nextEpisode() {
    const road = _roads[_playerRoadIdx];
    if (road && _playerEpisodeIdx < road.episodes.length - 1) {
      await this.playEpisode(_playerRoadIdx, _playerEpisodeIdx + 1);
    }
  },

  updateProgress(ms: number) {
    this._updateHistory(_playerRoadIdx, _playerEpisodeIdx, _playerEpisodeName, ms);
  },

  // ── 收藏 ──────────────────────────────────────────────────────────────

  setCollect(name: string, collectType: number, extra?: Partial<AnimeCollect>) {
    const key = name;
    const idx = _collection.findIndex((c) => c.key === key);
    if (collectType === 0) {
      if (idx >= 0) {
        _collection = _collection.filter((c) => c.key !== key);
      }
    } else {
      const entry: AnimeCollect = {
        key,
        name,
        image: extra?.image ?? _detailImage ?? "",
        collectType,
        ruleSource: extra?.ruleSource ?? _detailRuleName,
        sourceUrl: extra?.sourceUrl ?? _detailUrl,
        updatedAt: new Date().toISOString(),
      };
      if (idx >= 0) _collection[idx] = entry;
      else _collection = [entry, ..._collection];
    }
    saveJson(COLLECT_KEY, _collection);
    // Auto-sync to Bangumi if connected (fire-and-forget)
    if (_bangumiToken && collectType > 0) {
      this.syncToBangumi(name, collectType);
    }
  },

  getCollectType(name: string): number {
    return _collection.find((c) => c.key === name)?.collectType ?? 0;
  },

  // ── 历史 ──────────────────────────────────────────────────────────────

  _updateHistory(roadIdx: number, epIdx: number, epName: string, progressMs: number) {
    const key = `${_detailRuleName}:${_detailName}`;
    const entry: AnimeHistory = {
      key,
      name: _detailName,
      image: _detailImage,
      ruleName: _detailRuleName,
      sourceUrl: _detailUrl,
      lastRoad: roadIdx,
      lastEpisode: epIdx,
      lastEpisodeName: epName,
      progressMs,
      updatedAt: new Date().toISOString(),
    };
    const idx = _history.findIndex((h) => h.key === key);
    if (idx >= 0) _history[idx] = entry;
    else _history = [entry, ..._history];
    if (_history.length > 200) _history = _history.slice(0, 200);
    saveJson(HISTORY_KEY, _history);
  },

  removeHistory(key: string) {
    _history = _history.filter((h) => h.key !== key);
    saveJson(HISTORY_KEY, _history);
  },

  clearHistory() {
    _history = [];
    saveJson(HISTORY_KEY, _history);
  },

  // ── Bangumi 收藏同步 ──────────────────────────────────────────────────

  /** 设置 Bangumi token 并测试连接 */
  async setBangumiToken(token: string): Promise<string> {
    _bangumiSyncError = null;
    if (!token.trim()) {
      _bangumiToken = "";
      _bangumiUsername = "";
      saveJson(BANGUMI_TOKEN_KEY, "");
      saveJson(BANGUMI_USERNAME_KEY, "");
      return "";
    }
    try {
      const username = await invoke<string>("anime_bangumi_get_username", { token });
      _bangumiToken = token;
      _bangumiUsername = username;
      saveJson(BANGUMI_TOKEN_KEY, token);
      saveJson(BANGUMI_USERNAME_KEY, username);
      return username;
    } catch (e) {
      _bangumiSyncError = String(e);
      throw e;
    }
  },

  /** 断开 Bangumi 连接 */
  disconnectBangumi() {
    _bangumiToken = "";
    _bangumiUsername = "";
    _bangumiCollections = [];
    saveJson(BANGUMI_TOKEN_KEY, "");
    saveJson(BANGUMI_USERNAME_KEY, "");
  },

  /** 从 Bangumi 拉取远程收藏 */
  async loadBangumiCollection() {
    if (!_bangumiToken) return;
    _bangumiSyncLoading = true;
    _bangumiSyncError = null;
    _bangumiSyncProgress = "正在拉取远程收藏...";
    try {
      const remote = await invoke<BangumiCollectionEntry[]>(
        "anime_bangumi_get_all_collections",
        { token: _bangumiToken, username: _bangumiUsername || null },
      );
      _bangumiCollections = remote;
      _bangumiSyncProgress = `拉取完成，共 ${remote.length} 条`;
      // Proxy images for remote entries
      const urls = remote.filter(e => e.subject_image).map(e => e.subject_image);
      this._proxyImages(urls);
    } catch (e) {
      _bangumiSyncError = String(e);
      _bangumiSyncProgress = "";
    } finally {
      _bangumiSyncLoading = false;
    }
  },

  /** 同步远程收藏到本地（乐观合并） */
  async syncBangumiToLocal() {
    if (!_bangumiToken || _bangumiCollections.length === 0) return;
    const priority = _bangumiSyncPriority; // 0=localFirst, 1=bangumiFirst
    const remote = _bangumiCollections;
    const remoteMap = new Map<string, BangumiCollectionEntry>();
    for (const entry of remote) {
      remoteMap.set(entry.subject_name, entry);
      if (entry.subject_name_cn) remoteMap.set(entry.subject_name_cn, entry);
    }

    if (priority === 1) {
      // Bangumi 优先：用远程数据覆盖本地
      for (const entry of remote) {
        const localType = this.getCollectType(entry.subject_name);
        if (localType !== entry.collection_type) {
          this.setCollect(entry.subject_name, entry.collection_type, {
            image: entry.subject_image,
          });
        }
      }
      _bangumiSyncProgress = "Bangumi 优先同步完成";
    } else {
      // 本地优先：只补本地缺失的
      for (const entry of remote) {
        const localType = this.getCollectType(entry.subject_name);
        if (localType === 0) {
          // 本地没有 → 从远程拉过来
          this.setCollect(entry.subject_name, entry.collection_type, {
            image: entry.subject_image,
          });
        }
      }
      _bangumiSyncProgress = "本地优先同步完成";
    }
  },

  /** 把本地收藏上传到 Bangumi（逐条同步） */
  async syncLocalToBangumi() {
    if (!_bangumiToken) return;
    _bangumiSyncLoading = true;
    _bangumiSyncError = null;
    let synced = 0;
    let failed = 0;
    // 需要 bangumiId 才能上传 — 只同步有 subject 的条目
    for (const c of _collection) {
      // 从 bangumiCollections 中查找对应的 subject_id
      const remote = _bangumiCollections.find(
        r => r.subject_name === c.name || r.subject_name_cn === c.name
      );
      if (!remote) continue;
      try {
        await invoke<boolean>("anime_bangumi_update_collection", {
          token: _bangumiToken,
          subjectId: remote.subject_id,
          collectionType: c.collectType,
        });
        synced++;
      } catch {
        failed++;
      }
    }
    _bangumiSyncProgress = `上传完成: ${synced} 成功, ${failed} 失败`;
    _bangumiSyncLoading = false;
  },

  /** 单条同步：收藏变化时自动推送 Bangumi */
  async syncToBangumi(name: string, collectType: number) {
    if (!_bangumiToken || !_bangumiUsername) return;
    // Find the subject ID from remote collections
    const remote = _bangumiCollections.find(
      r => r.subject_name === name || r.subject_name_cn === name
    );
    if (!remote) return; // 没有对应 Bangumi 条目，跳过
    try {
      await invoke<boolean>("anime_bangumi_update_collection", {
        token: _bangumiToken,
        subjectId: remote.subject_id,
        collectionType: collectType,
      });
    } catch (e) {
      console.warn("Bangumi 同步失败:", e);
    }
  },

  // ── 导航 ──────────────────────────────────────────────────────────────

  setTab(tab: "recommend" | "calendar" | "my" | "rules") {
    _activeTab = tab;
    _error = null;
    if (tab === "recommend" && !_recInitialized) {
      this.loadRecommendations();
    }
    if (tab === "calendar" && _calendar.length === 0) {
      this.loadCalendar();
    }
    if (tab === "rules" && _catalog.length === 0) {
      this.loadCatalog();
    }
  },

  goHome() {
    _view = "home";
    _searchKeyword = "";
    _searchResults = [];
    _error = null;
  },

  // ── 图片搜番 (trace.moe) ──────────────────────────────────────────────

  async imageSearch(imageUrl: string) {
    if (!imageUrl.trim()) return;
    _imageSearchLoading = true;
    _imageSearchError = null;
    _imageSearchResults = [];
    try {
      _imageSearchResults = await invoke<TraceMoeResult[]>('anime_image_search', { imageUrl });
    } catch (e) {
      _imageSearchError = String(e);
      _imageSearchResults = [];
    } finally {
      _imageSearchLoading = false;
    }
  },

  clearImageSearch() {
    _imageSearchResults = [];
    _imageSearchError = null;
  },

  // ── 章节评论 ──────────────────────────────────────────────────────────

  async loadEpisodeComments(episodeId: number) {
    _episodeCommentsLoading = true;
    _episodeComments = [];
    try {
      _episodeComments = await invoke<BangumiEpisodeComment[]>('anime_bangumi_episode_comments', { episodeId });
    } catch (e) {
      console.warn('章节评论加载失败:', e);
      _episodeComments = [];
    } finally {
      _episodeCommentsLoading = false;
    }
  },
};
