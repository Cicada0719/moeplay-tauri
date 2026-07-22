// novel store 搜索提速（0.19.5）：全部源逐源并发合并、去重、取消与错误隔离。
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (value: string) => `asset://${value}`,
  invoke: vi.fn(),
}));

interface MockBook {
  id: string;
  source: string;
  title: string;
  subjects: string[];
  publicDomain: boolean;
  sourceUrl: string;
}

function book(source: string, id: string): MockBook {
  return { id, source, title: `${source}-${id}`, subjects: [], publicDomain: false, sourceUrl: "" };
}

const flush = async (times = 10) => {
  for (let i = 0; i < times; i++) await new Promise<void>((resolve) => setTimeout(resolve, 0));
};

async function loadStore(handler: (command: string, args?: Record<string, unknown>) => unknown) {
  vi.resetModules();
  const core = await import("../../api/core");
  core.setMockInvokeHandler(handler);
  const { novelStore } = await import("./store.svelte");
  return novelStore;
}

const ALL_SOURCES = ["biquge", "x80", "internetarchive", "openlibrary", "standardebooks", "gutenberg", "wikisource"];

describe("novel store 全源并发搜索（0.19.5）", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("拆成逐源并发调用：先到先合并、同 key 去重、进度计数完整", async () => {
    const calls: string[] = [];
    const store = await loadStore((command, args) => {
      expect(command).toBe("novel_search");
      const source = String(args?.source);
      calls.push(source);
      if (source === "biquge") return [book("biquge", "1"), book("biquge", "2"), book("biquge", "2")];
      if (source === "x80") return [book("x80", "1")];
      return [];
    });

    await store.search("测试");

    expect(calls.sort()).toEqual([...ALL_SOURCES].sort());
    expect(calls).not.toContain("all");
    expect(store.books.map((item) => `${item.source}:${item.id}`)).toEqual(["biquge:1", "biquge:2", "x80:1"]);
    expect(store.sourcesTotal).toBe(ALL_SOURCES.length);
    expect(store.sourcesDone).toBe(ALL_SOURCES.length);
    expect(store.loading).toBe(false);
    expect(store.error).toBe("");
  });

  it("单源失败不拖垮整体：其余源结果照常展示且不报错", async () => {
    const store = await loadStore((_command, args) => {
      const source = String(args?.source);
      if (source === "gutenberg") throw new Error("网络超时");
      if (source === "biquge") return [book("biquge", "1")];
      return [];
    });

    await store.search("测试");

    expect(store.books.map((item) => `${item.source}:${item.id}`)).toEqual(["biquge:1"]);
    expect(store.error).toBe("");
    expect(store.loading).toBe(false);
  });

  it("全部源失败时才显示错误", async () => {
    const store = await loadStore(() => {
      throw new Error("源不可用");
    });

    await store.search("测试");

    expect(store.books).toEqual([]);
    expect(store.error).toContain("源不可用");
    expect(store.loading).toBe(false);
  });

  it("取消后 loading 立即复位，迟到的结果被丢弃", async () => {
    const store = await loadStore((_command, args) => {
      const source = String(args?.source);
      return new Promise((resolve) => setTimeout(() => resolve([book(source, "1")]), 30));
    });

    const pending = store.search("测试");
    await flush(2);
    expect(store.loading).toBe(true);

    store.cancel();
    expect(store.loading).toBe(false);

    await Promise.all([pending, flush(40)]);
    expect(store.books).toEqual([]);
    expect(store.error).toBe("");
  });

  it("单源搜索保持单次调用", async () => {
    const calls: unknown[] = [];
    const store = await loadStore((command, args) => {
      expect(command).toBe("novel_search");
      calls.push(args?.source);
      return [book("biquge", "9")];
    });

    store.setSource("biquge");
    await store.search("测试");

    expect(calls).toEqual(["biquge"]);
    expect(store.books.map((item) => item.id)).toEqual(["9"]);
    expect(store.sourcesTotal).toBe(0);
  });
});
