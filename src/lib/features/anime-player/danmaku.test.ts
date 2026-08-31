import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCmd } from "../../api/core";
import { danmakuStore } from "./danmaku.svelte";

vi.mock("../../api/core", () => ({ invokeCmd: vi.fn() }));

const mockInvoke = vi.mocked(invokeCmd);

function ep(id: number, title: string) {
  return { episode_id: id, episode_title: title } as never;
}

describe("danmaku.svelte 弹幕模块", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("load 成功填充评论并关闭 loading", async () => {
    mockInvoke.mockResolvedValueOnce([{ time: 1, mode: 1, color: 1, text: "hi" }] as never);
    const pending = danmakuStore.load(42);
    expect(danmakuStore.loading).toBe(true);
    await pending;
    expect(danmakuStore.loading).toBe(false);
    expect(danmakuStore.comments).toHaveLength(1);
    expect(danmakuStore.comments[0].text).toBe("hi");
    expect(danmakuStore.episodeId).toBe(42);
    expect(mockInvoke).toHaveBeenCalledWith("anime_danmaku_get_comments", { episodeId: 42 });
  });

  it("load 失败置空评论", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("boom"));
    await danmakuStore.load(1);
    expect(danmakuStore.loading).toBe(false);
    expect(danmakuStore.comments).toHaveLength(0);
  });

  it("searchForAnime 空番名不发起请求", async () => {
    await danmakuStore.searchForAnime("   ");
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("searchForAnime 按集数索引匹配并加载对应分集", async () => {
    mockInvoke.mockResolvedValueOnce([
      { anime_id: 7, anime_title: "t", episodes: [ep(11, "第1话"), ep(12, "第2话")] },
    ] as never);
    mockInvoke.mockResolvedValueOnce([{ time: 1, mode: 1, color: 1, text: "x" }] as never);
    await danmakuStore.searchForAnime("t", 1); // episodeIdx=1 → 第2话（epNum=2）
    expect(danmakuStore.animeId).toBe(7);
    expect(danmakuStore.episodeId).toBe(12);
    expect(danmakuStore.comments).toHaveLength(1);
  });

  it("searchForAnime 无数字标题时回退到 episodeIdx 对应的最近分集", async () => {
    mockInvoke.mockResolvedValueOnce([
      { anime_id: 7, anime_title: "t", episodes: [ep(11, "SP"), ep(12, "OVA")] },
    ] as never);
    mockInvoke.mockResolvedValueOnce([] as never);
    await danmakuStore.searchForAnime("t", 0); // 无匹配 → Math.min(0, 1)=0 → 首集 SP
    expect(danmakuStore.episodeId).toBe(11);
  });
});
