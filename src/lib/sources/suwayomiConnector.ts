import type { ExtensionSourceCandidate } from "./extensionIndex";

export type SuwayomiConnectionStatus = "online" | "authRequired" | "offline" | "schemaMismatch";

export interface SuwayomiConnectionConfig {
  host?: string;
  port?: number;
  protocol?: "http" | "https";
  token?: string;
}

export interface SuwayomiStoredConnectionConfig {
  host: string;
  port: number;
  protocol: "http" | "https";
  token: string;
}

export interface SuwayomiProbeResult {
  status: SuwayomiConnectionStatus;
  endpoint: string;
  message: string;
  requiresAuth: boolean;
}

export interface SuwayomiGraphqlRequest {
  query: string;
  variables?: Record<string, unknown>;
}

export interface SuwayomiGraphqlError {
  message: string;
}

export interface SuwayomiGraphqlResponse<T> {
  data?: T;
  errors?: SuwayomiGraphqlError[];
}

export interface SuwayomiSourceNode {
  id: number;
  name: string;
  lang: string;
  iconUrl?: string | null;
  supportsLatest?: boolean;
  isConfigurable?: boolean;
  homeUrl?: string | null;
  baseUrl?: string | null;
}

export interface SuwayomiExtensionNode {
  name: string;
  pkgName: string;
  lang: string;
  isInstalled: boolean;
  hasUpdate: boolean;
  isObsolete: boolean;
}

export interface SuwayomiSourcesQueryData {
  sources: {
    totalCount: number;
    nodes: SuwayomiSourceNode[];
  };
}

export interface SuwayomiExtensionsQueryData {
  extensions: {
    totalCount: number;
    nodes: SuwayomiExtensionNode[];
  };
}

export interface SuwayomiRuntimeSnapshot {
  endpoint: string;
  sources: SuwayomiSourceNode[];
  installedExtensions: SuwayomiExtensionNode[];
  sourceTotal: number;
  extensionTotal: number;
}

export type SuwayomiFetch = (url: string, init?: RequestInit) => Promise<Pick<Response, "ok" | "status" | "json" | "text">>;

export const SUWAYOMI_DEFAULT_CONFIG: Required<Omit<SuwayomiConnectionConfig, "token">> = {
  protocol: "http",
  host: "127.0.0.1",
  port: 4567,
};

export const SUWAYOMI_CONFIG_STORAGE_KEY = "moeplay-suwayomi-config-v1";

export const SUWAYOMI_DEFAULT_STORED_CONFIG: SuwayomiStoredConnectionConfig = {
  ...SUWAYOMI_DEFAULT_CONFIG,
  token: "",
};

export const SUWAYOMI_SOURCES_QUERY = `query MoePlaySuwayomiSources($first: Int) {
  sources(first: $first) {
    totalCount
    nodes {
      id
      name
      lang
      iconUrl
      supportsLatest
      isConfigurable
      homeUrl
      baseUrl
    }
  }
}`;

export const SUWAYOMI_INSTALLED_EXTENSIONS_QUERY = `query MoePlaySuwayomiInstalledExtensions($first: Int) {
  extensions(first: $first, condition: { isInstalled: true }) {
    totalCount
    nodes {
      name
      pkgName
      lang
      isInstalled
      hasUpdate
      isObsolete
    }
  }
}`;

