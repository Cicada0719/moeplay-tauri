import { beforeEach, describe, expect, it, vi } from "vitest";
import { createSourceCenterStore } from "./store";
import type { SourceCenterApi, SourceDescriptor } from "./contracts";

const anime: SourceDescriptor = {
  providerId: "anime-fixture", mediaType: "anime", displayName: "Anime Fixture", kind: "builtin",
  capabilities: ["search", "resolve"], enabled: true, priority: 10,
  health: { state: "healthy", latencyMs: 42, lastCheckedAt: "2026-07-11T00:00:00Z", consecutiveFailures: 0, successRate: 0.99 },
  latencyMs: 42, lastCheckedAt: "2026-07-11T00:00:00Z", authState: "not_required", runtimeState: "available", languages: ["zh"], nsfw: "exclude", recentFailures: [],
};
const comic: SourceDescriptor = { ...anime, providerId: "comic-fixture", mediaType: "comic", displayName: "Comic Fixture", capabilities: ["search", "children"], priority: -2, enabled: false, runtimeState: "unavailable", health: { ...anime.health, state: "degraded" } };

function apiWith(overrides: Partial<SourceCenterApi> = {}): SourceCenterApi {
  return {
    listSourceDescriptors: vi.fn(async () => [anime, comic]),
    updateSourcePreference: vi.fn(async () => undefined),
    verifySource: vi.fn(async () => {}),
    verifySourcesBatch: vi.fn(async () => {}),
    resetSourceHealth: vi.fn(async () => undefined),
    refreshExtensionIndex: vi.fn(async () => null),
    getExtensionIndexSnapshot: vi.fn(async () => null),
    ...overrides,
  };
}

describe("Source Center store", () => {
  beforeEach(() => localStorage.clear());

  it("does not fetch an extension snapshot until a user configures an endpoint", async () => {
    const api = apiWith();
    const store = createSourceCenterStore(api);

    await store.load();
    expect(api.getExtensionIndexSnapshot).not.toHaveBeenCalled();

    await store.setExtensionIndexEndpoint("https://directory.example/extensions.json");
    expect(api.getExtensionIndexSnapshot).toHaveBeenCalledWith("https://directory.example/extensions.json");
  });

  it("filters unified sources and sends only enabled visible sources for batch verification", async () => {
    const api = apiWith();
    const store = createSourceCenterStore(api);
    await store.load();
    await store.setFilters({ mediaType: "anime" });
    expect(store.getSnapshot().sources.map((source) => source.providerId)).toEqual(["anime-fixture"]);
    await store.verifyVisible();
    expect(api.verifySourcesBatch).toHaveBeenCalledWith([{ providerId: "anime-fixture", mediaType: "anime" }]);
  });

  it("optimistically changes preference and rolls back when backend rejects it", async () => {
    const api = apiWith({ updateSourcePreference: vi.fn(async () => { throw new Error("denied"); }) });
    const store = createSourceCenterStore(api);
    await store.load();
    await store.toggleEnabled(anime);
    expect(store.getSnapshot().allSources.find((source) => source.providerId === anime.providerId)?.enabled).toBe(true);
    expect(store.getSnapshot().error).toBe("denied");
  });

  it("keeps a negative priority and clamps adjustment at the supported minimum", async () => {
    const api = apiWith({ listSourceDescriptors: vi.fn(async () => [{ ...anime, priority: -10_000 }]) });
    const store = createSourceCenterStore(api);
    await store.load();
    await store.adjustPriority(store.getSnapshot().sources[0], -1);

    expect(api.updateSourcePreference).toHaveBeenCalledWith(expect.objectContaining({ priority: -10_000 }));
  });
});
