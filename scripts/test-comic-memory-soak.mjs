#!/usr/bin/env node

import { chromium } from "@playwright/test";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

const ONE_MB = 1024 * 1024;
const DEFAULTS = Object.freeze({
  pages: 180,
  cycles: 4,
  stepPages: 3,
  settleMs: 250,
  maxInitialRequests: 18,
  maxJsHeapGrowthMb: 24,
  maxRssGrowthMb: 160,
  maxPeakRssDeltaMb: 512,
  maxDomNodeDrift: 500,
  browserChannel: "chrome",
  headed: false,
});

function usage() {
  console.log(`Usage: node scripts/test-comic-memory-soak.mjs [options]\n\n` +
    `Deterministically opens a ${DEFAULTS.pages}-page mocked Baozi chapter in the real ComicReader,\n` +
    `walks the prefetch/lazy-load path, repeatedly closes/reopens it, and checks memory plateaus.\n\n` +
    `Options:\n` +
    `  --pages <n>                    Chapter page count (default: ${DEFAULTS.pages})\n` +
    `  --cycles <n>                   Open/scroll/close cycles (default: ${DEFAULTS.cycles})\n` +
    `  --step-pages <n>               Pages advanced per scroll step (default: ${DEFAULTS.stepPages})\n` +
    `  --settle-ms <n>                Settle delay at checkpoints (default: ${DEFAULTS.settleMs})\n` +
    `  --max-initial-requests <n>      Initial image request ceiling (default: ${DEFAULTS.maxInitialRequests})\n` +
    `  --max-js-growth-mb <n>          Closed-state JS heap growth ceiling (default: ${DEFAULTS.maxJsHeapGrowthMb})\n` +
    `  --max-rss-growth-mb <n>         Closed-state browser-tree RSS growth ceiling (default: ${DEFAULTS.maxRssGrowthMb})\n` +
    `  --max-peak-rss-delta-mb <n>     Peak RSS above pre-reader baseline (default: ${DEFAULTS.maxPeakRssDeltaMb})\n` +
    `  --max-dom-node-drift <n>        Closed-state DOM node drift ceiling (default: ${DEFAULTS.maxDomNodeDrift})\n` +
    `  --base-url <url>                Reuse an existing Vite server instead of starting one\n` +
    `  --browser-channel <name>        Playwright browser channel (default: ${DEFAULTS.browserChannel})\n` +
    `  --headed                        Show the browser window\n` +
    `  --json <path>                   Write the full JSON report\n` +
    `  --markdown <path>               Write a Markdown summary\n` +
    `  --help                          Show this help\n`);
}

function parsePositiveInt(name, value, minimum = 1) {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < minimum) throw new Error(`${name} must be an integer >= ${minimum}`);
  return parsed;
}

function parsePositiveNumber(name, value, minimum = 0) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < minimum) throw new Error(`${name} must be a number >= ${minimum}`);
  return parsed;
}

function parseArgs(argv) {
  const options = { ...DEFAULTS, baseUrl: null, jsonPath: null, markdownPath: null };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = () => {
      const value = argv[++index];
      if (value === undefined) throw new Error(`Missing value for ${arg}`);
      return value;
    };
    if (arg === "--help" || arg === "-h") options.help = true;
    else if (arg === "--pages") options.pages = parsePositiveInt(arg, next(), 12);
    else if (arg === "--cycles") options.cycles = parsePositiveInt(arg, next());
    else if (arg === "--step-pages") options.stepPages = parsePositiveInt(arg, next());
    else if (arg === "--settle-ms") options.settleMs = parsePositiveInt(arg, next(), 0);
    else if (arg === "--max-initial-requests") options.maxInitialRequests = parsePositiveInt(arg, next());
    else if (arg === "--max-js-growth-mb") options.maxJsHeapGrowthMb = parsePositiveNumber(arg, next());
    else if (arg === "--max-rss-growth-mb") options.maxRssGrowthMb = parsePositiveNumber(arg, next());
    else if (arg === "--max-peak-rss-delta-mb") options.maxPeakRssDeltaMb = parsePositiveNumber(arg, next());
    else if (arg === "--max-dom-node-drift") options.maxDomNodeDrift = parsePositiveInt(arg, next(), 0);
    else if (arg === "--base-url") options.baseUrl = next().replace(/\/$/, "");
    else if (arg === "--browser-channel") options.browserChannel = next();
    else if (arg === "--headed") options.headed = true;
    else if (arg === "--json") options.jsonPath = path.resolve(next());
    else if (arg === "--markdown") options.markdownPath = path.resolve(next());
    else throw new Error(`Unknown option: ${arg}`);
  }
  options.maxInitialRequests = Math.min(options.pages, options.maxInitialRequests);
  return options;
}

