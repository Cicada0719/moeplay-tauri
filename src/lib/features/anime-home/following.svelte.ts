import { invokeCmd } from "../../api/core";

const STORAGE_KEY = "anime-following-v1";
const CHECK_INTERVAL_MS = 30 * 60 * 1000;
const STALE_AFTER_MS = 24 * 60 * 60 * 1000;

export type FollowingStatus = "idle" | "checking" | "ready" | "unknown";
export type FollowingErrorKind = "network" | "http" | "timeout" | "empty" | "invalid" | "unknown";

export interface FollowingEpisode {
  id: string;
  title: string;
  url: string;
  number: number | null;
  special: boolean;
}

export interface FollowingItem {
  key: string;
  contentId: string;
  title: string;
  image: string;
  sourceId: string;
  sourceUrl: string;
  seasonKey: string;
  knownEpisodes: FollowingEpisode[];
  watchedEpisodeIds: string[];
  baselineReady: boolean;
  pendingNoticeIds: string[];
  status: FollowingStatus;
  lastCheckedAt: number | null;
  errorKind: FollowingErrorKind | null;
  autoCheck: boolean;
  updatedAt: number;
}

export interface FollowingCollectionLike {
  key: string;
  name: string;
  image?: string;
  ruleSource?: string;
  sourceUrl?: string;
  contentId?: string;
  seasonKey?: string;
  collectType?: number;
}

export interface FollowingHistoryLike {
  name: string;
  ruleName: string;
  sourceUrl: string;
  lastEpisode: number;
  lastEpisodeName: string;
  progressMs: number;
  updatedAt: string;
}

export interface FollowingCheckResult {
  key: string;
  baselineCreated: boolean;
  addedEpisodeIds: string[];
  pendingNoticeIds: string[];
  knownEpisodeCount: number;
  status: FollowingStatus;
  errorKind: FollowingErrorKind | null;
}

export interface FollowingResumeTarget {
  episode: FollowingEpisode | null;
  episodeIndex: number;
  history: FollowingHistoryLike | null;
  reason: "unfinished" | "unwatched" | "empty";
}

type EpisodeLoader = (item: FollowingItem, signal: AbortSignal) => Promise<unknown>;

function loadItems(): FollowingItem[] {
  if (typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isFollowingItem).map(normalizeItem);
  } catch {
    return [];
  }
}

function saveItems(items: FollowingItem[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
  } catch {
    // The UI can still operate for this session when storage is unavailable.
  }
}

function isFollowingItem(value: unknown): value is FollowingItem {
  if (!value || typeof value !== "object") return false;
  const item = value as Partial<FollowingItem>;
  return typeof item.key === "string"
    && typeof item.title === "string"
    && typeof item.sourceId === "string"
    && Array.isArray(item.knownEpisodes)
    && Array.isArray(item.watchedEpisodeIds);
}

function normalizeItem(item: FollowingItem): FollowingItem {
  return {
    ...item,
    image: item.image ?? "",
    sourceUrl: item.sourceUrl ?? "",
    contentId: item.contentId || item.sourceUrl || item.key,
    seasonKey: item.seasonKey || "season-1",
    knownEpisodes: item.knownEpisodes.map(normalizeEpisode),
    watchedEpisodeIds: unique(item.watchedEpisodeIds),
    pendingNoticeIds: unique(item.pendingNoticeIds ?? []),
    baselineReady: Boolean(item.baselineReady),
    status: item.status === "ready" || item.status === "unknown" || item.status === "checking" ? item.status : "idle",
    lastCheckedAt: typeof item.lastCheckedAt === "number" ? item.lastCheckedAt : null,
    errorKind: item.errorKind ?? null,
    autoCheck: item.autoCheck === true,
    updatedAt: typeof item.updatedAt === "number" ? item.updatedAt : Date.now(),
  };
}

function normalizeEpisode(value: FollowingEpisode): FollowingEpisode {
  return {
    id: String(value.id),
    title: String(value.title || "未命名剧集"),
    url: String(value.url || ""),
    number: typeof value.number === "number" && Number.isFinite(value.number) ? value.number : null,
    special: Boolean(value.special),
  };
}

function unique(values: string[]): string[] {
  return [...new Set(values.filter(Boolean))];
}

