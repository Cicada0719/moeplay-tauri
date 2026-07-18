import { expect, test, type GamepadButtonName, type GamepadController, type Page } from "./fixtures";

async function connect(gamepad: GamepadController, page: Page) {
  await gamepad.connect();
  await page.waitForTimeout(120);
}

async function press(gamepad: GamepadController, page: Page, button: GamepadButtonName) {
  await gamepad.press(button, 90);
  await page.waitForTimeout(120);
}

async function expectVisibleControllerFocus(page: Page) {
  const focused = page.locator(":focus");
  await expect(focused).toHaveCount(1);
  await expect(focused).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-input-mode", "gamepad");
  return focused;
}

async function expectFocusInsideRoute(page: Page, view: string) {
  await expect.poll(() => page.evaluate((routeView) => {
    const route = document.querySelector(`[data-route-view="${routeView}"]`);
    return Boolean(route && document.activeElement && route.contains(document.activeElement));
  }, view)).toBe(true);
}

test.describe("normal-mode controller navigation", () => {
  test("D-pad reaches the global navigation and A activates the focused module", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);

    await press(gamepad, page, "dpadRight");
    await expectVisibleControllerFocus(page);
    const recordsButton = page.locator('[data-nav-view="records"]');
    await expect(recordsButton).toBeFocused();

    await press(gamepad, page, "a");
    await expect(page.locator('[data-route-view="records"]')).toBeVisible();
  });

  test("shoulder buttons cycle every primary page and D-pad enters each page", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);
    for (const view of ["records", "anime", "comic", "novel", "home"]) {
      await press(gamepad, page, "rightBumper");
      await expect(page.locator(`[data-route-view="${view}"]`)).toBeVisible();
      await press(gamepad, page, "dpadDown");
      await expectVisibleControllerFocus(page);
      await expectFocusInsideRoute(page, view);
    }

    await press(gamepad, page, "leftBumper");
    await expect(page.locator('[data-route-view="novel"]')).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-input-mode", "gamepad");
  });

  test("X focuses the current route search input", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);
    await press(gamepad, page, "x");
    await expect(page.locator("#library-search")).toBeFocused();
    await expect(page.locator("html")).toHaveAttribute("data-input-mode", "gamepad");
  });

  test("D-pad traverses utility actions, A opens tools, and B closes the drawer", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);

    for (let index = 0; index < 8; index++) await press(gamepad, page, "dpadRight");
    await expect(page.locator('[data-nav-action="tools"]')).toBeFocused();

    await press(gamepad, page, "a");
    await expect(page.getByRole("dialog", { name: "\u5de5\u5177" })).toBeVisible();

    await press(gamepad, page, "b");
    await expect(page.getByRole("dialog", { name: "\u5de5\u5177" })).toHaveCount(0);
  });
});
