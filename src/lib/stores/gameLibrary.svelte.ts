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
  toggleHidden as toggleHiddenApi,
  addSimpleTag as addSimpleTagApi,
  removeSimpleTag as removeSimpleTagApi,
  updateCompletionStatus as updateCompletionStatusApi,
  type CompletionStatus,
  type ScrapeResult,
  type Game as ApiGame,
} from "../api";
import {
  coverOf,
  developerOf,
  gameCompletionStatus,
  gameLastPlayed,
  gameRating,
  gameTotalSeconds,
  isInstalled,
  normalizeGame,
  originalNameOf,
  publisherOf,
  tagsOf,
  userFacingErrorMessage,
} from "../utils/game";

export type Game = ApiGame;

export interface SmartCollection {
  id: string;
  name: string;
  icon: string;
  color?: string;
  filters: {
    quickFilter?: string | null;
    filterTag?: string | null;
    searchQuery?: string;
    developer?: string;
    platform?: string;
    status?: string;
    minRating?: number;
    sortBy?: string;
    tags?: string[];
    tagMode?: "any" | "all";
    installed?: boolean;
    hasPlayed?: boolean;
  };
}

const COLLECTIONS_KEY = "moeplay-smart-collections";

function loadCollections(): SmartCollection[] {
  try {
    return JSON.parse(localStorage.getItem(COLLECTIONS_KEY) || "[]");
  } catch { return []; }
}
function saveCollections(cols: SmartCollection[]) {
  localStorage.setItem(COLLECTIONS_KEY, JSON.stringify(cols));
}

let _allGames = $state<Game[]>([]);
let _games = $state<Game[]>([]);
let _loading = $state(false);
let _loadError = $state<string | null>(null);
let _searchQuery = $state("");
let _filterTag = $state<string | null>(null);
let _quickFilter = $state<string | null>(null);
let _sortBy = $state("recent");
let _smartCollections = $state<SmartCollection[]>(loadCollections());
let _activeCollectionId = $state<string | null>(null);

/** Check if any field's pinyin matches the search query */
export function matchesPinyin(text: string | undefined | null, query: string): boolean {
  if (!text || !query) return false;
  const lower = text.toLowerCase();
  if (lower.includes(query)) return true;
  try {
    const fullPy = pinyin(text, { toneType: "none", type: "string" }).toLowerCase();
    if (fullPy.includes(query)) return true;
    const initials = pinyin(text, { pattern: "first", toneType: "none", type: "string" }).toLowerCase();
    if (initials.includes(query)) return true;
  } catch { /* pinyin-pro may fail on non-CJK */ }
  return false;
}

