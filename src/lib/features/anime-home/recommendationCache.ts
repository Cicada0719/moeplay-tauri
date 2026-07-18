export interface RecommendationSnapshot<T> {
  version: 1;
  storedAt: number;
  seasonal: T[];
  seasonalTotal: number;
  trending: T[];
  trendingTotal: number;
  topRated: T[];
  topRatedTotal: number;
}

function finiteNonNegative(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : fallback;
}

export function parseRecommendationSnapshot<T>(value: unknown): RecommendationSnapshot<T> | null {
  if (!value || typeof value !== "object") return null;
  const source = value as Partial<RecommendationSnapshot<T>>;
  if (source.version !== 1 || !Array.isArray(source.seasonal) || !Array.isArray(source.trending) || !Array.isArray(source.topRated)) {
    return null;
  }
  const storedAt = finiteNonNegative(source.storedAt, 0);
  if (storedAt <= 0) return null;
  return {
    version: 1,
    storedAt,
    seasonal: source.seasonal,
    seasonalTotal: finiteNonNegative(source.seasonalTotal, source.seasonal.length),
    trending: source.trending,
    trendingTotal: finiteNonNegative(source.trendingTotal, source.trending.length),
    topRated: source.topRated,
    topRatedTotal: finiteNonNegative(source.topRatedTotal, source.topRated.length),
  };
}

export function readRecommendationSnapshot<T>(storage: Pick<Storage, "getItem"> | null, key: string): RecommendationSnapshot<T> | null {
  if (!storage) return null;
  try {
    const raw = storage.getItem(key);
    return raw ? parseRecommendationSnapshot<T>(JSON.parse(raw)) : null;
  } catch {
    return null;
  }
}

export function writeRecommendationSnapshot<T>(storage: Pick<Storage, "setItem"> | null, key: string, snapshot: RecommendationSnapshot<T>): void {
  if (!storage) return;
  try { storage.setItem(key, JSON.stringify(snapshot)); } catch { /* quota/privacy mode: cache is optional */ }
}

export function isRecommendationSnapshotFresh(storedAt: number, now = Date.now(), maxAgeMs = 6 * 60 * 60 * 1000): boolean {
  return storedAt > 0 && now >= storedAt && now - storedAt <= maxAgeMs;
}
