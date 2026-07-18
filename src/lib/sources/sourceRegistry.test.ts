import { describe, expect, it } from "vitest";
import {
  SOURCE_ADAPTER_MANIFESTS,
  getSourceAdapterSummary,
  getSourceAdaptersByEcosystem,
  getSourceAdaptersByLifecycle,
  getSourceAdaptersByMediaType,
  getSourceAdaptersReadyForIndexImport,
} from "./sourceRegistry";

describe("source registry", () => {
  it("keeps unique source ids", () => {
    const ids = SOURCE_ADAPTER_MANIFESTS.map((source) => source.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("contains active game, anime, comic, and novel sources", () => {
    const active = getSourceAdaptersByLifecycle("active");
    expect(active.some((source) => source.mediaType === "game")).toBe(true);
    expect(active.some((source) => source.mediaType === "anime")).toBe(true);
    expect(active.some((source) => source.mediaType === "comic")).toBe(true);
    expect(active.some((source) => source.mediaType === "novel")).toBe(true);
  });

  it("tracks the v0.12.0 built-in comic adapters as active sources", () => {
    const activeIds = getSourceAdaptersByLifecycle("active").map((source) => source.id);
    expect(activeIds).toEqual(expect.arrayContaining([
      "picacg-current",
      "mangadex-api",
      "baozi-native",
      "dm5-web-sources",
    ]));
  });

  it("tracks GitHub reference ecosystems for manga and video expansion", () => {
    const references = getSourceAdaptersByLifecycle("reference");
    expect(references.some((source) => source.id === "tachiyomi-mihon-model")).toBe(true);
    expect(references.some((source) => source.id === "keiyoushi-extensions")).toBe(true);
    expect(references.some((source) => source.id === "kotatsu-parser-model")).toBe(true);
    expect(references.some((source) => source.id === "aniyomi-model")).toBe(true);
    expect(references.some((source) => source.id === "mangayomi-extensions")).toBe(true);
    expect(references.some((source) => source.id === "cloudstream-model")).toBe(true);
    expect(references.every((source) => source.referenceUrl)).toBe(true);
  });

  it("tracks visual novel metadata and legal storefront sources for games", () => {
    const gameIds = getSourceAdaptersByMediaType("game").map((source) => source.id);
    expect(gameIds).toEqual(expect.arrayContaining([
      "vndb-kana-api",
      "bangumi-game-api",
      "ymgal-api",
      "kungal-metadata",
      "dlsite-store-metadata",
      "getchu-store-metadata",
      "erogamescape-metadata",
      "itchio-visual-novel-store",
      "igdb-game-metadata",
    ]));

    expect(getSourceAdaptersByEcosystem("vndb")[0].capabilities).toEqual(expect.arrayContaining(["search", "detail", "metadata"]));
    expect(getSourceAdaptersByEcosystem("itchio")[0].lifecycle).toBe("planned");
    expect(getSourceAdaptersByEcosystem("igdb")[0].authMode).toBe("api-key");
    expect(getSourceAdaptersByMediaType("game").every((source) => !source.capabilities.includes("download") || source.id === "local-game-library")).toBe(true);
  });

  it("marks index import ecosystems separately from license-only references", () => {
    const importable = getSourceAdaptersReadyForIndexImport();
    expect(importable.map((source) => source.id)).toEqual(
      expect.arrayContaining(["tachiyomi-mihon-model", "keiyoushi-extensions", "aniyomi-model", "mangayomi-extensions"]),
    );
    expect(importable.every((source) => source.indexUrl)).toBe(true);
    expect(importable.every((source) => source.connectorKind === "index")).toBe(true);

    const kotatsu = getSourceAdaptersByEcosystem("kotatsu");
    expect(kotatsu).toHaveLength(1);
    expect(kotatsu[0].licenseRisk).toBe("high");
    expect(kotatsu[0].adoptionStrategy).toBe("study-contract");

    const cloudstream = getSourceAdaptersByEcosystem("cloudstream");
    expect(cloudstream).toHaveLength(1);
    expect(cloudstream[0].mediaType).toBe("video");
    expect(cloudstream[0].adoptionStrategy).toBe("study-contract");
  });

  it("summarizes media type and verification counts", () => {
    const summary = getSourceAdapterSummary();
    expect(summary.total).toBe(SOURCE_ADAPTER_MANIFESTS.length);
    expect(summary.byMediaType.anime).toBe(getSourceAdaptersByMediaType("anime").length);
    expect(summary.byMediaType.comic).toBeGreaterThanOrEqual(2);
    expect(summary.byMediaType.novel).toBe(2);
    expect(summary.byMediaType.game).toBeGreaterThanOrEqual(9);
    expect(summary.indexImportable).toBe(4);
    expect(summary.highLicenseRisk).toBe(3);
    expect(summary.requiresVerification).toBeGreaterThan(0);
  });
});
