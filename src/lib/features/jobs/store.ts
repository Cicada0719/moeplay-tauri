import type { AppTask } from "../../api/types";
import { tauriJobsApi } from "./api";
import type {
  Job,
  JobCounts,
  JobsApi,
  JobsFilter,
  JobsSnapshot,
  JobStatus,
} from "./contracts";
import { emptyBuckets, normalizeJob, normalizeJobProgress } from "./contracts";

export interface JobsStore {
  getSnapshot(): JobsSnapshot;
  subscribe(listener: (snapshot: JobsSnapshot) => void): () => void;
  load(filter?: JobsFilter): Promise<void>;
  refresh(): Promise<void>;
  setFilter(filter: JobsFilter, options?: { refresh?: boolean }): Promise<void>;
  enqueue(title: string, kind: string, idempotencyKey?: string): Promise<Job>;
  cancel(id: string): Promise<void>;
  pause(id: string): Promise<void>;
  resume(id: string): Promise<void>;
  retry(id: string): Promise<void>;
  clearFinished(): Promise<void>;
}

type MutableState = Omit<JobsSnapshot, "jobs" | "byStatus" | "counts">;
type ActionName = "pause" | "resume" | "retry" | "cancel";

const TERMINAL = new Set<JobStatus>(["succeeded", "failed", "cancelled"]);

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function normalizeFilter(filter: JobsFilter = {}): JobsFilter {
  const limit = filter.limit;
  return {
    status: filter.status ?? "all",
    kind: filter.kind ?? "all",
    ...(limit === undefined ? {} : { limit: Math.max(1, Math.trunc(limit)) }),
  };
}

function jobMatches(job: Job, filter: JobsFilter): boolean {
  return (
    (!filter.status || filter.status === "all" || job.status === filter.status) &&
    (!filter.kind || filter.kind === "all" || job.kind === filter.kind)
  );
}

export function sortJobs(jobs: readonly Job[]): Job[] {
  const timestamp = (value: string) => {
    const parsed = Date.parse(value);
    return Number.isFinite(parsed) ? parsed : 0;
  };
  return [...jobs].sort((a, b) => {
    const updated = timestamp(b.updatedAt) - timestamp(a.updatedAt);
    if (updated !== 0) return updated;
    const created = timestamp(b.createdAt) - timestamp(a.createdAt);
    if (created !== 0) return created;
    return a.id.localeCompare(b.id);
  });
}

function countsFor(jobs: readonly Job[]): JobCounts {
  return jobs.reduce<JobCounts>(
    (counts, job) => {
      counts.total += 1;
      if (job.status === "queued" || job.status === "running" || job.status === "paused") counts.active += 1;
      if (job.status === "failed") counts.failed += 1;
      if (job.status === "succeeded" || job.status === "cancelled") counts.completed += 1;
      return counts;
    },
    { total: 0, active: 0, failed: 0, completed: 0 }
  );
}

function project(allJobs: readonly Job[], state: MutableState): JobsSnapshot {
  const sortedAll = sortJobs(allJobs);
  const matching = sortedAll.filter((job) => jobMatches(job, state.filter));
  const jobs = state.filter.limit === undefined ? matching : matching.slice(0, state.filter.limit);
  const byStatus = emptyBuckets();
  for (const job of jobs) byStatus[job.status].push(job);
  return {
    ...state,
    allJobs: sortedAll,
    jobs,
    byStatus,
    counts: countsFor(sortedAll),
  };
}

function optimisticJob(job: Job, action: ActionName): Job {
  const updatedAt = new Date().toISOString();
  if (action === "pause") {
    return { ...job, status: "paused", pausable: false, resumable: true, cancellable: true, updatedAt, updated_at: updatedAt };
  }
  if (action === "resume") {
    return { ...job, status: "running", pausable: job.kind === "download", resumable: false, cancellable: true, updatedAt, updated_at: updatedAt };
  }
  if (action === "retry") {
    return {
      ...job,
      status: "queued",
      progress: 0,
      message: undefined,
      errorKind: undefined,
      retryable: false,
      resumable: false,
      cancellable: true,
      recovered: false,
      updatedAt,
      updated_at: updatedAt,
    };
  }
  return {
    ...job,
    status: "cancelled",
    cancellable: false,
    pausable: false,
    resumable: false,
    updatedAt,
    updated_at: updatedAt,
  };
}

function canRun(job: Job, action: ActionName): boolean {
  if (action === "pause") return job.pausable;
  if (action === "resume") return job.resumable;
  if (action === "retry") return job.retryable;
  return job.cancellable;
}

