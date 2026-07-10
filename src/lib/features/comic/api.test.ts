import { afterEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "../../api/core";
import { comicProviderApi } from "./api";

const descriptor = {
  id: "komga",
  kind: "komga" as const,
  name: "Komga",
  baseUrl: "https://example.com",
  origin: "https://example.com",
  username: "reader",
  authMode: "basic" as const,
  secretConfigured: true,
  manifest: {
    id: "komga",
    name: "Komga",
    resourceKinds: ["comic"],
    capabilities: ["search"],
    trust: "self_hosted",
    version: "batch2",
    enabled: true,
    requiresAuth: true,
    allowedHosts: ["example.com"],
  },
};

afterEach(() => clearMockInvokeHandler());

describe("comic provider API contract", () => {
  it("routes all eight commands through the shared provider command boundary", async () => {
    const commands: string[] = [];
    setMockInvokeHandler((command) => {
      commands.push(command);
      if (command === "comic_provider_list") return [descriptor];
      if (command === "comic_provider_remove") return true;
      if (command === "comic_provider_configure") return descriptor;
      if (command === "comic_provider_probe") return { providerId: "komga", reachable: true, authenticated: true, libraries: [] };
      if (command === "comic_provider_search") return [];
      if (command === "comic_provider_detail") return { series: { id: "s", providerId: "komga", title: "S" }, alternateTitles: [], genres: [] };
      if (command === "comic_provider_chapters") return [];
      return { mode: "unsupported", reason: "fixture", errorKind: "unsupported" };
    });

    await comicProviderApi.configure({ kind: "komga", baseUrl: "https://example.com", authMode: "basic", username: "reader", secret: "sentinel" });
    await comicProviderApi.list();
    await comicProviderApi.remove("komga");
    await comicProviderApi.probe("komga");
    await comicProviderApi.search("komga", { query: "x" });
    await comicProviderApi.detail("komga", "s");
    await comicProviderApi.chapters("komga", "s");
    await comicProviderApi.resolve("komga", "s", "c");

    expect(commands).toEqual([
      "comic_provider_configure",
      "comic_provider_list",
      "comic_provider_remove",
      "comic_provider_probe",
      "comic_provider_search",
      "comic_provider_detail",
      "comic_provider_chapters",
      "comic_provider_resolve",
    ]);
  });

  it("never exposes a remote credential in configure/list results", async () => {
    const sentinel = "SENTINEL_REMOTE_SECRET_MUST_NOT_RETURN";
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    setMockInvokeHandler((command, args) => {
      calls.push({ command, args });
      return command === "comic_provider_list" ? [descriptor] : descriptor;
    });

    const configured = await comicProviderApi.configure({
      kind: "komga",
      baseUrl: "https://example.com",
      authMode: "basic",
      username: "reader",
      secret: sentinel,
    });
    const listed = await comicProviderApi.list();

    expect(JSON.stringify([configured, listed])).not.toContain(sentinel);
    expect(configured.secretConfigured).toBe(true);
    expect(calls[0].args).toEqual({
      request: {
        kind: "komga",
        baseUrl: "https://example.com",
        authMode: "basic",
        username: "reader",
        secret: sentinel,
      },
    });
  });

  it("fills the Rust search pagination defaults and sends resolve identity as a request", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    setMockInvokeHandler((command, args) => {
      calls.push({ command, args });
      return command === "comic_provider_search"
        ? []
        : { mode: "unsupported", reason: "fixture", errorKind: "unsupported" };
    });

    await comicProviderApi.search("komga", { query: "frieren" });
    await comicProviderApi.resolve("komga", "series", "chapter");

    expect(calls).toEqual([
      {
        command: "comic_provider_search",
        args: { providerId: "komga", request: { page: 1, pageSize: 50, query: "frieren" } },
      },
      {
        command: "comic_provider_resolve",
        args: { providerId: "komga", request: { seriesId: "series", chapterId: "chapter" } },
      },
    ]);
  });});

