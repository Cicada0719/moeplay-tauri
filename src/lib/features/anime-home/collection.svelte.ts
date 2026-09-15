// 收藏集（Bangumi 风格分型收藏）独立存储（从 anime.svelte.ts 拆出）。

export interface AnimeCollect {
  key: string;
  name: string;
  image: string;
  collectType: number; // 1=在看 2=想看 3=搁置 4=看过 5=抛弃
  ruleSource?: string;
  sourceUrl?: string;
  /** v0.24 following identity; legacy records continue to use key/name. */
  contentId?: string;
  seasonKey?: string;
  updatedAt: string;
}

const COLLECT_KEY = "anime-collect";

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

/** 收藏集详情上下文（由主 store 注入，避免本模块依赖详情/播放器状态） */
export interface CollectDetailContext {
  image: string;
  ruleName: string;
  sourceUrl: string;
}

let _items = $state<AnimeCollect[]>(loadJson(COLLECT_KEY, []));
let _filter = $state(0); // 0=全部, 1-5=对应类型

export const collectionStore = {
  get items() { return _items; },
  get filter() { return _filter; },
  set filter(v: number) { _filter = v; },

  get filtered(): AnimeCollect[] {
    if (_filter === 0) return _items;
    return _items.filter((c) => c.collectType === _filter);
  },

  setCollect(name: string, collectType: number, extra: Partial<AnimeCollect> | undefined, detail: CollectDetailContext) {
    const key = name;
    const idx = _items.findIndex((c) => c.key === key);
    if (collectType === 0) {
      if (idx >= 0) _items = _items.filter((c) => c.key !== key);
    } else {
      const entry: AnimeCollect = {
        key,
        name,
        image: extra?.image ?? detail.image,
        collectType,
        ruleSource: extra?.ruleSource ?? detail.ruleName,
        sourceUrl: extra?.sourceUrl ?? detail.sourceUrl,
        contentId: extra?.contentId,
        seasonKey: extra?.seasonKey,
        updatedAt: new Date().toISOString(),
      };
      if (idx >= 0) _items[idx] = entry;
      else _items = [entry, ..._items];
    }
    saveJson(COLLECT_KEY, _items);
  },

  getType(name: string): number {
    return _items.find((c) => c.key === name)?.collectType ?? 0;
  },
};
