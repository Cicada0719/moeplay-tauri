import { beforeEach, describe, expect, it } from "vitest";
import { animeSearchHistoryStore } from "./history.svelte";

describe("history.svelte 番剧搜索历史", () => {
  beforeEach(() => {
    localStorage.clear();
    animeSearchHistoryStore.clear();
  });

  it("add 去重且保持最近在前，最多 20 条", () => {
    animeSearchHistoryStore.add("夏日");
    animeSearchHistoryStore.add("星空");
    animeSearchHistoryStore.add("夏日"); // 去重：移到最前
    expect(animeSearchHistoryStore.items).toEqual(["夏日", "星空"]);
    for (let i = 0; i < 25; i += 1) animeSearchHistoryStore.add(`k${i}`);
    expect(animeSearchHistoryStore.items).toHaveLength(20);
  });

  it("空白关键词不写入", () => {
    animeSearchHistoryStore.add("   ");
    expect(animeSearchHistoryStore.items).toHaveLength(0);
  });

  it("remove / clear", () => {
    animeSearchHistoryStore.add("a");
    animeSearchHistoryStore.add("b");
    animeSearchHistoryStore.remove("a");
    expect(animeSearchHistoryStore.items).toEqual(["b"]);
    animeSearchHistoryStore.clear();
    expect(animeSearchHistoryStore.items).toHaveLength(0);
    expect(localStorage.getItem("anime-search-history")).toBe("[]");
  });
});
