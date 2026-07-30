// 集成冒烟：store.search() → 流式合并去重 → 封面懒补入缓存。
// 通过 mock invokeCmd + 捕获 listen 回调模拟 Rust 侧流式事件，不起 dev server。

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const eventHandlers = new Map<string, (ev: { payload: unknown }) => void>();
const emitEvent = (name: string, payload: unknown) => eventHandlers.get(name)?.({ payload });

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: (ev: { payload: unknown }) => void) => {
    eventHandlers.set(name, handler);
    return () => { eventHandlers.delete(name); };
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
    eventHandlers.clear();
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
          emitEvent("anime-search-result", ["源A", [
            { name: "进击的巨人", url: "a/kyojin" },
            { name: "关于我转生变成史莱姆这档事 第二季", url: "a/slime2" },
          ]]);
          emitEvent("anime-search-result", ["源B", [
            { name: "进击的巨人【高清】", url: "b/kyojin" },
            { name: "关于我转生变成史莱姆这档事", url: "b/slime" },
          ]]);
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


// ── 逐源搜索状态 / 失败重试 ────────────────────────────────────────────

describe("anime search：逐源状态与失败重试", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
    eventHandlers.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("一源成功一源失败：searchSourceStatus 记录逐源状态，有结果时不报错", async () => {
    const store = await loadStore((command) => {
      switch (command) {
        case "anime_search_all":
          emitEvent("anime-search-result", ["源A", [{ name: "某番", url: "a/1" }]]);
          emitEvent("anime-search-source-status", { ruleName: "源A", status: "ok", count: 1 });
          emitEvent("anime-search-source-status", { ruleName: "源B", status: "error", count: 0, error: "搜索超时 (10s)" });
          return undefined;
        case "anime_bangumi_search":
          return [[], 0];
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await store.search("某番");

    expect(store.searchSourceStatus["源A"]).toEqual({ status: "ok", count: 1, error: undefined });
    expect(store.searchSourceStatus["源B"]).toEqual({ status: "error", count: 0, error: "搜索超时 (10s)" });
    expect(store.error).toBeNull();
    expect(store.mergedSearchResults.map((e) => e.name)).toEqual(["某番"]);
  });

  it("无结果且有失败源：错误文案提示失败源数量", async () => {
    const store = await loadStore((command) => {
      switch (command) {
        case "anime_search_all":
          emitEvent("anime-search-source-status", { ruleName: "源A", status: "empty", count: 0 });
          emitEvent("anime-search-source-status", { ruleName: "源B", status: "error", count: 0, error: "HTTP 502" });
          return undefined;
        case "anime_bangumi_search":
          return [[], 0];
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await store.search("不存在的番");

    expect(store.error).toBe("未找到结果（1 个源检索失败，可到「规则」页更新规则）");
  });

  it("retryFailedSources：失败源重试成功后补入结果并清除错误", async () => {
    let sourceBOk = false;
    const store = await loadStore((command, args) => {
      switch (command) {
        case "anime_search_all":
          emitEvent("anime-search-source-status", { ruleName: "源B", status: "error", count: 0, error: "搜索超时 (10s)" });
          return undefined;
        case "anime_search": {
          expect(args).toEqual({ ruleName: "源B", keyword: "某番" });
          if (!sourceBOk) throw new Error("搜索超时 (10s)");
          return [{ name: "某番", url: "b/1" }];
        }
        case "anime_bangumi_search":
          return [[], 0];
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await store.search("某番");
    expect(store.searchSourceStatus["源B"].status).toBe("error");
    expect(store.error).toContain("1 个源检索失败");

    sourceBOk = true;
    await store.retryFailedSources();

    expect(store.searchSourceStatus["源B"]).toEqual({ status: "ok", count: 1, error: undefined });
    expect(store.mergedSearchResults.map((e) => e.name)).toEqual(["某番"]);
    expect(store.mergedSearchResults[0].sources).toEqual(["源B"]);
    expect(store.error).toBeNull();
  });
});

// ── 规则目录：可更新检测 / 一键更新 / 缓存回退 ─────────────────────────

const makeRule = (name: string, version: string) => ({
  name,
  version,
  baseUrl: "https://example.com",
  searchURL: "",
  searchList: "",
  searchName: "",
  searchResult: "",
  chapterRoads: "",
  chapterResult: "",
  muliSources: true,
  useWebview: false,
  useNativePlayer: true,
  usePost: false,
  useLegacyParser: false,
  adBlocker: false,
  userAgent: "",
  referer: "",
  api: "",
  type: "",
});

const makeCatalogItem = (name: string, version: string) => ({
  name,
  version,
  useNativePlayer: true,
  antiCrawlerEnabled: false,
  author: "tester",
  lastUpdate: 0,
});

describe("规则目录：updatableRules / updateAllRules / 缓存回退", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
    eventHandlers.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("updatableRules：只包含已安装且仓库版本不同的规则", async () => {
    localStorage.setItem("anime-rules", JSON.stringify([makeRule("源A", "1.0"), makeRule("源B", "1.0")]));
    const store = await loadStore((command) => {
      if (command === "anime_github_rules_index") {
        return [makeCatalogItem("源A", "2.0"), makeCatalogItem("源B", "1.0"), makeCatalogItem("源C", "1.0")];
      }
      throw new Error(`unexpected command: ${command}`);
    });

    await store.loadCatalog();

    expect(store.updatableRules.map((r) => r.name)).toEqual(["源A"]);
  });

  it("updateAllRules：只更新可更新项，更新后 updatableRules 清空", async () => {
    localStorage.setItem("anime-rules", JSON.stringify([makeRule("源A", "1.0"), makeRule("源B", "1.0")]));
    const installed: string[] = [];
    const store = await loadStore((command, args) => {
      switch (command) {
        case "anime_github_rules_index":
          return [makeCatalogItem("源A", "2.0"), makeCatalogItem("源B", "1.0"), makeCatalogItem("源C", "1.0")];
        case "anime_install_github_rule":
          installed.push(String(args?.name));
          return makeRule(String(args?.name), "2.0");
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    });

    await store.loadCatalog();
    await store.updateAllRules();

    expect(installed).toEqual(["源A"]);
    expect(store.rules.find((r) => r.name === "源A")?.version).toBe("2.0");
    expect(store.rules.find((r) => r.name === "源B")?.version).toBe("1.0");
    expect(store.updatableRules).toEqual([]);
  });

  it("loadCatalog 失败：目录为空时回退到本地缓存，并报错", async () => {
    localStorage.setItem(
      "anime-rules-catalog-v1",
      JSON.stringify({ fetchedAt: 1, items: [makeCatalogItem("缓存源", "1.0")] }),
    );
    const store = await loadStore((command) => {
      if (command === "anime_github_rules_index") throw new Error("网络错误");
      throw new Error(`unexpected command: ${command}`);
    });

    await store.loadCatalog();

    expect(store.catalog.map((r) => r.name)).toEqual(["缓存源"]);
    expect(store.catalogError).toContain("网络错误");
  });

  it("loadCatalog 成功：写入缓存供下次失败时回退", async () => {
    const store = await loadStore((command) => {
      if (command === "anime_github_rules_index") return [makeCatalogItem("源A", "2.0")];
      throw new Error(`unexpected command: ${command}`);
    });

    await store.loadCatalog();

    const cached = JSON.parse(localStorage.getItem("anime-rules-catalog-v1") ?? "null");
    expect(cached?.items?.map((r: { name: string }) => r.name)).toEqual(["源A"]);
  });
});
