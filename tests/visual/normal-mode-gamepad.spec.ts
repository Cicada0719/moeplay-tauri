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

  test("controller switches games, opens the active archive, and exposes contextual HUD actions", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);
    await expect(page.getByTestId("gamepad-hints")).toContainText("手柄已连接");

    await press(gamepad, page, "dpadDown");
    await expect(page.locator("#library-search")).toBeFocused();
    await press(gamepad, page, "dpadDown");
    await expect(page.getByRole("button", { name: /折叠/ })).toBeFocused();
    await press(gamepad, page, "dpadDown");

    const firstGame = page.locator('[data-directory-game="fixture-game-1"]');
    const secondGame = page.locator('[data-directory-game="fixture-game-2"]');
    await expect(firstGame).toBeFocused();
    await expect(page.getByTestId("gamepad-hints")).toContainText("打开 星海回声 档案");
    await expect(page.getByTestId("gamepad-hints")).toContainText("Y");

    await press(gamepad, page, "dpadDown");
    await expect(secondGame).toBeFocused();
    await expect(secondGame).toHaveAttribute("data-gamepad-activate", "切换游戏");
    await press(gamepad, page, "a");
    await expect(page.getByTestId("game-unified-stage")).toHaveAttribute("data-selected-game", "fixture-game-2");
    await expect(secondGame).toHaveAttribute("data-gamepad-activate", "打开档案");

    const favorite = page.locator('[data-gamepad-secondary-action]');
    await expect(favorite).toContainText("收藏");
    await press(gamepad, page, "y");
    await expect(favorite).toContainText("已收藏");

    await press(gamepad, page, "a");
    await expect(page.locator('[data-route-view="game-detail"]')).toBeVisible();
    await expect(page.locator(".game-detail-primary")).toBeFocused();
    await expect(page.getByTestId("gamepad-hints")).toContainText("抓取元数据");
    await expect(page.locator(".game-detail-panel .body")).toBeVisible();
    await press(gamepad, page, "back");
    await expect(page.locator(".game-detail-panel .body")).toBeHidden();
    await expect(page.locator(".game-detail-panel .hero")).toBeVisible();
    await press(gamepad, page, "back");
    await expect(page.locator(".game-detail-panel .body")).toBeVisible();
    await press(gamepad, page, "b");
    await expect(page.locator('[data-route-view="home"]')).toBeVisible();
  });

  test("View toggles a persistent per-category focus layout and the HUD explains how to restore controls", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);
    const shell = page.getByTestId("app-shell");

    await press(gamepad, page, "back");
    await expect(shell).toHaveAttribute("data-workspace-focus", "true");
    await expect(shell).toHaveAttribute("data-workspace-focus-view", "home");
    await expect(page.locator(".mw-shell__topline")).toBeHidden();
    await expect(page.locator(".mw-shell__director")).toBeHidden();
    await expect(page.getByTestId("gamepad-hints")).toContainText("显示控件");

    await press(gamepad, page, "rightBumper");
    await expect(page.locator('[data-route-view="records"]')).toBeVisible();
    await expect(shell).not.toHaveAttribute("data-workspace-focus", "true");
    await press(gamepad, page, "leftBumper");
    await expect(page.locator('[data-route-view="home"]')).toBeVisible();
    await expect(shell).toHaveAttribute("data-workspace-focus-view", "home");

    await press(gamepad, page, "back");
    await expect(shell).not.toHaveAttribute("data-workspace-focus", "true");
    await expect(page.locator(".mw-shell__topline")).toBeVisible();
  });

  test("comic source tabs and compact layout are operable without mouse or keyboard", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);
    for (let index = 0; index < 3; index++) await press(gamepad, page, "rightBumper");
    await expect(page.locator('[data-route-view="comic"]')).toBeVisible();
    await page.waitForTimeout(500);

    await press(gamepad, page, "x");
    await expect(page.getByRole("searchbox", { name: "搜索普通漫画" })).toBeFocused();
    await press(gamepad, page, "dpadRight");
    const automatic = page.getByRole("tab", { name: /自动/ });
    await expect(automatic).toBeFocused();
    await press(gamepad, page, "dpadRight");
    const mangaDex = page.getByRole("tab", { name: /MangaDex/ });
    await expect(mangaDex).toBeFocused();
    await press(gamepad, page, "a");
    await expect(mangaDex).toHaveAttribute("aria-selected", "true");

    await press(gamepad, page, "back");
    await expect(page.getByTestId("app-shell")).toHaveAttribute("data-workspace-focus-view", "comic");
    await expect(page.locator(".comic-v2-shell .v2-page-header")).toBeHidden();
    await expect(page.locator(".comic-filter-bar")).toBeHidden();
    await press(gamepad, page, "back");
    await expect(page.locator(".comic-filter-bar")).toBeVisible();
  });

  test("anime and novel internal tabs expose deterministic controller paths and contextual hints", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);
    await press(gamepad, page, "rightBumper");
    await press(gamepad, page, "rightBumper");
    await expect(page.locator('[data-route-view="anime"]')).toBeVisible();

    await press(gamepad, page, "x");
    const animeSearch = page.locator('input[data-search-scope="anime"]');
    await expect(animeSearch).toBeFocused();
    await press(gamepad, page, "dpadDown");
    const recommendTab = page.locator("#anime-tab-recommend");
    await expect(recommendTab).toBeFocused();
    await press(gamepad, page, "dpadRight");
    const calendarTab = page.locator("#anime-tab-calendar");
    await expect(calendarTab).toBeFocused();
    await press(gamepad, page, "a");
    await expect(calendarTab).toHaveAttribute("aria-selected", "true");
    await expect(page.getByTestId("gamepad-hints")).toContainText("切换栏目");

    await press(gamepad, page, "rightBumper");
    await press(gamepad, page, "rightBumper");
    await expect(page.locator('[data-route-view="novel"]')).toBeVisible();
    const novelSearch = page.locator('input[data-search-scope="novel"]');
    await expect(novelSearch).toBeVisible();
    await press(gamepad, page, "x");
    await expect(novelSearch).toBeFocused();
    await press(gamepad, page, "dpadDown");
    const allSources = page.locator("#novel-source-tab-all");
    await expect(allSources).toBeFocused();
    await press(gamepad, page, "dpadRight");
    const gutenberg = page.locator("#novel-source-tab-gutenberg");
    await expect(gutenberg).toBeFocused();
    await press(gamepad, page, "a");
    await expect(gutenberg).toHaveAttribute("aria-selected", "true");
    await expect(page.getByTestId("gamepad-hints")).toContainText("切换来源");
  });

  test("every primary category can hide and restore secondary chrome with View", async ({ appPage: page, gamepad }) => {
    await connect(gamepad, page);
    const cases = [
      { view: "home", selector: ".mw-shell__topline" },
      { view: "records", selector: ".records-modebar" },
      { view: "anime", selector: ".editorial-chrome" },
      { view: "comic", selector: ".comic-filter-bar" },
      { view: "novel", selector: ".nv-searchbar" },
    ];

    for (const [index, item] of cases.entries()) {
      if (index > 0) await press(gamepad, page, "rightBumper");
      await expect(page.locator('[data-route-view="' + item.view + '"]')).toBeVisible();
      const chrome = page.locator(item.selector).first();
      await expect(chrome).toBeVisible();
      await press(gamepad, page, "back");
      await expect(page.getByTestId("app-shell")).toHaveAttribute("data-workspace-focus-view", item.view);
      await expect(chrome).toBeHidden();
      await press(gamepad, page, "back");
      await expect(chrome).toBeVisible();
    }
  });

});