function encodePart(value: string): string {
  return encodeURIComponent(value.trim() || "unknown");
}

export function followingKey(sourceId: string, contentId: string, seasonKey = "season-1"): string {
  return `anime:${encodePart(sourceId)}:${encodePart(contentId)}:${encodePart(seasonKey)}`;
}

const SPECIAL_PATTERN = /特别篇|特别版|番外|外传|ova|oad|sp(?:ecial)?|原声|总集篇/i;
const EPISODE_PATTERN = /第\s*(\d+(?:\.\d+)?)\s*(?:话|集|期)|(?:^|\b)(?:ep|episode)\s*(\d+(?:\.\d+)?)/i;
const SPECIAL_NUMBER_PATTERN = /(?:ova|oad|sp(?:ecial)?|特别篇|番外|外传)\s*(\d+(?:\.\d+)?)/i;

export function normalizeFollowingEpisode(value: { name?: string; title?: string; url?: string }, index = 0): FollowingEpisode {
  const title = String(value.name || value.title || `第 ${index + 1} 集`).trim();
  const url = String(value.url || "").trim();
  const match = title.match(EPISODE_PATTERN) || (SPECIAL_PATTERN.test(title) ? title.match(SPECIAL_NUMBER_PATTERN) : null);
  const number = match ? Number(match[1] || match[2]) : null;
  const special = SPECIAL_PATTERN.test(title);
  // Episode numbers keep renamed regular episodes stable. Specials retain their
  // title/URL identity so an OVA cannot become a regular episode by position.
  const identity = special
    ? `special:${number ?? "title"}:${number === null ? (url || title.toLocaleLowerCase()) : ""}`
    : number !== null
      ? `regular:${number}`
      : `regular:title:${url || title.toLocaleLowerCase()}`;
  return { id: identity, title, url, number, special };
}

export function normalizeFollowingEpisodes(value: unknown): FollowingEpisode[] {
  if (!Array.isArray(value)) return [];
  return value.map((episode, index) => {
    if (episode && typeof episode === "object") return normalizeFollowingEpisode(episode as { name?: string; title?: string; url?: string }, index);
    return normalizeFollowingEpisode({}, index);
  });
}

function collectionIdentity(collection: FollowingCollectionLike) {
  const sourceId = collection.ruleSource?.trim() || "unknown";
  const contentId = collection.contentId?.trim() || collection.sourceUrl?.trim() || collection.key;
  const seasonKey = collection.seasonKey?.trim() || "season-1";
  return { sourceId, contentId, seasonKey, key: followingKey(sourceId, contentId, seasonKey) };
}

function itemFromCollection(collection: FollowingCollectionLike, previous?: FollowingItem): FollowingItem {
  const identity = collectionIdentity(collection);
  return {
    key: identity.key,
    contentId: identity.contentId,
    title: collection.name,
    image: collection.image || previous?.image || "",
    sourceId: identity.sourceId,
    sourceUrl: collection.sourceUrl || previous?.sourceUrl || "",
    seasonKey: identity.seasonKey,
    knownEpisodes: previous?.knownEpisodes || [],
    watchedEpisodeIds: previous?.watchedEpisodeIds || [],
    baselineReady: previous?.baselineReady || false,
    pendingNoticeIds: previous?.pendingNoticeIds || [],
    status: previous?.status || "idle",
    lastCheckedAt: previous?.lastCheckedAt || null,
    errorKind: previous?.errorKind || null,
    // Manual checks are the default. Users can opt into the foreground timer
    // from the update center.
    autoCheck: previous?.autoCheck === true,
    updatedAt: Date.now(),
  };
}

function classifyError(error: unknown): FollowingErrorKind {
  const message = error instanceof Error ? error.message : String(error);
  if (/timeout|超时/i.test(message)) return "timeout";
  if (/403|404|429|5\d\d|http/i.test(message)) return "http";
  if (/empty|空|没有剧集/i.test(message)) return "empty";
  if (/invalid|格式|parse/i.test(message)) return "invalid";
  if (/network|网络|连接/i.test(message)) return "network";
  return "unknown";
}

let _items = $state<FollowingItem[]>(loadItems());
let autoTimer: ReturnType<typeof setInterval> | null = null;
let registeredLoader: EpisodeLoader | null = null;
let visibilityHandler: (() => void) | null = null;

