import { describe, expect, it } from "vitest";
import { createSearchCoverFetcher, type CoverFetchEntry } from "./covers";

function entries(...names: string[]): CoverFetchEntry[] {
  return names.map((name) => ({ key: name, name }));
}

const tick = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

describe("anime-search covers", () => {
  it("取首个带 image 的 subject.image 并投递", async () => {
    const fetcher = createSearchCoverFetcher();
    const got: [string, string][] = [];
    await fetcher.fetch(entries("某番"), {
      searchSubjects: async () => [{ image: "" }, { image: "https://img/cover.jpg" }],
      onCover: (key, image) => got.push([key, image]),
    });
    expect(got).toEqual([["某番", "https://img/cover.jpg"]]);
  });

  it("并发不超过 3", async () => {
    const fetcher = createSearchCoverFetcher();
    let active = 0;
    let maxActive = 0;
    const list = Array.from({ length: 10 }, (_, i) => ({ key: `k${i}`, name: `番${i}` }));
    await fetcher.fetch(list, {
      searchSubjects: async () => {
        active++;
        maxActive = Math.max(maxActive, active);
        await tick();
        active--;
        return [{ image: "u" }];
      },
      onCover: () => {},
    });
    expect(maxActive).toBeLessThanOrEqual(3);
  });

  it("同 key 只查一次：批内去重 + 成功后跨批缓存命中仍投递", async () => {
    const fetcher = createSearchCoverFetcher();
    let calls = 0;
    const got: string[] = [];
    const deps = {
      searchSubjects: async () => {
        calls++;
        return [{ image: "u1" }];
      },
      onCover: (_key: string, image: string) => got.push(image),
    };
    await fetcher.fetch([{ key: "k", name: "某番" }, { key: "k", name: "某番" }], deps);
    expect(calls).toBe(1);
    await fetcher.fetch(entries("某番2"), { ...deps }); // 不同 key → 新查询
    expect(calls).toBe(2);
    await fetcher.fetch([{ key: "k", name: "某番" }], deps); // 缓存命中，不再查询
    expect(calls).toBe(2);
    expect(got).toEqual(["u1", "u1", "u1"]);
  });

  it("无图/失败静默：不投递、不抛出，且同批不重复查询", async () => {
    const fetcher = createSearchCoverFetcher();
    let calls = 0;
    const got: string[] = [];
    await fetcher.fetch([{ key: "empty", name: "无图番" }, { key: "fail", name: "失败番" }, { key: "empty", name: "无图番" }], {
      searchSubjects: async (kw) => {
        calls++;
        if (kw === "失败番") throw new Error("network");
        return [{ image: "" }];
      },
      onCover: (_key, image) => got.push(image),
    });
    expect(calls).toBe(2);
    expect(got).toEqual([]);
  });

  it("token 过期（isCurrent=false）时不取新任务、结果不投递", async () => {
    const fetcher = createSearchCoverFetcher();
    let calls = 0;
    const got: string[] = [];
    await fetcher.fetch(entries("番1", "番2", "番3"), {
      isCurrent: () => false,
      searchSubjects: async () => {
        calls++;
        return [{ image: "u" }];
      },
      onCover: (_key, image) => got.push(image),
    });
    expect(calls).toBe(0);
    expect(got).toEqual([]);
  });

  it("查询中途过期：封面入缓存但不投递，下次 fetch 可从缓存激活", async () => {
    const fetcher = createSearchCoverFetcher();
    let current = true;
    const got: string[] = [];
    await fetcher.fetch([{ key: "k", name: "某番" }], {
      isCurrent: () => current,
      searchSubjects: async () => {
        current = false; // 模拟查询期间用户发起新搜索
        return [{ image: "u1" }];
      },
      onCover: (_key, image) => got.push(image),
    });
    expect(got).toEqual([]);
    current = true;
    await fetcher.fetch([{ key: "k", name: "某番" }], {
      isCurrent: () => current,
      searchSubjects: async () => [{ image: "never" }],
      onCover: (_key, image) => got.push(image),
    });
    expect(got).toEqual(["u1"]); // 缓存命中投递，未发起新查询
  });
});
