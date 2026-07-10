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

const baoziHtml = `
  <div class="classify-items"><div>
    <a class="comics-card__poster" href="/comic/baozi-test"><amp-img src="//img.test/baozi.jpg"></amp-img><div class="tabs"><span class="tab">连载中</span></div></a>
    <div class="comics-card__info"><div class="comics-card__title">包子测试漫画</div><div class="tags">包子作者</div></div>
  </div></div>
`;

const dm5Html = `
  <div class="mh-item mh-card-wrap">
    <div class="mh-item-detali"><a href="/manhua-test/" title="动漫屋测试漫画">动漫屋测试漫画</a></div>
    <div class="manga-info" tc="https://img.test/dm5.jpg" ts="3" au="测试作者," tus="连载中"></div>
  </div>
`;

test("v0.12 comic auto mode renders isolated parallel source sections", async ({ page }) => {
  await page.addInitScript(({ mockSettings, baoziSearchHtml, dm5SearchHtml }) => {
    const invoke = async (command: string, args?: Record<string, unknown>) => {
      if (command === "get_database_health") return { ready: true, mode: "ready", reason: null, db_path: "mock", schema_version: 3 };
      if (command === "get_settings") return mockSettings;
      if (command === "get_games") return [];
      if (command === "get_startup_mode_override") return null;
      if (command === "get_video_proxy_port") return 0;
      if (command === "manga_fetch_json") return {
        data: [{
          id: "md-test",
          attributes: { title: { zh: "MangaDex测试漫画" }, status: "ongoing" },
          relationships: [
            { type: "cover_art", attributes: { fileName: "cover.jpg" } },
            { type: "author", attributes: { name: "MD作者" } },
          ],
        }],
      };
      if (command === "manga_fetch_text") {
        const url = String(args?.url || "");
        return url.includes("baozimh") ? baoziSearchHtml : dm5SearchHtml;
      }
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
  }, { mockSettings: settings, baoziSearchHtml: baoziHtml, dm5SearchHtml: dm5Html });

  await page.goto("/?skip_wizard");
  await page.getByRole("button", { name: "漫画" }).click();
  await page.getByPlaceholder("搜索普通漫画...").fill("测试");
  await page.getByRole("button", { name: "搜索" }).click();

  await expect(page.getByRole("heading", { name: "MangaDex" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "包子漫画" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "DM5" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "1kkk" })).toBeVisible();
  await expect(page.getByText("MangaDex测试漫画")).toBeVisible();
  await expect(page.getByText("包子测试漫画")).toBeVisible();
});
