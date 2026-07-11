import type { ComicChapter, ComicDetail, ComicImage, ComicSummary } from "../stores/comic.svelte";

export type BaoziTextFetch = (url: string) => Promise<string>;

const BAOZI_BASE = "https://cn.baozimhcn.com";
const BAOZI_READER_BASE = "https://cn.dzmanga.com";
const SOURCE_PREFIX = "baozi:";
const SOURCE_LABEL = "包子漫画";
const BAOZI_BLOCKED_IMAGE_HOSTS = new Set(["static-tw.baozimh.com"]);
const BAOZI_IMAGE_MIRROR_HOST = "static-tw.baozimhcn.com";

function parseHtml(html: string): Document {
  return new DOMParser().parseFromString(html, "text/html");
}

function absoluteUrl(value: string, base = BAOZI_BASE): string {
  const trimmed = value.trim();
  if (!trimmed) return "";
  if (trimmed.startsWith("//")) return `https:${trimmed}`;
  try {
    return new URL(trimmed, base).toString();
  } catch {
    return trimmed;
  }
}

/**
 * 包子页面仍会下发被 Cloudflare 拒绝的 static-tw.baozimh.com 图片地址。
 * 同路径的 baozimhcn 镜像可直接作为 WebView 图片源，并保留尺寸/质量参数。
 */
export function normalizeBaoziImageUrl(value: string, base = BAOZI_BASE): string {
  const absolute = absoluteUrl(value, base);
  if (!absolute) return "";

  try {
    const url = new URL(absolute);
    if (BAOZI_BLOCKED_IMAGE_HOSTS.has(url.hostname.toLowerCase())) {
      url.hostname = BAOZI_IMAGE_MIRROR_HOST;
      url.protocol = "https:";
    }
    return url.toString();
  } catch {
    return absolute;
  }
}

function providerId(id: string): string {
  return id.startsWith(SOURCE_PREFIX) ? id : `${SOURCE_PREFIX}${id}`;
}

function rawId(id: string): string {
  return id.startsWith(SOURCE_PREFIX) ? id.slice(SOURCE_PREFIX.length) : id;
}

function text(element: Element | null | undefined): string {
  return element?.textContent?.replace(/\s+/g, " ").trim() ?? "";
}