function sortGames(arr: Game[], sortBy?: string): Game[] {
  const s = [...arr];
  const by = sortBy ?? _sortBy;
  switch (by) {
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
  let ef = _quickFilter;
  let et = _filterTag;
  let eq = _searchQuery;
  let eSort = _sortBy;
  let eDeveloper: string | undefined;
  let ePlatform: string | undefined;
  let eStatus: string | undefined;
  let eMinRating: number | undefined;

  if (_activeCollectionId) {
    const col = _smartCollections.find(c => c.id === _activeCollectionId);
    if (col) {
      ef = col.filters.quickFilter ?? ef;
      et = col.filters.filterTag ?? et;
      eq = col.filters.searchQuery ?? eq;
      eSort = col.filters.sortBy ?? eSort;
      eDeveloper = col.filters.developer;
      ePlatform = col.filters.platform;
      eStatus = col.filters.status;
      eMinRating = col.filters.minRating;
    }
  }

  if (!ef && !et && !eq && !eDeveloper && !ePlatform && !eStatus && !eMinRating) {
    _games = sortGames(_allGames, eSort);
    return;
  }
  const q = (eq || "").toLowerCase();
  const tag = (et || "").toLowerCase();
  const f = ef;
  _games = _allGames.filter(g => {
    if (f === "favorite" && !g.favorite) return false;
    if (f === "playing" && gameCompletionStatus(g) !== "playing") return false;
    if (f === "completed" && gameCompletionStatus(g) !== "completed") return false;
    if (f === "unplayed") {
      const st = gameCompletionStatus(g);
      if (st !== "not_started" && st !== "plan_to_play") return false;
    }
    if (f === "installed" && !isInstalled(g)) return false;
    if (f === "missing_metadata") {
      const complete = !!coverOf(g) && !!g.description?.trim() && tagsOf(g).length > 0;
      if (complete) return false;
    }
    if (f === "recent" && !gameLastPlayed(g)) return false;
    if (tag) {
      const allTags = tagsOf(g).map(t => t.toLowerCase());
      if (!allTags.includes(tag)) return false;
    }
    if (eDeveloper) {
      if (developerOf(g).toLowerCase() !== eDeveloper.toLowerCase()) return false;
    }
    if (ePlatform) {
      if ((g as any).platform?.toLowerCase() !== ePlatform.toLowerCase()) return false;
    }
    if (eStatus) {
      if (gameCompletionStatus(g) !== eStatus) return false;
    }
    if (eMinRating && eMinRating > 0) {
      if (gameRating(g) < eMinRating) return false;
    }
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
  _games = sortGames(_games, eSort);
}

export const libraryStore = {
  // ---- reactive ----
  get games() { return _games; },
  get allGames() { return _allGames; },
  get installedGames() { return _allGames.filter(isInstalled); },
  get loading() { return _loading; },
  get loadError() { return _loadError; },
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

  // ---- smart collections ----
  get smartCollections() { return _smartCollections; },
  get activeCollectionId() { return _activeCollectionId; },

  activateCollection(id: string | null) {
    _activeCollectionId = id;
    if (id) {
      _quickFilter = null;
      _filterTag = null;
      _searchQuery = "";
    }
    applyLocalFilter();
  },

  addCollection(name: string, icon: string, filters: SmartCollection["filters"]): SmartCollection {
    const col: SmartCollection = { id: crypto.randomUUID(), name, icon, filters };
    _smartCollections = [..._smartCollections, col];
    saveCollections(_smartCollections);
    return col;
  },

  updateCollection(id: string, updates: Partial<SmartCollection>) {
    _smartCollections = _smartCollections.map(c => c.id === id ? { ...c, ...updates } : c);
    saveCollections(_smartCollections);
  },

  removeCollection(id: string) {
    _smartCollections = _smartCollections.filter(c => c.id !== id);
    if (_activeCollectionId === id) {
      _activeCollectionId = null;
      applyLocalFilter();
    }
    saveCollections(_smartCollections);
  },

  saveCurrentAsCollection(name: string, icon: string): SmartCollection {
    return this.addCollection(name, icon, {
      quickFilter: _quickFilter,
      filterTag: _filterTag,
      searchQuery: _searchQuery || undefined,
      sortBy: _sortBy,
    });
  },

  exportCollection(id: string): string | null {
    const col = _smartCollections.find(c => c.id === id);
    if (!col) return null;
    return JSON.stringify({ ...col, id: undefined }, null, 2);
  },

  importCollection(json: string): SmartCollection | null {
    try {
      const data = JSON.parse(json);
      if (!data.name || !data.filters) return null;
      return this.addCollection(data.name, data.icon || "folder", data.filters);
    } catch { return null; }
  },

  duplicateCollection(id: string): SmartCollection | null {
    const col = _smartCollections.find(c => c.id === id);
    if (!col) return null;
    return this.addCollection(`${col.name} (副本)`, col.icon, { ...col.filters });
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

  // ---- game actions ----
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

  // ---- batch actions ----
  async batchDelete(ids: string[]) {
    const succeeded: string[] = [];
    for (const id of ids) {
      try {
        await deleteGameApi(id);
        succeeded.push(id);
      } catch (e) { console.error("Batch delete failed:", id, e); }
    }
    _allGames = _allGames.filter(g => !succeeded.includes(g.id));
    _games = _games.filter(g => !succeeded.includes(g.id));
    return succeeded.length;
  },

  async batchToggleHidden(ids: string[]) {
    for (const id of ids) {
      try { await toggleHiddenApi(id); } catch (e) { console.error("Batch hide failed:", id, e); }
    }
    await this.load();
    return ids.length;
  },

  async batchToggleFavorite(ids: string[]) {
    for (const id of ids) {
      try { await toggleFavoriteApi(id); } catch (e) { console.error("Batch fav failed:", id, e); }
    }
    await this.load();
    return ids.length;
  },

  async batchAddTag(ids: string[], tag: string) {
    for (const id of ids) {
      try { await addSimpleTagApi(id, tag); } catch (e) { console.error("Batch tag failed:", id, e); }
    }
    await this.load();
    return ids.length;
  },

  async batchSetStatus(ids: string[], status: CompletionStatus) {
    for (const id of ids) {
      try { await updateCompletionStatusApi(id, status); } catch (e) { console.error("Batch status failed:", id, e); }
    }
    await this.load();
    return ids.length;
  },
};
