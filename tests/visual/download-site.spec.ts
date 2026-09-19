import { test, expect } from "@playwright/test";
import { createServer, type Server } from "node:http";
import { readFile } from "node:fs/promises";
import path from "node:path";
let server: Server;
let baseURL: string;
const root = path.resolve("update-server/site");
test.beforeAll(async () => {
  server = createServer(async (req, res) => {
    const pathname = decodeURIComponent(new URL(req.url ?? "/", "http://localhost").pathname);
    const file = path.resolve(root, `.${pathname.endsWith("/") ? pathname + "index.html" : pathname}`);
    if (!file.startsWith(root + path.sep)) { res.writeHead(403).end(); return; }
    const mime = { ".html": "text/html; charset=utf-8", ".css": "text/css", ".js": "text/javascript", ".svg": "image/svg+xml", ".png": "image/png" };
    try { const bytes = await readFile(file); res.writeHead(200, { "Content-Type": mime[path.extname(file) as keyof typeof mime] ?? "application/octet-stream" }); res.end(bytes); }
    catch { res.writeHead(404).end(); }
  });
  await new Promise<void>(resolve => server.listen(0, "127.0.0.1", resolve));
  const address = server.address(); if (!address || typeof address === "string") throw new Error("No test server");
  baseURL = `http://127.0.0.1:${address.port}`;
});
test.afterAll(async () => { await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve())); });

test("download page renders verified manifest channels, bundled art and mobile layout", async ({ page }, testInfo) => {
  const external: string[] = [];
  page.on("request", req => { if (!req.url().startsWith(baseURL)) external.push(req.url()); });
  await page.route("**/release-manifest.json", route => route.fulfill({ json: { schemaVersion: 1, version: "0.23.0", commit: "a".repeat(40), publishedAt: "2026-09-08T00:00:00Z", notes: ["阅读位置精确恢复"], androidCompatibilityVerified: false,
    assets: [ ["installer", "windows", "x64", "exe"], ["release", "android", "arm64", "apk"], ["compat", "android", "arm64", "apk"] ].map(([channel, platform, architecture, ext]) => ({ channel, platform, architecture, file: `MoeGame_0.23.0_${channel}.${ext}`, size: 40 * 1024 * 1024, sha256: "a".repeat(64) })) } }));
  await page.goto(baseURL);
  await expect(page.locator("#windows-download")).toHaveAttribute("href", "/downloads/0.23.0/MoeGame_0.23.0_installer.exe");
  await expect(page.locator("#compatibility")).toContainText("不承诺保留数据升级");
  await expect(page.locator(".asset")).toHaveCount(3);
  await expect(page.locator("#release-status")).toContainText("v0.23.0");
  expect(external).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath("download-desktop.png"), fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await expect(page.locator("#android-download")).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath("download-mobile.png"), fullPage: true });
});
test("manifest failure retains working GitHub download links", async ({ page }) => {
  await page.goto(baseURL);
  await expect(page.locator("#release-status")).toContainText("GitHub 备用");
  await expect(page.locator("#windows-download")).toHaveAttribute("href", "https://github.com/Cicada0719/moeplay-tauri/releases");
});