function findFreePort() {
  return new Promise((resolve, reject) => {
    const server = http.createServer();
    server.unref();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close((error) => error ? reject(error) : resolve(port));
    });
  });
}

async function waitForHttp(url, child, timeoutMs = 60_000) {
  const deadline = Date.now() + timeoutMs;
  let lastError = "server did not respond";
  while (Date.now() < deadline) {
    if (child?.exitCode !== null) throw new Error(`Vite exited early with code ${child.exitCode}`);
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1_500) });
      if (response.ok) return;
      lastError = `HTTP ${response.status}`;
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`Timed out waiting for ${url}: ${lastError}`);
}

async function startVite(root) {
  const port = await findFreePort();
  const viteCli = path.join(root, "node_modules", "vite", "bin", "vite.js");
  const child = spawn(process.execPath, [viteCli, "--host", "127.0.0.1", "--port", String(port), "--strictPort"], {
    cwd: root,
    env: { ...process.env, BROWSER: "none", NO_COLOR: "1" },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  let logs = "";
  const collect = (chunk) => { logs = `${logs}${chunk}`.slice(-12_000); };
  child.stdout.on("data", collect);
  child.stderr.on("data", collect);
  const baseUrl = `http://127.0.0.1:${port}`;
  try {
    await waitForHttp(baseUrl, child);
    return { child, baseUrl, logs: () => logs };
  } catch (error) {
    stopOwnedProcessTree(child.pid);
    throw new Error(`${error instanceof Error ? error.message : error}\n${logs}`);
  }
}

function stopOwnedProcessTree(pid) {
  if (!pid || !Number.isInteger(pid)) return;
  if (process.platform === "win32") {
    spawnSync("taskkill.exe", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true });
    return;
  }
  try { process.kill(-pid, "SIGTERM"); } catch {}
  try { process.kill(pid, "SIGTERM"); } catch {}
}

function collectProcessTreeRssWindows(rootPid) {
  const script = [
    `$rootPid=${rootPid}`,
    "$processes=@(Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId)",
    "$ids=New-Object 'System.Collections.Generic.HashSet[int]'",
    "$null=$ids.Add([int]$rootPid)",
    "$changed=$true",
    "while($changed){$changed=$false;foreach($p in $processes){if($ids.Contains([int]$p.ParentProcessId)-and $ids.Add([int]$p.ProcessId)){$changed=$true}}}",
    "$rss=[int64]0",
    "foreach($id in $ids){try{$rss+=(Get-Process -Id $id -ErrorAction Stop).WorkingSet64}catch{}}",
    "[pscustomobject]@{rssBytes=$rss;processCount=$ids.Count}|ConvertTo-Json -Compress",
  ].join(";");
  const output = execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 15_000,
  }).trim();
  return JSON.parse(output);
}

function collectProcessTreeRssPosix(rootPid) {
  const output = execFileSync("ps", ["-eo", "pid=,ppid=,rss="], { encoding: "utf8", timeout: 10_000 });
  const rows = output.trim().split(/\r?\n/).map((line) => line.trim().split(/\s+/).map(Number));
  const ids = new Set([rootPid]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const [pid, ppid] of rows) {
      if (ids.has(ppid) && !ids.has(pid)) { ids.add(pid); changed = true; }
    }
  }
  let rssKb = 0;
  for (const [pid, , rss] of rows) if (ids.has(pid)) rssKb += rss || 0;
  return { rssBytes: rssKb * 1024, processCount: ids.size };
}