export function parseBaoziSearchResults(html: string): ComicSummary[] {
  const document = parseHtml(html);
  const results: ComicSummary[] = [];
  const seen = new Set<string>();

  for (const card of document.querySelectorAll(".classify-items > div")) {
    const poster = card.querySelector<HTMLAnchorElement>("a.comics-card__poster");
    const href = poster?.getAttribute("href") ?? "";
    const id = href.match(/\/comic\/([^/?#]+)/i)?.[1] ?? "";
    const title = text(card.querySelector(".comics-card__info .comics-card__title"));
    if (!id || !title || seen.has(id)) continue;
    seen.add(id);

    const coverElement = card.querySelector("a.comics-card__poster amp-img");
    const cover = coverElement?.getAttribute("src") || coverElement?.getAttribute("data-src") || "";
    const categories = Array.from(card.querySelectorAll("a.comics-card__poster .tabs .tab"))
      .map((element) => text(element))
      .filter(Boolean);

    results.push({
      id: providerId(id),
      title,
      author: text(card.querySelector(".comics-card__info .tags")) || SOURCE_LABEL,
      thumb_url: normalizeBaoziImageUrl(cover),
      categories: [SOURCE_LABEL, ...categories],
      likes_count: 0,
      total_views: 0,
      eps_count: 0,
      finished: categories.some((category) => category.includes("完结") || category.includes("完結")),
    });
  }

  return results;
}

export function parseBaoziDetail(html: string, id: string): { detail: ComicDetail; chapters: ComicChapter[] } {
  const document = parseHtml(html);
  const title = text(document.querySelector(".comics-detail__title")) || SOURCE_LABEL;
  const author = text(document.querySelector(".comics-detail__author")) || SOURCE_LABEL;
  const coverElement = document.querySelector(".comics-detail .l-content amp-img");
  const cover = coverElement?.getAttribute("src") || coverElement?.getAttribute("data-src") || "";
  const description = document.querySelector('meta[name="description"]')?.getAttribute("content")?.trim() || `来自 ${SOURCE_LABEL} 的漫画条目`;
  const rawTags = Array.from(document.querySelectorAll(".comics-detail .tag-list .tag"))
    .map((element) => text(element))
    .filter(Boolean);
  const finished = rawTags.some((tag) => tag.includes("已完结") || tag.includes("已完結"));
  const tags = rawTags.filter((tag) => !/连载|連載|完结|完結/.test(tag));
  const canonicalId = document.querySelector('meta[name="og:url"]')?.getAttribute("content")?.match(/\/comic\/([^/?#]+)/i)?.[1] || rawId(id);
  const chapterContainers = document.querySelectorAll(
    "#chapter-items > div, #chapters_other_list > div, .l-content .pure-g > div.comics-chapters",
  );
  const chapters: ComicChapter[] = [];
  const seen = new Set<string>();

  for (const container of chapterContainers) {
    const link = container.querySelector<HTMLAnchorElement>('a[href*="section_slot="]');
    if (!link) continue;
    let url: URL;
    try {
      url = new URL(link.getAttribute("href") || "", BAOZI_BASE);
    } catch {
      continue;
    }
    const section = url.searchParams.get("section_slot") || "";
    const chapter = url.searchParams.get("chapter_slot") || "";
    const mangaId = url.searchParams.get("comic_id") || canonicalId;
    if (!section || !chapter || !mangaId) continue;
    const chapterUrl = `${BAOZI_READER_BASE}/comic/chapter/${mangaId}/${section}_${chapter}_1.html`;
    if (seen.has(chapterUrl)) continue;
    seen.add(chapterUrl);
    chapters.push({
      id: chapterUrl,
      title: text(container.querySelector("span")) || text(link) || `章节 ${chapter}`,
      order: 0,
      updated_at: "",
    });
  }

  chapters.reverse();
  chapters.forEach((chapter, index) => {
    chapter.order = index + 1;
  });

  return {
    detail: {
      id: providerId(rawId(id)),
      title,
      author,
      description,
      thumb_url: normalizeBaoziImageUrl(cover),
      categories: [SOURCE_LABEL, ...tags.slice(0, 3)],
      tags,
      likes_count: 0,
      total_views: 0,
      eps_count: chapters.length,
      pages_count: 0,
      finished,
      is_liked: false,
      is_favourite: false,
      chinese_team: "",
      comments_count: 0,
      allow_comment: false,
      updated_at: "",
      created_at: "",
    },
    chapters,
  };
}

export function parseBaoziChapterPage(html: string, currentUrl: string): { images: string[]; nextUrl: string } {
  const document = parseHtml(html);
  const images = Array.from(
    document.querySelectorAll(".comic-contain > div:not(#div_top_ads):not(.mobadsq) amp-img"),
  )
    .map((element) => element.getAttribute("src") || element.getAttribute("data-src") || "")
    .map((value) => normalizeBaoziImageUrl(value, currentUrl))
    .filter(Boolean);
  const next = Array.from(document.querySelectorAll<HTMLAnchorElement>("div.next_chapter a"))
    .find((element) => text(element).includes("下一页"));
  return {
    images,
    nextUrl: next ? absoluteUrl(next.getAttribute("href") || "", currentUrl) : "",
  };
}

export async function searchBaozi(fetchText: BaoziTextFetch, keyword: string): Promise<ComicSummary[]> {
  const url = `${BAOZI_BASE}/search?q=${encodeURIComponent(keyword.trim())}`;
  return parseBaoziSearchResults(await fetchText(url));
}

export async function loadBaoziDetail(
  fetchText: BaoziTextFetch,
  id: string,
): Promise<{ detail: ComicDetail; chapters: ComicChapter[] }> {
  const comicId = rawId(id).replace(/^\/+|\/+$/g, "");
  if (!comicId || comicId.includes("..")) throw new Error("包子漫画 ID 无效");
  return parseBaoziDetail(await fetchText(`${BAOZI_BASE}/comic/${comicId}`), comicId);
}

export async function loadBaoziChapterImages(fetchText: BaoziTextFetch, chapterUrl: string): Promise<ComicImage[]> {
  let currentUrl = chapterUrl;
  const visitedPages = new Set<string>();
  const seenImages = new Set<string>();
  const images: ComicImage[] = [];

  for (let page = 0; page < 80 && currentUrl && !visitedPages.has(currentUrl); page += 1) {
    visitedPages.add(currentUrl);
    const parsed = parseBaoziChapterPage(await fetchText(currentUrl), currentUrl);
    for (const url of parsed.images) {
      if (seenImages.has(url)) continue;
      seenImages.add(url);
      images.push({ id: `baozi-page-${images.length + 1}`, url });
    }
    currentUrl = parsed.nextUrl;
  }

  if (images.length === 0) throw new Error("包子漫画未返回可阅读图片");
  return images;
}
