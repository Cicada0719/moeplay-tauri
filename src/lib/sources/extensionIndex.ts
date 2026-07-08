import type {
  ExtensionIndexFormat,
  MediaType,
  SourceAdapterManifest,
  SourceEcosystem,
  SourceLicenseRisk,
} from "./sourceRegistry";
import { getSourceAdaptersReadyForIndexImport } from "./sourceRegistry";

type IndexableExtensionEcosystem = Extract<SourceEcosystem, "tachiyomi" | "keiyoushi" | "aniyomi" | "mangayomi">;

export interface ExtensionIndexSource {
  name?: unknown;
  lang?: unknown;
  id?: unknown;
  baseUrl?: unknown;
  hasCloudflare?: unknown;
  versionId?: unknown;
}

export interface ExtensionIndexEntry {
  name?: unknown;
  pkg?: unknown;
  apk?: unknown;
  lang?: unknown;
  code?: unknown;
  version?: unknown;
  nsfw?: unknown;
  sources?: unknown;
  id?: unknown;
  baseUrl?: unknown;
  typeSource?: unknown;
  iconUrl?: unknown;
  dateFormat?: unknown;
  isNsfw?: unknown;
  hasCloudflare?: unknown;
  sourceCodeUrl?: unknown;
  apiUrl?: unknown;
  isManga?: unknown;
}

export interface ImportedExtensionSource {
  id: string;
  extensionName: string;
  sourceName: string;
  packageName: string;
  mediaType: MediaType;
  language: string;
  version: string;
  baseUrl: string;
  apkName: string;
  nsfw: boolean;
  hasCloudflare: boolean;
  licenseRisk: SourceLicenseRisk;
  requiresExternalRuntime: boolean;
  indexFormat?: ExtensionIndexFormat;
  typeSource?: string;
  sourceCodeUrl?: string;
  apiUrl?: string;
  iconUrl?: string;
}

export type ExtensionCandidateStatus =
  | "discoverable"
  | "requiresRuntime"
  | "nativeAdapterPlanned"
  | "unsupported";

export interface ExtensionSourceCandidate extends ImportedExtensionSource {
  repositoryId: string;
  repositoryName: string;
  status: ExtensionCandidateStatus;
  statusReason: string;
}

export interface ExtensionCandidateFilters {
  mediaType?: MediaType;
  language?: string;
  includeNsfw?: boolean;
  cloudflare?: boolean;
  hasBaseUrl?: boolean;
  status?: ExtensionCandidateStatus;
}

export interface ExtensionIndexCacheSummary {
  repositoryId: string;
  repositoryName: string;
  fetchedAt: string;
  sourceCount: number;
  languages: string[];
  cloudflareCount: number;
  nsfwCount: number;
  withBaseUrlCount: number;
}

export interface NormalizeExtensionIndexOptions {
  mediaType: MediaType;
  ecosystem: IndexableExtensionEcosystem;
  indexFormat?: ExtensionIndexFormat;
  licenseRisk?: SourceLicenseRisk;
}

export interface ExtensionIndexRepository {
  id: string;
  name: string;
  mediaType: MediaType;
  ecosystem: IndexableExtensionEcosystem;
  indexFormat: ExtensionIndexFormat;
  indexUrl: string;
  licenseRisk: SourceLicenseRisk;
}

