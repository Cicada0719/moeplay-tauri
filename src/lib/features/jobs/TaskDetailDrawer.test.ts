import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { AppTask } from "../../api/types";
import type { JobsStore } from "./store";
import { normalizeJob, type TaskDetail, type TaskDetailApi, type TaskEventsPage } from "./contracts";
import TaskActivityBadge from "./TaskActivityBadge.svelte";
import TaskDetailDrawer from "./TaskDetailDrawer.svelte";

const rawTask: AppTask & { retryable: boolean } = {
  id: "task-1",
  title: "导入演示资源",
  kind: "import",
  status: "failed",
  progress: 0.4,
  created_at: "2026-07-11T08:00:00Z",
  updated_at: "2026-07-11T08:02:00Z",
  message: "Authorization: Bearer task-secret",
  retryable: true,
};

function storeStub(): JobsStore {
  return {
    getSnapshot: () => ({}) as ReturnType<JobsStore["getSnapshot"]>,
    subscribe: () => () => {},
    load: async () => {},
    refresh: async () => {},
    setFilter: async () => {},
    enqueue: async () => normalizeJob(rawTask),
    cancel: vi.fn(async () => {}),
    pause: vi.fn(async () => {}),
    resume: vi.fn(async () => {}),
    retry: vi.fn(async () => {}),
    clearFinished: async () => {},
  };
}

function detailApiWith(overrides: Partial<TaskDetailApi> = {}): TaskDetailApi {
  return {
    getTaskDetail: vi.fn(async (): Promise<TaskDetail> => ({
      job: normalizeJob(rawTask),
      operation: {
        kind: "import" as const,
        fields: [
          { label: "来源", value: "https://user:password@example.invalid/library" },
          { label: "引用 ID", value: "fixture-reference" },
        ],
      },
    })),
    getTaskEvents: vi.fn(async (_id, query): Promise<TaskEventsPage> => {
      if (query?.afterSequence === 2) {
        return {
          events: [{ jobId: "task-1", sequence: 3, level: "error" as const, code: "remote_error", message: "token=timeline-secret", createdAt: "2026-07-11T08:03:00Z" }],
          nextAfterSequence: 3,
          hasMore: false,
        };
      }
      return {
        events: [
          { jobId: "task-1", sequence: 1, level: "info" as const, code: "job_running", message: "下载 https://user:password@example.invalid/file", createdAt: "2026-07-11T08:01:00Z" },
          { jobId: "task-1", sequence: 2, level: "warn" as const, code: "remote_retry", message: "Authorization: Bearer timeline-secret", createdAt: "2026-07-11T08:02:00Z" },
        ],
        nextAfterSequence: 2,
        hasMore: true,
      };
    }),
    ...overrides,
  };
}

describe("Task Detail Drawer", () => {
  it("renders safe details, filters events, and appends the next keyset page without clearing current events", async () => {
    const api = detailApiWith();
    render(TaskDetailDrawer, {
      props: {
        open: true,
        job: normalizeJob(rawTask),
        store: storeStub(),
        detailApi: api,
        onClose: vi.fn(),
        pageSize: 2,
      },
    });

    await screen.findByRole("list", { name: "任务事件时间线" });
    expect(screen.getAllByText("[已隐藏链接]", { exact: false }).length).toBeGreaterThan(0);
    expect(screen.queryByText(/password|timeline-secret|task-secret|example\.invalid/)).not.toBeInTheDocument();
    expect(screen.getByText("引用 ID")).toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "警告" }));
    expect(screen.getByText("remote_retry")).toBeInTheDocument();
    expect(screen.queryByText("job_running")).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "全部" }));
    await fireEvent.click(screen.getByRole("button", { name: "加载后续事件" }));
    await waitFor(() => expect(screen.getByText("remote_error")).toBeInTheDocument());
    expect(screen.getByText("job_running")).toBeInTheDocument();
    expect(api.getTaskEvents).toHaveBeenLastCalledWith("task-1", { afterSequence: 2, limit: 2 });
  });

  it("shows loading, empty, and error states without exposing backend error secrets", async () => {
    let rejectEvents!: (reason: Error) => void;
    const delayedEvents = new Promise<never>((_resolve, reject) => { rejectEvents = reject; });
    const api = detailApiWith({
      getTaskEvents: vi.fn(() => delayedEvents),
    });
    render(TaskDetailDrawer, {
      props: { open: true, job: normalizeJob(rawTask), store: storeStub(), detailApi: api, onClose: vi.fn() },
    });
    expect(screen.getByRole("status")).toHaveTextContent("正在加载任务事件");
    rejectEvents(new Error("Authorization: Bearer backend-secret"));
    await waitFor(() => expect(screen.getByRole("alert")).toBeInTheDocument());
    expect(screen.getByRole("alert")).not.toHaveTextContent("backend-secret");

    const empty = detailApiWith({
      getTaskEvents: vi.fn(async () => ({ events: [], nextAfterSequence: null, hasMore: false })),
    });
    render(TaskDetailDrawer, {
      props: { open: true, job: normalizeJob({ ...rawTask, id: "task-2" }), store: storeStub(), detailApi: empty, onClose: vi.fn() },
    });
    await screen.findByTestId("task-events-empty");
    expect(screen.getAllByTestId("task-events-empty").at(-1)).toHaveTextContent("暂无任务事件");
  });

  it("exposes a compact, polite active/failed badge for the coordinator to mount", () => {
    render(TaskActivityBadge, { props: { activeCount: 3, failedCount: 1 } });
    expect(screen.getByTestId("task-activity-badge")).toHaveAccessibleName("后台任务状态：进行中 3，失败 1");
  });
});
