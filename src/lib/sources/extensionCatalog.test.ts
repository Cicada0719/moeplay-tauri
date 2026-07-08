import { describe, expect, it } from "vitest";

import { buildExtensionCandidateCatalog } from "./extensionCatalog";
import type { ExtensionSourceCandidate } from "./extensionIndex";

function candidate(overrides: Partial<ExtensionSourceCandidate>): ExtensionSourceCandidate {
  return {
    id: "tachiyomi:base",
    extensionName: "Tachiyomi: Example",
    sourceName: "Example",
    packageName: "eu.kanade.tachiyomi.extension.en.example",
    mediaType: "comic",
    language: "en",
    version: "1.0",
    baseUrl: "https://example.test",
    apkName: "example.apk",
    nsfw: false,
    hasCloudflare: false,
    licenseRisk: "low",
    requiresExternalRuntime: true,
    repositoryId: "tachiyomi-mihon-model",
    repositoryName: "Tachiyomi / Mihon Extensions",
    status: "nativeAdapterPlanned",
    statusReason: "有 baseUrl，可评估手写原生适配",
    ...overrides,
  };
}

describe("extension candidate catalog", () => {
  it("builds rows, summaries and filter options from remote extension candidates", () => {
    const catalog = buildExtensionCandidateCatalog([
      candidate({ id: "anime:drive", sourceName: "Drive", mediaType: "video", language: "all", baseUrl: "", status: "requiresRuntime" }),
      candidate({
        id: "comic:cn",
        sourceName: "CN Comics",
        language: "zh",
        repositoryId: "aniyomi-model",
        repositoryName: "Aniyomi Extensions",
      }),
      candidate({ id: "comic:cf", sourceName: "CF Comics", hasCloudflare: true, status: "requiresRuntime", statusReason: "Cloudflare" }),
      candidate({ id: "comic:nsfw", sourceName: "Adult", nsfw: true, status: "unsupported", licenseRisk: "high" }),
    ]);

    expect(catalog.summary).toMatchObject({
      total: 4,
      visible: 4,
      repositories: 2,
      languages: 3,
      nsfw: 1,
      cloudflare: 1,
      withBaseUrl: 3,
    });
    expect(catalog.summary.byStatus).toEqual({
      discoverable: 0,
      requiresRuntime: 2,
      nativeAdapterPlanned: 1,
      unsupported: 1,
    });
    expect(catalog.filterOptions.languages).toEqual(["all", "en", "zh"]);
    expect(catalog.filterOptions.statuses).toEqual(["nativeAdapterPlanned", "requiresRuntime", "unsupported"]);
    expect(catalog.rows.map((row) => row.sourceName)).toEqual(["CN Comics", "CF Comics", "Drive", "Adult"]);
    expect(catalog.rows[0]).toMatchObject({
      mediaTypeLabel: "漫画",
      statusLabel: "可原生适配",
      badges: ["zh", "baseUrl"],
    });
  });

  it("filters visible rows while keeping the full catalog summary", () => {
    const catalog = buildExtensionCandidateCatalog(
      [
        candidate({ id: "comic:native", sourceName: "Native", language: "zh" }),
        candidate({ id: "comic:runtime", sourceName: "Runtime", baseUrl: "", status: "requiresRuntime" }),
      ],
      { status: "requiresRuntime" },
    );

    expect(catalog.summary.total).toBe(2);
    expect(catalog.summary.visible).toBe(1);
    expect(catalog.summary.byStatus.nativeAdapterPlanned).toBe(1);
    expect(catalog.rows).toHaveLength(1);
    expect(catalog.rows[0].sourceName).toBe("Runtime");
  });
});
