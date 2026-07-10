import { describe, expect, it } from "vitest";
import { createJobsStore, jobPercent } from "./store";
import { normalizeJob } from "./contracts";
import type { AppTask } from "../../api/types";

const task = (overrides: Partial<AppTask> = {}): AppTask => ({
  id: "job-1",
  title: "Job",
  kind: "test",
  status: "running",
  progress: 0.25,
  created_at: "2026-07-10T00:00:00Z",
  updated_at: "2026-07-10T00:00:01Z",
  ...overrides,
});

describe("jobs feature", () => {
  it("adapts legacy status and percentage responses", () => {
    const job = normalizeJob(task({ status: "pending", progress: 50 }));
    expect(job.status).toBe("queued");
    expect(job.progress).toBe(0.5);
    expect(jobPercent(job)).toBe(50);
  });

  it("groups all persistent job states", async () => {
    const api = {
      list: async () => [
        task({ id: "queued", status: "queued" }),
        task({ id: "paused", status: "paused" }),
        task({ id: "done", status: "completed" }),
        task({ id: "cancelled", status: "cancelled" }),
      ],
      cancel: async (id: string) => task({ id, status: "cancelled" }),
      clearFinished: async () => undefined,
      enqueue: async (title: string, kind: string) => task({ title, kind }),
    };
    const store = createJobsStore(api);
    await store.load();
    const snapshot = store.getSnapshot();
    expect(snapshot.byStatus.queued).toHaveLength(1);
    expect(snapshot.byStatus.paused).toHaveLength(1);
    expect(snapshot.byStatus.succeeded).toHaveLength(1);
    expect(snapshot.byStatus.cancelled).toHaveLength(1);
  });

  it("does not let an older refresh overwrite a newer refresh", async () => {
    let resolveOld!: (tasks: AppTask[]) => void;
    let resolveNew!: (tasks: AppTask[]) => void;
    const api = {
      list: () => new Promise<AppTask[]>((resolve) => {
        if (!resolveOld) resolveOld = resolve;
        else resolveNew = resolve;
      }),
      cancel: async (id: string) => task({ id, status: "cancelled" }),
      clearFinished: async () => undefined,
      enqueue: async (title: string, kind: string) => task({ title, kind }),
    };
    const store = createJobsStore(api);
    const oldLoad = store.load();
    const newLoad = store.load();
    resolveNew([task({ id: "new", title: "new" })]);
    await newLoad;
    resolveOld([task({ id: "old", title: "old" })]);
    await oldLoad;
    expect(store.getSnapshot().jobs.map((job) => job.id)).toEqual(["new"]);
  });

  it("forwards an idempotency key when enqueueing", async () => {
    let receivedKey: string | undefined;
    const api = {
      list: async () => [],
      enqueue: async (title: string, kind: string, idempotencyKey?: string) => {
        receivedKey = idempotencyKey;
        return task({ title, kind, status: "queued" });
      },
      cancel: async (id: string) => task({ id, status: "cancelled" }),
      clearFinished: async () => undefined,
    };
    const store = createJobsStore(api);
    await store.enqueue("Once", "import", "import-42");
    expect(receivedKey).toBe("import-42");
    expect(store.getSnapshot().byStatus.queued).toHaveLength(1);
  });

  it("keeps one projection when an idempotent enqueue reuses an existing job", async () => {
    const reused = task({ id: "same", status: "queued" });
    const api = {
      list: async () => [reused],
      enqueue: async () => reused,
      cancel: async (id: string) => task({ id, status: "cancelled" }),
      clearFinished: async () => undefined,
    };
    const store = createJobsStore(api);
    await store.load();
    await store.enqueue("same", "download", "download-key");
    expect(store.getSnapshot().jobs.map((job) => job.id)).toEqual(["same"]);
  });

  it("exposes recovered resumable state and refreshes after resume", async () => {
    let resumed = false;
    const recovered = {
      ...task({ id: "recovered", status: "paused", message: "已从上次运行恢复" }),
      recovered: true,
      resumable: true,
      retryable: true,
    } as AppTask & { recovered: boolean; resumable: boolean; retryable: boolean };
    const api = {
      list: async () => [resumed ? task({ id: "recovered", status: "running" }) : recovered],
      enqueue: async (title: string, kind: string) => task({ title, kind }),
      cancel: async (id: string) => task({ id, status: "cancelled" }),
      clearFinished: async () => undefined,
      resume: async () => { resumed = true; },
    };
    const store = createJobsStore(api);
    await store.load();
    expect(store.getSnapshot().byStatus.paused[0].recovered).toBe(true);
    await store.resume("recovered");
    expect(store.getSnapshot().byStatus.running).toHaveLength(1);
  });

  it("updates cancellation in the local projection", async () => {
    const api = {
      list: async () => [task()],
      cancel: async (id: string) => task({ id, status: "cancelled", message: "已取消" }),
      clearFinished: async () => undefined,
      enqueue: async (title: string, kind: string) => task({ title, kind }),
    };
    const store = createJobsStore(api);
    await store.load();
    await store.cancel("job-1");
    expect(store.getSnapshot().byStatus.cancelled[0].message).toBe("已取消");
  });
});


