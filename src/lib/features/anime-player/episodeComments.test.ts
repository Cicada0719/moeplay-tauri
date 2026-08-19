import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCmd } from "../../api/core";
import { episodeCommentsStore } from "./episodeComments.svelte";

vi.mock("../../api/core", () => ({ invokeCmd: vi.fn() }));

const mockInvoke = vi.mocked(invokeCmd);

describe("episodeComments.svelte 章节评论模块", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("load 成功填充评论并关闭 loading", async () => {
    mockInvoke.mockResolvedValueOnce([{ user: "u", avatar: "a", comment: "hi", date: "d" }] as never);
    const pending = episodeCommentsStore.load(9);
    expect(episodeCommentsStore.loading).toBe(true);
    await pending;
    expect(episodeCommentsStore.loading).toBe(false);
    expect(episodeCommentsStore.comments).toHaveLength(1);
    expect(episodeCommentsStore.comments[0].comment).toBe("hi");
    expect(mockInvoke).toHaveBeenCalledWith("anime_bangumi_episode_comments", { episodeId: 9 });
  });

  it("load 失败置空评论", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("boom"));
    await episodeCommentsStore.load(1);
    expect(episodeCommentsStore.loading).toBe(false);
    expect(episodeCommentsStore.comments).toHaveLength(0);
  });
});
