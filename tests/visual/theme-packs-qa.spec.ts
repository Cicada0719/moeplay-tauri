import {
  DEFAULT_APP_STATE,
  expect,
  test,
  type MockAppState,
} from "./fixtures/app-fixture";

/**
 * 0.14.0 新增主题包 QA：属性断言 + 截图留档（test-results/theme-qa/<id>/）。
 * 与 anime-theme-regression.spec.ts 同一确定性 fixture；不做快照比对。
 */

type NewPackId = "shift-editorial" | "phantom-pop" | "caution-industrial" | "astral-rail" | "borderless-lumen";

function themedState(themePack: NewPackId): MockAppState {
  const appearance = {
    theme_pack: themePack,
    color_mode: "pack-default",
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

const newPacks = [
  { name: "素纸编集", id: "shift-editorial", color: "light", decoration: "light-particles", accent: "#d4293c", bodyRgb: "rgb(241, 237, 228)" },
  { name: "魅影波普", id: "phantom-pop", color: "dark", decoration: "petals", accent: "#e6242f", bodyRgb: "rgb(10, 8, 9)" },
  { name: "警戒工业", id: "caution-industrial", color: "dark", decoration: "digital-rain", accent: "#f59e0b", bodyRgb: "rgb(11, 13, 16)" },
  { name: "星穹旅人", id: "astral-rail", color: "dark", decoration: "light-particles", accent: "#d8b45a", bodyRgb: "rgb(7, 8, 26)" },
  { name: "无界流光", id: "borderless-lumen", color: "dark", decoration: "petals", accent: "#7c5cff", bodyRgb: "rgb(4, 4, 8)" },
] as const;

for (const theme of newPacks) {
  test.describe(`${theme.name} theme pack QA`, () => {
    test.use({ appState: themedState(theme.id) });

    test(`applies ${theme.id} and captures QA screenshots`, async ({ appPage }) => {
      const root = appPage.locator("html");
      const stage = appPage.locator(".wallpaper-stage");

      // 首页（壁纸舞台 + 游戏库）
      await expect(root).toHaveAttribute("data-theme-pack", theme.id);
      await expect(root).toHaveAttribute("data-color-mode", theme.color);
      await expect(root).toHaveAttribute("data-decoration", theme.decoration);
      await expect(stage.locator(`.wallpaper-stage__decor--${theme.decoration}`)).toHaveCount(1);

      // 全套换肤：--accent 与 body 底色必须被 pack 覆盖（压过 scheme-c）
      const tokens = await appPage.evaluate(() => {
        const cs = getComputedStyle(document.documentElement);
        return { accent: cs.getPropertyValue("--accent").trim(), bodyBg: getComputedStyle(document.body).backgroundColor };
      });
      expect(tokens.accent).toBe(theme.accent);
      expect(tokens.bodyBg).toBe(theme.bodyRgb);

      await appPage.screenshot({ path: `test-results/theme-qa/${theme.id}/home.png` });

      // 设置页（主题选择器 + management 表面）
      await appPage.getByRole("banner").getByRole("button", { name: "打开设置" }).click();
      await expect(appPage).toHaveURL(/#settings$/);
      const selectedCard = appPage.getByRole("button", { name: new RegExp(theme.name) });
      await expect(selectedCard).toHaveAttribute("aria-pressed", "true");
      await expect(stage).toHaveAttribute("data-surface", "management");
      await expect(stage).toHaveAttribute("data-wallpaper-id", `builtin:${theme.id}:1`);
      await appPage.screenshot({ path: `test-results/theme-qa/${theme.id}/settings.png`, fullPage: true });
    });
  });
}


/**
 * Kinetic 电影化主视觉 QA（0.14.0 新增，追加块，不触碰上方既有断言）：
 * 舞台层必须存在、装饰性（aria-hidden）、不拦截指针，且首页既有交互
 * （搜索聚焦）不受影响。WebGL 与 fallback 两种路径均可接受。
 */
test.describe("Kinetic 电影化主视觉 QA", () => {
  test.use({ appState: themedState("phantom-pop") });

  test("首页挂载舞台层且不干扰既有交互", async ({ appPage }) => {
    const stage = appPage.locator('[data-testid="kinetic-stage"]');
    await expect(stage).toHaveCount(1);
    await expect(stage).toHaveAttribute("aria-hidden", "true");

    const mode = await stage.getAttribute("data-mode");
    expect(["webgl", "fallback"]).toContain(mode);

    const pointerEvents = await stage.evaluate((element) => getComputedStyle(element).pointerEvents);
    expect(pointerEvents).toBe("none");

    // 既有内容与焦点管理照常可用：搜索框可点击并聚焦。
    const search = appPage.locator("#library-search");
    await search.click();
    await expect(search).toBeFocused();

    await appPage.screenshot({ path: "test-results/theme-qa/kinetic-stage/home.png" });
  });
});