export function createJobsStore(api: JobsApi = tauriJobsApi): JobsStore {
  let state: MutableState = {
    allJobs: [],
    filter: normalizeFilter(),
    loading: false,
    refreshing: false,
    error: null,
    lastLoadedAt: null,
  };
  let snapshot = project([], state);
  const listeners = new Set<(value: JobsSnapshot) => void>();
  const actionTokens = new Map<string, symbol>();
  const pendingOptimistic = new Map<string, Job>();
  let clearToken: symbol | null = null;
  let clearTombstones = new Set<string>();
  let requestGeneration = 0;

  const clone = (): JobsSnapshot => ({
    ...snapshot,
    filter: { ...snapshot.filter },
    jobs: [...snapshot.jobs],
    allJobs: [...snapshot.allJobs],
    byStatus: Object.fromEntries(
      Object.entries(snapshot.byStatus).map(([status, jobs]) => [status, [...jobs]])
    ) as JobsSnapshot["byStatus"],
    counts: { ...snapshot.counts },
  });

  const publish = () => listeners.forEach((listener) => listener(clone()));

  const commit = (next: Partial<MutableState> = {}, jobs: readonly Job[] = state.allJobs) => {
    state = { ...state, ...next, allJobs: sortJobs(jobs) };
    snapshot = project(state.allJobs, state);
    publish();
  };

  const replaceOne = (job: Job) => {
    commit({}, [...state.allJobs.filter((current) => current.id !== job.id), job]);
  };

  const applyServerTasks = (tasks: AppTask[]) => {
    const serverJobs = tasks.map(normalizeJob);
    const serverById = new Map(serverJobs.map((job) => [job.id, job]));
    for (const [id, optimistic] of pendingOptimistic) serverById.set(id, optimistic);
    for (const id of clearTombstones) serverById.delete(id);
    commit(
      { error: null, lastLoadedAt: Date.now() },
      [...serverById.values()]
    );
  };

  const fetchJobs = async (initial: boolean) => {
    const generation = ++requestGeneration;
    const filter = { ...state.filter };
    commit(initial ? { loading: true, error: null } : { refreshing: true, error: null });
    try {
      const tasks = await api.list(filter);
      if (generation !== requestGeneration) return;
      applyServerTasks(tasks);
    } catch (error) {
      if (generation !== requestGeneration) return;
      commit({ error: errorMessage(error) });
    } finally {
      if (generation === requestGeneration) {
        commit(initial ? { loading: false } : { refreshing: false });
      }
    }
  };

  const runAction = async (id: string, action: ActionName) => {
    const method = api[action];
    const current = state.allJobs.find((job) => job.id === id);
    if (!method) {
      commit({ error: `当前后端不支持${action}任务` });
      return;
    }
    if (!current) {
      commit({ error: `任务不存在：${id}` });
      return;
    }
    if (!canRun(current, action)) {
      commit({ error: `任务当前不可执行 ${action} 操作` });
      return;
    }

    const token = Symbol(action);
    const rollback = current;
    const optimistic = optimisticJob(current, action);
    requestGeneration += 1;
    actionTokens.set(id, token);
    pendingOptimistic.set(id, optimistic);
    replaceOne(optimistic);
    commit({ error: null, loading: false, refreshing: false });

    try {
      const result = await method(id);
      if (actionTokens.get(id) !== token) return;
      actionTokens.delete(id);
      if (result) {
        pendingOptimistic.delete(id);
        replaceOne(normalizeJob(result));
      } else {
        await fetchJobs(false);
        pendingOptimistic.delete(id);
      }
    } catch (error) {
      if (actionTokens.get(id) !== token) return;
      pendingOptimistic.delete(id);
      actionTokens.delete(id);
      replaceOne(rollback);
      commit({ error: errorMessage(error) });
    }
  };

  return {
    getSnapshot: clone,
    subscribe(listener) {
      listeners.add(listener);
      listener(clone());
      return () => listeners.delete(listener);
    },
    async load(filter) {
      if (filter) state = { ...state, filter: normalizeFilter(filter) };
      await fetchJobs(state.allJobs.length === 0);
    },
    async refresh() {
      await fetchJobs(false);
    },
    async setFilter(filter, options = {}) {
      commit({ filter: normalizeFilter(filter) });
      if (options.refresh !== false) await fetchJobs(false);
    },
    async enqueue(title, kind, idempotencyKey) {
      requestGeneration += 1;
      commit({ loading: false, refreshing: false });
      try {
        const task = normalizeJob(await api.enqueue(title, kind, idempotencyKey));
        replaceOne(task);
        commit({ error: null });
        return task;
      } catch (error) {
        commit({ error: errorMessage(error) });
        throw error;
      }
    },
    async cancel(id) {
      await runAction(id, "cancel");
    },
    async pause(id) {
      await runAction(id, "pause");
    },
    async resume(id) {
      await runAction(id, "resume");
    },
    async retry(id) {
      await runAction(id, "retry");
    },
    async clearFinished() {
      const token = Symbol("clearFinished");
      requestGeneration += 1;
      const removed = state.allJobs.filter((job) => TERMINAL.has(job.status));
      clearToken = token;
      clearTombstones = new Set(removed.map((job) => job.id));
      commit({ error: null, loading: false, refreshing: false }, state.allJobs.filter((job) => !clearTombstones.has(job.id)));
      try {
        await api.clearFinished();
        if (clearToken !== token) return;
        clearToken = null;
        clearTombstones = new Set();
      } catch (error) {
        if (clearToken !== token) return;
        clearToken = null;
        clearTombstones = new Set();
        const currentIds = new Set(state.allJobs.map((job) => job.id));
        commit(
          { error: errorMessage(error) },
          [...state.allJobs, ...removed.filter((job) => !currentIds.has(job.id))]
        );
      }
    },
  };
}

export function jobPercent(job: Pick<Job, "progress">): number {
  return Math.round(normalizeJobProgress(job.progress) * 100);
}
