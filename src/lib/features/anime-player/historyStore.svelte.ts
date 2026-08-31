// 番剧观看历史独立存储（从 anime.svelte.ts 拆出）。

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

const HISTORY_KEY = "anime-history";

function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function saveJson(key: string, data: unknown) {
  localStorage.setItem(key, JSON.stringify(data));
}

let _items = $state<AnimeHistory[]>(loadJson(HISTORY_KEY, []));

export const historyStore = {
  get items() { return _items; },
  get(key: string): AnimeHistory | undefined {
    return _items.find((h) => h.key === key);
  },
  /** 新建或覆盖历史条目（最近在前，上限 200）并持久化 */
  upsert(entry: AnimeHistory) {
    const idx = _items.findIndex((h) => h.key === entry.key);
    if (idx >= 0) _items[idx] = entry;
    else _items = [entry, ..._items];
    if (_items.length > 200) _items = _items.slice(0, 200);
    saveJson(HISTORY_KEY, _items);
  },
  remove(key: string) {
    _items = _items.filter((h) => h.key !== key);
    saveJson(HISTORY_KEY, _items);
  },
  clear() {
    _items = [];
    saveJson(HISTORY_KEY, _items);
  },
};
