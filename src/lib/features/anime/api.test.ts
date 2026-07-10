import { afterEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "../../api/core";
import { createTauriAnimeProviderApi } from "./api";

afterEach(() => clearMockInvokeHandler());

describe("Tauri Anime Provider API", () => {
  it("uses the provider command payload shapes without exposing a configured token", async () => {
    const calls: Array<{ command: string; args: Record<string, unknown> }> = [];
    setMockInvokeHandler((command, args) => {
      calls.push({ command, args: args ?? {} });
      switch (command) {
        case "anime_provider_configure":
          return {
            id: "jellyfin", kind: "jellyfin", name: "Jellyfin", localFileCount: null,
            allowedPaths: null, baseUrl: "https://jellyfin.example.test", origin: "https://jellyfin.example.test",
            secretConfigured: true,
            manifest: { id: "jellyfin", name: "Jellyfin", resourceKinds: ["anime"], capabilities: [], trust: "self_hosted", version: "1", enabled: true, requiresAuth: true, allowedHosts: ["jellyfin.example.test"] },
          };
        case "anime_provider_list": return [];
        case "anime_provider_remove": return true;
        case "anime_provider_health": return [];
        case "anime_provider_pick_local_directory": return null;
        case "anime_provider_search": return { items: [], failures: [], providerHealth: [] };
        case "anime_provider_detail": return {};
        case "anime_provider_episodes": return [];
        case "anime_provider_resolve": return {};
        case "anime_provider_open_fallback": return { mode: "external" };
        default: throw new Error(`unexpected command ${command}`);
      }
    });
    const api = createTauriAnimeProviderApi();
    const token = "never-returned-token";
    const descriptor = await api.configure({ kind: "jellyfin", baseUrl: "https://jellyfin.example.test", token });
    await api.list();
    await api.remove("jellyfin");
    await api.health();
    await api.pickLocalDirectory();
    await api.search({ query: "show", limit: 10 }, new AbortController().signal, "jellyfin");
    await api.detail("jellyfin", "item-1", new AbortController().signal);
    await api.episodes("jellyfin", "item-1", new AbortController().signal);
    const episode = { providerId: "jellyfin", seriesId: "item-1", episodeId: "episode-1" };
    await api.resolve(episode, new AbortController().signal);
    await api.openFallback(episode);

    expect(calls.map((call) => call.command)).toEqual([
      "anime_provider_configure", "anime_provider_list", "anime_provider_remove", "anime_provider_health",
      "anime_provider_pick_local_directory", "anime_provider_search", "anime_provider_detail",
      "anime_provider_episodes", "anime_provider_resolve", "anime_provider_open_fallback",
    ]);
    expect(calls[0].args).toEqual({ request: { kind: "jellyfin", baseUrl: "https://jellyfin.example.test", token } });
    expect(JSON.stringify(descriptor)).not.toContain(token);
    expect(calls[5].args).toEqual({ query: { query: "show", limit: 10 }, providerId: "jellyfin" });
    expect(calls[8].args).toEqual({ request: { episode } });
    expect(calls[9].args).toEqual({ request: { episode } });
  });
});