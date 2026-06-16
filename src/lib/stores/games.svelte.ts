import { pinyin } from "pinyin-pro";
import {
  addGameByDialog,
  applyScrapeResult,
  deleteGame as deleteGameApi,
  getGames,
  launchGame,
  searchGames,
  setSaveDir,
  toggleFavorite as toggleFavoriteApi,
  type ScrapeResult,
  type Game as ApiGame,
} from "../api";
import {
  coverOf,
  developerOf,
  gameCompletionStatus,
  gameLastPlayed,
  normalizeGame,
  gameRating,
  gameTotalSeconds,
  isInstalled,
  originalNameOf,
  publisherOf,
  tagsOf,
  userFacingErrorMessage,
} from "../utils/game";

export type Game = ApiGame;

let _games = $state<Game[]>([]);
let _allGames = $state<Game[]>([]);
let _loading = $state(false);
let _loadError = $state<string | null>(null);
let _selectedId = $state<string | null>(null);
let _searchQuery = $state("");
let _filterTag = $state<string | null>(null);
let _quickFilter = $state<string | null>(null);
let _sortBy = $state("recent");

/** Check if any field's pinyin matches the search query */
function matchesPinyin(text: string | undefined | null, query: string): boolean {
  if (!text || !query) return false;
  const lower = text.toLowerCase();
  // Direct text match first
  if (lower.includes(query)) return true;
  try {
    // Full pinyin: "游戏王" → "youxiwang"
    const fullPy = pinyin(text, { toneType: "none", type: "string" }).toLowerCase();
    if (fullPy.includes(query)) return true;
    // First letters: "游戏王" → "yxw"
    const initials = pinyin(text, { pattern: "first", toneType: "none", type: "string" }).toLowerCase();
    if (initials.includes(query)) return true;
  } catch { /* pinyin-pro may fail on non-CJK */ }
  return false;
}

function sortGames(arr: Game[]): Game[] {
  const s = [...arr];
  switch (_sortBy) {
    case "name":
      s.sort((a, b) => (a.name ?? "").localeCompare(b.name ?? "", "zh-CN"));
      break;
    case "rating":
      s.sort((a, b) => gameRating(b) - gameRating(a));
      break;
    case "playtime":
      s.sort((a, b) => gameTotalSeconds(b) - gameTotalSeconds(a));
      break;
    case "added":
      s.sort((a, b) => new Date((b as any).created_at ?? 0).getTime() - new Date((a as any).created_at ?? 0).getTime());
      break;
    case "recent":
    default:
      s.sort((a, b) => {
        const lastA = gameLastPlayed(a);
        const lastB = gameLastPlayed(b);
        const tA = lastA ? new Date(lastA).getTime() : 0;
        const tB = lastB ? new Date(lastB).getTime() : 0;
        return tB - tA;
      });
  }
  return s;
}

function normalizeGames(games: Game[]): Game[] {
  return games.map((game) => normalizeGame(game));
}

function applyLocalFilter() {
  if (!_quickFilter && !_searchQuery && !_filterTag) {
    _games = sortGames(_allGames);
    return;
  }
  const q = (_searchQuery || "").toLowerCase();
  const tag = (_filterTag || "").toLowerCase();
  const f = _quickFilter;
  _games = _allGames.filter(g => {
    // quickFilter
    if (f === "favorite" && !g.favorite) return false;
    if (f === "playing" && gameCompletionStatus(g) !== "playing") return false;
    if (f === "completed" && gameCompletionStatus(g) !== "completed") return false;
    if (f === "unplayed") {
      const st = gameCompletionStatus(g);
      if (st !== "not_started" && st !== "plan_to_play") return false;
    }
    if (f === "installed" && !isInstalled(g)) return false;
    if (f === "missing_metadata") {
      // 待补全 = 缺封面 / 简介 / 标签 之一（不强制要求 vndb+bangumi ID，否则全部平台游戏都被算进来）
      const complete = !!coverOf(g) && !!g.description?.trim() && tagsOf(g).length > 0;
      if (complete) return false;
    }
    if (f === "recent" && !gameLastPlayed(g)) return false;
    // filterTag
    if (tag) {
      const allTags = tagsOf(g).map(t => t.toLowerCase());
      if (!allTags.includes(tag)) return false;
    }
    // searchQuery
    if (q) {
      const originalName = originalNameOf(g);
      const fields = [
        g.name,
        originalName,
        developerOf(g),
        publisherOf(g),
        g.engine,
        g.metadata?.engine,
        ...tagsOf(g),
      ].filter(Boolean).map(s => s!.toLowerCase());
      if (!fields.some(s => s.includes(q) || matchesPinyin(g.name, q) || matchesPinyin(originalName, q))) return false;
    }
    return true;
  });
  _games = sortGames(_games);
}

