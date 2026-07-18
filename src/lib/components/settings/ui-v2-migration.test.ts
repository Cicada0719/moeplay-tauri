import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function source(path: string): string {
  return readFileSync(resolve(process.cwd(), path), "utf8");
}

describe("settings UI-v2 migration contract", () => {
  it("rebuilds the page skeleton on shared ui-v2 primitives with the category index", () => {
    const page = source("src/lib/components/SettingsPage.svelte");
    expect(page).toContain('<PageShell as="div"');
    expect(page).toContain("<PageHeader");
    expect(page).toContain("<StateBoundary");
    expect(page).toContain('scrollToSettings("settings-appearance")');
    expect(page).toContain('scrollToSettings("settings-scrape")');
    expect(page).toContain('scrollToSettings("settings-maintenance")');
    expect(page).toContain('id="settings-appearance"');
    expect(page).toContain("stg-index");
    expect(page).toContain("v2-grain");
    expect(page).not.toContain('href="#settings-appearance"');
  });

  it("keeps the defensive settings logic fixed in 0.13.7 (defaults + AI toggle persistence)", () => {
    const page = source("src/lib/components/SettingsPage.svelte");
    const store = source("src/lib/stores/settings.svelte.ts");
    expect(page).toContain('setBooleanSetting("ai_enabled"');
    expect(page).toContain("(e.target as HTMLInputElement).checked");
    expect(page).toContain("normalizeAppearance(settingsStore.settings.appearance)");
    expect(page).toContain('settingsStore.settings.startup_mode ?? "fullscreen"');
    expect(page).toContain("settingsStore.settings.autostart_enabled ?? false");
    // Old payloads missing keys still merge over defaults instead of crashing render.
    expect(store).toContain("...defaultSettings, ...publicSettings");
  });

  it("delegates heavy sections to focused child components under settings/", () => {
    const page = source("src/lib/components/SettingsPage.svelte");
    expect(page).toContain("<ScrapeSection");
    expect(page).toContain("<LibrarySection");
    expect(page).toContain("<BangumiSection");
    expect(page).toContain("<PlayerSection");

    const scrape = source("src/lib/components/settings/ScrapeSection.svelte");
    const library = source("src/lib/components/settings/LibrarySection.svelte");
    const bangumi = source("src/lib/components/settings/BangumiSection.svelte");
    const player = source("src/lib/components/settings/PlayerSection.svelte");
    expect(scrape).toContain("scraper_proxy");
    expect(scrape).toContain('id="settings-scrape"');
    expect(scrape).toContain("toggleScrapeSetting");
    expect(library).toContain("watch_dirs");
    expect(library).toContain('id="settings-library"');
    expect(bangumi).toContain("setBangumiToken");
    expect(bangumi).toContain('id="settings-bangumi"');
    expect(player).toContain("skipOpening");
    expect(player).toContain('id="settings-player"');
  });

  it("pairs Chinese section titles with decorative kana/English annotations", () => {
    const page = source("src/lib/components/SettingsPage.svelte");
    const shared = source("src/lib/components/settings/settings-shared.css");
    expect(page).toContain("s-title-sub");
    expect(page).toContain("外観 / APPEARANCE");
    expect(shared).toContain(".s-title-sub");
    expect(shared).toContain("--font-mono");
  });

  it("wires the i18n store into the settings page and the global navigation", () => {
    const page = source("src/lib/components/SettingsPage.svelte");
    const nav = source("src/lib/shell/GlobalTopNavigation.svelte");
    const i18nStore = source("src/lib/stores/i18n.svelte.ts");
    expect(page).toContain('from "../stores/i18n.svelte"');
    expect(page).toContain('i18n.t("settings.section_appearance")');
    expect(page).toContain("setInterfaceLanguage");
    expect(nav).toContain('from "../stores/i18n.svelte"');
    expect(nav).toContain("menu.games");
    expect(nav).toContain('i18n.t("menu.settings")');
    // New keys exist in both dictionaries and the stale app title is refreshed.
    for (const key of ['"menu.records"', '"menu.anime"', '"menu.comic"', '"menu.novel"', '"settings.language"']) {
      expect(i18nStore).toContain(key);
    }
    expect(i18nStore).toContain("游戏·番剧·漫画管理器");
    expect(i18nStore).toContain("MoeGame — Games, Anime & Comics Manager");
    expect(i18nStore).not.toContain("Galgame 游戏管理器");
  });

  it("ships a pure-CSS grain utility on the settings shell that respects theme tokens", () => {
    const tokens = source("src/lib/styles/tokens-v2.css");
    expect(tokens).toContain(".v2-grain");
    expect(tokens).toContain("radial-gradient");
    expect(tokens).toContain("color-mix");
    expect(tokens).toContain("--v2-color-text");
    const page = source("src/lib/components/SettingsPage.svelte");
    expect(page).toContain('class="v2-grain stg-grain"');
  });

  it("keeps reduced-motion fallbacks for page-level transitions", () => {
    const page = source("src/lib/components/SettingsPage.svelte");
    const shared = source("src/lib/components/settings/settings-shared.css");
    expect(page).toContain("prefers-reduced-motion: reduce");
    expect(page).toContain('data-motion="reduce"');
    expect(shared).toContain("prefers-reduced-motion: reduce");
    expect(shared).toContain('data-motion="reduce"');
  });
});