function persist() {
  saveItems(_items);
}

function replace(item: FollowingItem) {
  const index = _items.findIndex((current) => current.key === item.key);
  if (index >= 0) _items[index] = item;
  else _items = [item, ..._items];
  persist();
}

async function fetchSourceEpisodes(item: FollowingItem, signal: AbortSignal): Promise<unknown> {
  if (!item.sourceId || item.sourceId === "unknown" || !item.sourceUrl) throw new Error("following source is not bound");
  const roads = await invokeCmd<Array<{ name: string; episodes: Array<{ name: string; url: string }> }>>("anime_fetch_roads", {
    ruleName: item.sourceId,
    pageUrl: item.sourceUrl,
  });
  if (signal.aborted) throw new DOMException("Aborted", "AbortError");
  return roads.flatMap((road) => Array.isArray(road?.episodes) ? road.episodes : []);
}

async function checkItem(item: FollowingItem, loader: EpisodeLoader): Promise<FollowingCheckResult> {
  const current = { ...item, status: "checking" as const, errorKind: null, updatedAt: Date.now() };
  replace(current);
  try {
    const loaded = normalizeFollowingEpisodes(await loader(current, new AbortController().signal));
    if (loaded.length === 0) throw new Error("following source returned empty episodes");
    const known = new Map(current.knownEpisodes.map((episode) => [episode.id, episode]));
    const addedEpisodeIds = current.baselineReady ? loaded.filter((episode) => !known.has(episode.id)).map((episode) => episode.id) : [];
    loaded.forEach((episode) => known.set(episode.id, episode));
    const next: FollowingItem = {
      ...current,
      knownEpisodes: [...known.values()],
      baselineReady: true,
      pendingNoticeIds: current.baselineReady ? unique([...current.pendingNoticeIds, ...addedEpisodeIds]) : current.pendingNoticeIds,
      status: "ready",
      lastCheckedAt: Date.now(),
      errorKind: null,
      updatedAt: Date.now(),
    };
    replace(next);
    return {
      key: next.key,
      baselineCreated: !current.baselineReady,
      addedEpisodeIds,
      pendingNoticeIds: next.pendingNoticeIds,
      knownEpisodeCount: next.knownEpisodes.length,
      status: next.status,
      errorKind: null,
    };
  } catch (error) {
    const next: FollowingItem = {
      ...current,
      status: "unknown",
      errorKind: classifyError(error),
      lastCheckedAt: current.lastCheckedAt,
      updatedAt: Date.now(),
    };
    replace(next);
    return {
      key: next.key,
      baselineCreated: false,
      addedEpisodeIds: [],
      pendingNoticeIds: next.pendingNoticeIds,
      knownEpisodeCount: next.knownEpisodes.length,
      status: next.status,
      errorKind: next.errorKind,
    };
  }
}

function itemNeedsCheck(item: FollowingItem, now = Date.now()): boolean {
  return item.autoCheck && (item.lastCheckedAt === null || now - item.lastCheckedAt >= CHECK_INTERVAL_MS);
}

function resumeTarget(item: FollowingItem, histories: readonly FollowingHistoryLike[]): FollowingResumeTarget {
  const candidates = histories
    .filter((history) => history.ruleName === item.sourceId && (history.sourceUrl === item.sourceUrl || history.name === item.title))
    .sort((left, right) => Date.parse(right.updatedAt) - Date.parse(left.updatedAt));
  const history = candidates[0] || null;
  if (history) {
    const episode = item.knownEpisodes[history.lastEpisode] || item.knownEpisodes.find((entry) => entry.title === history.lastEpisodeName) || null;
    if (episode && !item.watchedEpisodeIds.includes(episode.id)) {
      return { episode, episodeIndex: item.knownEpisodes.indexOf(episode), history, reason: "unfinished" };
    }
  }
  const episodeIndex = item.knownEpisodes.findIndex((episode) => !item.watchedEpisodeIds.includes(episode.id));
  return episodeIndex >= 0
    ? { episode: item.knownEpisodes[episodeIndex], episodeIndex, history, reason: "unwatched" }
    : { episode: null, episodeIndex: -1, history, reason: "empty" };
}