export const gameStore = {
  // ---- reactive ----
  get games() { return _games; },
  get allGames() { return _allGames; },
  get installedGames() {
    return _allGames.filter(isInstalled);
  },
  get loading() { return _loading; },
  get loadError() { return _loadError; },
  get selectedId() { return _selectedId; },
  get selectedGame() { return _allGames.find(g => g.id === _selectedId) || null; },
  get searchQuery() { return _searchQuery; },
  set searchQuery(v: string) {
    _searchQuery = v;
    applyLocalFilter();
  },
  get filterTag() { return _filterTag; },
  set filterTag(t: string | null) {
    _filterTag = t;
    applyLocalFilter();
  },
  get quickFilter() { return _quickFilter; },
  set quickFilter(f: string | null) {
    _quickFilter = f;
    applyLocalFilter();
  },
  get sortBy() { return _sortBy; },
  set sortBy(v: string) {
    _sortBy = v;
    applyLocalFilter();
  },

  // ---- data loading ----
  async load() {
    _loading = true;
    _loadError = null;
    try {
      _allGames = normalizeGames(await getGames());
      applyLocalFilter();
    } catch (e) {
      console.error("Failed to load games:", e);
      _loadError = userFacingErrorMessage(e);
    } finally {
      _loading = false;
    }
  },

  async search(query: string) {
    _loading = true;
    try {
      _allGames = normalizeGames(await searchGames(query));
      _games = _allGames;
    } catch (e) {
      console.error("Search failed:", e);
      _loadError = userFacingErrorMessage(e);
    } finally {
      _loading = false;
    }
  },

  async importGame() {
    try {
      await addGameByDialog();
      await this.load();
    } catch (e) {
      console.error("Import failed:", e);
      _loadError = userFacingErrorMessage(e);
    }
  },

  selectGame(id: string) {
    _selectedId = id;
  },

  // ---- game actions (aliases for component compatibility) ----
  async launch(id: string) {
    try {
      await launchGame(id);
    } catch (e) {
      console.error("Launch failed:", e);
      _loadError = userFacingErrorMessage(e);
    }
  },

  async launchJP(id: string) {
    try {
      await launchGame(id, true);
    } catch (e) {
      console.error("JP launch failed:", e);
      throw e;
    }
  },

  async toggleFavorite(id: string) {
    try {
      await toggleFavoriteApi(id);
      const nextFavorite = !(_allGames.find(g => g.id === id)?.favorite ?? false);
      _allGames = _allGames.map(g => g.id === id ? { ...g, favorite: nextFavorite } : g);
      _games = _games.map(g => g.id === id ? { ...g, favorite: nextFavorite } : g);
      applyLocalFilter();
    } catch (e) {
      console.error("Toggle favorite failed:", e);
      _loadError = userFacingErrorMessage(e);
    }
  },

  async toggleFav(id: string) {
    return this.toggleFavorite(id);
  },

  async deleteGame(id: string) {
    try {
      await deleteGameApi(id);
      _allGames = _allGames.filter(g => g.id !== id);
      _games = _games.filter(g => g.id !== id);
    } catch (e) {
      console.error("Delete failed:", e);
      _loadError = userFacingErrorMessage(e);
    }
  },

  async remove(id: string) {
    return this.deleteGame(id);
  },

  async updateSaveDir(id: string, path: string | null) {
    try {
      await setSaveDir(id, path);
      return true;
    } catch (e) {
      console.error("set_save_dir failed:", e);
      _loadError = userFacingErrorMessage(e);
      return false;
    }
  },

  async scrape(id: string, result: ScrapeResult) {
    try {
      const updated = normalizeGame(await applyScrapeResult(id, result));
      _allGames = _allGames.map(g => g.id === id ? updated : g);
      _games = _games.map(g => g.id === id ? updated : g);
      return updated;
    } catch (e) {
      console.error("Apply scrape failed:", e);
      _loadError = userFacingErrorMessage(e);
      throw e;
    }
  },
};
