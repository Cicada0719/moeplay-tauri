import { defineConfig, devices } from "@playwright/test";

const matrixTestMatch = /(?:accessibility-infra|visual|responsive|matrix)\.spec\.ts/;
const requestedChannel = process.env.PLAYWRIGHT_CHANNEL?.trim();
const browserChannel = requestedChannel === "bundled" ? undefined : (requestedChannel || "chrome");

const commonUse = {
  ...devices["Desktop Chrome"],
  baseURL: "http://localhost:1420",
  ...(browserChannel ? { channel: browserChannel } : {}),
  colorScheme: "dark" as const,
  locale: "zh-CN",
  timezoneId: "Asia/Shanghai",
  trace: "on-first-retry" as const,
};

/**
 * 萌游 MoeGame · 确定性视觉/E2E 测试配置
 *
 * desktop-standard 承载既有功能冒烟；其余 viewport 只运行显式的
 * visual/responsive/matrix/accessibility-infra 用例，避免把真实来源与媒体流程
 * 在 4K 等矩阵项目中重复执行。
 */
export default defineConfig({
  testDir: "tests/visual",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "list",
  expect: {
    toHaveScreenshot: {
      animations: "disabled",
      caret: "hide",
      scale: "css",
      maxDiffPixelRatio: 0.01,
    },
  },
  use: commonUse,
  projects: [
    {
      name: "desktop-standard",
      use: { viewport: { width: 1440, height: 900 } },
    },
    {
      name: "desktop-compact",
      testMatch: matrixTestMatch,
      use: { viewport: { width: 960, height: 640 } },
    },
    {
      name: "desktop-narrow",
      testMatch: matrixTestMatch,
      use: { viewport: { width: 720, height: 600 } },
    },
    {
      name: "couch-1080p",
      testMatch: matrixTestMatch,
      use: { viewport: { width: 1920, height: 1080 } },
    },
    {
      name: "couch-4k",
      testMatch: matrixTestMatch,
      use: { viewport: { width: 3840, height: 2160 } },
    },
    {
      name: "low-height",
      testMatch: matrixTestMatch,
      use: { viewport: { width: 1280, height: 720 } },
    },
    {
      name: "reduced-motion",
      testMatch: matrixTestMatch,
      use: {
        viewport: { width: 1440, height: 900 },
        reducedMotion: "reduce",
      },
    },
  ],
  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
  },
});
