import { invokeCmd } from "../../api/core";
import type { AppTask, DownloadTask } from "../../api/types";
import {
  mergeTaskEvents,
  normalizeTaskDetail,
  normalizeTaskEvent,
  type JobsApi,
  type JobsFilter,
  type TaskDetailApi,
  type TaskEventsPage,
  type TaskEventsQuery,
} from "./contracts";

function listArgs(filter: JobsFilter = {}): Record<string, unknown> {
  const args: Record<string, unknown> = {};
  if (filter.status && filter.status !== "all") args.status = filter.status;
  if (filter.kind && filter.kind !== "all") args.kind = filter.kind;
  if (filter.limit !== undefined) args.limit = filter.limit;
  return args;
}

/** Task Center command bridge. Commands use Tauri's camelCase argument convention. */
export const tauriJobsApi: JobsApi = {
  list: (filter) => invokeCmd<AppTask[]>("get_tasks", listArgs(filter)),
  cancel: (id) => invokeCmd<AppTask>("cancel_task", { id }),
  clearFinished: () => invokeCmd("clear_finished_tasks"),
  enqueue: (title, kind, idempotencyKey) => {
    const args: Record<string, unknown> = { title, kind };
    if (idempotencyKey) args.idempotencyKey = idempotencyKey;
    return invokeCmd<AppTask>("enqueue_task", args);
  },
  pause: (id) => invokeCmd<AppTask>("pause_task", { id }),
  resume: (id) => invokeCmd<AppTask>("resume_task", { id }),
  retry: (id) => invokeCmd<AppTask>("retry_task", { id }),
};

type RawEventsResponse =
  | unknown[]
  | {
      events?: unknown;
      nextAfterSequence?: unknown;
      next_after_sequence?: unknown;
      hasMore?: unknown;
      has_more?: unknown;
    };

function normalizeEventsPage(raw: RawEventsResponse, jobId: string, requestedLimit: number): TaskEventsPage {
  const response = Array.isArray(raw) ? { events: raw } : raw ?? {};
  const rawEvents = Array.isArray(response.events) ? response.events : [];
  const events = mergeTaskEvents([], rawEvents.map((event) => normalizeTaskEvent(event, jobId)));
  const explicitCursor = Number(response.nextAfterSequence ?? response.next_after_sequence);
  const lastSequence = events.at(-1)?.sequence ?? null;
  const nextAfterSequence = Number.isFinite(explicitCursor) && explicitCursor > 0
    ? Math.trunc(explicitCursor)
    : lastSequence;
  const explicitHasMore = response.hasMore ?? response.has_more;

  return {
    events,
    nextAfterSequence,
    hasMore: typeof explicitHasMore === "boolean" ? explicitHasMore : events.length >= requestedLimit,
  };
}

/**
 * Feature-local adapter for the Stage 3 detail commands. `get_task_detail` and
 * `get_task_events` are intentionally isolated here while command registration
 * is integrated; no arbitrary metadata is sent to or rendered by the drawer.
 */
export const tauriTaskDetailApi: TaskDetailApi = {
  async getTaskDetail(id) {
    return normalizeTaskDetail(await invokeCmd<unknown>("get_task_detail", { id }));
  },
  async getTaskEvents(id, query = {}) {
    const limit = Math.min(200, Math.max(1, Math.trunc(query.limit ?? 50)));
    const args: Record<string, unknown> = { id, limit };
    if (query.afterSequence !== undefined) args.afterSequence = query.afterSequence;
    const raw = await invokeCmd<RawEventsResponse>("get_task_events", args);
    return normalizeEventsPage(raw, id, limit);
  },
};

export interface StartPersistentDownloadOptions {
  url: string;
  filename: string;
  autoExtract?: boolean;
  autoImport?: boolean;
  idempotencyKey?: string;
  quotaBytes?: number;
}

export const persistentDownloadsApi = {
  start(options: StartPersistentDownloadOptions): Promise<DownloadTask> {
    const args: Record<string, unknown> = {
      url: options.url,
      filename: options.filename,
      autoExtract: options.autoExtract ?? false,
      autoImport: options.autoImport ?? false,
    };
    if (options.idempotencyKey) args.idempotencyKey = options.idempotencyKey;
    if (options.quotaBytes !== undefined) args.quotaBytes = options.quotaBytes;
    return invokeCmd<DownloadTask>("download_start", args);
  },
};
