import { invokeCmd } from "../../api/core";
import type { AppTask, DownloadTask } from "../../api/types";
import type { JobsApi, JobsFilter } from "./contracts";

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
