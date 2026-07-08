import { describe, expect, it } from "vitest";

import {
  buildMangaRuntimeEndpoint,
  buildMangaRuntimeHeaders,
  loadMangaRuntimeLibraries,
  normalizeMangaRuntimeConfig,
  probeMangaRuntime,
  toMangaSourceCandidates,
} from "./mangaRuntimeConnector";

function response(options: { ok?: boolean; status?: number; body?: unknown }): Pick<Response, "ok" | "status" | "json" | "text"> {
  return {
    ok: options.ok ?? true,
    status: options.status ?? 200,
    json: async () => options.body,
    text: async () => JSON.stringify(options.body ?? {}),
  };
}

describe("manga runtime connector", () => {
  it("normalizes runtime configs and builds readonly probe endpoints", () => {
    expect(normalizeMangaRuntimeConfig({ kind: "suwayomi", baseUrl: "http://localhost:4567/" })).toMatchObject({
      kind: "suwayomi",
      baseUrl: "http://localhost:4567",
    });
    expect(buildMangaRuntimeEndpoint({ kind: "suwayomi" })).toBe("http://127.0.0.1:4567/graphql");
    expect(buildMangaRuntimeEndpoint({ kind: "komga" })).toBe("http://127.0.0.1:25600/api/v1/libraries");
    expect(buildMangaRuntimeEndpoint({ kind: "lanraragi" })).toBe("http://127.0.0.1:3000/api/archives");
    expect(buildMangaRuntimeEndpoint({ kind: "kavita" })).toBe("http://127.0.0.1:5000/api/Library/libraries");
  });

  it("builds auth headers for token and basic-auth runtimes", () => {
    expect(buildMangaRuntimeHeaders({ kind: "suwayomi", token: " secret " })).toMatchObject({
      authorization: "Bearer secret",
    });
    expect(buildMangaRuntimeHeaders({ kind: "komga", username: "demo", password: "pass" })).toMatchObject({
      authorization: "Basic ZGVtbzpwYXNz",
    });
    expect(buildMangaRuntimeHeaders({ kind: "lanraragi", token: "key" })).toMatchObject({
      authorization: "Bearer key",
    });
  });

  it("probes online, auth required, schema mismatch and offline states", async () => {
    await expect(probeMangaRuntime(async () => response({ ok: true, status: 200 }), { kind: "komga" })).resolves.toMatchObject({
      status: "online",
      requiresAuth: false,
    });
    await expect(probeMangaRuntime(async () => response({ ok: false, status: 401 }), { kind: "kavita" })).resolves.toMatchObject({
      status: "authRequired",
      requiresAuth: true,
    });
    await expect(probeMangaRuntime(async () => response({ ok: false, status: 404 }), { kind: "lanraragi" })).resolves.toMatchObject({
      status: "schemaMismatch",
    });
    await expect(
      probeMangaRuntime(async () => {
        throw new Error("connection refused");
      }, { kind: "suwayomi" }),
    ).resolves.toMatchObject({ status: "offline" });
  });

  it("loads runtime libraries from Suwayomi GraphQL and REST-like servers", async () => {
    const calls: Array<{ url: string; init?: RequestInit }> = [];
    const suwayomiLibraries = await loadMangaRuntimeLibraries(
      async (url, init) => {
        calls.push({ url, init });
        return response({
          body: {
            data: {
              sources: {
                nodes: [{ id: 7, name: "MangaDex", lang: "all" }],
              },
            },
          },
        });
      },
      { kind: "suwayomi" },
    );
    const komgaLibraries = await loadMangaRuntimeLibraries(
      async () => response({ body: { content: [{ id: "lib-1", name: "本地漫画", language: "zh" }] } }),
      { kind: "komga", baseUrl: "http://komga.local" },
    );

    expect(calls[0].url).toBe("http://127.0.0.1:4567/graphql");
    expect(calls[0].init?.method).toBe("POST");
    expect(suwayomiLibraries[0]).toMatchObject({ id: "7", name: "MangaDex", kind: "suwayomi" });
    expect(komgaLibraries[0]).toMatchObject({
      id: "lib-1",
      name: "本地漫画",
      kind: "komga",
      baseUrl: "http://komga.local",
      language: "zh",
    });
  });

  it("maps connected libraries into comic source candidates without copying runtime code", () => {
    const candidates = toMangaSourceCandidates([
      { id: "lib-1", name: "Komga Library", kind: "komga", sourceName: "Komga", baseUrl: "http://komga.local", language: "zh" },
      { id: "main", name: "Kavita Library", kind: "kavita", sourceName: "Kavita", baseUrl: "http://kavita.local", language: "all" },
    ]);

    expect(candidates[0]).toMatchObject({
      id: "komga:lib-1",
      mediaType: "comic",
      repositoryId: "komga-runtime",
      status: "discoverable",
    });
    expect(candidates[1]).toMatchObject({
      repositoryId: "kavita-runtime",
      licenseRisk: "high",
      status: "unsupported",
    });
  });
});
