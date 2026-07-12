import { expect, test } from "./fixtures/app-fixture";
import { MEDIA_WORKSPACE_APP_STATE } from "./fixtures/v0134-fixtures";

test.use({ appState: MEDIA_WORKSPACE_APP_STATE });

test.describe("game creative stage contract", () => {
  test.beforeEach(async ({ appPage }) => {
    await expect(appPage.getByTestId("switch-home")).toBeVisible();
  });

  test("nodate-inspired Visual keeps media and a unique game directory on two faces", async ({ appPage }) => {
    await appPage.locator('[data-media-mode="visual"]').click();
    const stage = appPage.getByTestId("game-unified-stage");
    await expect(stage).toBeVisible();
    await expect(stage.locator(".nd-face--media")).toBeVisible();
    await expect(stage.locator(".nd-face--archive")).toBeVisible();
    await expect(stage.locator(".nd-media-map button")).toHaveCount(5);

    const titles = await stage.locator(".nd-directory button strong").allTextContents();
    expect(new Set(titles.map((title) => title.trim().toLocaleLowerCase("zh-CN"))).size).toBe(titles.length);

    await stage.locator(".nd-lead").click();
    await expect(appPage).toHaveURL(/#game-detail\?id=visual-fixture-owner$/);
  });

  test("Tao-inspired Scene renders one frame per unique game and switches selection", async ({ appPage }) => {
    await appPage.locator('[data-media-mode="scene"]').click();
    const scene = appPage.getByTestId("game-film-sequence");
    await expect(scene).toBeVisible();

    const frames = scene.locator("[data-film-game]");
    const ids = await frames.evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-film-game")));
    expect(new Set(ids).size).toBe(ids.length);

    await scene.focus();
    const beforeOwner = await scene.getAttribute("data-active-owner");
    await appPage.keyboard.press("ArrowDown");
    await expect(scene).not.toHaveAttribute("data-active-owner", beforeOwner || "");

    const afterKeyboard = await scene.getAttribute("data-active-owner");
    await scene.dispatchEvent("wheel", { deltaY: 80, deltaX: 0, deltaMode: 0 });
    await expect(scene).not.toHaveAttribute("data-active-owner", afterKeyboard || "");

    await appPage.reload();
    await expect(appPage.getByTestId("switch-home-scene")).toBeVisible();
    await expect(appPage.locator('[data-media-mode="scene"]')).toHaveAttribute("aria-pressed", "true");
  });
});

