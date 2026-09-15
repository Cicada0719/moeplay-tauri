import { beforeEach, describe, expect, it } from "vitest";
import { collectionStore, type AnimeCollect } from "./collection.svelte";

const DETAIL = { image: "cover.jpg", ruleName: "route", sourceUrl: "https://x" };

describe("collection.svelte 收藏集", () => {
  beforeEach(() => {
    localStorage.clear();
    // 重置模块状态
    collectionStore.setCollect("x", 0, undefined, DETAIL);
    collectionStore.filter = 0;
  });

  it("默认空列表 + 加收藏", () => {
    expect(collectionStore.items).toHaveLength(0);
    collectionStore.setCollect("a", 1, undefined, DETAIL);
    expect(collectionStore.items).toHaveLength(1);
    expect(collectionStore.getType("a")).toBe(1);
    const entry = collectionStore.items[0];
    expect(entry.name).toBe("a");
    expect(entry.image).toBe("cover.jpg");
    expect(entry.ruleSource).toBe("route");
    expect(entry.updatedAt).toBeTruthy();
  });

  it("extra 覆盖详情上下文；更新已有条目", () => {
    collectionStore.setCollect("a", 1, undefined, DETAIL);
    collectionStore.setCollect("a", 2, { image: "alt.jpg" }, DETAIL);
    expect(collectionStore.items).toHaveLength(1);
    expect(collectionStore.getType("a")).toBe(2);
    expect(collectionStore.items[0].image).toBe("alt.jpg");
    expect(collectionStore.items[0].ruleSource).toBe("route");
  });

  it("collectType=0 移除收藏", () => {
    collectionStore.setCollect("a", 1, undefined, DETAIL);
    collectionStore.setCollect("a", 0, undefined, DETAIL);
    expect(collectionStore.items).toHaveLength(0);
    expect(collectionStore.getType("a")).toBe(0);
  });

  it("filter/filtered 默认全量，设定后按类型过滤", () => {
    collectionStore.setCollect("a", 1, undefined, DETAIL);
    collectionStore.setCollect("b", 2, undefined, DETAIL);
    expect(collectionStore.filtered).toHaveLength(2);
    collectionStore.filter = 1;
    const filtered = collectionStore.filtered;
    expect(filtered).toHaveLength(1);
    expect(filtered[0].key).toBe("a");
    collectionStore.filter = 0;
    expect(collectionStore.filtered).toHaveLength(2);
  });

  it("写入 localStorage 持久化（键 anime-collect）", () => {
    collectionStore.setCollect("persist", 4, undefined, DETAIL);
    const raw = localStorage.getItem("anime-collect");
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw ?? "[]") as AnimeCollect[];
    expect(parsed.some((c) => c.key === "persist" && c.collectType === 4)).toBe(true);
  });

  it("持久化追番稳定身份字段", () => {
    collectionStore.setCollect("identity", 1, { contentId: "show-id", seasonKey: "season-2" }, DETAIL);
    expect(collectionStore.items[0]).toMatchObject({ contentId: "show-id", seasonKey: "season-2" });
    expect(JSON.parse(localStorage.getItem("anime-collect") || "[]")[0]).toMatchObject({
      contentId: "show-id",
      seasonKey: "season-2",
    });
  });
});
