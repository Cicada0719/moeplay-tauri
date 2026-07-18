// 集成冒烟：store.search() → 流式合并去重 → 封面懒补入缓存。
// 通过 mock invokeCmd + 捕获 listen 回调模拟 Rust 侧流式事件，不起 dev server。

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

let searchResultHandler: ((ev: { payload: unknown }) => void) | null = null;

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_name: string, handler: (ev: { payload: unknown }) => void) => {
    searchResultHandler = handler;
    return () => {};
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (value: string) => `asset://${value}`,
  invoke: vi.fn(),
}));

const flush = async (times = 10) => {
  for (let i = 0; i < times; i++) await new Promise<void>((resolve) => setTimeout(resolve, 0));
};

async function loadStore(handler: (command: string, args?: Record<string, unknown>) => unknown) {
  vi.resetModules();
  const core = await import("../../api/core");
  core.setMockInvokeHandler(handler);
  const { animeStore } = await import("../../stores/anime.svelte");
  return animeStore;
}

describe("anime search 集成：合并去重 + 封面补全", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
    searchResultHandler = null;
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("全源流式搜索：同名跨源合并、sources 累计、封面懒补后经 getImg 命中", async () => {
    let bangumiQueries = 0;
    const store = await loadStore((command, args) => {
      switch (command) {
        case "anime_search_all":
          // 模拟两个源流式到达（同名不同写法 + 季度后缀）
          searchResultHandler?.({
            payload: ["源A", [
              { name: "进击的巨人", url: "a/kyojin" },
              { name: "关于我转生变成史莱姆这档事 第二季", url: "a/slime2" },
            ]],
          });
          searchResultHandler?.({
            payload: ["源B", [
              { name: "进击的巨人【高清】", url: "b/kyojin" },
              { name: "关于我转生变成史莱姆这档事", url: "b/slime" },
            ]],
          });
          return undefined;
        case "anime_bangumi_search": {
          bangumiQueries++;
          const kw = String(args?.keyword ?? "");
          if (kw.includes("进击")) {
            return [[{ id: 1, name: "進撃の巨人", name_cn: "进击的巨人", image: "https://img/kyojin.jpg",
              summary: "", air_date: "", air_weekday: 0, rating: 0, rank: 0, eps_count: 25 }], 1];
          }
          return [[], 0]; // 史莱姆无匹配图 → 保持文字卡
        }
        case "anime_proxy_image":
          return "/cache/kyojin.jpg";
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await store.search("进击的巨人");

    // 合并：2 条；关键词命中的"进击的巨人"排前；两条都是 源A+源B
    const merged = store.mergedSearchResults;
    expect(merged.map((e) => e.name)).toEqual([
      "进击的巨人",
      "关于我转生变成史莱姆这档事 第二季",
    ]);
    expect(merged[0].sources).toEqual(["源A", "源B"]);
    expect(merged[0].items[0]).toEqual({ name: "进击的巨人", url: "a/kyojin" });
    expect(merged[1].sources).toEqual(["源A", "源B"]);

    await flush();

    // 封面：仅进击的巨人补到图，经 _proxyImages 入 _imgCache 后可读取 asset URL
    expect(store.getSearchCover(merged[0].key)).toBe("asset:///cache/kyojin.jpg");
    expect(store.getSearchCover(merged[1].key)).toBe("");

    // 同 key 去重：两条目各查一次（流式两次 refresh 未产生重复查询）
    expect(bangumiQueries).toBe(2);
  });

  it("单源搜索同样产出合并列表", async () => {
    const store = await loadStore((command, args) => {
      if (command === "anime_search") {
        expect(args).toEqual({ ruleName: "源A", keyword: "某番" });
        return [
          { name: "某番", url: "a/1" },
          { name: "某番 2", url: "a/2" }, // 纯数字差异 → 不合并
        ];
      }
      if (command === "anime_bangumi_search") return [[], 0];
      throw new Error(`unexpected command: ${command}`);
    });

    store.setSelectedRule("源A");
    await store.search("某番");

    const merged = store.mergedSearchResults;
    expect(merged).toHaveLength(2);
    expect(merged[0].sources).toEqual(["源A"]);
    expect(merged.map((e) => e.name)).toEqual(["某番", "某番 2"]);
    await flush();
  });
});