function collectProcessTreeRss(rootPid) {
  try {
    return process.platform === "win32" ? collectProcessTreeRssWindows(rootPid) : collectProcessTreeRssPosix(rootPid);
  } catch (error) {
    return { rssBytes: null, processCount: null, error: error instanceof Error ? error.message : String(error) };
  }
}

function mb(bytes) {
  return bytes === null || bytes === undefined ? null : Number((bytes / ONE_MB).toFixed(2));
}

async function forceGc(cdp) {
  try { await cdp.send("HeapProfiler.collectGarbage"); } catch {}
  await new Promise((resolve) => setTimeout(resolve, 50));
}

async function collectBrowserSample(page, cdp, browserPid, label, imageRequests) {
  await forceGc(cdp);
  const [{ metrics }, dom, pageStats] = await Promise.all([
    cdp.send("Performance.getMetrics"),
    cdp.send("Memory.getDOMCounters"),
    page.evaluate(() => ({
      documentElements: document.getElementsByTagName("*").length,
      imageElements: document.images.length,
      loadedComicImages: document.querySelectorAll(".comic-img.is-loaded").length,
      readerOpen: Boolean(document.querySelector(".reader-overlay")),
    })),
  ]);
  const metricMap = Object.fromEntries(metrics.map(({ name, value }) => [name, value]));
  const rss = collectProcessTreeRss(browserPid);
  return {
    label,
    capturedAt: new Date().toISOString(),
    jsHeapUsedBytes: Math.round(metricMap.JSHeapUsedSize ?? 0),
    jsHeapTotalBytes: Math.round(metricMap.JSHeapTotalSize ?? 0),
    jsHeapUsedMb: mb(metricMap.JSHeapUsedSize ?? 0),
    browserTreeRssBytes: rss.rssBytes,
    browserTreeRssMb: mb(rss.rssBytes),
    browserProcessCount: rss.processCount,
    rssError: rss.error ?? null,
    documents: dom.documents,
    domNodes: dom.nodes,
    jsEventListeners: dom.jsEventListeners,
    ...pageStats,
    uniqueComicImageRequests: imageRequests.size,
  };
}

function buildFixtures(pageCount) {
  const chapterImages = Array.from({ length: pageCount }, (_, index) =>
    `<div><amp-img src="https://img.test/comic-page-${String(index + 1).padStart(4, "0")}.svg"></amp-img></div>`
  ).join("");
  return {
    searchHtml: `<div class="classify-items"><div>
      <a class="comics-card__poster" href="/comic/soak-test"><amp-img src="//img.test/cover.svg"></amp-img></a>
      <div class="comics-card__info"><div class="comics-card__title">内存浸泡测试漫画</div><div class="tags">确定性测试</div></div>
    </div></div>`,
    detailHtml: `<meta name="og:url" content="https://www.baozimh.com/comic/soak-test" />
      <meta name="description" content="ComicReader long chapter deterministic memory soak" />
      <div class="comics-detail"><div class="l-content">
        <amp-img src="//img.test/detail.svg"></amp-img>
        <div class="comics-detail__title">内存浸泡测试漫画</div>
        <div class="comics-detail__author">确定性测试</div>
      </div></div>
      <div id="chapter-items"><div><a href="/user/page_direct?comic_id=soak-test&amp;section_slot=0&amp;chapter_slot=1"><span>长章节</span></a></div></div>`,
    chapterHtml: `<div class="comic-contain">${chapterImages}</div>`,
  };
}

const mockSettings = {
  theme: "dark", watch_dirs: [], auto_scrape: true, language: "zh", minimize_to_tray: false,
  vndb_enabled: true, bangumi_enabled: true, dlsite_enabled: true, touchgal_enabled: true,
  erogamescape_enabled: true, ymgal_enabled: true, kungal_enabled: true, steam_enabled: true,
  pcgw_enabled: true, scraper_proxy: "", ai_enabled: false,
  ai_api_url: "https://api.openai.com/v1/chat/completions", ai_api_key: "", ai_model: "gpt-4o-mini",
  nsfw_display_mode: "blur", autostart_enabled: false, startup_mode: "fullscreen", steam_id: "", steam_api_key: "",
};

