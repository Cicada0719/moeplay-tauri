import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import {
  getSourcesFor,
  setSourceProvider,
  setSourceSwitchHandler,
  sortSourcesByHealth,
  sourcesFor,
  switchSource,
  validateSwitchSourceParams,
  type SourceInfo,
  type SwitchSourceParams,
} from "./sourceSwitch";

describe("sortSourcesByHealth", () => {
  it("按健康度排序：ok > unknown > degraded，同级按名称", () => {
    const sources: SourceInfo[] = [
      { id: "a", name: "Alpha", contentType: "anime", health: "degraded" },
      { id: "b", name: "Bravo", contentType: "anime", health: "unknown" },
      { id: "c", name: "Charlie", contentType: "anime", health: "ok" },
      { id: "d", name: "Delta", contentType: "anime", health: "ok" },
    ];
    expect(sortSourcesByHealth(sources).map((s) => s.id)).toEqual(["c", "d", "b", "a"]);
  });
});

describe("switchSource", () => {
  it("调用已注册处理器并携带进度保持参数", async () => {
    const handler = vi.fn().mockResolvedValue(undefined);
    setSourceSwitchHandler(handler);

    await switchSource({ contentId: "anime-1", chapterId: "ep5", positionSec: 750, targetSourceId: "src-b" });
    expect(handler).toHaveBeenCalledWith({
      contentId: "anime-1",
      chapterId: "ep5",
      positionSec: 750,
      targetSourceId: "src-b",
    });

    setSourceSwitchHandler(null);
  });

  it("未注册处理器时走 TODO 桩（不抛错）", async () => {
    setSourceSwitchHandler(null);
    await expect(switchSource({ contentId: "anime-1", targetSourceId: "src-b" })).resolves.toBeUndefined();
  });

  it("参数校验：contentId 为空 / positionSec 为负 / targetSourceId 非字符串时抛错", async () => {
    const bad: Partial<SwitchSourceParams>[] = [
      { contentId: "", targetSourceId: "src-b" },
      { contentId: "anime-1", positionSec: -1, targetSourceId: "src-b" },
      { contentId: "anime-1", positionSec: Number.NaN, targetSourceId: "src-b" },
      { contentId: "anime-1", targetSourceId: 123 as unknown as string },
    ];
    for (const params of bad) {
      await expect(switchSource(params as SwitchSourceParams)).rejects.toThrow(TypeError);
    }
    // 空 targetSourceId（打开选源面板）与缺失可选字段为合法参数
    await expect(switchSource({ contentId: "anime-1", targetSourceId: "" })).resolves.toBeUndefined();
  });

  it("validateSwitchSourceParams 直接暴露供调用方预检", () => {
    expect(() => validateSwitchSourceParams({ contentId: "", targetSourceId: "" })).toThrow(TypeError);
    expect(() => validateSwitchSourceParams({ contentId: "a", positionSec: 1.5, targetSourceId: "s" })).not.toThrow();
  });
});

describe("getSourcesFor", () => {
  it("使用已注册 provider；未注册时返回空列表", () => {
    setSourceProvider((contentType) => [{ id: "s", name: "Src", contentType, health: "ok" }]);
    expect(getSourcesFor("anime")).toHaveLength(1);

    setSourceProvider(null);
    expect(getSourcesFor("anime")).toEqual([]);
  });
});

describe("sourcesFor 可读 store", () => {
  it("响应式读取适配层源健康列表，provider 重注册后自动刷新", () => {
    setSourceProvider(() => [{ id: "s1", name: "Src1", contentType: "anime", health: "ok" }]);
    const store = sourcesFor("anime");
    expect(get(store).map((s) => s.id)).toEqual(["s1"]);

    setSourceProvider(() => [
      { id: "s2", name: "Src2", contentType: "anime", health: "degraded" },
      { id: "s1", name: "Src1", contentType: "anime", health: "ok" },
    ]);
    expect(get(store).map((s) => s.id)).toEqual(["s2", "s1"]);

    setSourceProvider(null);
    expect(get(store)).toEqual([]);
  });
});
