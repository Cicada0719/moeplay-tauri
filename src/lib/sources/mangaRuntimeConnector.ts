import type { ExtensionSourceCandidate } from "./extensionIndex";

export type MangaRuntimeKind = "suwayomi" | "komga" | "lanraragi" | "kavita";
export type MangaRuntimeStatus = "online" | "authRequired" | "offline" | "schemaMismatch";

export interface MangaRuntimeConfig {
  kind: MangaRuntimeKind;
  baseUrl?: string;
  token?: string;
  username?: string;
  password?: string;
}

export interface NormalizedMangaRuntimeConfig {
  kind: MangaRuntimeKind;
  baseUrl: string;
  token: string;
  username: string;
  password: string;
}

export interface MangaRuntimeProbeResult {
  kind: MangaRuntimeKind;
  status: MangaRuntimeStatus;
  endpoint: string;
  message: string;
  requiresAuth: boolean;
}

export interface MangaRuntimeLibrary {
  id: string;
  name: string;
  kind: MangaRuntimeKind;
  sourceName: string;
  baseUrl: string;
  language: string;
}

export type MangaRuntimeFetch = (url: string, init?: RequestInit) => Promise<Pick<Response, "ok" | "status" | "json" | "text">>;

const DEFAULT_RUNTIME_URLS: Record<MangaRuntimeKind, string> = {
  suwayomi: "http://127.0.0.1:4567",
  komga: "http://127.0.0.1:25600",
  lanraragi: "http://127.0.0.1:3000",
  kavita: "http://127.0.0.1:5000",
};

const RUNTIME_LABELS: Record<MangaRuntimeKind, string> = {
  suwayomi: "Suwayomi",
  komga: "Komga",
  lanraragi: "LANraragi",
  kavita: "Kavita",
};

const PROBE_PATHS: Record<MangaRuntimeKind, string> = {
  suwayomi: "/graphql",
  komga: "/api/v1/libraries",
  lanraragi: "/api/archives",
  kavita: "/api/Library/libraries",
};

function trimUrl(value: string): string {
  return value.trim().replace(/\/+$/, "");
}

