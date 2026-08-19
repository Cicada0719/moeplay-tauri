import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCmd } from "../../api/core";
import { imageSearchStore } from "./imageSearch.svelte";

vi.mock("../../api/core", () => ({ invokeCmd: vi.fn() }));

const mockInvoke = vi.mocked(invokeCmd);

describe("imageSearch.svelte 图片搜番模块", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    imageSearchStore.clear();
  });

  it("search 成功填充结果并关闭 loading", async () => {
    mockInvoke.mockResolvedValueOnce([{ anilist_id: 1, similarity: 0.9 }] as never);
    const pending = imageSearchStore.search("https://img/x.png");
    expect(imageSearchStore.loading).toBe(true);
    await pending;
    expect(imageSearchStore.loading).toBe(false);
    expect(imageSearchStore.results).toHaveLength(1);
    expect(imageSearchStore.error).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith("anime_image_search", { imageUrl: "https://img/x.png" });
  });

  it("search 空地址不发起请求", async () => {
    await imageSearchStore.search("");
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("search 失败记录错误并清空结果", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("net"));
    await imageSearchStore.search("https://img/x.png");
    expect(imageSearchStore.loading).toBe(false);
    expect(imageSearchStore.results).toHaveLength(0);
    expect(imageSearchStore.error).toContain("net");
  });

  it("clear 清空结果与错误", async () => {
    mockInvoke.mockResolvedValueOnce([{ anilist_id: 1 }] as never);
    await imageSearchStore.search("https://img/x.png");
    imageSearchStore.clear();
    expect(imageSearchStore.results).toHaveLength(0);
    expect(imageSearchStore.error).toBeNull();
  });
});
