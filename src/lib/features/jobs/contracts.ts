import type { AppTask, TaskStatus } from "../../api/types";

export const JOB_KINDS = [
  "download",
  "import",
  "scrape",
  "provider_verify",
  "ai",
  "backup",
  "restore",
  "diagnostics",
  "update",
  "generic",
] as const;

export type JobKind = (typeof JOB_KINDS)[number];

export const JOB_STATUSES = [
  "queued",
  "running",
  "paused",
  "succeeded",
  "failed",
  "cancelled",
] as const;

export type JobStatus = (typeof JOB_STATUSES)[number];
export type JobStatusFilter = JobStatus | "all";
export type JobKindFilter = JobKind | "all";

export interface JobSource {
  area: string;
  entityId?: string;
  label?: string;
}

/** Canonical application-level task projection. Snake-case timestamps are kept for legacy consumers. */
export interface TaskCenterJob {
  id: string;
  kind: JobKind;
  title: string;
  status: JobStatus;
  progress: number;
  message?: string;
  errorKind?: string;
  retryable: boolean;
  resumable: boolean;
  cancellable: boolean;
  /** Extension used by existing download UI until the global panel owns capability rendering. */
  pausable: boolean;
  recovered: boolean;
  createdAt: string;
  updatedAt: string;
  idempotencyKey?: string;
  source?: JobSource;
  /** Original values are useful for diagnostics when a newer backend adds an enum member. */
  rawKind?: string;
  rawStatus?: string;
  metadata?: unknown;
  created_at: string;
  updated_at: string;
}

/** Compatibility alias used by DownloadPage and JobsPanel. */
export type Job = TaskCenterJob;

export interface JobsFilter {
  status?: JobStatusFilter;
  kind?: JobKindFilter;
  limit?: number;
}

export interface JobsApi {
  list(filter?: JobsFilter, signal?: AbortSignal): Promise<AppTask[]>;
  cancel(id: string): Promise<AppTask | void>;
  clearFinished(): Promise<void>;
  enqueue(title: string, kind: string, idempotencyKey?: string): Promise<AppTask>;
  pause?(id: string): Promise<AppTask | void>;
  resume?(id: string): Promise<AppTask | void>;
  retry?(id: string): Promise<AppTask | void>;
}

export interface JobCounts {
  total: number;
  active: number;
  failed: number;
  completed: number;
}

export interface JobsSnapshot {
  /** Current filtered projection. */
  jobs: Job[];
  /** Full locally-known projection, before client-side filters. */
  allJobs: Job[];
  byStatus: Record<JobStatus, Job[]>;
  counts: JobCounts;
  filter: Readonly<JobsFilter>;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  lastLoadedAt: number | null;
}

type RawTask = AppTask & {
  kind?: unknown;
  status?: unknown;
  recovered?: unknown;
  resumable?: unknown;
  retryable?: unknown;
  cancellable?: unknown;
  pausable?: unknown;
  errorKind?: unknown;
  error_kind?: unknown;
  idempotencyKey?: unknown;
  idempotency_key?: unknown;
  source?: unknown;
  createdAt?: unknown;
  updatedAt?: unknown;
};

const KIND_SET = new Set<string>(JOB_KINDS);
const STATUS_SET = new Set<string>(JOB_STATUSES);

export function normalizeJobStatus(status: TaskStatus | string): JobStatus {
  if (status === "pending") return "queued";
  if (status === "completed") return "succeeded";
  return STATUS_SET.has(status) ? (status as JobStatus) : "failed";
}

export function normalizeJobKind(kind: unknown): JobKind {
  return typeof kind === "string" && KIND_SET.has(kind) ? (kind as JobKind) : "generic";
}

/** Convert legacy 0..100 responses without changing the new 0..1 contract. */
export function normalizeJobProgress(progress: number): number {
  if (!Number.isFinite(progress)) return 0;
  const fraction = progress > 1 ? progress / 100 : progress;
  return Math.min(1, Math.max(0, fraction));
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function normalizeSource(value: unknown): JobSource | undefined {
  if (!value || typeof value !== "object") return undefined;
  const source = value as Record<string, unknown>;
  const area = optionalString(source.area);
  if (!area) return undefined;
  return {
    area,
    entityId: optionalString(source.entityId ?? source.entity_id),
    label: optionalString(source.label),
  };
}

export function normalizeJob(task: AppTask): Job {
  const raw = task as RawTask;
  const rawStatus = typeof raw.status === "string" ? raw.status : "unknown";
  const rawKind = typeof raw.kind === "string" ? raw.kind : "unknown";
  const status = normalizeJobStatus(rawStatus);
  const kind = normalizeJobKind(rawKind);
  const createdAt = optionalString(raw.createdAt) ?? optionalString(raw.created_at) ?? "";
  const updatedAt = optionalString(raw.updatedAt) ?? optionalString(raw.updated_at) ?? createdAt;
  const isActive = status === "queued" || status === "running" || status === "paused";
  const isUnknownStatus = !STATUS_SET.has(rawStatus) && rawStatus !== "pending" && rawStatus !== "completed";

  return {
    id: String(raw.id),
    title: optionalString(raw.title) ?? "未命名任务",
    kind,
    status,
    progress: normalizeJobProgress(Number(raw.progress)),
    message: optionalString(raw.message),
    errorKind: optionalString(raw.errorKind ?? raw.error_kind),
    retryable: !isUnknownStatus && (typeof raw.retryable === "boolean" ? raw.retryable : status === "failed"),
    resumable: !isUnknownStatus && (typeof raw.resumable === "boolean" ? raw.resumable : status === "paused"),
    cancellable: !isUnknownStatus && (typeof raw.cancellable === "boolean" ? raw.cancellable : isActive),
    pausable: !isUnknownStatus && (typeof raw.pausable === "boolean" ? raw.pausable : kind === "download" && status === "running"),
    recovered: raw.recovered === true,
    createdAt,
    updatedAt,
    idempotencyKey: optionalString(raw.idempotencyKey ?? raw.idempotency_key),
    source: normalizeSource(raw.source),
    rawKind: kind === "generic" && rawKind !== "generic" ? rawKind : undefined,
    rawStatus: isUnknownStatus ? rawStatus : undefined,
    metadata: raw.metadata,
    created_at: createdAt,
    updated_at: updatedAt,
  };
}

export function emptyBuckets(): Record<JobStatus, Job[]> {
  return {
    queued: [],
    running: [],
    paused: [],
    succeeded: [],
    failed: [],
    cancelled: [],
  };
}
