import { afterEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "../../api/core";
import { scheduleOfflineChapters } from "./scheduler";
import { OFFLINE_PARSE_CONCURRENCY } from "./runtime";

afterEach(() => clearMockInvokeHandler());

describe("offline chapter scheduler", () => {
  it("keeps source parsing at two concurrent chapters and supplies each result", async () => {
    let active = 0;
    let peak = 0;
    const supplied: string[] = [];
    setMockInvokeHandler((command, args) => {
      if (command === "offline_enqueue") {
        const request = (args as { request: { chapters: Array<{ chapterId: string }> } }).request;
        return { chapters: request.chapters.map((chapter) => ({ offlineChapterKey: `key-${chapter.chapterId}`, readable: false })) };
      }
      if (command === "offline_supply_chapter") {
        const request = (args as { request: { chapterKey: string } }).request;
        supplied.push(request.chapterKey);
        return { offlineChapterKey: request.chapterKey, readable: true };
      }
      throw new Error(`unexpected ${command}`);
    });
    const result = await scheduleOfflineChapters({
      contentType: "novel", sourceId: "source", contentId: "book", title: "Book", chapters: Array.from({ length: 5 }, (_, i) => ({ id: String(i), order: i, title: `Chapter ${i}` })), currentId: "0", following: 4,
      resolve: async (chapter) => { active += 1; peak = Math.max(peak, active); await new Promise((resolve) => setTimeout(resolve, 1)); active -= 1; return { body: chapter.id }; },
    });
    expect(OFFLINE_PARSE_CONCURRENCY).toBe(2);
    expect(peak).toBeLessThanOrEqual(2);
    expect(supplied).toHaveLength(5);
    expect(result.failures).toEqual([]);
    expect(result.chapters.every((chapter) => chapter.readable)).toBe(true);
  });

  it("reports resolver failures while allowing other chapters to finish", async () => {
    setMockInvokeHandler((command, args) => command === "offline_enqueue"
      ? { chapters: (args as { request: { chapters: Array<{ chapterId: string }> } }).request.chapters.map((chapter) => ({ offlineChapterKey: chapter.chapterId })) }
      : { offlineChapterKey: (args as { request: { chapterKey: string } }).request.chapterKey, readable: true });
    const result = await scheduleOfflineChapters({
      contentType: "manga", sourceId: "baozi", contentId: "book", title: "Book", chapters: [{ id: "1", order: 1 }, { id: "2", order: 2 }], currentId: "1", following: 1,
      resolve: async (chapter) => { if (chapter.id === "1") throw new Error("source unavailable"); return { resources: [] }; },
    });
    expect(result.failures).toEqual([{ chapterId: "1", error: "source unavailable" }]);
    expect(result.chapters[1].readable).toBe(true);
  });
});
