import { describe, expect, it } from "vitest";
import {
  getExtensionIndexRepositories,
  createExtensionIndexCacheSummaries,
  filterExtensionSourceCandidates,
  loadExtensionIndexSnapshots,
  normalizeExtensionIndex,
  normalizeMangayomiIndex,
  summarizeExtensionCandidateStatuses,
  summarizeImportedExtensionSources,
  toExtensionSourceCandidates,
} from "./extensionIndex";
import { SOURCE_ADAPTER_MANIFESTS } from "./sourceRegistry";

describe("extension index normalization", () => {
  it("normalizes Tachiyomi index sources without executing apk extensions", () => {
    const sources = normalizeExtensionIndex(
      [
        {
          name: "Tachiyomi: LANraragi",
          pkg: "eu.kanade.tachiyomi.extension.all.lanraragi",
          apk: "tachiyomi-all.lanraragi-v1.4.15.apk",
          lang: "all",
          code: 15,
          version: "1.4.15",
          nsfw: 0,
          sources: [
            {
              name: "LANraragi (1)",
              lang: "all",
              id: "4482480338677079857",
              baseUrl: "http://127.0.0.1:3000",
              versionId: 1,
              hasCloudflare: 1,
            },
          ],
        },
      ],
      { mediaType: "comic", ecosystem: "tachiyomi" },
    );

    expect(sources).toHaveLength(1);
    expect(sources[0]).toMatchObject({
      id: "tachiyomi:4482480338677079857",
      extensionName: "Tachiyomi: LANraragi",
      sourceName: "LANraragi (1)",
      mediaType: "comic",
      version: "1.4.15",
      baseUrl: "http://127.0.0.1:3000",
      hasCloudflare: true,
      requiresExternalRuntime: true,
    });
  });

  it("normalizes Aniyomi entries with multiple video sources from a JSON string", () => {
    const payload = JSON.stringify([
      {
        name: "Aniyomi: Jellyfin",
        pkg: "eu.kanade.tachiyomi.animeextension.all.jellyfin",
        apk: "aniyomi-all.jellyfin-v14.17.apk",
        lang: "all",
        version: "14.17",
        sources: [
          { name: "Jellyfin (1)", lang: "all", id: "1100359934660540567", baseUrl: "" },
          { name: "Jellyfin (2)", lang: "all", id: "5716273013801838310", baseUrl: "" },
        ],
      },
    ]);

    const sources = normalizeExtensionIndex(payload, { mediaType: "video", ecosystem: "aniyomi" });

    expect(sources).toHaveLength(2);
    expect(sources.map((source) => source.id)).toEqual([
      "aniyomi:1100359934660540567",
      "aniyomi:5716273013801838310",
    ]);
    expect(sources.every((source) => source.mediaType === "video")).toBe(true);
    expect(sources.every((source) => source.packageName.includes("animeextension"))).toBe(true);
  });

  it("normalizes Mangayomi index sources without executing Dart or JavaScript plugins", () => {
    const sources = normalizeMangayomiIndex(
      [
        {
          name: "1st Kiss-Manga (unoriginal)",
          id: 638504049,
          baseUrl: "https://1stkissmanga.org",
          lang: "en",
          typeSource: "madara",
          iconUrl: "https://example.test/icon.png",
          isNsfw: false,
          hasCloudflare: false,
          sourceCodeUrl: "https://raw.githubusercontent.com/kodjodevf/mangayomi-extensions/main/dart/manga/madara.dart",
          apiUrl: "",
          version: "0.1.3",
          isManga: true,
        },
      ],
      { mediaType: "comic", ecosystem: "mangayomi", indexFormat: "mangayomi" },
    );

    expect(sources).toHaveLength(1);
    expect(sources[0]).toMatchObject({
      id: "mangayomi:638504049",
      extensionName: "Mangayomi: madara",
      sourceName: "1st Kiss-Manga (unoriginal)",
      baseUrl: "https://1stkissmanga.org",
      language: "en",
      version: "0.1.3",
      indexFormat: "mangayomi",
      typeSource: "madara",
      sourceCodeUrl: "https://raw.githubusercontent.com/kodjodevf/mangayomi-extensions/main/dart/manga/madara.dart",
      requiresExternalRuntime: true,
    });
  });

  it("summarizes imported source language, nsfw, cloudflare, and baseUrl coverage", () => {
    const sources = normalizeExtensionIndex(
      [
        {
          name: "Tachiyomi: Example",
          pkg: "eu.kanade.tachiyomi.extension.en.example",
          lang: "en",
          nsfw: 1,
          sources: [
            { name: "Example", id: "example", baseUrl: "https://example.test", hasCloudflare: 0 },
            { name: "Example Alt", id: "example-alt", lang: "all", hasCloudflare: true },
          ],
        },
      ],
      { mediaType: "comic", ecosystem: "tachiyomi" },
    );

    expect(summarizeImportedExtensionSources(sources)).toEqual({
      total: 2,
      nsfw: 2,
      cloudflare: 1,
      languages: ["all", "en"],
      withBaseUrl: 1,
    });
  });

  it("discovers only manifests with standard remote extension indexes", () => {
    const repositories = getExtensionIndexRepositories(SOURCE_ADAPTER_MANIFESTS);

    expect(repositories.map((repository) => repository.id)).toEqual([
      "tachiyomi-mihon-model",
      "keiyoushi-extensions",
      "aniyomi-model",
      "mangayomi-extensions",
    ]);
    expect(repositories.filter((repository) => repository.indexFormat === "tachiyomi")).toHaveLength(3);
    expect(repositories.find((repository) => repository.id === "mangayomi-extensions")?.indexFormat).toBe("mangayomi");
    expect(repositories.every((repository) => repository.licenseRisk === "low")).toBe(true);
  });

  it("loads multiple remote index snapshots through an injected fetcher", async () => {
    const repositories = getExtensionIndexRepositories(SOURCE_ADAPTER_MANIFESTS);
    const snapshots = await loadExtensionIndexSnapshots(async (repository) => {
      if (repository.ecosystem === "tachiyomi") {
        return [
          {
            name: "Tachiyomi: Komga",
            pkg: "eu.kanade.tachiyomi.extension.all.komga",
            lang: "all",
            version: "1.4.50",
            sources: [{ name: "Komga", id: "4508733312114627536", baseUrl: "" }],
          },
        ];
      }
      if (repository.ecosystem === "mangayomi") {
        return [
          {
            name: "MangaDex",
            id: 1,
            lang: "all",
            version: "0.1.0",
            baseUrl: "https://mangadex.org",
            typeSource: "mangadex",
            isManga: true,
          },
        ];
      }

      return JSON.stringify([
        {
          name: "Aniyomi: Google Drive",
          pkg: "eu.kanade.tachiyomi.animeextension.all.googledrive",
          lang: "all",
          version: "14.15",
          sources: [{ name: "Google Drive", id: "4222017068256633289", baseUrl: "https://drive.google.com" }],
        },
      ]);
    }, repositories);

    expect(snapshots).toHaveLength(4);
    expect(snapshots.map((snapshot) => snapshot.summary.total)).toEqual([1, 1, 1, 1]);
    expect(snapshots[0].sources[0].id).toBe("tachiyomi:4508733312114627536");
    expect(snapshots[2].sources[0].id).toBe("aniyomi:4222017068256633289");
    expect(snapshots[2].summary.withBaseUrl).toBe(1);
    expect(snapshots[3].sources[0].id).toBe("mangayomi:1");
  });

  it("classifies imported sources into candidate statuses", async () => {
    const repositories = getExtensionIndexRepositories(SOURCE_ADAPTER_MANIFESTS);
    const snapshots = await loadExtensionIndexSnapshots(async (repository) => {
      if (repository.ecosystem === "mangayomi") {
        return [
          { name: "Mangayomi Runtime", id: "mangayomi-runtime", baseUrl: "", typeSource: "custom", isManga: true },
          { name: "Mangayomi Native", id: "mangayomi-native", baseUrl: "https://example.test", typeSource: "madara", isManga: true },
          { name: "Mangayomi CF", id: "mangayomi-cf", baseUrl: "https://cf.example.test", hasCloudflare: true, typeSource: "madara", isManga: true },
        ];
      }

      return [
        {
          name: `${repository.name}: Fixture`,
          pkg: `fixture.${repository.ecosystem}`,
          lang: repository.ecosystem === "tachiyomi" ? "en" : "all",
          version: "1.0.0",
          sources: [
            { name: "Runtime Only", id: `${repository.ecosystem}-runtime`, baseUrl: "" },
            { name: "Native Candidate", id: `${repository.ecosystem}-native`, baseUrl: "https://example.test" },
            { name: "Cloudflare Candidate", id: `${repository.ecosystem}-cf`, baseUrl: "https://cf.example.test", hasCloudflare: 1 },
          ],
        },
      ];
    }, repositories);

    const candidates = toExtensionSourceCandidates(snapshots);
    expect(summarizeExtensionCandidateStatuses(candidates)).toEqual({
      discoverable: 0,
      requiresRuntime: 9,
      nativeAdapterPlanned: 3,
      unsupported: 0,
    });
    expect(filterExtensionSourceCandidates(candidates, { hasBaseUrl: true })).toHaveLength(8);
    expect(filterExtensionSourceCandidates(candidates, { cloudflare: true })).toHaveLength(4);
    expect(filterExtensionSourceCandidates(candidates, { mediaType: "comic" })).toHaveLength(9);
    expect(filterExtensionSourceCandidates(candidates, { status: "nativeAdapterPlanned" })).toHaveLength(3);
    expect(candidates.every((candidate) => candidate.repositoryId && candidate.statusReason)).toBe(true);
  });

  it("creates cache summaries suitable for a readonly source catalog", async () => {
    const [repository] = getExtensionIndexRepositories(SOURCE_ADAPTER_MANIFESTS);
    const snapshots = await loadExtensionIndexSnapshots(
      async () => [
        {
          name: "Tachiyomi: Fixture",
          pkg: "fixture.tachiyomi",
          lang: "en",
          nsfw: 1,
          sources: [
            { name: "Fixture One", id: "fixture-1", baseUrl: "https://fixture.test", hasCloudflare: 1 },
            { name: "Fixture Two", id: "fixture-2", lang: "all", baseUrl: "" },
          ],
        },
      ],
      [repository],
    );

    expect(createExtensionIndexCacheSummaries(snapshots, "2026-07-08T00:00:00.000Z")).toEqual([
      {
        repositoryId: "tachiyomi-mihon-model",
        repositoryName: "Tachiyomi / Mihon Extensions",
        fetchedAt: "2026-07-08T00:00:00.000Z",
        sourceCount: 2,
        languages: ["all", "en"],
        cloudflareCount: 1,
        nsfwCount: 2,
        withBaseUrlCount: 1,
      },
    ]);
  });
});
