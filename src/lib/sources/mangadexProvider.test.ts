import { describe, expect, it } from "vitest";

import {
  loadMangaDexChapterImages,
  loadMangaDexChapters,
  loadMangaDexDetail,
  searchMangaDex,
  type MangaDexFetch,
} from "./mangadexProvider";

function response(options: { ok?: boolean; status?: number; body?: unknown }): Pick<Response, "ok" | "status" | "json"> {
  return {
    ok: options.ok ?? true,
    status: options.status ?? 200,
    json: async () => options.body,
  };
}

describe("mangadex provider", () => {
  it("searches public manga and maps results into shared comic summaries", async () => {
    const urls: string[] = [];
    const fetcher: MangaDexFetch = async (url) => {
      urls.push(url);
      return response({
        body: {
          data: [
            {
              id: "m1",
              attributes: { title: { en: "Frieren" }, status: "ongoing" },
              relationships: [
                { type: "cover_art", attributes: { fileName: "cover.jpg" } },
                { type: "author", attributes: { name: "Yamada" } },
              ],
            },
          ],
        },
      });
    };

    const results = await searchMangaDex(fetcher, "frieren");

    expect(urls[0]).toContain("title=frieren");
    expect(urls[0]).toContain("contentRating%5B%5D=safe");
    expect(results[0]).toMatchObject({
      id: "mangadex:m1",
      title: "Frieren",
      author: "Yamada",
      thumb_url: "https://uploads.mangadex.org/covers/m1/cover.jpg.256.jpg",
    });
  });

  it("loads details and chapters into the existing reader contract", async () => {
    const detail = await loadMangaDexDetail(
      async () => response({
        body: {
          data: {
            id: "m1",
            attributes: {
              title: { en: "Frieren" },
              description: { en: "Journey after the end." },
              tags: [{ attributes: { name: { en: "Fantasy" } } }],
              status: "completed",
            },
            relationships: [{ type: "cover_art", attributes: { fileName: "cover.jpg" } }],
          },
        },
      }),
      "m1",
    );
    const chapters = await loadMangaDexChapters(
      async () => response({
        body: {
          data: [
            { id: "c1", attributes: { chapter: "1", title: "The End", translatedLanguage: "en", pages: 20 } },
            { id: "c2", attributes: { chapter: "1", title: "终章", translatedLanguage: "zh", pages: 18 } },
          ],
        },
      }),
      "m1",
    );

    expect(detail).toMatchObject({
      id: "mangadex:m1",
      title: "Frieren",
      description: "Journey after the end.",
      tags: ["Fantasy"],
      finished: true,
    });
    expect(chapters).toEqual([
      { id: "c1", title: "The End · en", order: 1, updated_at: "" },
      { id: "c2", title: "终章 · zh", order: 2, updated_at: "" },
    ]);
  });

  it("paginates the feed beyond MangaDex's 100-chapter page limit", async () => {
    const urls: string[] = [];
    const fetcher: MangaDexFetch = async (url) => {
      urls.push(url);
      const offset = Number(new URL(url).searchParams.get("offset") ?? 0);
      const count = offset === 0 ? 100 : 20;
      return response({
        body: {
          result: "ok",
          total: 120,
          data: Array.from({ length: count }, (_, index) => ({
            id: `c${offset + index + 1}`,
            attributes: { chapter: String(offset + index + 1), translatedLanguage: "en", pages: 10 },
          })),
        },
      });
    };

    const chapters = await loadMangaDexChapters(fetcher, "m1", 150);

    expect(urls).toHaveLength(2);
    expect(new URL(urls[0]).searchParams.get("limit")).toBe("100");
    expect(new URL(urls[0]).searchParams.get("offset")).toBe("0");
    expect(new URL(urls[1]).searchParams.get("offset")).toBe("100");
    expect(chapters).toHaveLength(120);
    expect(chapters.at(-1)).toMatchObject({ id: "c120", order: 120 });
  });

  it("drops external, unavailable and zero-page chapters that cannot be read in-app", async () => {
    const chapters = await loadMangaDexChapters(
      async () => response({
        body: {
          result: "ok",
          total: 4,
          data: [
            { id: "external", attributes: { chapter: "1", translatedLanguage: "en", pages: 0, externalUrl: "https://example.com/read" } },
            { id: "unavailable", attributes: { chapter: "2", translatedLanguage: "en", pages: 12, isUnavailable: true } },
            { id: "empty", attributes: { chapter: "3", translatedLanguage: "en", pages: 0 } },
            { id: "readable", attributes: { chapter: "4", translatedLanguage: "en", pages: 14 } },
          ],
        },
      }),
      "m1",
    );

    expect(chapters).toEqual([
      { id: "readable", title: "第 4 话 · en", order: 1, updated_at: "" },
    ]);
  });

  it("surfaces a MangaDex error payload even when the HTTP response is successful", async () => {
    await expect(searchMangaDex(
      async () => response({
        body: {
          result: "error",
          errors: [{ title: "Bad Request", detail: "Invalid order parameter" }],
        },
      }),
      "frieren",
    )).rejects.toThrow("MangaDex 请求失败：Invalid order parameter");
  });

  it("loads chapter images from MangaDex at-home server", async () => {
    const images = await loadMangaDexChapterImages(
      async () => response({
        body: {
          baseUrl: "https://uploads.example",
          chapter: {
            hash: "hash",
            dataSaver: ["1.jpg", "2.jpg"],
            data: ["1-large.jpg"],
          },
        },
      }),
      "c1",
    );

    expect(images).toEqual([
      { id: "c1:1", url: "https://uploads.example/data-saver/hash/1.jpg" },
      { id: "c1:2", url: "https://uploads.example/data-saver/hash/2.jpg" },
    ]);
  });
});
