export const SOURCE_MEDIA_TYPES = ["anime", "comic", "external_runtime"] as const;
export type SourceMediaType = (typeof SOURCE_MEDIA_TYPES)[number];
export type SourceMediaTypeFilter = SourceMediaType | "all";

/** Rust ProviderCapability values exposed by `list_source_descriptors`. */
export const SOURCE_CAPABILITIES = ["probe", "search", "detail", "children", "resolve", "progress_read", "progress_write", "download", "verify"] as const;
export type SourceCapability = (typeof SOURCE_CAPABILITIES)[number];
export type SourceCapabilityFilter = SourceCapability | "all";

/** Rust SourceRuntimeState values exposed by `list_source_descriptors`. */
export const SOURCE_RUNTIME_STATES = ["available", "unavailable", "deferred", "unknown"] as const;
export type SourceRuntimeState = (typeof SOURCE_RUNTIME_STATES)[number];

/** Rust ProviderHealthState values exposed by `list_source_descriptors`. */
export const SOURCE_HEALTH_STATES = ["healthy", "degraded", "open_circuit", "disabled", "unknown"] as const;
export type SourceHealthState = (typeof SOURCE_HEALTH_STATES)[number];

/** Rust SourceAuthState values exposed by `list_source_descriptors`. */
export const SOURCE_AUTH_STATES = ["not_required", "configured", "missing", "unknown"] as const;
export type SourceAuthState = (typeof SOURCE_AUTH_STATES)[number];

export type SourceNsfwMode = "allow" | "exclude" | "only" | "unknown";
export type SourceNsfwFilter = SourceNsfwMode | "all";
export const SOURCE_PRIORITY_MIN = -10_000;
export const SOURCE_PRIORITY_MAX = 10_000;

export interface SourceFailure { code: string; message: string; occurredAt?: string; }
export interface SourceHealth { state: SourceHealthState; latencyMs: number | null; lastCheckedAt: string | null; consecutiveFailures: number; successRate: number | null; lastFailure?: SourceFailure; }
/** Unified projection for Anime, Comic, and external runtime sources. */
export interface SourceDescriptor {
  providerId: string; mediaType: SourceMediaType; kind: string; displayName: string;
  capabilities: SourceCapability[]; enabled: boolean; priority: number; health: SourceHealth;
  latencyMs: number | null; lastCheckedAt: string | null; authState: SourceAuthState; runtimeState: SourceRuntimeState;
  languages: string[]; nsfw: SourceNsfwMode; recentFailures: SourceFailure[];
}
export interface SourceRef { providerId: string; mediaType: SourceMediaType; }
export interface SourcePreferenceUpdate extends SourceRef { enabled: boolean; priority: number; }
export interface SourceFilters { mediaType: SourceMediaTypeFilter; capability: SourceCapabilityFilter; language: string | "all"; nsfw: SourceNsfwFilter; runtime: SourceRuntimeState | "all"; }
export interface ExtensionIndexSnapshot { entries: Array<{ id: string; name: string }>; fetchedAt: string | null; expiresAt: string | null; isOfflineSnapshot: boolean; lastError: string | null; }
export interface SourceCenterApi {
 listSourceDescriptors(): Promise<SourceDescriptor[]>;
 updateSourcePreference(update: SourcePreferenceUpdate): Promise<SourceDescriptor | void>;
 verifySource(source: SourceRef): Promise<void>;
 verifySourcesBatch(sources: SourceRef[]): Promise<void>;
 resetSourceHealth(source: SourceRef): Promise<SourceDescriptor | void>;
 refreshExtensionIndex(endpoint: string | null): Promise<ExtensionIndexSnapshot | null>;
 getExtensionIndexSnapshot(endpoint: string | null): Promise<ExtensionIndexSnapshot | null>;
}
export interface SourceCenterSnapshot { sources: SourceDescriptor[]; allSources: SourceDescriptor[]; filters: SourceFilters; extensionIndex: ExtensionIndexSnapshot | null; extensionIndexEndpoint: string | null; loading: boolean; refreshing: boolean; error: string | null; actionKeys: string[]; lastLoadedAt: number | null; }

