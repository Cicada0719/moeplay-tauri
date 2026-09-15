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

function validEntry(value: unknown): value is AnimeHistory {
  if (!value || typeof value !== "object") return false;
  const entry = value as AnimeHistory;
  return [entry.key, entry.name, entry.ruleName, entry.sourceUrl, entry.lastEpisodeName].every((v) => typeof v === "string")
    && entry.key.length > 0 && entry.name.trim().length > 0
    && Number.isInteger(entry.lastRoad) && entry.lastRoad >= 0
    && Number.isInteger(entry.lastEpisode) && entry.lastEpisode >= 0
    && Number.isFinite(entry.progressMs) && entry.progressMs >= 0
    && typeof entry.updatedAt === "string" && Number.isFinite(Date.parse(entry.updatedAt));
}

let _items = $state<AnimeHistory[]>(loadJson(HISTORY_KEY, []));

export const historyStore = {
  get items() { return _items; },
  snapshot(): AnimeHistory[] { return _items.map((entry) => ({ ...entry })); },
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
  previewImport(entries: AnimeHistory[]): { added: number; updated: number; skipped: number } {
    const map = new Map(_items.map((entry) => [entry.key, entry]));
    let added = 0; let updated = 0; let skipped = 0;
    for (const entry of entries) {
      if (!validEntry(entry)) { skipped += 1; continue; }
      const old = map.get(entry.key);
      if (!old) { added += 1; map.set(entry.key, entry); }
      else if (Date.parse(entry.updatedAt) > Date.parse(old.updatedAt)) { updated += 1; map.set(entry.key, entry); }
      else skipped += 1;
    }
    return { added, updated, skipped };
  },
  importEntries(entries: AnimeHistory[]): { imported: number; skipped: number; failed: number } {
    const valid = entries.filter(validEntry);
    const invalid = entries.length - valid.length;
    const preview = this.previewImport(valid);
    if (!preview.added && !preview.updated) return { imported: 0, skipped: invalid + preview.skipped, failed: 0 };
    const map = new Map(_items.map((entry) => [entry.key, entry]));
    for (const entry of valid) {
      const old = map.get(entry.key);
      if (!old || Date.parse(entry.updatedAt) > Date.parse(old.updatedAt)) map.set(entry.key, entry);
    }
    const next = [...map.values()].sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt)).slice(0, 200);
    try {
      // Persist first. A quota/security failure must not be reported as an import.
      saveJson(HISTORY_KEY, next);
      _items = next;
      return { imported: preview.added + preview.updated, skipped: invalid + preview.skipped, failed: 0 };
    } catch {
      return { imported: 0, skipped: invalid + preview.skipped, failed: preview.added + preview.updated };
    }
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
