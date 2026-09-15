import { afterEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "./core";
import { offlineApi } from "./offline";

afterEach(() => clearMockInvokeHandler());

describe("offline command API", () => {
  it("keeps the Task 05 request envelope and command names stable", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    setMockInvokeHandler((command, args) => {
      calls.push({ command, args });
      if (command === "offline_enqueue") return { chapters: [] };
      if (command === "offline_list") return [];
      if (command === "offline_stats") return { chapterCount: 0, completeCount: 0, pendingCount: 0, failedCount: 0, bytes: 0 };
      return { offlineChapterKey: "key", readable: false };
    });
    await offlineApi.enqueue({ contentType: "novel", sourceId: "gutenberg", contentId: "book", title: "Book", chapters: [{ chapterId: "45" }] });
    await offlineApi.supply({ chapterKey: "key", body: "text" });
    await offlineApi.list();
    await offlineApi.getChapter("key");
    await offlineApi.control({ chapterKey: "key", action: "retry" });
    await offlineApi.stats();
    expect(calls.map((call) => call.command)).toEqual([
      "offline_enqueue", "offline_supply_chapter", "offline_list", "offline_get_chapter", "offline_control", "offline_stats",
    ]);
    expect(calls[0].args).toEqual({ request: expect.objectContaining({ contentType: "novel", chapters: [{ chapterId: "45" }] }) });
    expect(calls[4].args).toEqual({ request: { chapterKey: "key", action: "retry" } });
  });
});
