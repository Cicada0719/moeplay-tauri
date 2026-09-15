import { beforeEach, describe, expect, it, vi } from "vitest";
import { IDBFactory } from "fake-indexeddb";
import { historyStore, type AnimeHistory } from "../anime-player/historyStore.svelte";
import { ReadingRepository, type ReadingPosition } from "../reading-history/repository";
import { parseMediaHistoryBackup } from "./backup";

const book = (source: string, id: string, title = id) => ({ id, source, title, subjects: [], publicDomain: true, sourceUrl: "" });
const novelPosition = (source: string, id: string, chapter: string, updatedAt: number): ReadingPosition => ({
  kind: "novel", source, contentId: id, title: id, chapterId: chapter, chapterTitle: chapter,
  progress: .45, updatedAt, metadata: { book: book(source, id) },
});
const anime = (key: string): AnimeHistory => ({ key, name: key, image: "", ruleName: "source", sourceUrl: "https://example.test/?token=secret",
  lastRoad: 0, lastEpisode: 1, lastEpisodeName: "第 2 集", progressMs: 1200, updatedAt: new Date(1000).toISOString() });

beforeEach(() => {
  globalThis.indexedDB = new IDBFactory();
  localStorage.clear();
  historyStore.clear();
});

describe("media history backup format", () => {
  it("parses all three types and isolates malformed rows without exporting secrets", () => {
    const parsed = parseMediaHistoryBackup({
      format: "moeplay-media-history", version: 1, exportedAt: 10, coverage: ["anime", "manga", "novel"],
      anime: [{ ...anime("anime-1"), sourceId: "source", title: "番剧", updatedAt: 10 }, null],
      manga: [{ sourceId: "manga-source", contentId: "m1", title: "漫画", chapterId: "c1", chapterTitle: "第一话", pageIndex: 6, pageId: "p7", updatedAt: 11 }, { pageIndex: -1 }],
      novel: [{ sourceId: "novel-source", contentId: "n1", title: "小说", chapterId: "c1", chapterTitle: "第一章", progress: .4, updatedAt: 12 }],
    });
    expect(parsed.backup.anime).toHaveLength(1);
    expect(parsed.backup.manga[0]).toMatchObject({ chapterId: "c1", pageIndex: 6, pageId: "p7" });
    expect(parsed.backup.novel[0]).toMatchObject({ chapterId: "c1", progress: .4 });
    expect(parsed.skipped).toBe(2);
    expect(JSON.stringify(parsed.backup)).not.toContain("secret");
  });

  it("accepts reading-history v2 and clearly identified legacy arrays", () => {
    const v2 = parseMediaHistoryBackup(JSON.stringify({ format: "moeplay-reading-history", version: 2, positions: [novelPosition("s", "n", "c", 5)] }));
    expect(v2.backup.novel).toHaveLength(1);
    const legacy = parseMediaHistoryBackup([{ id: "mangadex:m", title: "漫画", last_order: 12, last_title: "12话", ts: 50 }, { nope: true }]);
    expect(legacy.backup.manga[0]).toMatchObject({ sourceId: "mangadex", contentId: "m", pageIndex: 0 });
    expect(legacy.skipped).toBe(1);
  });
});

describe("backup merge behavior", () => {
  it("keeps media identities separate and makes equal/repeated imports idempotent", async () => {
    const repo = new ReadingRepository();
    await repo.importPositions([novelPosition("source-a", "same-id", "c1", 10), novelPosition("source-b", "same-id", "c1", 10)]);
    expect(repo.positions).toHaveLength(2);
    const repeated = await repo.importPositions([novelPosition("source-a", "same-id", "c1", 10)]);
    expect(repeated.imported).toBe(0);
    expect(repo.positions).toHaveLength(2);
    historyStore.importEntries([anime("a")]);
    expect(historyStore.importEntries([anime("a")]).imported).toBe(0);
  });

  it("does not count a localStorage failure as imported", () => {
    const original = localStorage.setItem;
    vi.spyOn(localStorage, "setItem").mockImplementation(() => { throw new Error("quota"); });
    expect(historyStore.importEntries([anime("failed")])).toMatchObject({ imported: 0, failed: 1 });
    expect(historyStore.get("failed")).toBeUndefined();
    vi.spyOn(localStorage, "setItem").mockRestore();
    localStorage.setItem = original;
  });
});