function asObject(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function asString(value: unknown, fallback = ""): string {
  if (typeof value === "string") return value;
  if (typeof value === "number" && Number.isFinite(value)) return String(value);
  return fallback;
}

function encodeBase64Ascii(value: string): string {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let output = "";
  let index = 0;

  while (index < value.length) {
    const byte1 = value.charCodeAt(index++) & 0xff;
    const byte2 = index < value.length ? value.charCodeAt(index++) & 0xff : Number.NaN;
    const byte3 = index < value.length ? value.charCodeAt(index++) & 0xff : Number.NaN;
    const triplet = (byte1 << 16) | ((Number.isNaN(byte2) ? 0 : byte2) << 8) | (Number.isNaN(byte3) ? 0 : byte3);

    output += chars[(triplet >> 18) & 63];
    output += chars[(triplet >> 12) & 63];
    output += Number.isNaN(byte2) ? "=" : chars[(triplet >> 6) & 63];
    output += Number.isNaN(byte3) ? "=" : chars[triplet & 63];
  }

  return output;
}

export function normalizeMangaRuntimeConfig(config: MangaRuntimeConfig): NormalizedMangaRuntimeConfig {
  const kind = config.kind;
  const baseUrl = trimUrl(config.baseUrl || DEFAULT_RUNTIME_URLS[kind]);

  return {
    kind,
    baseUrl,
    token: typeof config.token === "string" ? config.token.trim() : "",
    username: typeof config.username === "string" ? config.username.trim() : "",
    password: typeof config.password === "string" ? config.password : "",
  };
}

export function buildMangaRuntimeEndpoint(config: MangaRuntimeConfig, path = PROBE_PATHS[config.kind]): string {
  const normalized = normalizeMangaRuntimeConfig(config);
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;

  return `${normalized.baseUrl}${normalizedPath}`;
}

export function buildMangaRuntimeHeaders(config: MangaRuntimeConfig): HeadersInit {
  const normalized = normalizeMangaRuntimeConfig(config);
  const headers: Record<string, string> = {
    accept: "application/json",
  };

  if (normalized.kind === "komga" && normalized.username && normalized.password) {
    headers.authorization = `Basic ${encodeBase64Ascii(`${normalized.username}:${normalized.password}`)}`;
  } else if (normalized.token) {
    headers.authorization = `Bearer ${normalized.token}`;
  }

  if (normalized.kind === "lanraragi" && normalized.token) {
    headers.authorization = `Bearer ${normalized.token}`;
  }

  return headers;
}

export async function probeMangaRuntime(
  fetcher: MangaRuntimeFetch,
  config: MangaRuntimeConfig,
): Promise<MangaRuntimeProbeResult> {
  const normalized = normalizeMangaRuntimeConfig(config);
  const endpoint = buildMangaRuntimeEndpoint(normalized);
  const label = RUNTIME_LABELS[normalized.kind];

  try {
    const response = await fetcher(endpoint, {
      method: "GET",
      headers: buildMangaRuntimeHeaders(normalized),
    });

    if (response.status === 401 || response.status === 403) {
      return {
        kind: normalized.kind,
        status: "authRequired",
        endpoint,
        message: `${label} 已响应，但需要认证`,
        requiresAuth: true,
      };
    }

    if (response.ok) {
      return {
        kind: normalized.kind,
        status: "online",
        endpoint,
        message: `${label} 可访问`,
        requiresAuth: false,
      };
    }

    return {
      kind: normalized.kind,
      status: "schemaMismatch",
      endpoint,
      message: `${label} 探测返回 HTTP ${response.status}`,
      requiresAuth: false,
    };
  } catch {
    return {
      kind: normalized.kind,
      status: "offline",
      endpoint,
      message: `未检测到 ${label} 服务`,
      requiresAuth: false,
    };
  }
}

function normalizeLibrary(raw: unknown, config: NormalizedMangaRuntimeConfig): MangaRuntimeLibrary {
  const item = asObject(raw);
  const id = asString(item.id ?? item.key ?? item.archiveId ?? item.name, "runtime-library");
  const name = asString(item.name ?? item.title ?? item.label, RUNTIME_LABELS[config.kind]);

  return {
    id,
    name,
    kind: config.kind,
    sourceName: RUNTIME_LABELS[config.kind],
    baseUrl: config.baseUrl,
    language: asString(item.lang ?? item.language, "all"),
  };
}

function extractLibraryItems(payload: unknown, kind: MangaRuntimeKind): unknown[] {
  if (kind === "suwayomi") {
    const data = asObject(asObject(payload).data);
    const sources = asObject(data.sources);
    return asArray(sources.nodes);
  }

  const object = asObject(payload);
  return asArray(payload)
    .concat(asArray(object.content))
    .concat(asArray(object.items))
    .concat(asArray(object.data));
}

export async function loadMangaRuntimeLibraries(
  fetcher: MangaRuntimeFetch,
  config: MangaRuntimeConfig,
): Promise<MangaRuntimeLibrary[]> {
  const normalized = normalizeMangaRuntimeConfig(config);
  const endpoint = buildMangaRuntimeEndpoint(normalized);
  const init: RequestInit =
    normalized.kind === "suwayomi"
      ? {
          method: "POST",
          headers: { ...buildMangaRuntimeHeaders(normalized), "content-type": "application/json" },
          body: JSON.stringify({
            query: "query MoePlayRuntimeSources($first: Int) { sources(first: $first) { nodes { id name lang baseUrl homeUrl } } }",
            variables: { first: 50 },
          }),
        }
      : {
          method: "GET",
          headers: buildMangaRuntimeHeaders(normalized),
        };
  const response = await fetcher(endpoint, init);

  if (!response.ok) {
    return [];
  }

  const payload = await response.json();
  return extractLibraryItems(payload, normalized.kind).map((item) => normalizeLibrary(item, normalized));
}

export function toMangaSourceCandidates(libraries: MangaRuntimeLibrary[]): ExtensionSourceCandidate[] {
  return libraries.map((library) => ({
    id: `${library.kind}:${library.id}`,
    extensionName: `${RUNTIME_LABELS[library.kind]} Runtime`,
    sourceName: library.name,
    packageName: `${library.kind}.${library.id}`,
    mediaType: "comic",
    language: library.language,
    version: "runtime",
    baseUrl: library.baseUrl,
    apkName: "",
    nsfw: false,
    hasCloudflare: false,
    licenseRisk: library.kind === "kavita" ? "high" : "low",
    requiresExternalRuntime: true,
    repositoryId: `${library.kind}-runtime`,
    repositoryName: `${RUNTIME_LABELS[library.kind]} Runtime`,
    status: library.kind === "kavita" ? "unsupported" : "discoverable",
    statusReason: library.kind === "kavita" ? "GPL 服务仅通过外部 API 边界参考" : "已连接运行时，可继续读取库/章节/页",
  }));
}
