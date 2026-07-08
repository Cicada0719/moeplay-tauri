import type { ComicChapter, ComicDetail, ComicSummary } from "../stores/comic.svelte";

export type Dm5TextFetch = (url: string) => Promise<string>;
export type Dm5SourceKey = "dm5" | "ikkk";

export const DM5_SOURCE_CONFIG: Record<Dm5SourceKey, { label: string; baseUrl: string; prefix: string }> = {
  dm5: { label: "DM5", baseUrl: "https://www.dm5.com", prefix: "dm5:" },
  ikkk: { label: "1kkk", baseUrl: "https://www.1kkk.com", prefix: "ikkk:" },
};

function decodeHtml(value: string): string {
  return value
    .replace(/&nbsp;|&#160;/gi, " ")
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, "\"")
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .trim();
}

function readAttr(html: string, name: string): string {
  const match = html.match(new RegExp(`\\s${name}="([^"]*)"`, "i"));
  return decodeHtml(match?.[1] ?? "");
}

function absoluteUrl(pathOrUrl: string, source: Dm5SourceKey = "dm5"): string {
  if (pathOrUrl.startsWith("http://") || pathOrUrl.startsWith("https://")) return pathOrUrl;
  const baseUrl = DM5_SOURCE_CONFIG[source].baseUrl;
  return `${baseUrl}${pathOrUrl.startsWith("/") ? "" : "/"}${pathOrUrl}`;
}

function stripTags(value: string): string {
  return decodeHtml(value.replace(/<[^>]+>/g, " "))
    .replace(/\s+/g, " ")
    .replace(/\s+（/g, "（")
    .trim();
}

function cleanChapterTitle(value: string): string {
  return stripTags(value).replace(/（\d+P）/gi, "").replace(/\(\d+P\)/gi, "").trim();
}

function chapterListHtml(html: string): string {
  const start = html.search(/id="chapterlistload"/i);
  if (start < 0) return html;
  const rest = html.slice(start);
  const end = rest.search(/<div\s+class="detail-list-form-con"|<h2>看过《|<div\s+class="index-manga"/i);
  return end > 0 ? rest.slice(0, end) : rest;
}

export function parseDm5SearchResults(html: string, source: Dm5SourceKey = "dm5"): ComicSummary[] {
  const results: ComicSummary[] = [];
  const seen = new Set<string>();
  const itemRegex = /<div class="mh-item mh-card-wrap">([\s\S]*?)(?=<div class="mh-item mh-card-wrap">|<div class="pager|$)/gi;
  const config = DM5_SOURCE_CONFIG[source];
  let match: RegExpExecArray | null;

  while ((match = itemRegex.exec(html)) && results.length < 30) {
    const block = match[1];
    const link = block.match(/<a\s+href="(\/(?:manhua-[^"\/]+|manhua\d+)\/)"\s+title="([^"]+)"/i);
    const info = block.match(/<div class="manga-info"[\s\S]*?>/i)?.[0] ?? "";
    if (!link) continue;

    const path = link[1];
    if (seen.has(path)) continue;
    seen.add(path);

    const authorRaw = readAttr(info, "au");
    const author = decodeURIComponent(authorRaw).replace(/[,+-]+$/g, "").replace(/-/g, " / ") || "动漫屋";
    const latest = readAttr(info, "te") || readAttr(info, "tn");
    const status = readAttr(info, "tus");
    results.push({
      id: `${config.prefix}${path}`,
      title: decodeHtml(link[2]),
      author,
      thumb_url: readAttr(info, "tc"),
      categories: [config.label, latest].filter(Boolean),
      likes_count: 0,
      total_views: 0,
      eps_count: Number(readAttr(info, "ts")) || 0,
      finished: status.includes("完结"),
    });
  }

  return results;
}

export function parseDm5Detail(html: string, path: string, source: Dm5SourceKey = "dm5"): { detail: ComicDetail; chapters: ComicChapter[] } {
  const config = DM5_SOURCE_CONFIG[source];
  const title = decodeHtml(html.match(/DM5_COMIC_MNAME="([^"]+)"/)?.[1] ?? html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/i)?.[1] ?? "动漫屋漫画");
  const cover = decodeHtml(html.match(/class="banner_detail_form"[\s\S]*?<img[^>]+src="([^"]+)"/i)?.[1] ?? "");
  const desc = stripTags(html.match(/<p class="content"[^>]*>([\s\S]*?)<\/p>/i)?.[1] ?? html.match(/<meta content="([^"]+)" name="Description"/i)?.[1] ?? "");
  const statusText = stripTags(html.match(/detail-list-title[\s\S]*?<a[^>]*class="block[^"]*"[^>]*>([\s\S]*?)<\/a>/i)?.[1] ?? "");
  const latestText = stripTags(html.match(/detail-list-title[\s\S]*?<span class="s">([\s\S]*?)<\/span>/i)?.[1] ?? "");
  const chapterBlock = chapterListHtml(html);
  const chapterMatches = Array.from(chapterBlock.matchAll(/<a[^>]+href="(\/(?:m\d+|ch\d+-\d+)\/)"[^>]*title="([^"]*)"[^>]*>([\s\S]*?)<\/a>/gi));
  const seen = new Set<string>();
  const chapters = chapterMatches
    .filter((match) => {
      if (seen.has(match[1])) return false;
      seen.add(match[1]);
      return true;
    })
    .map((match, index) => ({
      id: absoluteUrl(match[1], source),
      title: cleanChapterTitle(match[2] || match[3]) || `第 ${index + 1} 话`,
      order: index + 1,
      updated_at: "",
    }));

  return {
    detail: {
      id: `${config.prefix}${path}`,
      title,
      author: "动漫屋",
      description: desc || `来自 ${config.label}/动漫屋系的普通漫画条目`,
      thumb_url: cover,
      categories: [config.label, statusText, latestText].filter(Boolean),
      tags: [],
      likes_count: 0,
      total_views: 0,
      eps_count: chapters.length,
      pages_count: 0,
      finished: statusText.includes("完结"),
      is_liked: false,
      is_favourite: false,
      chinese_team: config.label,
      comments_count: 0,
      allow_comment: false,
      updated_at: "",
      created_at: "",
    },
    chapters,
  };
}

export async function searchDm5(fetchText: Dm5TextFetch, keyword: string, source: Dm5SourceKey = "dm5"): Promise<ComicSummary[]> {
  const url = `${DM5_SOURCE_CONFIG[source].baseUrl}/search?title=${encodeURIComponent(keyword)}&language=1`;
  return parseDm5SearchResults(await fetchText(url), source);
}

export async function loadDm5Detail(fetchText: Dm5TextFetch, id: string): Promise<{ detail: ComicDetail; chapters: ComicChapter[] }> {
  const source: Dm5SourceKey = id.startsWith(DM5_SOURCE_CONFIG.ikkk.prefix) ? "ikkk" : "dm5";
  const prefix = DM5_SOURCE_CONFIG[source].prefix;
  const path = id.startsWith(prefix) ? id.slice(prefix.length) : id;
  return parseDm5Detail(await fetchText(absoluteUrl(path, source)), path, source);
}
