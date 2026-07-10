export type ProviderHealthState = "healthy" | "degraded" | "open_circuit" | "disabled" | "unknown";
export type ComicProviderKind = "local" | "komga" | "kavita";
export type ComicProviderAuthMode = "none" | "basic" | "bearer" | "api_key";

export interface ComicSearchRequest {
  query: string;
  libraryId?: string;
  page?: number;
  pageSize?: number;
}

export interface ComicLibrary {
  id: string;
  name: string;
  path?: string;
  kind?: string;
}

export interface ComicProviderProbe {
  providerId: string;
  reachable: boolean;
  authenticated: boolean;
  serverVersion?: string;
  latencyMs?: number;
  libraries: ComicLibrary[];
}

export interface ComicSeries {
  id: string;
  providerId: string;
  libraryId?: string;
  title: string;
  sortTitle?: string;
  summary?: string;
  coverUrl?: string;
  language?: string;
  year?: number;
}

export interface ComicSeriesDetail {
  series: ComicSeries;
  alternateTitles: string[];
  genres: string[];
  status?: string;
  totalChapters?: number;
}

export interface ComicChapter {
  identity: { providerId: string; seriesId: string; volumeId?: string; chapterId: string; stableKey: string };
  title: string;
  sort: { volumeNumber?: number; chapterNumber?: number; ordinal?: number; title: string };
  language?: string;
  languageSource: "provider" | "manifest" | "filename" | "unknown";
  pageCount?: number;
  publishedAt?: string;
  fileName?: string;
}

export type ComicResolvedTarget =
  | { mode: "native_hls"; url: string; headers: [string, string][] }
  | { mode: "image_pages"; pages: string[]; headers: [string, string][] }
  | { mode: "webview"; url: string; allowedHosts: string[] }
  | { mode: "external"; url: string; reason: string }
  | { mode: "unsupported"; reason: string; errorKind: string }
  | { mode: "native_file"; path: string };

export interface ComicProviderManifest {
  id: string;
  name: string;
  resourceKinds: string[];
  capabilities: string[];
  trust: string;
  version: string;
  enabled: boolean;
  requiresAuth: boolean;
  allowedHosts: string[];
}

export interface ComicProviderDescriptor {
  id: string;
  kind: ComicProviderKind;
  name: string;
  localRoot?: string;
  baseUrl?: string;
  origin?: string;
  username?: string;
  authMode: ComicProviderAuthMode;
  secretConfigured: boolean;
  manifest: ComicProviderManifest;
}

export type ComicProviderConfigureRequest =
  | { kind: "local"; root: string }
  | {
      kind: "komga" | "kavita";
      baseUrl: string;
      authMode: ComicProviderAuthMode;
      username?: string;
      /** One-time input. Rust stores it in SecretStore and never returns it. */
      secret?: string;
    };

export interface ComicProviderCommandError {
  kind: string;
  message: string;
  retryable: boolean;
  providerId?: string;
  operation?: string;
}

export interface ComicProviderApi {
  configure(request: ComicProviderConfigureRequest): Promise<ComicProviderDescriptor>;
  list(): Promise<ComicProviderDescriptor[]>;
  remove(providerId: string): Promise<boolean>;
  probe(providerId: string): Promise<ComicProviderProbe>;
  search(providerId: string, request: ComicSearchRequest): Promise<ComicSeries[]>;
  detail(providerId: string, seriesId: string): Promise<ComicSeriesDetail>;
  chapters(providerId: string, seriesId: string): Promise<ComicChapter[]>;
  resolve(providerId: string, seriesId: string, chapterId: string): Promise<ComicResolvedTarget>;
}

export interface ComicFeatureState {
  generation: number;
  providerId?: string;
  providers: ComicProviderDescriptor[];
  loading: boolean;
  error?: unknown;
  series: ComicSeries[];
  detailsBySeries: Record<string, ComicSeriesDetail>;
  chaptersBySeries: Record<string, ComicChapter[]>;
  targetsByChapter: Record<string, ComicResolvedTarget>;
  probesByProvider: Record<string, ComicProviderProbe>;
}

export type ComicReaderDecision =
  | { kind: "images"; pages: string[]; headers: [string, string][] }
  | { kind: "local_file"; path: string }
  | { kind: "external"; url: string; reason: string }
  | { kind: "blocked"; reason: string };
