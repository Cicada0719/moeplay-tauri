import { cancelTask, clearFinishedTasks, getTasks } from "../../api";
import { invokeCmd } from "../../api/core";
import type { AppTask, DownloadTask } from "../../api/types";
import type { JobsApi } from "./contracts";

/** Feature-local DTO bridge so the legacy command can carry a durable key. */
export const tauriJobsApi: JobsApi = {
  list: getTasks,
  cancel: cancelTask,
  clearFinished: clearFinishedTasks,
  enqueue: (title, kind, idempotencyKey) => {
    const args: Record<string, unknown> = { title, kind };
    if (idempotencyKey) args.idempotencyKey = idempotencyKey;
    return invokeCmd<AppTask>("enqueue_task", args);
  },
  pause: (id) => invokeCmd("download_pause", { taskId: id }),
  resume: (id) => invokeCmd("download_resume", { taskId: id }),
  retry: (id) => invokeCmd("download_retry", { taskId: id }),
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
