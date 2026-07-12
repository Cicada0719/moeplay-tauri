import { expect, test } from "@playwright/test";

test.describe("game media workspace v2", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?skip_wizard#home");
    await expect(page.getByTestId("switch-home")).toBeVisible();
  });

  test("switches Visual, Index and Scene without leaving the game library route", async ({ page }) => {
    const visual = page.locator('[data-media-mode="visual"]');
    const index = page.locator('[data-media-mode="index"]');
    const scene = page.locator('[data-media-mode="scene"]');

    await visual.click();
    await expect(page.getByTestId("switch-home-stage")).toBeVisible();
    await expect(visual).toHaveAttribute("aria-pressed", "true");

    await index.click();
    await expect(page.getByTestId("all-games-panel")).toBeVisible();
    await expect(page.getByTestId("all-games-grid")).toBeVisible();
    await expect(index).toHaveAttribute("aria-pressed", "true");

    await scene.click();
    await expect(page.getByTestId("switch-home-scene")).toBeVisible();
    const sceneEntryCount = await page.locator("[data-film-game]").count();
    expect(sceneEntryCount).toBeGreaterThanOrEqual(1);
    await expect(scene).toHaveAttribute("aria-pressed", "true");
    await expect(page).toHaveURL(/#home$/);
  });

  test("search temporarily uses Index and restores the selected director mode", async ({ page }) => {
    const scene = page.locator('[data-media-mode="scene"]');
    await scene.click();
    await expect(page.getByTestId("switch-home-scene")).toBeVisible();

    const search = page.getByRole("searchbox", { name: "搜索游戏库" });
    await search.fill("Summer");
    await expect(page.getByTestId("all-games-panel")).toBeVisible();
    await expect(scene).toHaveAttribute("aria-pressed", "true");

    await page.getByRole("button", { name: "清空搜索" }).click();
    await expect(search).toBeFocused();
    await expect(search).toHaveValue("");
    await expect(page.getByTestId("switch-home-scene")).toBeVisible();
  });
  test("Visual wheel steps one title per gesture and ignores controls", async ({ page }) => {
    const visual = page.locator('[data-media-mode="visual"]');
    await visual.click();
    const stage = page.getByTestId("game-unified-stage");
    await stage.focus();
    const before = await stage.getAttribute("data-selected-game");
    await stage.dispatchEvent("wheel", { deltaY: 90, deltaX: 0, deltaMode: 0 });
    await page.waitForTimeout(100);
    const after = await stage.getAttribute("data-selected-game");
    expect(after).not.toBe(before);
    await stage.dispatchEvent("wheel", { deltaY: 120, deltaX: 0, deltaMode: 0 });
    await page.waitForTimeout(100);
    await expect(stage).toHaveAttribute("data-selected-game", after || "");
  });

});

test.describe("v0.13.4 media workspace viewport and motion contract", () => {
  test("900x600 keeps all mode controls and a usable Visual stage", async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 600 });
    await page.goto("/?skip_wizard#home");
    await expect(page.getByTestId("switch-home")).toBeVisible();
    await expect(page.locator('[data-media-mode="visual"]')).toBeVisible();
    await expect(page.locator('[data-media-mode="index"]')).toBeVisible();
    await expect(page.locator('[data-media-mode="scene"]')).toBeVisible();
    await expect(page.getByTestId("game-unified-stage")).toBeVisible();
  });

  test("1920x1080 and reduced-motion expose the same navigation paths without continuous animation", async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/?skip_wizard#home");
    await expect(page.getByTestId("switch-home")).toBeVisible();
    await page.locator('[data-media-mode="visual"]').click();
    await expect(page.getByTestId("game-unified-stage")).toHaveAttribute("data-reduced-motion", "true");
    await page.locator('[data-media-mode="scene"]').click();
    await expect(page.getByTestId("switch-home-scene")).toBeVisible();
  });
});