function imageSvg(url) {
  const match = /comic-page-(\d+)/.exec(url);
  const index = match ? Number(match[1]) : 0;
  const hue = (index * 47) % 360;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="360" height="540" viewBox="0 0 360 540">
    <rect width="360" height="540" fill="hsl(${hue} 34% 16%)"/>
    <path d="M0 ${80 + index % 120} L360 ${220 + index % 180} V540 H0Z" fill="hsl(${(hue + 45) % 360} 52% 28%)"/>
    <text x="24" y="70" fill="white" font-size="28" font-family="sans-serif">PAGE ${index || "COVER"}</text>
  </svg>`;
}

function delta(last, first, key) {
  if (last?.[key] === null || last?.[key] === undefined || first?.[key] === null || first?.[key] === undefined) return null;
  return Number((last[key] - first[key]).toFixed(2));
}

function makeCheck(name, actual, limit, unit, predicate = (value) => value <= limit) {
  return { name, actual, limit, unit, passed: actual !== null && predicate(actual) };
}

function renderMarkdown(report) {
  const rows = report.checks.map((check) =>
    `| ${check.name} | ${check.actual ?? "unavailable"} ${check.unit} | ${check.limit} ${check.unit} | ${check.passed ? "PASS" : "FAIL"} |`
  ).join("\n");
  const cycleRows = report.cycles.map((cycle) =>
    `| ${cycle.cycle} | ${cycle.open.jsHeapUsedMb} | ${cycle.open.browserTreeRssMb ?? "n/a"} | ${cycle.closed.jsHeapUsedMb} | ${cycle.closed.browserTreeRssMb ?? "n/a"} | ${cycle.closed.domNodes} | ${cycle.closed.uniqueComicImageRequests} |`
  ).join("\n");
  return `# Comic long-chapter memory soak\n\n` +
    `- Captured: ${report.capturedAt}\n` +
    `- Result: **${report.passed ? "PASS" : "FAIL"}**\n` +
    `- Browser: ${report.environment.browserChannel}; ${report.options.headed ? "headed" : "headless"}\n` +
    `- Scenario: ${report.options.pages} pages, ${report.options.cycles} open/scroll/close cycles, deterministic Tauri + image mocks\n\n` +
    `## Thresholds\n\n| Check | Actual | Limit | Result |\n|---|---:|---:|---|\n${rows}\n\n` +
    `## Cycle samples\n\n| Cycle | Open JS MB | Open RSS MB | Closed JS MB | Closed RSS MB | Closed DOM nodes | Unique page requests |\n|---:|---:|---:|---:|---:|---:|---:|\n${cycleRows}\n\n` +
    `RSS is the Chromium process tree working set. JS heap and DOM counters come from the Chrome DevTools Protocol after explicit garbage collection. ` +
    `This script measures deterministic regression ceilings; it does not replace multi-hour manual OS-pressure testing.\n`;
}

