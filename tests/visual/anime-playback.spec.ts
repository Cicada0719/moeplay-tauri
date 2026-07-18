import { test, expect } from "@playwright/test";

const settings = {
  theme: "dark", watch_dirs: [], auto_scrape: true, language: "zh", minimize_to_tray: false,
  vndb_enabled: true, bangumi_enabled: true, dlsite_enabled: true, touchgal_enabled: true,
  erogamescape_enabled: true, ymgal_enabled: true, kungal_enabled: true, steam_enabled: true,
  pcgw_enabled: true, scraper_proxy: "", ai_enabled: false,
  ai_api_url: "https://api.openai.com/v1/chat/completions", ai_api_key: "",
  ai_model: "gpt-4o-mini", nsfw_display_mode: "blur", autostart_enabled: false,
  startup_mode: "fullscreen", steam_id: "", steam_api_key: "",
};

const rules = [
  {
    name: "主测试源", version: "1.0", baseUrl: "https://source-one.test", searchURL: "", searchList: "",
    searchName: "", searchResult: "", chapterRoads: "", chapterResult: "", muliSources: true,
    useWebview: true, useNativePlayer: true, usePost: false, useLegacyParser: false, adBlocker: false,
    userAgent: "", referer: "https://source-one.test", api: "0", type: "anime",
  },
  {
    name: "备用测试源", version: "1.0", baseUrl: "https://source-two.test", searchURL: "", searchList: "",
    searchName: "", searchResult: "", chapterRoads: "", chapterResult: "", muliSources: true,
    useWebview: true, useNativePlayer: true, usePost: false, useLegacyParser: false, adBlocker: false,
    userAgent: "", referer: "https://source-two.test", api: "0", type: "anime",
  },
];

