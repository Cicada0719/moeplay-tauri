// historyApi.ts 单测（spec §6.1 U14~U15，mock Tauri invoke）
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { clearMockInvokeHandler, setMockInvokeHandler } from "../api/core";
import { deleteHistory, historyCache, queryHistory } from "./historyApi";
import type { HistoryItem } from "./types";

function makeItem(id: string, overrides: Partial<HistoryItem> = {}): HistoryItem {
  return {
    id,
    contentId: `content-${id}`,
    contentType: "manga",
    title: `标题 ${id}`,
    cover: null,
    sourceId: "src-a",
    chapterId: null,
    chapterTitle: null,
    pageIndex: 0,
    positionSec: 0,
    scrollPct: 0,
    progress: 0,
    updatedAt: Date.now(),
    deviceId: "dev",
    deleted: false,
    ...overrides,
  };
}

beforeEach(() => {
  setMockInvokeHandler(() => {
    throw new Error("unexpected invoke in this test");
  });
});

afterEach(() => {
  clearMockInvokeHandler();
  historyCache.set(null);
  vi.restoreAllMocks();
});

describe("queryHistory", () => {
  it("U14：透传筛选参数并返回分页结构，同时写入缓存", async () => {
    const item = makeItem("1", { contentType: "novel" });
    const listHandler = vi.fn(() => [item]);
    setMockInvokeHandler(listHandler);

    const page = await queryHistory({ contentType: "novel", keyword: "标题" });

    expect(listHandler).toHaveBeenCalledWith("history_list", {
      contentType: "novel",
      keyword: "标题",
      limit: 100000,
      offset: 0,
    });
    expect(page.items).toEqual([item]);
    expect(page.total).toBe(1);
    expect(get(historyCache)).toEqual([item]);
  });

  it("U14：keyword 为空 / contentType 缺省时传 null（适配子任务 4 签名）", async () => {
    const listHandler = vi.fn(() => []);
    setMockInvokeHandler(listHandler);
    await queryHistory({});
    expect(listHandler).toHaveBeenCalledWith("history_list", {
      contentType: null,
      keyword: null,
      limit: 100000,
      offset: 0,
    });
  });

  it("U14：invoke 抛错时转为中文语境 Error", async () => {
    setMockInvokeHandler(() => {
      throw new Error("backend boom");
    });
    await expect(queryHistory()).rejects.toThrow("历史记录读取失败");
  });
});

describe("deleteHistory", () => {
  it("U15：删除后缓存中对应条目被剔除", async () => {
    const item1 = makeItem("1");
    const item2 = makeItem("2");
    setMockInvokeHandler((command, args) => {
      if (command === "history_list") return [item1, item2];
      if (command === "history_delete") {
        expect(args).toHaveProperty("id");
        return undefined;
      }
      throw new Error(`unexpected command ${command}`);
    });

    await queryHistory({});
    expect(get(historyCache)).toHaveLength(2);

    await deleteHistory([item1.id]);
    expect(get(historyCache)).toEqual([item2]);

    await deleteHistory(["missing-id"]);
    expect(get(historyCache)).toEqual([item2]);
  });

  it("U15：invoke 抛错时转为中文语境 Error 且不修改缓存", async () => {
    const item = makeItem("1");
    setMockInvokeHandler((command) => {
      if (command === "history_list") return [item];
      throw new Error("db locked");
    });
    await queryHistory({});
    await expect(deleteHistory([item.id])).rejects.toThrow("删除历史记录失败");
    expect(get(historyCache)).toHaveLength(1);
  });
});
