import { test, expect, DEFAULT_APP_STATE } from "./fixtures";
const appearance = { theme_pack: "borderless-lumen", color_mode: "dark", wallpaper_rotation: "fixed", fixed_wallpaper_id: "builtin:borderless-lumen:1", mascot_enabled: false, decorative_effects: false, online_gallery_enabled: false };
test.use({ appState: { ...DEFAULT_APP_STATE,
  settings: { ...DEFAULT_APP_STATE.settings, appearance },
  localStorage: { ...DEFAULT_APP_STATE.localStorage, "moeplay-appearance-v1": JSON.stringify(appearance) },
  games: DEFAULT_APP_STATE.games.map((game, index) => ({ ...game,
    metadata: { ...game.metadata, developer: "MoePlay Original", cover: `http://127.0.0.1:1420/src/lib/assets/themes/astral-rail/wallpaper-${index + 1}.jpg`, background: `http://127.0.0.1:1420/src/lib/assets/themes/astral-rail/wallpaper-${index + 1}.jpg` },
  })),
} });
test("release homepage uses bundled original art and has no overflowing document", async ({ appPage: page }, testInfo) => {
  await expect(page.locator("html")).toHaveAttribute("data-theme-pack", "borderless-lumen");
  await expect(page.locator('img[src*="astral-rail"]').first()).toBeVisible();
  await page.locator('img[src*="astral-rail"]').first().evaluate(async (img: HTMLImageElement) => { await img.decode(); });
  await page.screenshot({ path: testInfo.outputPath("desktop.png"), animations: "disabled" });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