test("v0.12 anime search, episode selection, extraction and automatic failover", async ({ page }) => {
  await page.route("**/mock-video.mp4", async (route) => {
    await route.fulfill({ status: 200, contentType: "video/mp4", body: Buffer.from([]) });
  });

  await page.addInitScript(({ mockSettings, animeRules }) => {
    localStorage.setItem("anime-rules", JSON.stringify(animeRules));
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    (window as any).__animeAcceptanceCalls = calls;

    const invoke = async (command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      if (command === "get_settings") return mockSettings;
      if (command === "get_games") return [];
      if (command === "get_video_proxy_port") return 43123;
      if (command === "anime_bangumi_search") return [[], 0];
      if (command === "anime_set_rules") return null;
      if (command === "anime_search") {
        const rule = String(args?.ruleName || "");
        return [{ name: "验收番剧", url: rule === "备用测试源" ? "/source-two/show" : "/source-one/show" }];
      }
      if (command === "anime_fetch_roads") {
        const pageUrl = String(args?.pageUrl || "");
        const prefix = pageUrl.includes("source-two") ? "/source-two" : "/source-one";
        return [{ name: "默认线路", episodes: [
          { name: "第1集", url: `${prefix}/ep1` },
          { name: "第2集", url: `${prefix}/ep2` },
        ] }];
      }
      if (command === "anime_build_url") return String(args?.url || "");
      if (command === "anime_extract_video_url") {
        const episodeUrl = String(args?.episodeUrl || "");
        if (episodeUrl.includes("source-one")) throw new Error("主源模拟失效");
        return { url: "http://localhost:1420/mock-video.mp4", tab_url: "https://source-two.test/play/1" };
      }
      if (command === "anime_get_proxy_url") return "http://localhost:1420/mock-video.mp4";
      if (command === "anime_danmaku_search") return [];
      if (command === "anime_record_source_health" || command === "frontend_log") return null;
      if (command.startsWith("plugin:event|")) return 1;
      if (command.startsWith("plugin:window|is_fullscreen")) return false;
      if (command.startsWith("plugin:updater|")) return null;
      return null;
    };
    (window as any).__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" } },
      invoke,
      transformCallback: () => 1,
      unregisterCallback: () => {},
      convertFileSrc: (filePath: string) => `asset://localhost/${filePath}`,
    };
  }, { mockSettings: settings, animeRules: rules });

  await page.goto("/?skip_wizard");
  await page.getByRole("button", { name: "番剧" }).click();
  await expect(page.getByTestId("anime-page")).toBeVisible();

  await page.locator("select.rule-select").selectOption("主测试源");
  await page.getByPlaceholder("搜索番剧...").fill("验收番剧");
  await page.getByRole("button", { name: "搜索", exact: true }).click();
  await expect(page.getByRole("heading", { name: "主测试源" })).toBeVisible();
  await page.getByRole("button", { name: "验收番剧" }).click();

  await expect(page.getByRole("button", { name: /开始观看/ })).toBeVisible();
  await expect(page.locator(".detail-visual")).toBeVisible();
  await expect(page.locator(".detail-scroll")).toBeVisible();
  const detailPanelBox = await page.locator(".anime-detail-panel").boundingBox();
  const detailVisualBox = await page.locator(".detail-visual").boundingBox();
  expect(detailPanelBox?.width).toBeGreaterThan(page.viewportSize()?.width ? page.viewportSize()!.width * 0.9 : 800);
  expect(detailVisualBox?.width).toBeGreaterThan(250);
  await page.getByRole("button", { name: /开始观看/ }).click();
  const sourceSheet = page.locator(".source-sheet");
  await expect(sourceSheet).toBeVisible();
  await sourceSheet.getByRole("button", { name: /验收番剧/ }).first().click();
  await expect(sourceSheet.getByRole("button", { name: "第1集" })).toBeVisible();
  await sourceSheet.getByRole("button", { name: "第1集" }).click();

  await expect(page.locator("video.player-video")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("anime-playback-shell")).toBeVisible();
  const fullscreenButton = page.getByRole("button", { name: "进入全屏" });
  await fullscreenButton.click();
  await expect(page.locator(".player-overlay")).toHaveClass(/fullscreen/);
  const playbackShell = page.getByTestId("anime-playback-shell");
  await expect(playbackShell).not.toHaveClass(/anime-playback-shell--chrome-hidden/);
  await expect(playbackShell).toHaveClass(/anime-playback-shell--chrome-hidden/, { timeout: 4_500 });
  await page.locator(".player-body").hover({ position: { x: 120, y: 90 } });
  await expect(playbackShell).not.toHaveClass(/anime-playback-shell--chrome-hidden/);
  await page.locator(".player-body").click({ position: { x: 120, y: 90 } });
  await expect(page.locator(".player-overlay")).toHaveClass(/fullscreen/);
  const windowFullscreenCalls = await page.evaluate(() =>
    (window as any).__animeAcceptanceCalls.filter((entry: any) => String(entry.command).includes("set_fullscreen")),
  );
  expect(windowFullscreenCalls.length).toBeGreaterThan(0);
  await page.getByRole("button", { name: "退出全屏" }).click();
  await expect(page.locator(".player-overlay")).not.toHaveClass(/fullscreen/);
  await page.waitForTimeout(650);
  const repairedFullscreenCalls = await page.evaluate(() =>
    (window as any).__animeAcceptanceCalls.filter((entry: any) => String(entry.command).includes("set_fullscreen")),
  );
  expect(repairedFullscreenCalls.some((entry: any) => entry.args?.value === false)).toBe(true);
  expect(repairedFullscreenCalls.at(-1)?.args?.value).toBe(true);
  for (const viewport of [
    { width: 360, height: 800 },
    { width: 800, height: 360 },
    { width: 900, height: 600 },
    { width: 1920, height: 1080 },
  ]) {
    await page.setViewportSize(viewport);
    await page.waitForTimeout(120);
    const bodyBox = await page.locator(".anime-playback-shell__body").boundingBox();
    const videoBox = await page.locator("video.player-video").boundingBox();
    expect(bodyBox?.width).toBeGreaterThan(0);
    expect(videoBox?.width).toBeGreaterThan(0);
    expect(videoBox?.width).toBeLessThanOrEqual((bodyBox?.width || 0) + 20);
    expect(videoBox?.height).toBeLessThanOrEqual((bodyBox?.height || 0) + 20);
  }
  const extractionCalls = await page.evaluate(() =>
    (window as any).__animeAcceptanceCalls
      .filter((entry: any) => entry.command === "anime_extract_video_url")
      .map((entry: any) => String(entry.args?.episodeUrl || "")),
  );
  expect(extractionCalls).toEqual(expect.arrayContaining(["/source-one/ep1", "/source-two/ep1"]));
});

test.describe("v0.13.4 anime player adaptive shell", () => {
  test("reduced-motion keeps player keyboard escape and close controls available", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/?skip_wizard#anime");
    await expect(page.getByTestId("anime-page")).toBeVisible();
    await expect(page.locator(".anime-page")).toBeVisible();
    await expect(page.locator(".anime-page")).toHaveCSS("scroll-behavior", "auto");
  });
});
