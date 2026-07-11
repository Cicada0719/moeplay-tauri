import AxeBuilder from "@axe-core/playwright";
import {
  DEFAULT_APP_STATE,
  expect,
  test,
  type MockAppState,
} from "./fixtures/app-fixture";

type ThemePackId = "yozakura" | "after-school" | "neon-isekai";
type ColorMode = "pack-default" | "contrast";

function themedState(themePack: ThemePackId, colorMode: ColorMode = "pack-default"): MockAppState {
  const appearance = {
    theme_pack: themePack,
    color_mode: colorMode,
    wallpaper_rotation: "fixed",
    fixed_wallpaper_id: `builtin:${themePack}:1`,
    mascot_enabled: true,
    decorative_effects: true,
    online_gallery_enabled: false,
  };

  return {
    ...DEFAULT_APP_STATE,
    settings: { ...DEFAULT_APP_STATE.settings, appearance },
    localStorage: {
      ...DEFAULT_APP_STATE.localStorage,
      "moeplay-appearance-v1": JSON.stringify(appearance),
    },
    commandResults: {
      list_wallpapers: [],
      refresh_wallpaper_manifest: {
        revision: "fixture",
        downloaded: 0,
        skipped: 0,
        failed: 0,
      },
    },
  };
}

const themes = [
  { name: "夜樱终端", id: "yozakura", color: "dark", decoration: "petals" },
  { name: "青空放课后", id: "after-school", color: "light", decoration: "light-particles" },
  { name: "霓虹异界", id: "neon-isekai", color: "dark", decoration: "digital-rain" },
] as const;

for (const theme of themes) {
  test.describe(`${theme.name} theme setting`, () => {
    test.use({ appState: themedState(theme.id) });

    test(`renders ${theme.id} through settings and WallpaperStage`, async ({ appPage }) => {
      await appPage.getByTestId("system-dock").getByRole("button", { name: "打开设置" }).click();
      await expect(appPage).toHaveURL(/#settings$/);

      const root = appPage.locator("html");
      const stage = appPage.locator(".wallpaper-stage");
      const selectedCard = appPage.getByRole("button", { name: new RegExp(theme.name) });

      await expect(root).toHaveAttribute("data-theme-pack", theme.id);
      await expect(root).toHaveAttribute("data-color-mode", theme.color);
      await expect(root).toHaveAttribute("data-decoration", theme.decoration);
      await expect(selectedCard).toHaveAttribute("aria-pressed", "true");
      await expect(stage).toHaveAttribute("data-surface", "management");
      await expect(stage).toHaveAttribute("data-wallpaper-id", `builtin:${theme.id}:1`);
      await expect(stage.locator(`.wallpaper-stage__decor--${theme.decoration}`)).toHaveCount(1);

      await expect(appPage).toHaveScreenshot(`settings-theme-${theme.id}.png`, {
        fullPage: true,
      });
    });
  });
}

test.describe("anime theme surface and accessibility modes", () => {
  test.use({ appState: themedState("yozakura") });

  test("management surface applies a stronger, quieter wallpaper treatment", async ({ appPage }) => {
    const stage = appPage.locator(".wallpaper-stage");
    const image = stage.locator(".wallpaper-stage__image");
    const scrim = stage.locator(".wallpaper-stage__scrim");

    await expect(stage).toHaveAttribute("data-surface", "browse");
    const browseFilter = await image.evaluate((element) => getComputedStyle(element).filter);
    const browseScrim = await scrim.evaluate((element) => getComputedStyle(element).backgroundColor);

    await appPage.getByTestId("system-dock").getByRole("button", { name: "打开设置" }).click();
    await expect(stage).toHaveAttribute("data-surface", "management");

    const managementFilter = await image.evaluate((element) => getComputedStyle(element).filter);
    const managementScrim = await scrim.evaluate((element) => getComputedStyle(element).backgroundColor);

    expect(managementFilter).not.toBe(browseFilter);
    expect(managementScrim).not.toBe(browseScrim);
    expect(managementFilter).toContain("brightness");
    await expect(appPage).toHaveScreenshot("management-wallpaper-treatment.png", { fullPage: true });
  });

  test("reduced motion disables wallpaper transitions and animated decoration", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/?skip_wizard", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("app-shell")).toBeVisible();

    await expect(page.locator("html")).toHaveAttribute("data-motion", "reduce");
    await expect(page.locator(".wallpaper-stage__image")).toHaveCSS("transition-duration", "0s");
    await expect(page.locator(".wallpaper-stage__decor")).toHaveCSS("display", "none");
    await expect(page).toHaveScreenshot("library-reduced-motion.png", { fullPage: true });
  });
});

test.describe("contrast anime theme accessibility", () => {
  test.use({ appState: themedState("yozakura", "contrast") });

  test("contrast removes decorative effects and has no serious settings violations", async ({ appPage }) => {
    await appPage.getByTestId("system-dock").getByRole("button", { name: "打开设置" }).click();

    const root = appPage.locator("html");
    await expect(root).toHaveAttribute("data-color-mode", "contrast");
    await expect(root).toHaveAttribute("data-theme", "contrast");
    await expect(root).toHaveAttribute("data-decoration", "none");
    expect(await appPage.locator(".wallpaper-stage__decor").evaluateAll((nodes) => nodes.map((node) => getComputedStyle(node).display))).not.toContain("block");
    await expect(appPage.getByRole("radio", { name: "高对比" })).toHaveAttribute("aria-checked", "true");

    const results = await new AxeBuilder({ page: appPage })
      .include('[aria-label="二次元主题包"]')
      .include(".wallpaper-setting")
      .analyze();
    const blocking = results.violations.filter(
      ({ impact }) => impact === "serious" || impact === "critical",
    );
    expect(blocking.map(({ id, impact, nodes }) => ({
      id,
      impact,
      targets: nodes.map((node) => node.target),
    }))).toEqual([]);

    await expect(appPage).toHaveScreenshot("settings-contrast.png", { fullPage: true });
  });
});
