import { describe, expect, it, vi } from "vitest";
import {
  getSourcesFor,
  setSourceProvider,
  setSourceSwitchHandler,
  sortSourcesByHealth,
  switchSource,
  type SourceInfo,
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
});

describe("getSourcesFor", () => {
  it("使用已注册 provider；未注册时返回空列表", () => {
    setSourceProvider((contentType) => [{ id: "s", name: "Src", contentType, health: "ok" }]);
    expect(getSourcesFor("anime")).toHaveLength(1);

    setSourceProvider(null);
    expect(getSourcesFor("anime")).toEqual([]);
  });
});
