/**
 * Tauri Anime Provider contract. It is intentionally independent from the
 * legacy anime page and player source shapes.
 */
export type ProviderErrorKind =
  | "network"
  | "timeout"
  | "rate_limited"
  | "auth_required"
  | "captcha_required"
  | "parse_changed"
  | "geo_blocked"
  | "embed_blocked"
  | "unsupported_drm"
  | "policy_blocked"
  | "cancelled"
  | "unsupported"
  | "unknown";

export interface ProviderError {
  kind: ProviderErrorKind;
  message: string;
  retryable: boolean;
  retryAfterMs: number | null;
  providerId: string | null;
  operation: string | null;
}

export type AnimeResolvedTarget =
  | { mode: "native_hls"; url: string; headers: Array<[string, string]> }
  | { mode: "native_file"; path: string }
  | { mode: "webview"; url: string; allowedHosts: string[] }
  | { mode: "external"; url: string; reason: string }
  | { mode: "unsupported"; reason: string; errorKind: ProviderErrorKind };

export interface AnimeEpisodeIdentity {
  providerId: string;
  seriesId: string;
  episodeId: string;
}

export interface AnimeSearchQuery {
  query: string;
  limit: number | null;
}

export interface AnimeSearchItem {
  providerId: string;
  itemId: string;
  title: string;
  originalTitle: string | null;
  synopsis: string | null;
  artworkUrl: string | null;
}

export interface AnimeDetail {
  providerId: string;
  itemId: string;
  title: string;
  originalTitle: string | null;
  synopsis: string | null;
  artworkUrl: string | null;
  genres: string[];
}

export interface AnimeEpisode {
  identity: AnimeEpisodeIdentity;
  title: string;
  number: number | null;
  artworkUrl: string | null;
}

export interface AnimeResolveResponse {
  episode: AnimeEpisodeIdentity;
  target: AnimeResolvedTarget;
}

export interface AnimeProviderHealth {
  providerId: string;
  operation: string;
  state: "healthy" | "degraded" | "open_circuit" | "disabled" | "unknown";
  successCount: number;
  failureCount: number;
  consecutiveFailures: number;
  latencyMsEma: number | null;
  lastSuccessAt: string | null;
  lastFailureAt: string | null;
  circuitOpenUntil: string | null;
  lastErrorKind: ProviderErrorKind | null;
}

export interface AnimeSearchResponse {
  items: AnimeSearchItem[];
  failures: ProviderError[];
  providerHealth: AnimeProviderHealth[];
}

export type AnimeProviderKind = "local_media" | "jellyfin";

export interface AnimeLocalMediaEpisodeInput {
  id: string;
  title: string;
  number: number | null;
  /** An explicit, user-selected media file; arbitrary paths are not accepted. */
  path: string;
}

export interface AnimeLocalMediaSeriesInput {
  id: string;
  title: string;
  originalTitle: string | null;
  synopsis: string | null;
  artworkUrl: string | null;
  genres: string[];
  episodes: AnimeLocalMediaEpisodeInput[];
}

export interface AnimeLocalMediaScanResult {
  directory: string;
  allowedPaths: string[];
  library: AnimeLocalMediaSeriesInput[];
  seriesCount: number;
  fileCount: number;
  skippedCount: number;
  warnings: string[];
}

export interface AnimeProviderFallbackOpenResponse {
  mode: "native_file" | "webview" | "external";
}

export type AnimeProviderConfigureRequest =
  | {
    kind: "local_media";
    library: AnimeLocalMediaSeriesInput[];
    allowedPaths: string[];
  }
  | {
    kind: "jellyfin";
    baseUrl: string;
    /** One-time input only. It is persisted in SecretStore and never returned. */
    token?: string | null;
  };

/** Safe source metadata returned by configure/list. It excludes media entries and tokens. */
export interface AnimeProviderDescriptor {
  id: string;
  kind: AnimeProviderKind;
  name: string;
  localFileCount: number | null;
  allowedPaths: string[] | null;
  baseUrl: string | null;
  origin: string | null;
  secretConfigured: boolean;
  manifest: {
    id: string;
    name: string;
    resourceKinds: string[];
    capabilities: string[];
    trust: string;
    version: string;
    enabled: boolean;
    requiresAuth: boolean;
    allowedHosts: string[];
  };
}

/** Integration-facing API. Tauri DTO field names are passed through verbatim. */
export interface AnimeProviderApi {
  configure(request: AnimeProviderConfigureRequest, signal?: AbortSignal): Promise<AnimeProviderDescriptor>;
  list(signal?: AbortSignal): Promise<AnimeProviderDescriptor[]>;
  remove(providerId: string, signal?: AbortSignal): Promise<boolean>;
  health(signal?: AbortSignal): Promise<AnimeProviderHealth[]>;
  pickLocalDirectory(signal?: AbortSignal): Promise<AnimeLocalMediaScanResult | null>;
  search(query: AnimeSearchQuery, signal: AbortSignal, providerId?: string | null): Promise<AnimeSearchResponse>;
  detail(providerId: string, itemId: string, signal: AbortSignal): Promise<AnimeDetail>;
  episodes(providerId: string, seriesId: string, signal: AbortSignal): Promise<AnimeEpisode[]>;
  resolve(episode: AnimeEpisodeIdentity, signal: AbortSignal): Promise<AnimeResolveResponse>;
  openFallback(episode: AnimeEpisodeIdentity, signal?: AbortSignal): Promise<AnimeProviderFallbackOpenResponse>;
}