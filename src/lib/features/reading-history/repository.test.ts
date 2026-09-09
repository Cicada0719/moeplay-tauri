import { beforeEach, describe, expect, it, vi } from "vitest";
import { IDBFactory, IDBDatabase } from "fake-indexeddb";
import { ReadingRepository, bookKey, latestBooks, migrateLegacy, validPosition, type ReadingPosition } from "./repository";

const position = (chapter = "30", updatedAt = 1000): ReadingPosition => ({
  kind: "novel", source: "gutenberg", contentId: "book", title: "测试书",
  chapterId: chapter, chapterTitle: `第 ${chapter} 章`, progress: .45, updatedAt,
  metadata: { book: { id: "book", source: "gutenberg", title: "测试书", subjects: [], publicDomain: true, sourceUrl: "" } },
});
beforeEach(() => { globalThis.indexedDB = new IDBFactory(); localStorage.clear(); });

describe("reading history v2", () => {
  it("does not resurrect a deleted book from an earlier failed save", async () => {
    const repo = new ReadingRepository(); await repo.save(position());
    const transaction = vi.spyOn(IDBDatabase.prototype, "transaction");
    transaction.mockImplementationOnce(() => { throw new Error("quota"); });
    await expect(repo.save(position("30", 2000))).rejects.toThrow("quota");
    transaction.mockRestore();
    await repo.remove(bookKey(position()));
    await repo.retry();
    expect(repo.positions).toHaveLength(0);
    expect(JSON.parse(await repo.exportJSON()).positions).toHaveLength(0);
  });
  it("retains other books after 80 chapters and resumes most recently visited, not furthest", async () => {
    const repo = new ReadingRepository();
    const other = { ...position(), contentId: "other", title: "另一本书", metadata: { book: { id: "other", source: "gutenberg", title: "另一本书" } } };
    await repo.save(other);
    for (let i = 0; i < 80; i++) await repo.save(position(String(i), 2000 + i));
    await repo.save(position("30", 3000));
    expect(repo.positions).toHaveLength(81);
    expect(latestBooks(repo.positions)).toHaveLength(2);
    expect(latestBooks(repo.positions)[0]).toMatchObject({ chapterId: "30", progress: .45 });
    const reopened = new ReadingRepository(); await reopened.init();
    expect(reopened.positions).toHaveLength(81);
  });
  it("migrates valid rows idempotently, preserves original bytes and ignores damaged rows", async () => {
    const legacy = JSON.stringify([{ id: "mangadex:m", title: "漫画", last_order: 12, last_title: "12话", ts: 50 }, null, { id: "broken" }]);
    localStorage.setItem("picacg-history", legacy);
    localStorage.setItem("moeplay-novel-history-v1", "broken json");
    const repo = new ReadingRepository(); await repo.init();
    expect(repo.positions).toHaveLength(1);
    expect(repo.positions[0]).toMatchObject({ chapterId: "12", pageIndex: 0, source: "mangadex", contentId: "m" });
    await repo.remove(bookKey(repo.positions[0]));
    const reopened = new ReadingRepository(); await reopened.init();
    expect(reopened.positions).toHaveLength(0);
    expect(localStorage.getItem("picacg-history")).toBe(legacy);
  });
  it("restores chapter 12 page 7 after restart and round-trips backup", async () => {
    const repo = new ReadingRepository();
    const comic: ReadingPosition = { ...position("12"), kind: "comic", pageIndex: 6, pageId: "stable-page-7" };
    await repo.save(comic);
    const json = await repo.exportJSON();
    await repo.remove(bookKey(comic));
    expect((await repo.importJSON(json)).imported).toBe(1);
    const reopened = new ReadingRepository(); await reopened.init();
    expect(reopened.positions[0]).toEqual(comic);
  });
  it("does not overwrite newer chapter positions with an old import or another window", async () => {
    const first = new ReadingRepository(); const second = new ReadingRepository();
    await Promise.all([first.init(), second.init()]);
    await first.save(position("30", 2000));
    await second.save(position("30", 1000));
    expect(second.positions[0].updatedAt).toBe(2000);
    await expect(first.importJSON('{"version":1}')).rejects.toThrow("不支持");
    const result = await first.importJSON(JSON.stringify({ format: "moeplay-reading-history", version: 2, positions: [position("31"), null, { progress: NaN }] }));
    expect(result).toEqual({ imported: 1, skipped: 2 });
  });
  it("reports storage failure, does not mark ready, and can retry without erasing legacy", async () => {
    const original = globalThis.indexedDB;
    Object.defineProperty(globalThis, "indexedDB", { configurable: true, writable: true, value: { open() { throw new Error("quota"); } } });
    const repo = new ReadingRepository();
    await expect(repo.save(position())).rejects.toThrow("quota");
    expect(repo.ready).toBe(false); expect(repo.error).toContain("重试");
    globalThis.indexedDB = original;
    await repo.retry();
    expect(repo.positions).toHaveLength(1); expect(repo.error).toBe("");
  });
  it("keeps identities unambiguous and rejects invalid progress and page indices", () => {
    expect(bookKey({ kind: "novel", source: "a:b", contentId: "c" })).not.toBe(bookKey({ kind: "novel", source: "a", contentId: "b:c" }));
    for (const progress of [NaN, Infinity, -1, 1.1]) expect(validPosition({ ...position(), progress })).toBe(false);
    expect(validPosition({ ...position(), pageIndex: -1 })).toBe(false);
    expect(migrateLegacy([null, {}], [null, {}])).toEqual([]);
  });
});
