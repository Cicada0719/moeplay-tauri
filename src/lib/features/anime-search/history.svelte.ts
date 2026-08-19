// 番剧搜索历史独立存储（从 anime.svelte.ts 拆出；含旧格式自动迁移）。

const HISTORY_KEY = "anime-search-history";

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

// 旧版 SearchDrawer 存的是 {keyword,timestamp}[]，自动迁移为 string[]
function loadSearchHistory(): string[] {
  const raw = loadJson<unknown[]>(HISTORY_KEY, []);
  return raw
    .map((item) => (typeof item === "string" ? item : (item as { keyword?: string })?.keyword ?? ""))
    .filter(Boolean);
}

let _items = $state<string[]>(loadSearchHistory());

export const animeSearchHistoryStore = {
  get items() { return _items; },
  add(keyword: string) {
    const trimmed = keyword.trim();
    if (!trimmed) return;
    _items = [trimmed, ..._items.filter((k) => k !== trimmed)].slice(0, 20);
    saveJson(HISTORY_KEY, _items);
  },
  remove(keyword: string) {
    _items = _items.filter((k) => k !== keyword);
    saveJson(HISTORY_KEY, _items);
  },
  clear() {
    _items = [];
    saveJson(HISTORY_KEY, _items);
  },
};
