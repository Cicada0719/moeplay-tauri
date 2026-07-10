import { describe, expect, it } from "vitest";
import type {
  AnimeProviderApi,
  AnimeProviderDescriptor,
  AnimeSearchResponse,
  ProviderError,
} from "./contracts";
import { createRequestGate } from "./request-gate";
import { createAnimeProviderFeatureStore } from "./store";

interface Deferred<T> {
  promise: Promise<T>;
  resolve(value: T): void;
  reject(error: unknown): void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function provider(id = "local_media"): AnimeProviderDescriptor {
  return {
    id,
    kind: id === "jellyfin" ? "jellyfin" : "local_media",
    name: id === "jellyfin" ? "Jellyfin" : "Local media",
    localFileCount: id === "local_media" ? 1 : null,
    allowedPaths: id === "local_media" ? ["C:\\media"] : null,
    baseUrl: id === "jellyfin" ? "https://jellyfin.example.test" : null,
    origin: id === "jellyfin" ? "https://jellyfin.example.test" : null,
    secretConfigured: id === "jellyfin",
    manifest: {
      id,
      name: id,
      resourceKinds: ["anime"],
      capabilities: ["search", "detail", "children", "resolve"],
      trust: "self_hosted",
      version: "1",
      enabled: true,
      requiresAuth: id === "jellyfin",
      allowedHosts: id === "jellyfin" ? ["jellyfin.example.test"] : [],
    },
  };
}

function searchResponse(title: string, failures: ProviderError[] = []): AnimeSearchResponse {
  return {
    items: [{
      providerId: "local_media",
      itemId: title,
      title,
      originalTitle: null,
      synopsis: null,
      artworkUrl: null,
    }],
    failures,
    providerHealth: [],
  };
}

function unusedApi(overrides: Partial<AnimeProviderApi> = {}): AnimeProviderApi {
  return {
    configure: async () => provider(),
    list: async () => [],
    remove: async () => false,
    health: async () => [],
    pickLocalDirectory: async () => null,
    search: async () => { throw new Error("not used"); },
    detail: async () => { throw new Error("not used"); },
    episodes: async () => { throw new Error("not used"); },
    resolve: async () => { throw new Error("not used"); },
    openFallback: async () => { throw new Error("not used"); },
    ...overrides,
  };
}

describe("anime provider request gate", () => {
  it("cancels the prior generation and never revives it", () => {
    const gate = createRequestGate();
    const first = gate.begin();
    const second = gate.begin();
    expect(first.signal.aborted).toBe(true);
    expect(second.signal.aborted).toBe(false);
    expect(gate.isCurrent(first.generation)).toBe(false);
    expect(gate.isCurrent(second.generation)).toBe(true);

    gate.cancel();
    expect(second.signal.aborted).toBe(true);
    expect(gate.isCurrent(second.generation)).toBe(false);
  });
});

describe("anime provider feature store", () => {
  it("isolates a late search response after a newer generation wins", async () => {
    const pending: Array<{ query: string; signal: AbortSignal; result: Deferred<AnimeSearchResponse> }> = [];
    const api = unusedApi({
      search(query, signal) {
        const result = deferred<AnimeSearchResponse>();
        pending.push({ query: query.query, signal, result });
        return result.promise;
      },
    });
    const store = createAnimeProviderFeatureStore(api);

    const first = store.search("old");
    const second = store.search("new");
    expect(pending).toHaveLength(2);
    expect(pending[0].signal.aborted).toBe(true);
    expect(pending[1].signal.aborted).toBe(false);

    pending[1].result.resolve(searchResponse("new result"));
    await second;
    expect(store.getSnapshot().searchItems.map((item) => item.title)).toEqual(["new result"]);
    expect(store.getSnapshot().isSearching).toBe(false);

    pending[0].result.resolve(searchResponse("stale result"));
    await first;
    expect(store.getSnapshot().searchItems.map((item) => item.title)).toEqual(["new result"]);
    expect(store.getSnapshot().searchGeneration).toBe(2);
  });

  it("ignores a late response after explicit cancellation", async () => {
    const result = deferred<AnimeSearchResponse>();
    const store = createAnimeProviderFeatureStore(unusedApi({ search: () => result.promise }));
    const search = store.search("cancel me");
    store.cancelSearch();
    result.resolve(searchResponse("must not render"));
    await search;

    const state = store.getSnapshot();
    expect(state.isSearching).toBe(false);
    expect(state.searchItems).toEqual([]);
    expect(state.searchGeneration).toBe(2);
  });

  it("configures an explicit local source and resolves its selected episode end-to-end", async () => {
    const calls: string[] = [];
    const store = createAnimeProviderFeatureStore(unusedApi({
      configure: async (request) => {
        calls.push(request.kind);
        return provider();
      },
      search: async () => searchResponse("Fixture Show"),
      detail: async () => ({
        providerId: "local_media", itemId: "Fixture Show", title: "Fixture Show",
        originalTitle: null, synopsis: null, artworkUrl: null, genres: ["Animation"],
      }),
      episodes: async () => [{
        identity: { providerId: "local_media", seriesId: "Fixture Show", episodeId: "episode-1" },
        title: "Episode 1", number: 1, artworkUrl: null,
      }],
      resolve: async (episode) => ({
        episode,
        target: { mode: "native_file", path: "C:\\media\\episode-1.mkv" },
      }),
    }));

    await store.configure({
      kind: "local_media",
      allowedPaths: ["C:\\media"],
      library: [{
        id: "Fixture Show", title: "Fixture Show", originalTitle: null, synopsis: null,
        artworkUrl: null, genres: ["Animation"],
        episodes: [{ id: "episode-1", title: "Episode 1", number: 1, path: "C:\\media\\episode-1.mkv" }],
      }],
    });
    await store.search("Fixture");
    await store.loadDetail("local_media", "Fixture Show");
    await store.loadEpisodes("local_media", "Fixture Show");
    await store.resolve(store.getSnapshot().episodes[0].identity);

    expect(calls).toEqual(["local_media"]);
    expect(store.getSnapshot().selectedProviderId).toBe("local_media");
    expect(store.getSnapshot().resolution?.target).toEqual({ mode: "native_file", path: "C:\\media\\episode-1.mkv" });
  });

  it("keeps healthy source results when another source reports a failure", async () => {
    const failure: ProviderError = {
      kind: "network", message: "anime provider network request failed", retryable: true,
      retryAfterMs: 1000, providerId: "jellyfin", operation: "search",
    };
    const store = createAnimeProviderFeatureStore(unusedApi({
      search: async () => searchResponse("Local result", [failure]),
    }));

    await store.search("result");
    expect(store.getSnapshot().searchItems.map((item) => item.title)).toEqual(["Local result"]);
    expect(store.getSnapshot().searchFailures).toEqual([failure]);
    expect(store.getSnapshot().error).toBeNull();
  });

  it("switches configured sources without retaining a Jellyfin token in state", async () => {
    const token = "do-not-render-this";
    const store = createAnimeProviderFeatureStore(unusedApi({
      configure: async () => provider("jellyfin"),
      list: async () => [provider(), provider("jellyfin")],
    }));

    await store.configure({ kind: "jellyfin", baseUrl: "https://jellyfin.example.test", token });
    store.selectProvider("jellyfin");
    const serialized = JSON.stringify(store.getSnapshot());

    expect(store.getSnapshot().selectedProviderId).toBe("jellyfin");
    expect(serialized).not.toContain(token);
  });

  it("routes search to the selected provider and clears stale content when switching", async () => {
    const routed: Array<string | null | undefined> = [];
    const store = createAnimeProviderFeatureStore(unusedApi({
      list: async () => [provider(), provider("jellyfin")],
      search: async (_query, _signal, providerId) => {
        routed.push(providerId);
        return searchResponse("selected result");
      },
    }));

    await store.refreshProviders();
    store.selectProvider("jellyfin");
    await store.search("selected");
    expect(routed).toEqual(["jellyfin"]);
    expect(store.getSnapshot().searchItems).toHaveLength(1);

    store.selectProvider("local_media");
    expect(store.getSnapshot().searchItems).toEqual([]);
    expect(store.getSnapshot().selectedDetail).toBeNull();
  });

  it("never retains protected playback headers in feature state", async () => {
    const secret = "private-playback-token";
    const store = createAnimeProviderFeatureStore(unusedApi({
      resolve: async (episode) => ({
        episode,
        target: {
          mode: "native_hls",
          url: "http://127.0.0.1:43123/anime-provider/session/route",
          headers: [["X-Emby-Token", secret]],
        },
      }),
    }));
    const episode = { providerId: "jellyfin", seriesId: "series", episodeId: "episode" };
    const resolution = await store.resolve(episode);

    expect(resolution?.target).toEqual({
      mode: "native_hls",
      url: "http://127.0.0.1:43123/anime-provider/session/route",
      headers: [],
    });
    expect(JSON.stringify(store.getSnapshot())).not.toContain(secret);
  });

});