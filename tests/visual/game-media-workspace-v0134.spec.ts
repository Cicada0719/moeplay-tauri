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

  test("cover-wall delete confirmation is centered against the viewport instead of a card cell", async ({ appPage }) => {
    await appPage.locator('[data-media-mode="index"]').click();
    const deleteButton = appPage.locator('button[aria-label^="删除 "]').first();
    await expect(deleteButton).toBeVisible();
    await deleteButton.click();

    const root = appPage.locator('[data-testid^="delete-dialog-"]');
    const dialog = root.getByRole("alertdialog");
    await expect(dialog).toBeVisible();
    const placement = await root.evaluate((node) => {
      const rect = node.getBoundingClientRect();
      const dialogRect = node.querySelector("dialog")!.getBoundingClientRect();
      return {
        parent: node.parentElement?.tagName,
        root: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        dialogCenter: { x: dialogRect.x + dialogRect.width / 2, y: dialogRect.y + dialogRect.height / 2 },
        viewport: { width: innerWidth, height: innerHeight },
      };
    });

    expect(placement.parent).toBe("BODY");
    expect(Math.abs(placement.root.x)).toBeLessThan(1);
    expect(Math.abs(placement.root.y)).toBeLessThan(1);
    expect(Math.abs(placement.root.width - placement.viewport.width)).toBeLessThan(1);
    expect(Math.abs(placement.root.height - placement.viewport.height)).toBeLessThan(1);
    expect(Math.abs(placement.dialogCenter.x - placement.viewport.width / 2)).toBeLessThan(2);
    expect(Math.abs(placement.dialogCenter.y - placement.viewport.height / 2)).toBeLessThan(2);

    await dialog.getByRole("button", { name: "取消" }).click();
    await expect(dialog).toHaveCount(0);
  });

  test("focus layout restores the archive control without framing the title or shifting the cover", async ({ appPage, gamepad }) => {
    await appPage.locator('[data-media-mode="visual"]').click();
    await gamepad.connect();
    await appPage.waitForTimeout(120);

    const shell = appPage.getByTestId("app-shell");
    const stage = appPage.getByTestId("game-unified-stage");
    const activeGame = appPage.locator('[data-directory-game="visual-fixture-owner"]');
    const cover = appPage.locator(".nd-cover-window");

    await gamepad.press("back", 90);
    await appPage.waitForTimeout(120);
    await expect(shell).toHaveAttribute("data-workspace-focus", "true");
    await expect(stage).toHaveAttribute("data-controller-surface", "");
    await activeGame.focus();
    const before = await cover.boundingBox();
    expect(before).not.toBeNull();

    await gamepad.press("a", 90);
    await appPage.waitForTimeout(120);
    await expect(appPage.locator('[data-route-view="game-detail"]')).toBeVisible();
    await gamepad.press("b", 90);
    await appPage.waitForTimeout(120);
    await expect(appPage.locator('[data-route-view="home"]')).toBeVisible();
    // 0.19.7: the stage root is the stable gamepad focus anchor; per-game
    // directory buttons are re-keyed on every selection change and never hold it.
    await expect(stage).toBeFocused();
    await expect(appPage.locator(".nd-title-block h1")).not.toHaveAttribute("tabindex");

    const after = await cover.boundingBox();
    expect(after).not.toBeNull();
    expect(Math.abs(after!.x - before!.x)).toBeLessThan(2);
    expect(Math.abs(after!.y - before!.y)).toBeLessThan(2);
    expect(Math.abs(after!.width - before!.width)).toBeLessThan(2);
    expect(Math.abs(after!.height - before!.height)).toBeLessThan(2);

    await stage.focus();
    await expect(stage).toBeFocused();
    await expect.poll(() => stage.evaluate((node) => {
      const style = getComputedStyle(node);
      return { outline: style.outlineStyle, shadow: style.boxShadow };
    })).toEqual({ outline: "none", shadow: "none" });
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