export interface ExtensionIndexSnapshot {
  repository: ExtensionIndexRepository;
  sources: ImportedExtensionSource[];
  summary: ReturnType<typeof summarizeImportedExtensionSources>;
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function asString(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function asScalarString(value: unknown, fallback = ""): string {
  if (typeof value === "string") return value;
  if (typeof value === "number" && Number.isFinite(value)) return String(value);
  return fallback;
}

function asBooleanFlag(value: unknown): boolean {
  return value === true || value === 1 || value === "1";
}

function asBooleanish(value: unknown): boolean {
  return value === true || value === 1 || value === "1" || value === "true";
}

function makeFallbackId(packageName: string, sourceName: string, index: number): string {
  const sourceSlug = sourceName.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
  return `${packageName || "extension"}:${sourceSlug || index}`;
}

export function parseExtensionIndexPayload(payload: string | unknown): ExtensionIndexEntry[] {
  if (typeof payload === "string") {
    const parsed = JSON.parse(payload) as unknown;
    return asArray(parsed) as ExtensionIndexEntry[];
  }

  return asArray(payload) as ExtensionIndexEntry[];
}

export function normalizeExtensionIndex(
  payload: string | unknown,
  options: NormalizeExtensionIndexOptions,
): ImportedExtensionSource[] {
  if (options.indexFormat === "mangayomi" || options.ecosystem === "mangayomi") {
    return normalizeMangayomiIndex(payload, options);
  }

  const entries = parseExtensionIndexPayload(payload);

  return entries.flatMap((entry, entryIndex) => {
    const extensionName = asString(entry.name, `Extension ${entryIndex + 1}`);
    const packageName = asString(entry.pkg);
    const language = asString(entry.lang, "all");
    const version = asString(entry.version, String(entry.code ?? "draft"));
    const apkName = asString(entry.apk);
    const nsfw = asBooleanFlag(entry.nsfw);

    return asArray(entry.sources).map((rawSource, sourceIndex) => {
      const source = rawSource as ExtensionIndexSource;
      const sourceName = asString(source.name, extensionName);
      const sourceLanguage = asString(source.lang, language);
      const sourceId = asString(source.id, makeFallbackId(packageName, sourceName, sourceIndex));

      return {
        id: `${options.ecosystem}:${sourceId}`,
        extensionName,
        sourceName,
        packageName,
        mediaType: options.mediaType,
        language: sourceLanguage,
        version,
        baseUrl: asString(source.baseUrl),
        apkName,
        nsfw,
        hasCloudflare: asBooleanFlag(source.hasCloudflare),
        licenseRisk: options.licenseRisk ?? "low",
        requiresExternalRuntime: true,
        indexFormat: options.indexFormat ?? "tachiyomi",
      } satisfies ImportedExtensionSource;
    });
  });
}

export function normalizeMangayomiIndex(
  payload: string | unknown,
  options: NormalizeExtensionIndexOptions,
): ImportedExtensionSource[] {
  const entries = parseExtensionIndexPayload(payload);

  return entries
    .filter((entry) => asBooleanish(entry.isManga) || options.mediaType === "comic")
    .map((entry, index) => {
      const sourceName = asString(entry.name, `Mangayomi Source ${index + 1}`);
      const sourceId = asScalarString(entry.id, makeFallbackId("mangayomi", sourceName, index));
      const typeSource = asString(entry.typeSource, "custom");
      const sourceCodeUrl = asString(entry.sourceCodeUrl);
      const apiUrl = asString(entry.apiUrl);

      return {
        id: `${options.ecosystem}:${sourceId}`,
        extensionName: `Mangayomi: ${typeSource}`,
        sourceName,
        packageName: sourceCodeUrl || `mangayomi.${typeSource}.${sourceId}`,
        mediaType: options.mediaType,
        language: asString(entry.lang, "all"),
        version: asString(entry.version, "draft"),
        baseUrl: asString(entry.baseUrl),
        apkName: "",
        nsfw: asBooleanish(entry.isNsfw),
        hasCloudflare: asBooleanish(entry.hasCloudflare),
        licenseRisk: options.licenseRisk ?? "low",
        requiresExternalRuntime: true,
        indexFormat: "mangayomi",
        typeSource,
        sourceCodeUrl,
        apiUrl,
        iconUrl: asString(entry.iconUrl),
      } satisfies ImportedExtensionSource;
    });
}

function isIndexEcosystem(ecosystem: SourceEcosystem): ecosystem is IndexableExtensionEcosystem {
  return ecosystem === "tachiyomi" || ecosystem === "keiyoushi" || ecosystem === "aniyomi" || ecosystem === "mangayomi";
}

export function getExtensionIndexRepositories(
  manifests: SourceAdapterManifest[] = getSourceAdaptersReadyForIndexImport(),
): ExtensionIndexRepository[] {
  return manifests.flatMap((manifest) => {
    if (!manifest.indexUrl || !isIndexEcosystem(manifest.ecosystem)) {
      return [];
    }

    return [
      {
        id: manifest.id,
        name: manifest.name,
        mediaType: manifest.mediaType,
        ecosystem: manifest.ecosystem,
        indexFormat: manifest.indexFormat ?? "tachiyomi",
        indexUrl: manifest.indexUrl,
        licenseRisk: manifest.licenseRisk,
      },
    ];
  });
}

export async function loadExtensionIndexSnapshots(
  fetchIndex: (repository: ExtensionIndexRepository) => Promise<string | unknown>,
  repositories: ExtensionIndexRepository[] = getExtensionIndexRepositories(),
): Promise<ExtensionIndexSnapshot[]> {
  const snapshots = await Promise.all(
    repositories.map(async (repository) => {
      const payload = await fetchIndex(repository);
      const sources = normalizeExtensionIndex(payload, {
        mediaType: repository.mediaType,
        ecosystem: repository.ecosystem,
        indexFormat: repository.indexFormat,
        licenseRisk: repository.licenseRisk,
      });

      return {
        repository,
        sources,
        summary: summarizeImportedExtensionSources(sources),
      } satisfies ExtensionIndexSnapshot;
    }),
  );

  return snapshots;
}

export function getExtensionCandidateStatus(source: ImportedExtensionSource): Pick<ExtensionSourceCandidate, "status" | "statusReason"> {
  if (source.licenseRisk === "high") {
    return { status: "unsupported", statusReason: "许可证边界较高，仅保留为参考源" };
  }

  if (source.hasCloudflare) {
    return { status: "requiresRuntime", statusReason: "源声明 Cloudflare，优先交给兼容运行时或网页验证" };
  }

  if (source.indexFormat === "mangayomi") {
    return { status: "requiresRuntime", statusReason: "Mangayomi 索引可发现，需外部运行时或手写适配后阅读" };
  }

  if (source.baseUrl) {
    return { status: "nativeAdapterPlanned", statusReason: "有 baseUrl，可评估手写原生适配" };
  }

  return { status: "requiresRuntime", statusReason: "索引可发现，但需要扩展运行时解析" };
}

export function toExtensionSourceCandidates(snapshots: ExtensionIndexSnapshot[]): ExtensionSourceCandidate[] {
  return snapshots.flatMap((snapshot) =>
    snapshot.sources.map((source) => ({
      ...source,
      repositoryId: snapshot.repository.id,
      repositoryName: snapshot.repository.name,
      ...getExtensionCandidateStatus(source),
    })),
  );
}

export function filterExtensionSourceCandidates(
  candidates: ExtensionSourceCandidate[],
  filters: ExtensionCandidateFilters,
): ExtensionSourceCandidate[] {
  return candidates.filter((candidate) => {
    if (filters.mediaType && candidate.mediaType !== filters.mediaType) {
      return false;
    }
    if (filters.language && candidate.language !== filters.language) {
      return false;
    }
    if (filters.includeNsfw === false && candidate.nsfw) {
      return false;
    }
    if (filters.cloudflare !== undefined && candidate.hasCloudflare !== filters.cloudflare) {
      return false;
    }
    if (filters.hasBaseUrl !== undefined && Boolean(candidate.baseUrl) !== filters.hasBaseUrl) {
      return false;
    }
    if (filters.status && candidate.status !== filters.status) {
      return false;
    }

    return true;
  });
}

export function summarizeExtensionCandidateStatuses(candidates: ExtensionSourceCandidate[]) {
  const statuses: ExtensionCandidateStatus[] = ["discoverable", "requiresRuntime", "nativeAdapterPlanned", "unsupported"];

  return Object.fromEntries(
    statuses.map((status) => [status, candidates.filter((candidate) => candidate.status === status).length]),
  ) as Record<ExtensionCandidateStatus, number>;
}

export function createExtensionIndexCacheSummaries(
  snapshots: ExtensionIndexSnapshot[],
  fetchedAt: string,
): ExtensionIndexCacheSummary[] {
  return snapshots.map((snapshot) => ({
    repositoryId: snapshot.repository.id,
    repositoryName: snapshot.repository.name,
    fetchedAt,
    sourceCount: snapshot.summary.total,
    languages: snapshot.summary.languages,
    cloudflareCount: snapshot.summary.cloudflare,
    nsfwCount: snapshot.summary.nsfw,
    withBaseUrlCount: snapshot.summary.withBaseUrl,
  }));
}

export function summarizeImportedExtensionSources(sources: ImportedExtensionSource[]) {
  return {
    total: sources.length,
    nsfw: sources.filter((source) => source.nsfw).length,
    cloudflare: sources.filter((source) => source.hasCloudflare).length,
    languages: Array.from(new Set(sources.map((source) => source.language))).sort(),
    withBaseUrl: sources.filter((source) => source.baseUrl.length > 0).length,
  };
}
