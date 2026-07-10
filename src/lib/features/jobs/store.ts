import type { AppTask } from "../../api/types";
import { tauriJobsApi } from "./api";
import type { JobsApi, JobsSnapshot, Job } from "./contracts";
import { emptyBuckets, normalizeJob, normalizeJobProgress } from "./contracts";

export interface JobsStore {
  getSnapshot(): JobsSnapshot;
  subscribe(listener: (snapshot: JobsSnapshot) => void): () => void;
  load(): Promise<void>;
  enqueue(title: string, kind: string, idempotencyKey?: string): Promise<Job>;
  cancel(id: string): Promise<void>;
  pause(id: string): Promise<void>;
  resume(id: string): Promise<void>;
  retry(id: string): Promise<void>;
  clearFinished(): Promise<void>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function createJobsStore(api: JobsApi = tauriJobsApi): JobsStore {
  let snapshot: JobsSnapshot = {
    jobs: [],
    byStatus: emptyBuckets(),
    loading: false,
    error: null,
    lastLoadedAt: null,
  };
  const listeners = new Set<(state: JobsSnapshot) => void>();
  let requestGeneration = 0;

  const clone = (): JobsSnapshot => ({
    ...snapshot,
    jobs: [...snapshot.jobs],
    byStatus: Object.fromEntries(
      Object.entries(snapshot.byStatus).map(([status, jobs]) => [status, [...jobs]])
    ) as JobsSnapshot["byStatus"],
  });
  const publish = () => listeners.forEach((listener) => listener(clone()));
  const patch = (next: Partial<JobsSnapshot>) => {
    snapshot = { ...snapshot, ...next };
    publish();
  };
  const setJobs = (tasks: AppTask[]) => {
    const jobs = tasks.map(normalizeJob).sort((a, b) => b.updated_at.localeCompare(a.updated_at));
    const byStatus = emptyBuckets();
    jobs.forEach((job) => byStatus[job.status].push(job));
    patch({ jobs, byStatus, error: null, lastLoadedAt: Date.now() });
  };
  const runJobAction = async (
    id: string,
    action: JobsApi["pause"] | JobsApi["resume"] | JobsApi["retry"],
    label: string
  ) => {
    if (!action) {
      patch({ error: `当前后端不支持${label}任务` });
      return;
    }
    const generation = ++requestGeneration;
    try {
      await action(id);
      const tasks = await api.list();
      if (generation === requestGeneration) setJobs(tasks);
    } catch (error) {
      if (generation === requestGeneration) patch({ error: errorMessage(error) });
    }
  };

  return {
    getSnapshot: clone,
    subscribe(listener) {
      listeners.add(listener);
      listener(clone());
      return () => listeners.delete(listener);
    },
    async load() {
      const generation = ++requestGeneration;
      patch({ loading: true, error: null });
      try {
        const tasks = await api.list();
        if (generation !== requestGeneration) return;
        setJobs(tasks);
      } catch (error) {
        if (generation !== requestGeneration) return;
        patch({ error: errorMessage(error) });
      } finally {
        if (generation === requestGeneration) patch({ loading: false });
      }
    },
    async enqueue(title, kind, idempotencyKey) {
      const generation = ++requestGeneration;
      try {
        const task = normalizeJob(await api.enqueue(title, kind, idempotencyKey));
        if (generation === requestGeneration) {
          setJobs([...snapshot.jobs.filter((job) => job.id !== task.id), task]);
        }
        return task;
      } catch (error) {
        if (generation === requestGeneration) patch({ error: errorMessage(error) });
        throw error;
      }
    },
    async cancel(id) {
      const generation = ++requestGeneration;
      try {
        const task = await api.cancel(id);
        if (generation !== requestGeneration) return;
        const next = snapshot.jobs.map((job) => (job.id === id ? normalizeJob(task) : job));
        setJobs(next);
      } catch (error) {
        if (generation === requestGeneration) patch({ error: errorMessage(error) });
      }
    },
    async pause(id) {
      await runJobAction(id, api.pause, "暂停");
    },
    async resume(id) {
      await runJobAction(id, api.resume, "继续");
    },
    async retry(id) {
      await runJobAction(id, api.retry, "重试");
    },
    async clearFinished() {
      const generation = ++requestGeneration;
      try {
        await api.clearFinished();
        if (generation !== requestGeneration) return;
        setJobs(snapshot.jobs.filter((job) => !["succeeded", "failed", "cancelled"].includes(job.status)));
      } catch (error) {
        if (generation === requestGeneration) patch({ error: errorMessage(error) });
      }
    },
  };
}

export function jobPercent(job: Pick<Job, "progress">): number {
  return Math.round(normalizeJobProgress(job.progress) * 100);
}
