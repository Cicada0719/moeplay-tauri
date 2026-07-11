import { expect, test } from "./fixtures/app-fixture";

const TASK = {
  id: "task-observability-1",
  title: "导入演示资源",
  kind: "import",
  status: "failed",
  progress: 0.4,
  created_at: "2026-07-11T08:00:00Z",
  updated_at: "2026-07-11T08:02:00Z",
  message: "Authorization: Bearer task-list-secret",
  retryable: true,
  cancellable: false,
  resumable: false,
  pausable: false,
  source: { area: "library", entity_id: "fixture-reference", label: "演示库" },
  error_kind: "remote_timeout",
};

test.use({
  appState: {
    settings: {
      theme: "dark",
      watch_dirs: [],
      auto_scrape: true,
      language: "zh",
      startup_mode: "fullscreen",
    },
    games: [],
    commandResults: {
      get_tasks: [TASK],
      get_task_detail: {
        job: TASK,
        operation: {
          version: 1,
          operation: { Import: { source: "https://user:password@example.invalid/library", reference_id: "fixture-reference" } },
        },
      },
      get_task_events: [
        { job_id: TASK.id, sequence: 1, level: "info", code: "job_running", message: "请求 https://user:password@example.invalid/library", progress: 0.2, created_at: "2026-07-11T08:01:00Z" },
        { job_id: TASK.id, sequence: 2, level: "error", code: "remote_timeout", message: "Authorization: Bearer timeline-secret", created_at: "2026-07-11T08:02:00Z" },
      ],
    },
    localStorage: {
      "moegame-startup-migrated-v1": "1",
      "moeplay-theme": "dark",
    },
  },
});

test("task detail drawer filters redacted events and restores focus to its opener", async ({ appPage: page }) => {
  await page.getByRole("button", { name: "打开工具抽屉" }).click();
  await page.getByRole("button", { name: "打开任务中心" }).click();

  const opener = page.getByRole("button", { name: "查看 导入演示资源 详情" });
  await opener.focus();
  await opener.press("Enter");

  const drawer = page.getByRole("dialog", { name: "任务详情" });
  await expect(drawer).toBeVisible();
  await expect(drawer.getByRole("list", { name: "任务事件时间线" })).toBeVisible();
  await expect(drawer.getByText("[已隐藏链接]", { exact: true }).first()).toBeVisible();
  await expect(drawer.getByText("[已隐藏]", { exact: false }).first()).toBeVisible();
  await expect(drawer.getByText(/password|timeline-secret|task-list-secret|example\.invalid/)).toHaveCount(0);

  await drawer.getByRole("button", { name: "错误" }).click();
  await expect(drawer.getByRole("list", { name: "任务事件时间线" }).getByText("remote_timeout")).toBeVisible();
  await expect(drawer.getByRole("list", { name: "任务事件时间线" }).getByText("job_running")).toHaveCount(0);

  await page.keyboard.press("Escape");
  await expect(drawer).toBeHidden();
  await expect(opener).toBeFocused();
});
