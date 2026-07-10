export interface SearchItemLike { name: string; url: string }
export interface EpisodeLike { name: string; url: string }
export interface RoadLike<E extends EpisodeLike = EpisodeLike> { name: string; episodes: E[] }

export interface EpisodeMatch<E extends EpisodeLike = EpisodeLike, R extends RoadLike<E> = RoadLike<E>> {
  road: R;
  roadIndex: number;
  episode: E;
  episodeIndex: number;
  score: number;
  reason: "exact-title" | "episode-number" | "title-similarity" | "index-fallback";
}

const TITLE_NOISE = ["在线观看", "在线播放", "全集", "动漫", "动画", "番剧", "高清", "中字", "字幕", "1080p", "720p"];
const SPECIAL_MARKERS = /(?:剧场版|总集篇|特别篇|ova|oad|ona|special|sp\b|pv\b|预告|解说)/i;

export function normalizeAnimeTitle(value: string): string {
  let normalized = value.normalize("NFKC").toLocaleLowerCase()
    .replace(/[【\[（(].*?[】\]）)]/g, " ")
    .replace(/[·・•:：/\\|_—–-]+/g, " ");
  for (const token of TITLE_NOISE) normalized = normalized.replaceAll(token, " ");
  return normalized.replace(/[^\p{L}\p{N}]+/gu, "").trim();
}

function bigrams(value: string): string[] {
  if (value.length < 2) return value ? [value] : [];
  return Array.from({ length: value.length - 1 }, (_, index) => value.slice(index, index + 2));
}

function similarity(a: string, b: string): number {
  if (!a || !b) return 0;
  if (a === b) return 1;
  const left = bigrams(a);
  const right = bigrams(b);
  const counts = new Map<string, number>();
  for (const gram of right) counts.set(gram, (counts.get(gram) ?? 0) + 1);
  let overlap = 0;
  for (const gram of left) {
    const count = counts.get(gram) ?? 0;
    if (count > 0) { overlap++; counts.set(gram, count - 1); }
  }
  return 2 * overlap / Math.max(1, left.length + right.length);
}

export function scoreAnimeTitle(query: string, candidate: string): number {
  const q = normalizeAnimeTitle(query);
  const c = normalizeAnimeTitle(candidate);
  if (!q || !c) return -Infinity;
  let score = Math.round(similarity(q, c) * 500);
  if (q === c) score += 1_000;
  else if (c.includes(q)) score += 650 - Math.min(200, c.length - q.length);
  else if (q.includes(c)) score += 450 - Math.min(160, q.length - c.length);
  if (SPECIAL_MARKERS.test(query) !== SPECIAL_MARKERS.test(candidate)) score -= 600;
  return score;
}

export function rankSearchItems<T extends SearchItemLike>(query: string, items: readonly T[]): T[] {
  return items.map((item, index) => ({ item, index, score: scoreAnimeTitle(query, item.name) }))
    .sort((a, b) => b.score - a.score || a.index - b.index)
    .map(({ item }) => item);
}

const CHINESE_DIGITS: Record<string, number> = { 零:0, 〇:0, 一:1, 二:2, 两:2, 三:3, 四:4, 五:5, 六:6, 七:7, 八:8, 九:9 };
function parseChineseInteger(value: string): number | null {
  if (/^\d+(?:\.\d+)?$/.test(value)) return Number(value);
  if (!/^[零〇一二两三四五六七八九十百]+$/.test(value)) return null;
  let total = 0, current = 0;
  for (const char of value) {
    if (char === "十" || char === "百") { total += (current || 1) * (char === "十" ? 10 : 100); current = 0; }
    else current = CHINESE_DIGITS[char] ?? 0;
  }
  return total + current;
}

export function extractEpisodeNumber(value: string): number | null {
  const normalized = value.normalize("NFKC").toLocaleLowerCase();
  const labelled = normalized.match(/第\s*([零〇一二两三四五六七八九十百\d]+(?:\.\d+)?)\s*(?:集|话|話|期)/i);
  if (labelled) return parseChineseInteger(labelled[1]);
  const seasonEpisode = normalized.match(/s\d{1,2}\s*e(?:p(?:isode)?)?\s*0*(\d+(?:\.\d+)?)/i);
  if (seasonEpisode) return Number(seasonEpisode[1]);
  const ep = normalized.match(/(?:^|\s|[-_])(?:ep(?:isode)?|e)\s*0*(\d+(?:\.\d+)?)(?:\s|$|[-_])/i);
  if (ep) return Number(ep[1]);
  const numbers = [...normalized.matchAll(/(?:^|[^\d])(\d+(?:\.\d+)?)(?=$|[^\d])/g)]
    .map((match) => Number(match[1])).filter((n) => Number.isFinite(n) && !(n >= 1900 && n <= 2100));
  return numbers.length ? numbers[numbers.length - 1] : null;
}

export function findBestEpisodeMatch<E extends EpisodeLike, R extends RoadLike<E>>(
  roads: readonly R[], target: { episodeName: string; episodeIndex: number },
): EpisodeMatch<E, R> | null {
  const targetTitle = normalizeAnimeTitle(target.episodeName);
  const targetNumber = extractEpisodeNumber(target.episodeName);
  const targetSpecial = SPECIAL_MARKERS.test(target.episodeName);
  let best: EpisodeMatch<E, R> | null = null;
  for (let roadIndex = 0; roadIndex < roads.length; roadIndex++) {
    const road = roads[roadIndex];
    for (let episodeIndex = 0; episodeIndex < road.episodes.length; episodeIndex++) {
      const episode = road.episodes[episodeIndex];
      const episodeTitle = normalizeAnimeTitle(episode.name);
      const episodeNumber = extractEpisodeNumber(episode.name);
      let score = 0;
      let reason: EpisodeMatch<E, R>["reason"] = "index-fallback";
      if (targetTitle && targetTitle === episodeTitle) { score = 1_200; reason = "exact-title"; }
      else if (targetSpecial !== SPECIAL_MARKERS.test(episode.name)) score = -1_000;
      else if (targetNumber !== null && episodeNumber !== null) {
        if (Math.abs(targetNumber - episodeNumber) < 0.001) { score = 900; reason = "episode-number"; }
        else score = -900;
      } else {
        const titleScore = similarity(targetTitle, episodeTitle);
        score = Math.round(titleScore * 500) + (episodeIndex === target.episodeIndex ? 260 : 0);
        reason = titleScore >= 0.5 ? "title-similarity" : "index-fallback";
      }
      if (score >= 0 && target.episodeIndex < road.episodes.length) score += 10;
      if (!best || score > best.score) best = { road, roadIndex, episode, episodeIndex, score, reason };
    }
  }
  if (!best) return null;
  if (targetNumber !== null && !["exact-title", "episode-number"].includes(best.reason)) return null;
  return best.score >= 250 ? best : null;
}
