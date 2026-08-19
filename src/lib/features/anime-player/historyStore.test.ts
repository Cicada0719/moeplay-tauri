import { beforeEach, describe, expect, it } from "vitest";
import { historyStore, type AnimeHistory } from "./historyStore.svelte";

function entry(key: string, progressMs = 0): AnimeHistory {
  return { key, name: key, image: "", ruleName: "r", sourceUrl: "u", lastRoad: 0, lastEpisode: 0, lastEpisodeName: "", progressMs, updatedAt: "2026-01-01" };
}

describe("historyStore.svelte 番剧观看历史", () => {
  beforeEach(() => {
    localStorage.clear();
    historyStore.clear();
  });

  it("upsert 新建置顶，覆盖已有条目（保持原位）", () => {
    historyStore.upsert(entry("a"));
    historyStore.upsert(entry("b"));
    historyStore.upsert(entry("a", 5000));
    expect(historyStore.items).toHaveLength(2);
    // 覆盖已有条目不改变原有顺序（与原 store 行为一致）
    expect(historyStore.items.map((h) => h.key)).toEqual(["b", "a"]);
    expect(historyStore.get("a")?.progressMs).toBe(5000);
  });

  it("上限 200 条", () => {
    for (let i = 0; i < 250; i += 1) historyStore.upsert(entry(`k${i}`, i));
    expect(historyStore.items).toHaveLength(200);
  });

  it("remove / clear / get 缺失返回 undefined", () => {
    historyStore.upsert(entry("a"));
    expect(historyStore.get("nope")).toBeUndefined();
    historyStore.remove("a");
    expect(historyStore.get("a")).toBeUndefined();
    historyStore.upsert(entry("a"));
    historyStore.clear();
    expect(historyStore.items).toHaveLength(0);
    expect(localStorage.getItem("anime-history")).toBe("[]");
  });
});