function writeReport(filePath, contents) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, contents, "utf8");
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) { usage(); return; }

  const root = path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/(?:([A-Za-z]:))/, "$1")), "..");
  let vite = null;
  let browserServer = null;
  let browser = null;
  let context = null;
  const imageRequests = new Set();
  const samples = [];

  try {
    const baseUrl = options.baseUrl ?? (vite = await startVite(root)).baseUrl;
    browserServer = await chromium.launchServer({
      channel: options.browserChannel,
      host: "127.0.0.1",
      headless: !options.headed,
      args: ["--enable-precise-memory-info", "--disable-background-networking"],
    });
    const browserPid = browserServer.process().pid;
    const wsEndpoint = browserServer.wsEndpoint().replace("ws://localhost:", "ws://127.0.0.1:").replace("ws://[::1]:", "ws://127.0.0.1:");
    browser = await chromium.connect(wsEndpoint);
    context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      colorScheme: "dark",
      locale: "zh-CN",
      timezoneId: "Asia/Shanghai",
      serviceWorkers: "block",
    });
    const page = await context.newPage();
    const fixtures = buildFixtures(options.pages);

    await page.route("https://img.test/**", async (route) => {
      const url = route.request().url();
      if (url.includes("/comic-page-")) imageRequests.add(url);
      await new Promise((resolve) => setTimeout(resolve, 2));
      await route.fulfill({ status: 200, contentType: "image/svg+xml", body: imageSvg(url), headers: { "cache-control": "public, max-age=3600" } });
    });
    await page.addInitScript(({ settings, searchHtml, detailHtml, chapterHtml }) => {
      const invoke = async (command, args) => {
        if (command === "get_database_health") return { ready: true, mode: "ready", reason: null, db_path: "mock", schema_version: 3 };
        if (command === "get_settings") return settings;
        if (command === "get_games") return [];
        if (command === "get_startup_mode_override") return null;
        if (command === "get_video_proxy_port") return 0;
        if (command === "manga_fetch_json") return { data: [] };
        if (command === "manga_fetch_text") {
          const url = String(args?.url || "");
          if (url.includes("dzmanga.com/comic/chapter")) return chapterHtml;
          if (url.includes("baozimh") && !url.includes("/search")) return detailHtml;
          return url.includes("baozimh") ? searchHtml : "";
        }
        if (command.startsWith("plugin:window|is_fullscreen")) return false;
        if (command.startsWith("plugin:updater|")) return null;
        return null;
      };
      window.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } }, invoke,
        transformCallback: () => 1, unregisterCallback: () => {},
        convertFileSrc: (filePath) => `asset://localhost/${filePath}`,
      };
    }, { settings: mockSettings, ...fixtures });

    const cdp = await context.newCDPSession(page);
    await cdp.send("Performance.enable");
    await cdp.send("HeapProfiler.enable");

    await page.goto(`${baseUrl}/?skip_wizard`, { waitUntil: "domcontentloaded" });
    await page.getByRole("button", { name: "漫画" }).click();
    await page.getByPlaceholder("搜索普通漫画...").fill("内存浸泡");
    await page.getByRole("button", { name: "搜索" }).click();
    const baoziSection = page.locator(".ordinary-source-section").filter({ has: page.getByRole("heading", { name: "包子漫画", exact: true }) });
    const comicCard = baoziSection.getByRole("button", { name: /内存浸泡测试漫画/ });
    await comicCard.dispatchEvent("click");
    const chapterButton = page.getByRole("button", { name: "长章节" });
    await chapterButton.waitFor({ state: "visible" });

    const baseline = await collectBrowserSample(page, cdp, browserPid, "detail-baseline", imageRequests);
    samples.push(baseline);
    const cycles = [];
    let initial = null;

    for (let cycle = 1; cycle <= options.cycles; cycle += 1) {
      await chapterButton.click();
      const reader = page.getByRole("dialog", { name: "长章节" });
      await reader.waitFor({ state: "visible" });
      await page.locator(".comic-img").first().waitFor({ state: "visible" });
      await page.waitForTimeout(options.settleMs);

      if (!initial) {
        initial = await page.locator(".comic-img").evaluateAll((images) => ({
          imageCount: images.length,
          eagerCount: images.filter((image) => image.loading === "eager").length,
          lazyCount: images.filter((image) => image.loading === "lazy").length,
        }));
        initial.requestCount = imageRequests.size;
      }

      const visitPage = async (index) => {
        await page.locator(`[data-reader-page-index="${index}"]`).evaluate((element) => {
          const scrollRoot = document.querySelector(".reader-scroll");
          if (!(scrollRoot instanceof HTMLElement)) throw new Error("reader scroll root is missing");
          scrollRoot.scrollTop = element.offsetTop;
          scrollRoot.dispatchEvent(new Event("scroll"));
        });
        await page.waitForFunction((pageIndex) => {
          const image = document.querySelector(`[data-reader-page-index="${pageIndex}"] img`);
          return image instanceof HTMLImageElement && image.loading === "eager" && image.complete;
        }, index, { timeout: 2_000 });
      };
      for (let index = 0; index < options.pages; index += options.stepPages) await visitPage(index);
      await visitPage(options.pages - 1);
      await page.waitForFunction((expected) => document.querySelectorAll(".comic-img.is-loaded").length >= expected, options.pages, { timeout: 15_000 }).catch(() => {});
      await page.waitForTimeout(options.settleMs);
      const open = await collectBrowserSample(page, cdp, browserPid, `cycle-${cycle}-open`, imageRequests);
      samples.push(open);

      await page.keyboard.press("Escape");
      await reader.waitFor({ state: "detached" });
      await page.waitForTimeout(options.settleMs);
      const closed = await collectBrowserSample(page, cdp, browserPid, `cycle-${cycle}-closed`, imageRequests);
      samples.push(closed);
      cycles.push({ cycle, open, closed });
    }

    const firstClosed = cycles[0].closed;
    const lastClosed = cycles.at(-1).closed;
    const peakRss = Math.max(...samples.map((sample) => sample.browserTreeRssMb ?? Number.NEGATIVE_INFINITY));
    const jsGrowthMb = delta(lastClosed, firstClosed, "jsHeapUsedMb");
    const rssGrowthMb = delta(lastClosed, firstClosed, "browserTreeRssMb");
    const peakRssDeltaMb = Number.isFinite(peakRss) && baseline.browserTreeRssMb !== null ? Number((peakRss - baseline.browserTreeRssMb).toFixed(2)) : null;
    const domNodeDrift = lastClosed.domNodes - firstClosed.domNodes;
    const checks = [
      makeCheck("All mocked chapter pages rendered", initial.imageCount, options.pages, "pages", (value) => value === options.pages),
      makeCheck("Initial eager prefetch window", initial.eagerCount, Math.min(3, options.pages), "images", (value) => value === Math.min(3, options.pages)),
      makeCheck("Initial image requests", initial.requestCount, options.maxInitialRequests, "requests"),
      makeCheck("Full chapter requested after traversal", imageRequests.size, options.pages, "requests", (value) => value === options.pages),
      makeCheck("Closed-state JS heap growth", jsGrowthMb, options.maxJsHeapGrowthMb, "MB"),
      ...(rssGrowthMb === null ? [{ name: "Closed-state browser RSS growth", actual: null, limit: options.maxRssGrowthMb, unit: "MB", passed: false }] : [makeCheck("Closed-state browser RSS growth", rssGrowthMb, options.maxRssGrowthMb, "MB")]),
      ...(peakRssDeltaMb === null ? [{ name: "Peak browser RSS over detail baseline", actual: null, limit: options.maxPeakRssDeltaMb, unit: "MB", passed: false }] : [makeCheck("Peak browser RSS over detail baseline", peakRssDeltaMb, options.maxPeakRssDeltaMb, "MB")]),
      makeCheck("Closed-state DOM node drift", domNodeDrift, options.maxDomNodeDrift, "nodes"),
    ];

    const report = {
      schemaVersion: 1,
      capturedAt: new Date().toISOString(),
      passed: checks.every((check) => check.passed),
      environment: {
        platform: process.platform,
        arch: process.arch,
        node: process.version,
        osRelease: os.release(),
        browserChannel: options.browserChannel,
        baseUrl,
        browserPid,
      },
      options: {
        pages: options.pages, cycles: options.cycles, stepPages: options.stepPages, settleMs: options.settleMs,
        maxInitialRequests: options.maxInitialRequests, maxJsHeapGrowthMb: options.maxJsHeapGrowthMb,
        maxRssGrowthMb: options.maxRssGrowthMb, maxPeakRssDeltaMb: options.maxPeakRssDeltaMb,
        maxDomNodeDrift: options.maxDomNodeDrift, headed: options.headed,
      },
      initial,
      baseline,
      cycles,
      derived: { jsGrowthMb, rssGrowthMb, peakRssDeltaMb, domNodeDrift },
      checks,
    };

    const json = `${JSON.stringify(report, null, 2)}\n`;
    const markdown = renderMarkdown(report);
    if (options.jsonPath) writeReport(options.jsonPath, json);
    if (options.markdownPath) writeReport(options.markdownPath, markdown);
    console.log(markdown);
    if (options.jsonPath) console.log(`JSON: ${options.jsonPath}`);
    if (options.markdownPath) console.log(`Markdown: ${options.markdownPath}`);
    if (!report.passed) process.exitCode = 1;
  } finally {
    await context?.close().catch(() => {});
    await browser?.close().catch(() => {});
    await browserServer?.close().catch(() => {});
    if (vite?.child?.pid) stopOwnedProcessTree(vite.child.pid);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack : error);
  process.exitCode = 1;
});


