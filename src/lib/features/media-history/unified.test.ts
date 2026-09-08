import { describe, expect, it } from "vitest";
import type { AnimeHistory } from "../anime-player/historyStore.svelte";
import type { NovelHistoryEntry } from "../novel/types";
import type { ReadRecord } from "../../stores/comic.svelte";
import { buildUnifiedMediaHistory, mediaHistoryActionLabel, mediaHistoryKindLabel } from "./unified";

const anime: AnimeHistory = {
  key: "rule:番剧",
  name: "夏日动画",
  image: "anime.jpg",
  ruleName: "本地源",
  sourceUrl: "https://example.test/anime",
  lastRoad: 0,
  lastEpisode: 2,
  lastEpisodeName: "第 3 集",
  progressMs: 12_000,
  updatedAt: "2026-08-31T12:00:00Z",
};
const comic: ReadRecord = {
  id: "comic-1",
  title: "海边漫画",
  thumb_url: "comic.jpg",
  author: "作者",
  last_order: 8,
  last_title: "第 8 话",
  ts: new Date("2026-08-31T11:00:00Z").getTime(),
};
const novel: NovelHistoryEntry = {
  key: "biquge:novel-1:chapter-2",
  book: { id: "novel-1", source: "biquge", title: "山间小说", subjects: [], publicDomain: false, sourceUrl: "", coverUrl: "novel.jpg" },
  chapterId: "chapter-2",
  chapterTitle: "第二章",
  progress: 0.42,
  updatedAt: new Date("2026-08-31T13:00:00Z").getTime(),
};

describe("unified media history", () => {
  it("keeps every book available for filtering before pagination", () => {
    const manyAnime = Array.from({ length: 140 }, (_, i) => ({ ...anime, key: String(i) }));
    const earlierChapter = { ...novel, key: "earlier", chapterId: "1", updatedAt: novel.updatedAt - 1 };
    const items = buildUnifiedMediaHistory({ anime: manyAnime, comic: [comic], novel: [earlierChapter, novel] });
    expect(items).toHaveLength(142);
    expect(items.filter(item => item.kind === "novel")).toHaveLength(1);
    expect(items.find(item => item.kind === "novel")?.payload).toBe(novel);
    expect(items.filter(item => item.kind === "comic")).toHaveLength(1);
  });
  it("merges anime, comic and novel histories in newest-first order", () => {
    const items = buildUnifiedMediaHistory({ anime: [anime], comic: [comic], novel: [novel] });
    expect(items.map((item) => item.kind)).toEqual(["novel", "anime", "comic"]);
    expect(items[0]).toMatchObject({ id: 'novel:["biquge","novel-1"]', title: "山间小说", positionLabel: "第二章 · 42%", progress: 0.42 });
    expect(items[1]).toMatchObject({ sourceLabel: "本地源", positionLabel: "第 3 集" });
  });

  it("keeps the original payload and applies a stable limit", () => {
    const items = buildUnifiedMediaHistory({ anime: [anime], comic: [comic], novel: [novel] }, 2);
    expect(items).toHaveLength(2);
    expect(items[0].payload).toBe(novel);
    expect(mediaHistoryKindLabel("comic")).toBe("漫画");
    expect(mediaHistoryActionLabel("anime")).toBe("继续观看");
  });

  it("does not discard a history item with an invalid timestamp", () => {
    const item = { ...anime, key: "invalid-time", updatedAt: "not-a-date" };
    expect(buildUnifiedMediaHistory({ anime: [item], comic: [], novel: [] })[0].updatedAt).toBe(0);
  });
});
