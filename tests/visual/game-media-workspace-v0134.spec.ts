import { expect, test } from "./fixtures/app-fixture";
import { MEDIA_WORKSPACE_APP_STATE } from "./fixtures/v0134-fixtures";

test.use({ appState: MEDIA_WORKSPACE_APP_STATE });

test.describe("v0.13.4 game media workspace contract", () => {
  test.beforeEach(async ({ appPage }) => {
    await expect(appPage.getByTestId("switch-home")).toBeVisible();
  });

  test("Visual renders exactly five owned slots and every slot exposes an action", async ({ appPage }) => {
    await appPage.locator('[data-media-mode="visual"]').click();
    const visual = appPage.locator(".gv-stage");
    const slots = visual.locator("[data-visual-slot]");

    await expect(slots).toHaveCount(5);
    for (let index = 0; index < 5; index += 1) await expect(slots.nth(index)).toHaveAttribute("type", "button");
    for (const role of ["lead", "scene-a", "scene-b", "continue", "featured"]) {
      const slot = visual.locator(`[data-visual-slot="${role}"]`);
      await expect(slot).toHaveCount(1);
      await expect(slot).toHaveAttribute("data-action-type", /^(open-item|open-media|select-item|none)$/);
    }
    await expect(visual.locator('[data-visual-slot="lead"]')).toHaveAttribute("data-owner-item-id", "visual-fixture-owner");

    await slots.nth(0).click();
    await expect(appPage).toHaveURL(/#game-detail\?id=visual-fixture-owner$/);
  });

  test("Scene accepts keyboard and one-wheel navigation, then restores the saved mode", async ({ appPage }) => {
    await appPage.locator('[data-media-mode="scene"]').click();
    const scene = appPage.locator(".mw-v2-scene");
    await expect(scene).toBeVisible();
    const viewport = scene.locator(".mw-v2-scene__viewport");
    await viewport.focus();

    const beforeOwner = await scene.getAttribute("data-active-owner");
    await appPage.keyboard.press("ArrowDown");
    await expect(scene).not.toHaveAttribute("data-active-owner", beforeOwner || "");

    await viewport.dispatchEvent("wheel", { deltaY: 120, deltaX: 0, deltaMode: 0 });
    await expect(scene).toBeVisible();

    await appPage.reload();
    await expect(appPage.getByTestId("switch-home-scene")).toBeVisible();
    await expect(appPage.locator('[data-media-mode="scene"]')).toHaveAttribute("aria-pressed", "true");
  });
});