function normalizeHost(host: string): string {
  return host.replace(/^https?:\/\//, "").replace(/\/+$/, "") || SUWAYOMI_DEFAULT_CONFIG.host;
}

function normalizePort(port: unknown): number {
  const value = Number(port);
  if (!Number.isInteger(value) || value < 1 || value > 65535) {
    return SUWAYOMI_DEFAULT_CONFIG.port;
  }
  return value;
}

function normalizeProtocol(protocol: unknown): "http" | "https" {
  return protocol === "https" ? "https" : "http";
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

export function normalizeSuwayomiConfig(config: SuwayomiConnectionConfig = {}): SuwayomiStoredConnectionConfig {
  return {
    protocol: normalizeProtocol(config.protocol),
    host: normalizeHost(config.host ?? SUWAYOMI_DEFAULT_CONFIG.host),
    port: normalizePort(config.port),
    token: typeof config.token === "string" ? config.token.trim() : "",
  };
}

export function loadSuwayomiConfig(storage: Pick<Storage, "getItem"> | undefined = globalThis.localStorage): SuwayomiStoredConnectionConfig {
  if (!storage) {
    return SUWAYOMI_DEFAULT_STORED_CONFIG;
  }

  try {
    const raw = storage.getItem(SUWAYOMI_CONFIG_STORAGE_KEY);
    if (!raw) {
      return SUWAYOMI_DEFAULT_STORED_CONFIG;
    }

    const stored = asRecord(JSON.parse(raw));
    return normalizeSuwayomiConfig({
      protocol: stored.protocol as SuwayomiConnectionConfig["protocol"],
      host: typeof stored.host === "string" ? stored.host : undefined,
      port: typeof stored.port === "number" || typeof stored.port === "string" ? Number(stored.port) : undefined,
      token: typeof stored.token === "string" ? stored.token : "",
    });
  } catch {
    return SUWAYOMI_DEFAULT_STORED_CONFIG;
  }
}

export function saveSuwayomiConfig(
  config: SuwayomiConnectionConfig,
  storage: Pick<Storage, "setItem"> | undefined = globalThis.localStorage,
): SuwayomiStoredConnectionConfig {
  const normalized = normalizeSuwayomiConfig(config);
  storage?.setItem(SUWAYOMI_CONFIG_STORAGE_KEY, JSON.stringify(normalized));
  return normalized;
}

export function clearSuwayomiConfig(storage: Pick<Storage, "removeItem"> | undefined = globalThis.localStorage): SuwayomiStoredConnectionConfig {
  storage?.removeItem(SUWAYOMI_CONFIG_STORAGE_KEY);
  return SUWAYOMI_DEFAULT_STORED_CONFIG;
}

export function buildSuwayomiBaseUrl(config: SuwayomiConnectionConfig = {}): string {
  const normalized = normalizeSuwayomiConfig(config);

  return `${normalized.protocol}://${normalized.host}:${normalized.port}`;
}

export function buildSuwayomiGraphqlEndpoint(config: SuwayomiConnectionConfig = {}): string {
  return `${buildSuwayomiBaseUrl(config)}/graphql`;
}

function buildHeaders(config: SuwayomiConnectionConfig): HeadersInit {
  const headers: Record<string, string> = {
    accept: "application/json",
    "content-type": "application/json",
  };

  if (config.token) {
    headers.authorization = `Bearer ${config.token}`;
  }

  return headers;
}

export function withSuwayomiTimeout(fetcher: SuwayomiFetch, timeoutMs = 2500): SuwayomiFetch {
  return async (url, init) => {
    const controller = new AbortController();
    const timeout = globalThis.setTimeout(() => controller.abort(), timeoutMs);

    try {
      return await fetcher(url, { ...init, signal: init?.signal ?? controller.signal });
    } finally {
      globalThis.clearTimeout(timeout);
    }
  };
}

export async function probeSuwayomiServer(
  fetcher: SuwayomiFetch,
  config: SuwayomiConnectionConfig = {},
): Promise<SuwayomiProbeResult> {
  const endpoint = buildSuwayomiGraphqlEndpoint(config);

  try {
    const response = await fetcher(endpoint, {
      method: "GET",
      headers: { accept: "text/html,application/json" },
    });

    if (response.status === 401 || response.status === 403) {
      return {
        status: "authRequired",
        endpoint,
        message: "Suwayomi 已响应，但 GraphQL 入口需要认证",
        requiresAuth: true,
      };
    }

    if (response.ok) {
      return {
        status: "online",
        endpoint,
        message: "Suwayomi GraphQL 入口可访问",
        requiresAuth: false,
      };
    }

    return {
      status: "schemaMismatch",
      endpoint,
      message: `Suwayomi GraphQL 探测返回 HTTP ${response.status}`,
      requiresAuth: false,
    };
  } catch {
    return {
      status: "offline",
      endpoint,
      message: "未检测到本地 Suwayomi 服务",
      requiresAuth: false,
    };
  }
}

export async function querySuwayomiGraphql<T>(
  fetcher: SuwayomiFetch,
  request: SuwayomiGraphqlRequest,
  config: SuwayomiConnectionConfig = {},
): Promise<SuwayomiGraphqlResponse<T>> {
  const response = await fetcher(buildSuwayomiGraphqlEndpoint(config), {
    method: "POST",
    headers: buildHeaders(config),
    body: JSON.stringify(request),
  });

  if (response.status === 401 || response.status === 403) {
    return { errors: [{ message: "Suwayomi GraphQL 入口需要认证" }] };
  }

  if (!response.ok) {
    return { errors: [{ message: `Suwayomi GraphQL 返回 HTTP ${response.status}` }] };
  }

  return (await response.json()) as SuwayomiGraphqlResponse<T>;
}

export async function listSuwayomiSources(
  fetcher: SuwayomiFetch,
  config: SuwayomiConnectionConfig = {},
  first = 50,
): Promise<SuwayomiGraphqlResponse<SuwayomiSourcesQueryData>> {
  return querySuwayomiGraphql<SuwayomiSourcesQueryData>(
    fetcher,
    { query: SUWAYOMI_SOURCES_QUERY, variables: { first } },
    config,
  );
}

export async function listSuwayomiInstalledExtensions(
  fetcher: SuwayomiFetch,
  config: SuwayomiConnectionConfig = {},
  first = 50,
): Promise<SuwayomiGraphqlResponse<SuwayomiExtensionsQueryData>> {
  return querySuwayomiGraphql<SuwayomiExtensionsQueryData>(
    fetcher,
    { query: SUWAYOMI_INSTALLED_EXTENSIONS_QUERY, variables: { first } },
    config,
  );
}

export async function loadSuwayomiRuntimeSnapshot(
  fetcher: SuwayomiFetch,
  config: SuwayomiConnectionConfig = {},
  first = 50,
): Promise<SuwayomiGraphqlResponse<SuwayomiRuntimeSnapshot>> {
  const [sourcesResponse, extensionsResponse] = await Promise.all([
    listSuwayomiSources(fetcher, config, first),
    listSuwayomiInstalledExtensions(fetcher, config, first),
  ]);
  const errors = [...(sourcesResponse.errors ?? []), ...(extensionsResponse.errors ?? [])];

  if (errors.length > 0) {
    return { errors };
  }

  const sources = sourcesResponse.data?.sources;
  const extensions = extensionsResponse.data?.extensions;

  if (!sources || !extensions) {
    return { errors: [{ message: "Suwayomi GraphQL 响应缺少 sources 或 extensions" }] };
  }

  return {
    data: {
      endpoint: buildSuwayomiGraphqlEndpoint(config),
      sources: sources.nodes,
      installedExtensions: extensions.nodes,
      sourceTotal: sources.totalCount,
      extensionTotal: extensions.totalCount,
    },
  };
}

export function summarizeSuwayomiErrors(response: Pick<SuwayomiGraphqlResponse<unknown>, "errors">): string {
  return response.errors?.map((error) => error.message).filter(Boolean).join("；") || "Suwayomi 查询失败";
}

export function toSuwayomiRuntimeCandidates(sources: SuwayomiSourceNode[]): ExtensionSourceCandidate[] {
  return sources.map((source) => ({
    id: `suwayomi:${source.id}`,
    extensionName: "Suwayomi Runtime",
    sourceName: source.name,
    packageName: `suwayomi.source.${source.id}`,
    mediaType: "comic",
    language: source.lang || "all",
    version: "runtime",
    baseUrl: source.baseUrl ?? source.homeUrl ?? "",
    apkName: "",
    nsfw: false,
    hasCloudflare: false,
    licenseRisk: "medium",
    requiresExternalRuntime: true,
    repositoryId: "suwayomi-runtime",
    repositoryName: "Suwayomi 本地运行时",
    status: source.baseUrl || source.homeUrl ? "discoverable" : "requiresRuntime",
    statusReason: source.baseUrl || source.homeUrl ? "本地运行时已安装该源，可继续接入阅读查询" : "本地运行时可见，但缺少主页地址",
  }));
}