export const followingStore = {
  get items() { return _items; },
  get activeItems() { return _items.filter((item) => item.status !== "unknown" || item.knownEpisodes.length > 0); },
  get pendingCount() { return _items.reduce((count, item) => count + item.pendingNoticeIds.length, 0); },
  get(key: string) { return _items.find((item) => item.key === key); },

  follow(collection: FollowingCollectionLike): FollowingItem {
    const identity = collectionIdentity(collection);
    const previous = _items.find((item) => item.key === identity.key);
    if (previous
      && previous.title === collection.name
      && previous.image === (collection.image || previous.image)
      && previous.sourceUrl === (collection.sourceUrl || previous.sourceUrl)
      && previous.sourceId === identity.sourceId
      && previous.contentId === identity.contentId
      && previous.seasonKey === identity.seasonKey) return previous;
    const item = itemFromCollection(collection, previous);
    replace(item);
    return item;
  },

  syncCollections(collections: readonly FollowingCollectionLike[]) {
    for (const collection of collections) {
      if (collection.collectType === undefined || collection.collectType === 1) this.follow(collection);
    }
  },

  unfollow(key: string) {
    _items = _items.filter((item) => item.key !== key);
    persist();
  },

  setAutoCheck(key: string, enabled: boolean) {
    const item = this.get(key);
    if (!item) return;
    replace({ ...item, autoCheck: enabled, updatedAt: Date.now() });
  },

  async check(key: string, loader: EpisodeLoader = fetchSourceEpisodes) {
    const item = this.get(key);
    if (!item) return null;
    return checkItem(item, loader);
  },

  async checkAll(loader: EpisodeLoader = fetchSourceEpisodes, autoOnly = false) {
    const results: FollowingCheckResult[] = [];
    const items = autoOnly ? _items.filter((item) => itemNeedsCheck(item)) : _items;
    for (const item of items) results.push(await checkItem(item, loader));
    return results;
  },

  markWatched(key: string, episodeId: string, watched = true) {
    const item = this.get(key);
    if (!item) return;
    const watchedEpisodeIds = watched
      ? unique([...item.watchedEpisodeIds, episodeId])
      : item.watchedEpisodeIds.filter((id) => id !== episodeId);
    replace({ ...item, watchedEpisodeIds, updatedAt: Date.now() });
  },

  markNextUnwatched(key: string) {
    const item = this.get(key);
    const episode = item?.knownEpisodes.find((entry) => !item.watchedEpisodeIds.includes(entry.id));
    if (episode) this.markWatched(key, episode.id, true);
    return episode || null;
  },

  markPlaybackProgress(key: string, episodeId: string, positionSeconds: number, durationSeconds: number) {
    if (durationSeconds > 0 && positionSeconds / durationSeconds >= 0.9) this.markWatched(key, episodeId, true);
  },

  consumeNotices(key: string) {
    const item = this.get(key);
    if (!item || item.pendingNoticeIds.length === 0) return;
    replace({ ...item, pendingNoticeIds: [], updatedAt: Date.now() });
  },

  resume(key: string, histories: readonly FollowingHistoryLike[]) {
    const item = this.get(key);
    return item ? resumeTarget(item, histories) : { episode: null, episodeIndex: -1, history: null, reason: "empty" as const };
  },

  startAutoCheck(loader: EpisodeLoader = fetchSourceEpisodes) {
    registeredLoader = loader;
    if (autoTimer !== null || typeof window === "undefined") return;
    const run = () => {
      if (document.visibilityState !== "visible" || !registeredLoader) return;
      void this.checkAll(registeredLoader, true);
    };
    autoTimer = setInterval(run, CHECK_INTERVAL_MS);
    visibilityHandler = run;
    document.addEventListener("visibilitychange", run);
  },

  stopAutoCheck() {
    if (autoTimer !== null) clearInterval(autoTimer);
    if (visibilityHandler) document.removeEventListener("visibilitychange", visibilityHandler);
    autoTimer = null;
    registeredLoader = null;
    visibilityHandler = null;
  },

  /** Test/runtime migration hook; it never writes an empty state over storage. */
  reload() {
    _items = loadItems();
  },
};

export { CHECK_INTERVAL_MS, STALE_AFTER_MS, fetchSourceEpisodes };
