import { describe, expect, it } from "vitest";

import {
  buildSuwayomiBaseUrl,
  buildSuwayomiGraphqlEndpoint,
  clearSuwayomiConfig,
  loadSuwayomiRuntimeSnapshot,
  loadSuwayomiConfig,
  listSuwayomiInstalledExtensions,
  listSuwayomiSources,
  normalizeSuwayomiConfig,
  probeSuwayomiServer,
  saveSuwayomiConfig,
  summarizeSuwayomiErrors,
  toSuwayomiRuntimeCandidates,
  withSuwayomiTimeout,
  type SuwayomiFetch,
} from "./suwayomiConnector";

function response(options: { ok?: boolean; status?: number; body?: unknown }): Pick<Response, "ok" | "status" | "json" | "text"> {
  return {
    ok: options.ok ?? true,
    status: options.status ?? 200,
    json: async () => options.body,
    text: async () => JSON.stringify(options.body ?? {}),
  };
}

function memoryStorage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
  };
}

describe("suwayomi connector", () => {
  it("builds local graphql endpoints with safe defaults", () => {
    expect(buildSuwayomiBaseUrl()).toBe("http://127.0.0.1:4567");
    expect(buildSuwayomiGraphqlEndpoint()).toBe("http://127.0.0.1:4567/graphql");
    expect(buildSuwayomiGraphqlEndpoint({ host: "http://localhost/", port: 5000 })).toBe("http://localhost:5000/graphql");
  });

  it("normalizes and persists connection config", () => {
    const storage = memoryStorage();
    const saved = saveSuwayomiConfig(
      { protocol: "https", host: "https://suwayomi.local/", port: 8443, token: " secret " },
      storage,
    );

    expect(saved).toEqual({ protocol: "https", host: "suwayomi.local", port: 8443, token: "secret" });
    expect(loadSuwayomiConfig(storage)).toEqual(saved);
    expect(clearSuwayomiConfig(storage)).toEqual({ protocol: "http", host: "127.0.0.1", port: 4567, token: "" });
    expect(loadSuwayomiConfig(storage)).toEqual({ protocol: "http", host: "127.0.0.1", port: 4567, token: "" });
  });

  it("falls back from invalid stored config", () => {
    const storage = memoryStorage({
      "moeplay-suwayomi-config-v1": JSON.stringify({ protocol: "ftp", host: "", port: 99999, token: 12 }),
    });

    expect(loadSuwayomiConfig(storage)).toEqual({ protocol: "http", host: "127.0.0.1", port: 4567, token: "" });
    expect(normalizeSuwayomiConfig({ host: "http://localhost", port: 0 })).toEqual({
      protocol: "http",
      host: "localhost",
      port: 4567,
      token: "",
    });
  });

  it("probes online, auth required, schema mismatch and offline states", async () => {
    await expect(probeSuwayomiServer(async () => response({ ok: true, status: 200 }))).resolves.toMatchObject({
      status: "online",
      requiresAuth: false,
    });
    await expect(probeSuwayomiServer(async () => response({ ok: false, status: 401 }))).resolves.toMatchObject({
      status: "authRequired",
      requiresAuth: true,
    });
    await expect(probeSuwayomiServer(async () => response({ ok: false, status: 404 }))).resolves.toMatchObject({
      status: "schemaMismatch",
    });
    await expect(
      probeSuwayomiServer(async () => {
        throw new Error("connection refused");
      }),
    ).resolves.toMatchObject({ status: "offline" });
  });

  it("wraps fetch calls with an abort signal for local runtime probes", async () => {
    let hasSignal = false;
    const fetcher = withSuwayomiTimeout(async (_url, init) => {
      hasSignal = init?.signal instanceof AbortSignal;
      return response({ ok: true, status: 200 });
    });

    await fetcher("http://127.0.0.1:4567/graphql");

    expect(hasSignal).toBe(true);
  });

  it("queries sources through injected fetcher", async () => {
    const calls: Array<{ url: string; init?: RequestInit }> = [];
    const fetcher: SuwayomiFetch = async (url, init) => {
      calls.push({ url, init });
      return response({
        body: {
          data: {
            sources: {
              totalCount: 1,
              nodes: [{ id: 7, name: "Komga", lang: "all", baseUrl: "http://komga.test" }],
            },
          },
        },
      });
    };

    const result = await listSuwayomiSources(fetcher, { token: "secret" }, 25);

    expect(result.data?.sources.totalCount).toBe(1);
    expect(result.data?.sources.nodes[0].name).toBe("Komga");
    expect(calls[0].url).toBe("http://127.0.0.1:4567/graphql");
    expect(calls[0].init?.method).toBe("POST");
    expect(calls[0].init?.headers).toMatchObject({ authorization: "Bearer secret" });
    expect(JSON.parse(String(calls[0].init?.body))).toMatchObject({ variables: { first: 25 } });
  });

  it("queries installed extensions without write operations", async () => {
    let body = "";
    const result = await listSuwayomiInstalledExtensions(async (_url, init) => {
      body = String(init?.body);
      return response({
        body: {
          data: {
            extensions: {
              totalCount: 1,
              nodes: [
                {
                  name: "MangaDex",
                  pkgName: "eu.kanade.tachiyomi.extension.all.mangadex",
                  lang: "all",
                  isInstalled: true,
                  hasUpdate: false,
                  isObsolete: false,
                },
              ],
            },
          },
        },
      });
    });

    expect(result.data?.extensions.nodes[0].pkgName).toContain("mangadex");
    expect(body).toContain("isInstalled: true");
    expect(body).not.toContain("install");
    expect(body).not.toContain("updateExtension");
  });

  it("loads a read-only runtime snapshot and maps sources into MoePlay candidates", async () => {
    const fetcher: SuwayomiFetch = async (_url, init) => {
      const body = JSON.parse(String(init?.body));

      if (String(body.query).includes("sources")) {
        return response({
          body: {
            data: {
              sources: {
                totalCount: 2,
                nodes: [
                  { id: 1, name: "Komga", lang: "all", homeUrl: "http://komga.test", baseUrl: "http://komga.test" },
                  { id: 2, name: "Local", lang: "zh" },
                ],
              },
            },
          },
        });
      }

      return response({
        body: {
          data: {
            extensions: {
              totalCount: 1,
              nodes: [
                {
                  name: "Komga",
                  pkgName: "eu.kanade.tachiyomi.extension.all.komga",
                  lang: "all",
                  isInstalled: true,
                  hasUpdate: false,
                  isObsolete: false,
                },
              ],
            },
          },
        },
      });
    };

    const snapshot = await loadSuwayomiRuntimeSnapshot(fetcher);
    const candidates = toSuwayomiRuntimeCandidates(snapshot.data?.sources ?? []);

    expect(snapshot.data).toMatchObject({ sourceTotal: 2, extensionTotal: 1 });
    expect(candidates).toHaveLength(2);
    expect(candidates[0]).toMatchObject({
      id: "suwayomi:1",
      sourceName: "Komga",
      repositoryName: "Suwayomi 本地运行时",
      status: "discoverable",
      baseUrl: "http://komga.test",
    });
    expect(candidates[1]).toMatchObject({ status: "requiresRuntime", statusReason: "本地运行时可见，但缺少主页地址" });
  });

  it("summarizes runtime query errors", async () => {
    const snapshot = await loadSuwayomiRuntimeSnapshot(async (_url, init) => {
      const body = JSON.parse(String(init?.body));
      if (String(body.query).includes("sources")) {
        return response({ body: { errors: [{ message: "sources failed" }] } });
      }
      return response({ body: { errors: [{ message: "extensions failed" }] } });
    });

    expect(snapshot.data).toBeUndefined();
    expect(summarizeSuwayomiErrors(snapshot)).toBe("sources failed；extensions failed");
  });
});
