import { defineConfig, devices } from "@playwright/test";

/**
 * 萌游 MoeGame · 视觉/E2E 测试配置
 *
 * 使用本机已安装的 Google Chrome，避免在 CI/本地重复下载 Chromium。
 */
export default defineConfig({
  testDir: "tests/visual",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "list",
  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    channel: "chrome",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
  },
});
