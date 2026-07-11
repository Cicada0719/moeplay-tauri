import { expect, test } from "./fixtures/app-fixture";

const TASKS = [
  {
    id: "job-running",
    title: "下载演示资源",
    kind: "download",
    status: "running",
    progress: 0.42,
    created_at: "2026-07-11T08:00:00Z",
    updated_at: "2026-07-11T08:02:00Z",
    message: "正在下载 https://example.invalid/private/file.zip",
    recovered: false,
    resumable: true,
    retryable: true,
  },
  {
    id: "job-failed",
    title: "验证番剧来源",
    kind: "provider_verify",
    status: "failed",
    progress: 0.1,
    created_at: "2026-07-11T07:00:00Z",
    updated_at: "2026-07-11T07:01:00Z",
    message: "source timeout",
    recovered: true,
    resumable: false,
    retryable: true,
  },
];

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
    commandResults: { get_tasks: TASKS },
    localStorage: {
      "moegame-startup-migrated-v1": "1",
      "moeplay-theme": "dark",
    },
  },
});

test("task center is globally navigable, filtered and redacts URLs", async ({ appPage: page }) => {
  await page.getByRole("button", { name: "打开工具抽屉" }).click();
  await page.getByRole("button", { name: "打开任务中心" }).click();

  await expect(page).toHaveURL(/#tasks$/);
  await expect(page.getByTestId("task-center-page")).toBeVisible();
  await expect(page.getByText("下载演示资源")).toBeVisible();
  await expect(page.getByText("验证番剧来源")).toBeVisible();
  await expect(page.getByText("[已隐藏链接]", { exact: false })).toBeVisible();
  await expect(page.getByText("example.invalid", { exact: false })).toHaveCount(0);

  await page.getByTestId("task-center-list").getByRole("button", { name: /失败 1/ }).click();
  await expect(page.getByText("验证番剧来源")).toBeVisible();
  await expect(page.getByText("下载演示资源")).toHaveCount(0);
  await expect(page.getByText("从上次会话恢复")).toBeVisible();
});
