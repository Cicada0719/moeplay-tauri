import { beforeEach, describe, expect, it } from "vitest";
import {
  followingKey,
  followingStore,
  normalizeFollowingEpisode,
  type FollowingCollectionLike,
} from "./following.svelte";

const COLLECTION: FollowingCollectionLike = {
  key: "show-a",
  name: "Show A",
  image: "cover.jpg",
  ruleSource: "source-a",
  sourceUrl: "https://source.test/show-a",
  contentId: "show-a-id",
  seasonKey: "season-1",
  collectType: 1,
};

describe("followingStore 追番更新中心", () => {
  beforeEach(() => {
    localStorage.clear();
    followingStore.reload();
  });

  it("以来源、作品和季隔离身份，并能幂等关联旧收藏", () => {
    const first = followingStore.follow(COLLECTION);
    const second = followingStore.follow({ ...COLLECTION, name: "Show A 改名" });
    const otherSeason = followingStore.follow({ ...COLLECTION, seasonKey: "season-2" });

    expect(first.key).toBe(followingKey("source-a", "show-a-id", "season-1"));
    expect(first.autoCheck).toBe(false);
    expect(second.key).toBe(first.key);
    expect(followingStore.items).toHaveLength(2);
    expect(otherSeason.key).not.toBe(first.key);
    expect(followingStore.get(first.key)?.title).toBe("Show A 改名");
  });

  it("首次检查只建立基线，后续新增只产生一次提醒", async () => {
    const item = followingStore.follow(COLLECTION);
    const first = await followingStore.check(item.key, async () => [
      { name: "第 1 集", url: "/1" },
      { name: "第 2 集", url: "/2" },
    ]);
    expect(first?.baselineCreated).toBe(true);
    expect(first?.addedEpisodeIds).toEqual([]);
    expect(followingStore.get(item.key)?.pendingNoticeIds).toEqual([]);

    const second = await followingStore.check(item.key, async () => [
      { name: "第 1 集", url: "/1" },
      { name: "第 2 集", url: "/2" },
      { name: "第 3 集", url: "/3" },
    ]);
    expect(second?.addedEpisodeIds).toHaveLength(1);
    expect(second?.pendingNoticeIds).toHaveLength(1);

    const repeat = await followingStore.check(item.key, async () => [
      { name: "第 1 集", url: "/1" },
      { name: "第 2 集", url: "/2" },
      { name: "第 3 集", url: "/3" },
    ]);
    expect(repeat?.addedEpisodeIds).toEqual([]);
    expect(followingStore.pendingCount).toBe(1);
    followingStore.consumeNotices(item.key);
    expect(followingStore.pendingCount).toBe(0);
  });

  it("来源失败不清空已知剧集和未看状态", async () => {
    const item = followingStore.follow(COLLECTION);
    await followingStore.check(item.key, async () => [{ name: "第 1 集", url: "/1" }]);
    const episodeId = followingStore.get(item.key)?.knownEpisodes[0].id ?? "";
    followingStore.markWatched(item.key, episodeId);
    const result = await followingStore.check(item.key, async () => { throw new Error("network timeout"); });
    const saved = followingStore.get(item.key);
    expect(result?.status).toBe("unknown");
    expect(saved?.knownEpisodes).toHaveLength(1);
    expect(saved?.watchedEpisodeIds).toEqual([episodeId]);
    expect(saved?.errorKind).toBe("timeout");
  });

  it("最近未完成优先，其次选择首个未看，并支持 90% 完成标记", async () => {
    const item = followingStore.follow(COLLECTION);
    await followingStore.check(item.key, async () => [
      { name: "第 1 集", url: "/1" },
      { name: "第 2 集", url: "/2" },
      { name: "第 3 集", url: "/3" },
    ]);
    const saved = followingStore.get(item.key)!;
    followingStore.markWatched(item.key, saved.knownEpisodes[0].id);
    followingStore.markPlaybackProgress(item.key, saved.knownEpisodes[1].id, 90, 100);
    const afterFinish = followingStore.get(item.key)!;
    expect(afterFinish.watchedEpisodeIds).toContain(saved.knownEpisodes[1].id);
    const target = followingStore.resume(item.key, [{
      name: "Show A", ruleName: "source-a", sourceUrl: COLLECTION.sourceUrl!, lastEpisode: 2,
      lastEpisodeName: "第 3 集", progressMs: 30_000, updatedAt: new Date().toISOString(),
    }]);
    expect(target.reason).toBe("unfinished");
    expect(target.episode?.id).toBe(saved.knownEpisodes[2].id);
  });

  it("重命名的普通集保持编号身份，特别篇不会冒充正片", () => {
    expect(normalizeFollowingEpisode({ name: "第 3 集", url: "/3" }).id).toBe("regular:3");
    expect(normalizeFollowingEpisode({ name: "第 3 话（重制版）", url: "/3-new" }).id).toBe("regular:3");
    expect(normalizeFollowingEpisode({ name: "特别篇 1", url: "/sp-1" }).special).toBe(true);
    expect(normalizeFollowingEpisode({ name: "特别篇 1", url: "/sp-1" }).id).toContain("special:");
  });

  it("自动检查只调度用户开启的条目", async () => {
    const manual = followingStore.follow(COLLECTION);
    const automatic = followingStore.follow({ ...COLLECTION, seasonKey: "season-2" });
    followingStore.setAutoCheck(automatic.key, true);
    const checked: string[] = [];
    await followingStore.checkAll(async (item) => {
      checked.push(item.key);
      return [{ name: "第 1 集", url: "/1" }];
    }, true);
    expect(checked).toEqual([automatic.key]);
    expect(followingStore.get(manual.key)?.baselineReady).toBe(false);
  });
});
