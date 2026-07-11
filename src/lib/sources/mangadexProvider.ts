import type { ComicChapter, ComicDetail, ComicImage, ComicSummary } from "../stores/comic.svelte";

export interface MangaDexMangaResponse {
  result?: string;
  data?: MangaDexManga[];
  errors?: MangaDexApiError[];
  limit?: number;
  offset?: number;
  total?: number;
}

export interface MangaDexSingleMangaResponse {
  result?: string;
  data?: MangaDexManga;
  errors?: MangaDexApiError[];
}

export interface MangaDexChapterResponse {
  result?: string;
  data?: MangaDexChapter[];
  errors?: MangaDexApiError[];
  limit?: number;
  offset?: number;
  total?: number;
}

export interface MangaDexAtHomeResponse {
  result?: string;
  errors?: MangaDexApiError[];
  baseUrl?: string;
  chapter?: {
    hash?: string;
    data?: string[];
    dataSaver?: string[];
  };
}

interface MangaDexApiError {
  title?: string;
  detail?: string;
  status?: number;
}

interface MangaDexManga {
  id: string;
  attributes?: {
    title?: Record<string, string>;
    altTitles?: Array<Record<string, string>>;
    description?: Record<string, string>;
    tags?: Array<{ attributes?: { name?: Record<string, string> } }>;
    status?: string;
    year?: number;
    updatedAt?: string;
    createdAt?: string;
  };
  relationships?: MangaDexRelationship[];
}

interface MangaDexChapter {
  id: string;
  attributes?: {
    title?: string;
    chapter?: string;
    pages?: number;
    translatedLanguage?: string;
    updatedAt?: string;
    externalUrl?: string | null;
    isUnavailable?: boolean;
  };
}

interface MangaDexRelationship {
  id: string;
  type: string;
  attributes?: {
    fileName?: string;
    name?: string;
  };
}

export type MangaDexFetch = (url: string, init?: RequestInit) => Promise<Pick<Response, "ok" | "status" | "json">>;

const MANGADEX_API = "https://api.mangadex.org";
const COVER_BASE = "https://uploads.mangadex.org/covers";
const SOURCE_NAME = "MangaDex";
const SOURCE_PREFIX = "mangadex:";
const FALLBACK_COVER = "";
const PREFERRED_LANGUAGES = ["zh", "zh-hk", "zh-ro", "en"];
const MAX_FEED_PAGE_SIZE = 100;
const DEFAULT_CHAPTER_LIMIT = 500;

function pickLocalized(value: Record<string, string> | undefined, fallback = ""): string {
  if (!value) return fallback;
  for (const lang of PREFERRED_LANGUAGES) {
    if (value[lang]) return value[lang];
  }
  return Object.values(value).find(Boolean) ?? fallback;
}

function coverUrl(manga: MangaDexManga): string {
  const cover = manga.relationships?.find((relationship) => relationship.type === "cover_art");
  const fileName = cover?.attributes?.fileName;
  return fileName ? `${COVER_BASE}/${manga.id}/${fileName}.256.jpg` : FALLBACK_COVER;
}

function authorName(manga: MangaDexManga): string {
  const author = manga.relationships?.find((relationship) => relationship.type === "author" || relationship.type === "artist");
  return author?.attributes?.name || SOURCE_NAME;
}

function chapterOrder(_chapter: MangaDexChapter, index: number): number {
  return index + 1;
}

function appendMulti(params: URLSearchParams, key: string, values: string[]) {
  for (const value of values) params.append(key, value);
}

