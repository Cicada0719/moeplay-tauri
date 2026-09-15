import { describe, expect, it } from "vitest";
import { findOfflineChapter, offlineSupportsSource, selectOfflineChapters } from "./model";
import type { OfflineChapter } from "../../api/offline";

const chapters = Array.from({ length: 9 }, (_, index) => ({ id: `c${index + 1}`, order: index + 1, title: `第${index + 1}章` }));

describe("offline chapter selection", () => {
  it("expands current chapter plus the default five loaded entries", () => {
    expect(selectOfflineChapters(chapters, "c2").map((chapter) => chapter.id)).toEqual(["c2", "c3", "c4", "c5", "c6", "c7"]);
  });

  it("does not invent chapters beyond the loaded catalog", () => {
    expect(selectOfflineChapters(chapters, "c8", 5).map((chapter) => chapter.id)).toEqual(["c8", "c9"]);
    expect(selectOfflineChapters(chapters, "missing")).toEqual([]);
  });

  it("finds a matching chapter without conflating sources or content", () => {
    const row = { contentType: "manga" as const, sourceId: "baozi", contentId: "book", chapterId: "c2", offlineChapterKey: "key" } as unknown as OfflineChapter;
    expect(findOfflineChapter([row], row)).toBe(row);
    expect(findOfflineChapter([row], { ...row, sourceId: "dm5" })).toBeUndefined();
  });

  it("explains unsupported web and external readers", () => {
    expect(offlineSupportsSource("dm5").supported).toBe(false);
    expect(offlineSupportsSource("provider", "external").reason).toContain("外部");
    expect(offlineSupportsSource("baozi").supported).toBe(true);
  });
});
