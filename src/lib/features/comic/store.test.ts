import { describe, expect, it } from "vitest";
import { createComicProviderStore } from "./store";
import type { ComicProviderApi, ComicProviderDescriptor } from "./types";

const series = [{ id: "new", providerId: "komga", title: "new" }];

const providers: ComicProviderDescriptor[] = [
  {
    id: "komga",
    kind: "komga",
    name: "Komga",
    baseUrl: "https://example.com",
    origin: "https://example.com",
    username: "reader",
    authMode: "basic",
    secretConfigured: true,
    manifest: {
      id: "komga",
      name: "Komga",
      resourceKinds: ["comic"],
      capabilities: ["search", "resolve"],
      trust: "self_hosted",
      version: "batch2",
      enabled: true,
      requiresAuth: true,
      allowedHosts: ["example.com"],
    },
  },
  {
    id: "local-fixture",
    kind: "local",
    name: "Local",
    localRoot: "C:/Comics",
    authMode: "none",
    secretConfigured: false,
    manifest: {
      id: "local-fixture",
      name: "Local",
      resourceKinds: ["comic"],
      capabilities: ["search", "resolve"],
      trust: "user_configured",
      version: "batch2",
      enabled: true,
      requiresAuth: false,
      allowedHosts: [],
    },
  },
];

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}

function apiFor(search: ComicProviderApi["search"]): ComicProviderApi {
  return {
    configure: async () => providers[0],
    list: async () => providers,
    remove: async () => true,
    probe: async (providerId) => ({ providerId, reachable: true, authenticated: true, libraries: [] }),
    search,
    detail: async () => ({ series: series[0], alternateTitles: [], genres: [] }),
    chapters: async () => [],
    resolve: async () => ({ mode: "unsupported", reason: "fixture", errorKind: "unsupported" }),
  };
}

describe("comic provider feature store", () => {
  it("does not let an older search response overwrite a newer generation", async () => {
    const oldRequest = deferred<typeof series>();
    const newRequest = deferred<typeof series>();
    let calls = 0;
    const store = createComicProviderStore(apiFor(async () => {
      calls += 1;
      return calls === 1 ? oldRequest.promise : newRequest.promise;
    }));

    const oldSearch = store.search("komga", { query: "old" });
    const newSearch = store.search("komga", { query: "new" });
    newRequest.resolve(series);
    await newSearch;
    oldRequest.resolve([{ id: "old", providerId: "komga", title: "old" }]);
    expect(await oldSearch).toBeUndefined();
    expect(store.state.series).toEqual(series);
    expect(store.state.loading).toBe(false);
  });

  it("keeps page retry and prefetch decisions pure at the store boundary", () => {
    const store = createComicProviderStore(apiFor(async () => []));
    expect(store.planPageRetry(2, 0, 3, true)?.attempt).toBe(1);
    expect(store.planPageRetry(2, 2, 3, true)).toBeUndefined();
    expect(store.prefetchWindow(2, 5, 1)).toEqual([1, 3]);
  });

  it("cancels an in-flight resolve by bumping generation", async () => {
    const request = deferred<{ mode: "image_pages"; pages: string[]; headers: [string, string][] }>();
    const store = createComicProviderStore({
      ...apiFor(async () => []),
      resolve: async () => request.promise,
    });
    const pending = store.resolve("kavita", "s", "c");
    const cancelled = store.cancelPending();
    request.resolve({ mode: "image_pages", pages: ["file:///page.jpg"], headers: [] });
    expect(await pending).toBeUndefined();
    expect(cancelled.loading).toBe(false);
    expect(store.state.targetsByChapter).toEqual({});
  });

  it("loads, switches, and removes providers without retaining stale content", async () => {
    const searched: string[] = [];
    const store = createComicProviderStore(apiFor(async (providerId) => {
      searched.push(providerId);
      return [{ id: providerId, providerId, title: providerId }];
    }));

    await store.refreshProviders();
    expect(store.state.providerId).toBe("komga");
    store.selectProvider("local-fixture");
    await store.searchSelected({ query: "local" });
    expect(searched).toEqual(["local-fixture"]);
    expect(store.state.series[0].providerId).toBe("local-fixture");

    store.selectProvider("komga");
    expect(store.state.series).toEqual([]);
    await store.removeProvider("komga");
    expect(store.state.providerId).toBe("local-fixture");
    expect(store.state.providers.map((provider) => provider.id)).toEqual(["local-fixture"]);
  });

  it("loads detail and chapters under one cancellation generation", async () => {
    const store = createComicProviderStore({
      ...apiFor(async () => series),
      detail: async () => ({ series: series[0], alternateTitles: ["alt"], genres: ["genre"], totalChapters: 1 }),
      chapters: async () => [{
        identity: { providerId: "komga", seriesId: "new", chapterId: "c1", stableKey: "komga:new:c1" },
        title: "Chapter 1",
        sort: { chapterNumber: 1, title: "Chapter 1" },
        languageSource: "provider",
        pageCount: 12,
      }],
    });

    const result = await store.loadSeries("komga", "new");
    expect(result?.detail.alternateTitles).toEqual(["alt"]);
    expect(result?.chapters[0].identity.chapterId).toBe("c1");
    expect(store.state.detailsBySeries.new.totalChapters).toBe(1);
    expect(store.state.chaptersBySeries.new).toHaveLength(1);
  });});