const media = new Set<string>(SOURCE_MEDIA_TYPES);
const caps = new Set<string>(SOURCE_CAPABILITIES);
const runtime = new Set<string>(SOURCE_RUNTIME_STATES);
const healthStates = new Set<string>(SOURCE_HEALTH_STATES);
const authStates = new Set<string>(SOURCE_AUTH_STATES);
const obj = (v: unknown): Record<string, unknown> => v && typeof v === "object" ? v as Record<string, unknown> : {};
const text = (v: unknown, fallback = ""): string => typeof v === "string" && v.trim() ? v.trim() : fallback;
const nullableText = (v: unknown): string | null => text(v) || null;
const finite = (v: unknown): number | null => { const n = typeof v === "number" ? v : Number(v); return Number.isFinite(n) ? n : null; };
const nonNegative = (v: unknown): number | null => { const n = finite(v); return n !== null && n >= 0 ? n : null; };
const asMedia = (v: unknown): SourceMediaType => { const candidate = text(v); return media.has(candidate) ? candidate as SourceMediaType : "anime"; };
const asRuntime = (v: unknown): SourceRuntimeState => { const candidate = text(v); return runtime.has(candidate) ? candidate as SourceRuntimeState : "unknown"; };
const asHealth = (v: unknown): SourceHealthState => { const candidate = text(v); return healthStates.has(candidate) ? candidate as SourceHealthState : "unknown"; };
const asAuth = (v: unknown): SourceAuthState => { const candidate = text(v); return authStates.has(candidate) ? candidate as SourceAuthState : "unknown"; };

export const sourceKey = (source: SourceRef): string => source.mediaType + ":" + source.providerId;
export const clampSourcePriority = (value: unknown): number => Math.min(SOURCE_PRIORITY_MAX, Math.max(SOURCE_PRIORITY_MIN, Math.trunc(finite(value) ?? 0)));
export function emptySourceFilters(): SourceFilters { return { mediaType: "all", capability: "all", language: "all", nsfw: "all", runtime: "all" }; }

