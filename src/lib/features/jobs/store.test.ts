import { describe, expect, it, vi } from "vitest";
import type { AppTask } from "../../api/types";
import { normalizeJob } from "./contracts";
import type { JobsApi } from "./contracts";
import { createJobsStore, jobPercent, sortJobs } from "./store";

const task = (overrides: Partial<AppTask> = {}): AppTask => ({
  id: "job-1",
  title: "Job",
  kind: "download",
  status: "running",
  progress: 0.25,
  created_at: "2026-07-10T00:00:00Z",
  updated_at: "2026-07-10T00:00:01Z",
  ...overrides,
});

const apiWith = (overrides: Partial<JobsApi> = {}): JobsApi => ({
  list: async () => [],
  cancel: async (id) => task({ id, status: "cancelled" }),
  clearFinished: async () => undefined,
  enqueue: async (title, kind) => task({ title, kind, status: "queued" }),
  pause: async (id) => task({ id, status: "paused" }),
  resume: async (id) => task({ id, status: "running" }),
  retry: async (id) => task({ id, status: "queued", progress: 0 }),
  ...overrides,
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("jobs Task Center projection", () => {
  it("adapts legacy values and safely downgrades unknown enum members", () => {
    const legacy = normalizeJob(task({ status: "pending", progress: 50 }));
    expect(legacy.status).toBe("queued");
    expect(legacy.progress).toBe(0.5);
    expect(jobPercent(legacy)).toBe(50);

    const unknown = normalizeJob({
      ...task(),
      kind: "future_kind",
      status: "future_status",
    } as unknown as AppTask);
    expect(unknown).toMatchObject({
      kind: "generic",
      status: "failed",
      rawKind: "future_kind",
      rawStatus: "future_status",
      retryable: false,
      resumable: false,
      cancellable: false,
    });
  });

  it("maps canonical fields, capabilities, source and legacy timestamp aliases", () => {
    const job = normalizeJob({
      ...task({ status: "paused" }),
      recovered: true,
      retryable: true,
      resumable: true,
      cancellable: true,
      error_kind: "network",
      idempotency_key: "download:1",
      source: { area: "library", entity_id: "game-1", label: "Game" },
    } as unknown as AppTask);
    expect(job).toMatchObject({
      recovered: true,
      retryable: true,
      resumable: true,
      cancellable: true,
      errorKind: "network",
      idempotencyKey: "download:1",
      source: { area: "library", entityId: "game-1", label: "Game" },
      createdAt: job.created_at,
      updatedAt: job.updated_at,
    });
  });

  it("groups states, exposes counts, and sorts deterministically", async () => {
    const store = createJobsStore(apiWith({
      list: async () => [
        task({ id: "b", status: "queued", updated_at: "2026-07-10T00:00:02Z" }),
        task({ id: "a", status: "paused", updated_at: "2026-07-10T00:00:02Z" }),
        task({ id: "done", status: "completed" }),
        task({ id: "failed", status: "failed" }),
        task({ id: "cancelled", status: "cancelled" }),
      ],
    }));
    await store.load();
    const snapshot = store.getSnapshot();
    expect(snapshot.jobs.map((job) => job.id).slice(0, 2)).toEqual(["a", "b"]);
    expect(snapshot.byStatus.queued).toHaveLength(1);
    expect(snapshot.byStatus.paused).toHaveLength(1);
    expect(snapshot.byStatus.succeeded).toHaveLength(1);
    expect(snapshot.counts).toEqual({ total: 5, active: 2, failed: 1, completed: 2 });
  });

  it("supports immediate client filtering and forwards filters on refresh", async () => {
    const list = vi.fn(async () => [
      task({ id: "failed", kind: "ai", status: "failed" }),
    ]);
    const store = createJobsStore(apiWith({ list }));
    await store.load();
    await store.setFilter({ status: "failed", kind: "ai", limit: 5 }, { refresh: false });
    expect(store.getSnapshot().jobs.map((job) => job.id)).toEqual(["failed"]);
    await store.refresh();
    expect(list).toHaveBeenLastCalledWith({ status: "failed", kind: "ai", limit: 5 });
  });

  it("does not let an older refresh overwrite a newer refresh", async () => {
    const oldRequest = deferred<AppTask[]>();
    const newRequest = deferred<AppTask[]>();
    const list = vi.fn()
      .mockImplementationOnce(() => oldRequest.promise)
      .mockImplementationOnce(() => newRequest.promise);
    const store = createJobsStore(apiWith({ list }));
    const oldLoad = store.load();
    const newLoad = store.load();
    newRequest.resolve([task({ id: "new", title: "new" })]);
    await newLoad;
    oldRequest.resolve([task({ id: "old", title: "old" })]);
    await oldLoad;
    expect(store.getSnapshot().jobs.map((job) => job.id)).toEqual(["new"]);
  });

  it("keeps existing data visible when a background refresh fails", async () => {
    const list = vi.fn()
      .mockResolvedValueOnce([task({ id: "kept" })])
      .mockRejectedValueOnce(new Error("offline"));
    const store = createJobsStore(apiWith({ list }));
    await store.load();
    await store.refresh();
    expect(store.getSnapshot()).toMatchObject({
      error: "offline",
      refreshing: false,
    });
    expect(store.getSnapshot().jobs.map((job) => job.id)).toEqual(["kept"]);
  });

  it("forwards an idempotency key and de-duplicates a reused job", async () => {
    const reused = task({ id: "same", status: "queued" });
    const enqueue = vi.fn(async () => reused);
    const store = createJobsStore(apiWith({ list: async () => [reused], enqueue }));
    await store.load();
    await store.enqueue("same", "download", "download-key");
    expect(enqueue).toHaveBeenCalledWith("same", "download", "download-key");
    expect(store.getSnapshot().jobs.map((job) => job.id)).toEqual(["same"]);
  });

  it("optimistically applies actions and rolls back a failed mutation", async () => {
    const pauseRequest = deferred<AppTask | void>();
    const store = createJobsStore(apiWith({
      list: async () => [task({ id: "active" })],
      pause: () => pauseRequest.promise,
    }));
    await store.load();
    const pause = store.pause("active");
    expect(store.getSnapshot().byStatus.paused.map((job) => job.id)).toEqual(["active"]);
    pauseRequest.reject(new Error("pause denied"));
    await pause;
    expect(store.getSnapshot().byStatus.running.map((job) => job.id)).toEqual(["active"]);
    expect(store.getSnapshot().error).toBe("pause denied");
  });

  it("does not allow an in-flight old list response to erase an optimistic action", async () => {
    const oldRefresh = deferred<AppTask[]>();
    const pauseRequest = deferred<AppTask | void>();
    const list = vi.fn()
      .mockResolvedValueOnce([task({ id: "active" })])
      .mockImplementationOnce(() => oldRefresh.promise);
    const store = createJobsStore(apiWith({ list, pause: () => pauseRequest.promise }));
    await store.load();
    const refresh = store.refresh();
    const pause = store.pause("active");
    oldRefresh.resolve([task({ id: "active", status: "running" })]);
    await refresh;
    expect(store.getSnapshot().byStatus.paused).toHaveLength(1);
    pauseRequest.resolve(task({ id: "active", status: "paused" }));
    await pause;
    expect(store.getSnapshot().byStatus.paused).toHaveLength(1);
  });

  it("invalidates an older refresh even when it resolves after an action succeeds", async () => {
    const oldRefresh = deferred<AppTask[]>();
    const list = vi.fn()
      .mockResolvedValueOnce([task({ id: "active" })])
      .mockImplementationOnce(() => oldRefresh.promise);
    const store = createJobsStore(apiWith({ list }));
    await store.load();
    const refresh = store.refresh();
    await store.pause("active");
    oldRefresh.resolve([task({ id: "active", status: "running" })]);
    await refresh;
    expect(store.getSnapshot().byStatus.paused).toHaveLength(1);
    expect(store.getSnapshot().refreshing).toBe(false);
  });

  it("optimistically clears terminal jobs and restores them on failure", async () => {
    const clearRequest = deferred<void>();
    const store = createJobsStore(apiWith({
      list: async () => [task({ id: "active" }), task({ id: "done", status: "succeeded" })],
      clearFinished: () => clearRequest.promise,
    }));
    await store.load();
    const clear = store.clearFinished();
    expect(store.getSnapshot().jobs.map((job) => job.id)).toEqual(["active"]);
    clearRequest.reject(new Error("clear failed"));
    await clear;
    expect(store.getSnapshot().jobs.map((job) => job.id).sort()).toEqual(["active", "done"]);
    expect(store.getSnapshot().error).toBe("clear failed");
  });

  it("refreshes after a void resume response and exposes recovered state", async () => {
    let resumed = false;
    const recovered = {
      ...task({ id: "recovered", status: "paused" }),
      recovered: true,
      resumable: true,
    } as unknown as AppTask;
    const store = createJobsStore(apiWith({
      list: async () => [resumed ? task({ id: "recovered", status: "running" }) : recovered],
      resume: async () => { resumed = true; },
    }));
    await store.load();
    expect(store.getSnapshot().byStatus.paused[0].recovered).toBe(true);
    await store.resume("recovered");
    expect(store.getSnapshot().byStatus.running).toHaveLength(1);
  });

  it("exports stable sorting for equal timestamps", () => {
    const jobs = [normalizeJob(task({ id: "z" })), normalizeJob(task({ id: "a" }))];
    expect(sortJobs(jobs).map((job) => job.id)).toEqual(["a", "z"]);
  });
});
