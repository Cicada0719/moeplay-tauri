export type ResourceKind = "game" | "anime" | "comic";

export type ProviderCapability =
  | "probe"
  | "search"
  | "detail"
  | "children"
  | "resolve"
  | "progress_read"
  | "progress_write"
  | "download"
  | "verify";

export type ProviderTrust =
  | "built_in"
  | "public_api"
  | "user_configured"
  | "self_hosted"
  | "catalog_only"
  | "disabled";

export interface ProviderManifest {
  id: string;
  name: string;
  resourceKinds: ResourceKind[];
  capabilities: ProviderCapability[];
  trust: ProviderTrust;
  version: string;
  enabled: boolean;
  requiresAuth: boolean;
  allowedHosts: string[];
}

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
  retryAfterMs?: number | null;
  providerId?: string | null;
  operation?: string | null;
}

export type ProviderHealthState =
  | "healthy"
  | "degraded"
  | "open_circuit"
  | "disabled"
  | "unknown";

export interface ProviderHealth {
  providerId: string;
  operation: string;
  state: ProviderHealthState;
  successCount: number;
  failureCount: number;
  consecutiveFailures: number;
  latencyMsEma?: number | null;
  lastSuccessAt?: string | null;
  lastFailureAt?: string | null;
  circuitOpenUntil?: string | null;
  lastErrorKind?: ProviderErrorKind | null;
}

export type HeaderPair = [string, string];

export type ResolvedTarget =
  | { mode: "native_hls"; url: string; headers: HeaderPair[] }
  | { mode: "native_file"; path: string }
  | { mode: "image_pages"; pages: string[]; headers: HeaderPair[] }
  | { mode: "webview"; url: string; allowedHosts: string[] }
  | { mode: "external"; url: string; reason: string }
  | { mode: "unsupported"; reason: string; errorKind: ProviderErrorKind };

export type ActivityEventType =
  | "started"
  | "progressed"
  | "completed"
  | "rated"
  | "favorited"
  | "imported"
  | "failed";

export interface ActivityEvent {
  id: string;
  resourceKind: ResourceKind;
  resourceId: string;
  eventType: ActivityEventType;
  startedAt: string;
  endedAt?: string | null;
  durationSeconds?: number | null;
  providerId?: string | null;
  payload: unknown;
}

export interface ProgressRecord {
  resourceKind: ResourceKind;
  resourceId: string;
  providerId?: string | null;
  position: unknown;
  updatedAt: string;
  completed: boolean;
}

export type BackgroundJobStatus =
  | "queued"
  | "running"
  | "paused"
  | "succeeded"
  | "failed"
  | "cancelled";

export interface BackgroundJob {
  id: string;
  kind: string;
  title: string;
  status: BackgroundJobStatus;
  progress: number;
  createdAt: string;
  updatedAt: string;
  error?: ProviderError | null;
  metadata: unknown;
}

export function isTerminalJob(status: BackgroundJobStatus): boolean {
  return status === "succeeded" || status === "failed" || status === "cancelled";
}

export function canRetryProviderError(error: ProviderError): boolean {
  return error.retryable && ![
    "auth_required",
    "policy_blocked",
    "unsupported_drm",
    "cancelled",
  ].includes(error.kind);
}