/** Explicitly maps the camelCase Rust DTO into the frontend SourceDescriptor. */
export function normalizeSourceDescriptor(value: unknown): SourceDescriptor {
 const raw = obj(value);
 const health = obj(raw.health);
 const lastCheckedAt = nullableText(raw.lastCheckedAt ?? raw.last_checked_at ?? health.lastCheckedAt ?? health.last_checked_at);
 const latencyMs = nonNegative(raw.latencyMs ?? raw.latency_ms ?? raw.latency ?? health.latencyMs ?? health.latency_ms);
 const failure = (item: unknown): SourceFailure | undefined => {
   const x = obj(item);
   const code = text(x.code ?? x.kind) || text(item);
   const message = text(x.message);
   return code || message ? { code: code || "unknown_error", message: message || "来源验证失败", occurredAt: nullableText(x.occurredAt ?? x.occurred_at) ?? undefined } : undefined;
 };
 const failureValues: unknown[] = Array.isArray(raw.recentFailures ?? raw.recent_failures) ? (raw.recentFailures ?? raw.recent_failures) as unknown[] : [];
 const recentFailures = failureValues.map(failure).filter((item): item is SourceFailure => Boolean(item)).slice(0, 10);
 const nsfwText = text(raw.nsfw ?? raw.nsfwMode ?? raw.nsfw_mode, "unknown");
 const nsfw: SourceNsfwMode = ["allow", "exclude", "only", "unknown"].includes(nsfwText) ? nsfwText as SourceNsfwMode : "unknown";
 const lastFailure = failure(health.lastFailure ?? health.last_failure ?? health.lastErrorKind ?? health.last_error_kind);
 return {
   providerId: text(raw.providerId ?? raw.provider_id, "unknown-provider"),
   mediaType: asMedia(raw.mediaType ?? raw.media_type),
   kind: text(raw.kind, "builtin"),
   displayName: text(raw.displayName ?? raw.display_name ?? raw.name, text(raw.providerId ?? raw.provider_id, "未知来源")),
   capabilities: [...new Set((Array.isArray(raw.capabilities) ? raw.capabilities : []).map((item) => text(item)).filter((item): item is SourceCapability => caps.has(item)))],
   enabled: typeof raw.enabled === "boolean" ? raw.enabled : true,
   priority: clampSourcePriority(raw.priority),
   health: {
     state: asHealth(health.state ?? raw.healthState ?? raw.health_state),
     latencyMs,
     lastCheckedAt,
     consecutiveFailures: Math.max(0, Math.trunc(nonNegative(health.consecutiveFailures ?? health.consecutive_failures) ?? 0)),
     successRate: nonNegative(health.successRate ?? health.success_rate),
     lastFailure,
   },
   latencyMs,
   lastCheckedAt,
   authState: asAuth(raw.authState ?? raw.auth_state),
   runtimeState: asRuntime(raw.runtimeState ?? raw.runtime_state),
   languages: [...new Set((Array.isArray(raw.languages) ? raw.languages : []).map((v) => text(v).toLowerCase()).filter(Boolean))],
   nsfw,
   recentFailures: recentFailures.length ? recentFailures : lastFailure ? [lastFailure] : [],
 };
}
export function normalizeSourceDescriptors(value: unknown): SourceDescriptor[] { const raw = Array.isArray(value) ? value : obj(value).sources ?? obj(value).descriptors ?? []; return sortSources((Array.isArray(raw) ? raw : []).map(normalizeSourceDescriptor)); }
export function normalizeExtensionIndexSnapshot(value: unknown): ExtensionIndexSnapshot {
 const response = obj(value);
 const refresh = obj(response.refresh);
 const raw = Object.keys(refresh).length ? obj(refresh.snapshot) : response;
 const entries: unknown[] = Array.isArray(raw.entries ?? raw.extensions) ? (raw.entries ?? raw.extensions) as unknown[] : [];
 const refreshState = text(refresh.state ?? response.state);
 return {
   entries: entries.map((entry) => { const item = obj(entry); return { id: text(item.id, "unknown-extension"), name: text(item.name, "未知扩展") }; }),
   fetchedAt: nullableText(raw.fetchedAt ?? raw.fetched_at),
   expiresAt: nullableText(raw.expiresAt ?? raw.expires_at),
   isOfflineSnapshot: raw.isOfflineSnapshot === true || raw.is_offline_snapshot === true || refreshState === "offline_snapshot",
   lastError: nullableText(raw.lastError ?? raw.last_error ?? refresh.warningCode ?? refresh.warning_code),
 };
}
export function filterSources(sources: readonly SourceDescriptor[], filters: SourceFilters): SourceDescriptor[] { return sortSources(sources.filter((source) => (filters.mediaType === "all" || source.mediaType === filters.mediaType) && (filters.capability === "all" || source.capabilities.includes(filters.capability)) && (filters.language === "all" || source.languages.includes(filters.language)) && (filters.nsfw === "all" || source.nsfw === filters.nsfw) && (filters.runtime === "all" || source.runtimeState === filters.runtime))); }
export function sortSources(sources: readonly SourceDescriptor[]): SourceDescriptor[] { const rank: Record<SourceHealthState, number> = { healthy: 0, degraded: 1, unknown: 2, disabled: 3, open_circuit: 4 }; return [...sources].sort((a,b) => Number(b.enabled)-Number(a.enabled) || b.priority-a.priority || rank[a.health.state]-rank[b.health.state] || a.displayName.localeCompare(b.displayName, "zh-CN")); }