async function requestJson<T extends { result?: string; errors?: MangaDexApiError[] }>(
  fetcher: MangaDexFetch,
  path: string,
  params?: URLSearchParams,
): Promise<T> {
  const url = `${MANGADEX_API}${path}${params ? `?${params}` : ""}`;
  const response = await fetcher(url, { headers: { accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`MangaDex 返回 HTTP ${response.status}`);
  }
  const payload = (await response.json()) as T;
  if (payload.result && payload.result !== "ok") {
    const error = payload.errors?.[0];
    const message = error?.detail || error?.title || "API 返回失败结果";
    throw new Error(`MangaDex 请求失败：${message}`);
  }
  return payload;
}

export function normalizeMangaDexSummary(manga: MangaDexManga): ComicSummary {
  return {
    id: `${SOURCE_PREFIX}${manga.id}`,
    title: pickLocalized(manga.attributes?.title, "MangaDex"),
    author: authorName(manga),
    thumb_url: coverUrl(manga),
    categories: ["MangaDex"],
    likes_count: 0,
    total_views: 0,
    eps_count: 0,
    finished: manga.attributes?.status === "completed",
  };
}

export function normalizeMangaDexDetail(manga: MangaDexManga): ComicDetail {
  const tags = manga.attributes?.tags?.map((tag) => pickLocalized(tag.attributes?.name)).filter(Boolean) ?? [];
  const title = pickLocalized(manga.attributes?.title, "MangaDex");
  const altTitle = manga.attributes?.altTitles?.map((item) => pickLocalized(item)).find(Boolean);

  return {
    id: `${SOURCE_PREFIX}${manga.id}`,
    title,
    author: authorName(manga),
    description: pickLocalized(manga.attributes?.description, altTitle || "来自 MangaDex 的漫画条目"),
    thumb_url: coverUrl(manga),
    categories: ["MangaDex"],
    tags,
    likes_count: 0,
    total_views: 0,
    eps_count: 0,
    pages_count: 0,
    finished: manga.attributes?.status === "completed",
    is_liked: false,
    is_favourite: false,
    chinese_team: SOURCE_NAME,
    comments_count: 0,
    allow_comment: false,
    updated_at: manga.attributes?.updatedAt ?? "",
    created_at: manga.attributes?.createdAt ?? "",
  };
}

export function normalizeMangaDexChapters(chapters: MangaDexChapter[]): ComicChapter[] {
  return chapters
    .filter((chapter) => (
      !chapter.attributes?.externalUrl
      && !chapter.attributes?.isUnavailable
      && (chapter.attributes?.pages ?? 1) > 0
    ))
    .map((chapter, index) => {
      const order = chapterOrder(chapter, index);
      const language = chapter.attributes?.translatedLanguage || "all";
      const title = chapter.attributes?.title
        || (chapter.attributes?.chapter ? `第 ${chapter.attributes.chapter} 话` : `第 ${order} 话`);

      return {
        id: chapter.id,
        title: `${title} · ${language}`,
        order,
        updated_at: chapter.attributes?.updatedAt ?? "",
      };
    });
}

export async function searchMangaDex(fetcher: MangaDexFetch, keyword: string, limit = 20): Promise<ComicSummary[]> {
  const params = new URLSearchParams();
  params.set("title", keyword);
  params.set("limit", String(limit));
  params.set("order[relevance]", "desc");
  params.append("includes[]", "cover_art");
  params.append("includes[]", "author");
  appendMulti(params, "contentRating[]", ["safe", "suggestive"]);

  const payload = await requestJson<MangaDexMangaResponse>(fetcher, "/manga", params);
  return (payload.data ?? []).map(normalizeMangaDexSummary);
}

export async function loadMangaDexDetail(fetcher: MangaDexFetch, mangaId: string): Promise<ComicDetail> {
  const params = new URLSearchParams();
  params.append("includes[]", "cover_art");
  params.append("includes[]", "author");
  const payload = await requestJson<MangaDexSingleMangaResponse>(fetcher, `/manga/${mangaId}`, params);
  if (!payload.data) throw new Error("MangaDex 未返回漫画详情");
  return normalizeMangaDexDetail(payload.data);
}

export async function loadMangaDexChapters(
  fetcher: MangaDexFetch,
  mangaId: string,
  limit = DEFAULT_CHAPTER_LIMIT,
): Promise<ComicChapter[]> {
  const requestedLimit = Math.max(0, Math.floor(limit));
  if (requestedLimit === 0) return [];

  const chapters: MangaDexChapter[] = [];
  let offset = 0;

  while (chapters.length < requestedLimit) {
    const pageSize = Math.min(MAX_FEED_PAGE_SIZE, requestedLimit - chapters.length);
    const params = new URLSearchParams();
    params.set("limit", String(pageSize));
    params.set("offset", String(offset));
    params.set("order[chapter]", "asc");
    appendMulti(params, "translatedLanguage[]", PREFERRED_LANGUAGES);

    const payload = await requestJson<MangaDexChapterResponse>(fetcher, `/manga/${mangaId}/feed`, params);
    const page = payload.data ?? [];
    chapters.push(...page);
    offset += page.length;

    if (page.length === 0 || offset >= (payload.total ?? offset) || page.length < pageSize) break;
  }

  return normalizeMangaDexChapters(chapters);
}

export async function loadMangaDexChapterImages(fetcher: MangaDexFetch, chapterId: string): Promise<ComicImage[]> {
  const payload = await requestJson<MangaDexAtHomeResponse>(fetcher, `/at-home/server/${chapterId}`);
  const baseUrl = payload.baseUrl;
  const hash = payload.chapter?.hash;
  const pages = payload.chapter?.dataSaver?.length ? payload.chapter.dataSaver : payload.chapter?.data;

  if (!baseUrl || !hash || !pages?.length) {
    throw new Error("MangaDex 未返回章节图片");
  }

  const quality = payload.chapter?.dataSaver?.length ? "data-saver" : "data";
  return pages.map((page, index) => ({
    id: `${chapterId}:${index + 1}`,
    url: `${baseUrl}/${quality}/${hash}/${page}`,
  }));
}
