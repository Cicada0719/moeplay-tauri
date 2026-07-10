import type { AppTask, BackgroundJobStatus, TaskStatus } from "../../api/types";

export type JobStatus = BackgroundJobStatus;

export interface Job extends Omit<AppTask, "status"> {
  status: JobStatus;
  recovered?: boolean;
  resumable?: boolean;
  retryable?: boolean;
}

export interface JobsApi {
  list(): Promise<AppTask[]>;
  cancel(id: string): Promise<AppTask>;
  clearFinished(): Promise<void>;
  enqueue(title: string, kind: string, idempotencyKey?: string): Promise<AppTask>;
  pause?(id: string): Promise<void>;
  resume?(id: string): Promise<void>;
  retry?(id: string): Promise<void>;
}

export interface JobsSnapshot {
  jobs: Job[];
  byStatus: Record<JobStatus, Job[]>;
  loading: boolean;
  error: string | null;
  lastLoadedAt: number | null;
}

export const JOB_STATUSES: readonly JobStatus[] = [
  "queued",
  "running",
  "paused",
  "succeeded",
  "failed",
  "cancelled",
] as const;

export function normalizeJobStatus(status: TaskStatus): JobStatus {
  if (status === "pending") return "queued";
  if (status === "completed") return "succeeded";
  return status;
}

/** Convert legacy 0..100 responses without changing the new 0..1 contract. */
export function normalizeJobProgress(progress: number): number {
  if (!Number.isFinite(progress)) return 0;
  const fraction = progress > 1 ? progress / 100 : progress;
  return Math.min(1, Math.max(0, fraction));
}

export function normalizeJob(task: AppTask): Job {
  return {
    ...task,
    status: normalizeJobStatus(task.status),
    progress: normalizeJobProgress(task.progress),
    recovered: Boolean((task as AppTask & { recovered?: boolean }).recovered),
    resumable: Boolean((task as AppTask & { resumable?: boolean }).resumable),
    retryable: Boolean((task as AppTask & { retryable?: boolean }).retryable),
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
