import type { Locator, Page } from "@playwright/test";
import {
  DEFAULT_APP_STATE,
  MOCK_GAMES,
  expect,
  getInvocationLog,
  test,
  type GamepadButtonName,
  type GamepadController,
} from "./fixtures";

const gamesWithDistinctBackgrounds = MOCK_GAMES.map((game, index) => ({
  ...game,
  metadata: {
    ...game.metadata,
    background: `C:\\FixtureArt\\big-picture-${index + 1}.jpg`,
  },
}));

const desktopState = {
  ...DEFAULT_APP_STATE,
  settings: {
    ...DEFAULT_APP_STATE.settings,
    startup_mode: "fullscreen",
    theme: "cinema",
  },
  games: gamesWithDistinctBackgrounds,
  commandResults: {
    launch_game: null,
    toggle_favorite: null,
  },
};

const bigPictureState = {
  ...desktopState,
  settings: {
    ...desktopState.settings,
    startup_mode: "big-picture",
  },
};

const bigPicture = (page: Page) => page.locator('section[aria-label="大屏模式"]');
const wheel = (page: Page) => page.getByRole("listbox", { name: "大屏游戏列表" });

async function connectGamepad(gamepad: GamepadController, page: Page): Promise<void> {
  await gamepad.connect();
  await page.waitForTimeout(100);
}

async function pressGamepad(
  gamepad: GamepadController,
  page: Page,
  button: GamepadButtonName,
): Promise<void> {
  await gamepad.press(button, 80);
  // The shared runtime intentionally waits for a neutral frame after a zone/scope change.
  await page.waitForTimeout(80);
}

async function expectRovingSelection(
  container: Locator,
  selectedSelector = '[aria-selected="true"]',
): Promise<Locator> {
  const selected = container.locator(selectedSelector);
  await expect(selected).toHaveCount(1);
  await expect(selected).toBeVisible();
  await expect(selected).toBeFocused();
  await expect(selected).toHaveAttribute("tabindex", "0");
  return selected;
}

async function expectFocusedIndex(container: Locator, attribute: string): Promise<string> {
  const roving = container.locator(`[${attribute}][tabindex="0"]`);
  await expect(roving).toHaveCount(1);
  await expect(roving).toBeFocused();
  return (await roving.getAttribute(attribute)) ?? "";
}

async function openSearchWithGamepad(
  page: Page,
  gamepad: GamepadController,
): Promise<Locator> {
  const root = bigPicture(page);
  await pressGamepad(gamepad, page, "b");
  await expect(root).toHaveAttribute("data-active-zone", "top-nav");
  await pressGamepad(gamepad, page, "dpadRight");
  await pressGamepad(gamepad, page, "dpadRight");
  await expect(page.getByRole("button", { name: "搜索游戏" })).toHaveAttribute("tabindex", "0");
  await pressGamepad(gamepad, page, "a");
  const search = page.getByRole("dialog", { name: "搜索游戏" });
  await expect(search).toBeVisible();
  return search;
}

test.describe("Big Picture gamepad-only entry", () => {
  test.use({ appState: desktopState });

  test("Start enters Big Picture without mouse or keyboard input", async ({ appPage: page, gamepad }) => {
    await expect(page.getByTestId("switch-home")).toBeVisible();
    await expect(bigPicture(page)).toHaveCount(0);

    await connectGamepad(gamepad, page);
    await pressGamepad(gamepad, page, "start");

    await expect(bigPicture(page)).toBeVisible();
    await expect(bigPicture(page)).toHaveAttribute("data-active-zone", "wheel");
    await expectRovingSelection(wheel(page));
  });
});

test.describe("Big Picture gamepad zone navigation", () => {
  test.use({ appState: bigPictureState });

  test("wheel and hero open detail, launch, return and preserve the selected DOM focus", async ({
    appPage: page,
    gamepad,
  }) => {
    const root = bigPicture(page);
    const gameWheel = wheel(page);
    await expect(root).toBeVisible();
    await expect(root).toHaveAttribute("data-active-zone", "wheel");
    await connectGamepad(gamepad, page);

    await expect((await expectRovingSelection(gameWheel))).toHaveAccessibleName("星海回声");
    await pressGamepad(gamepad, page, "dpadDown");
    const secondGame = await expectRovingSelection(gameWheel);
    await expect(secondGame).toHaveAccessibleName("夏日列车");
    await expect(secondGame).toHaveAttribute("aria-current", "true");

    await pressGamepad(gamepad, page, "dpadRight");
    await expect(root).toHaveAttribute("data-active-zone", "hero");
    const hero = page.locator('[data-focus-zone="hero"][data-active="true"]');
    const launchFromHero = hero.getByRole("button", { name: "开始游戏" });
    await expect(launchFromHero).toBeFocused();
    await expect(launchFromHero).toHaveAttribute("tabindex", "0");

    await pressGamepad(gamepad, page, "dpadRight");
    await pressGamepad(gamepad, page, "dpadRight");
    const heroDetail = hero.getByRole("button", { name: "详情" });
    await expect(heroDetail).toBeFocused();
    await expect(heroDetail).toHaveClass(/zone-focus/);

    await pressGamepad(gamepad, page, "a");
    const detail = page.getByRole("dialog", { name: "夏日列车" });
    await expect(detail).toBeVisible();
    await expect(root).toHaveAttribute("data-active-zone", "detail");
    const detailLaunch = detail.getByRole("button", { name: "启动", exact: true });
    await expect(detailLaunch).toBeFocused();

    await pressGamepad(gamepad, page, "a");
    await expect.poll(async () => (
      await getInvocationLog(page)
    ).filter(({ command }) => command === "launch_game").length).toBe(1);

    await pressGamepad(gamepad, page, "b");
    await expect(detail).toBeHidden();
    await expect(root).toHaveAttribute("data-active-zone", "hero");
    await expect(heroDetail).toBeFocused();

    await pressGamepad(gamepad, page, "b");
    await expect(root).toHaveAttribute("data-active-zone", "wheel");
    await expect((await expectRovingSelection(gameWheel))).toHaveAccessibleName("夏日列车");
  });

  test("shoulder buttons enter media and keep its roving item focused", async ({ appPage: page, gamepad }) => {
    const root = bigPicture(page);
    const gameWheel = wheel(page);
    await connectGamepad(gamepad, page);
    await pressGamepad(gamepad, page, "dpadDown");
    await expect((await expectRovingSelection(gameWheel))).toHaveAccessibleName("夏日列车");

    await pressGamepad(gamepad, page, "rightBumper");
    await expect(root).toHaveAttribute("data-active-zone", "media");
    const media = page.locator('[data-focus-zone="media"][data-active="true"]');
    await expect(media).toBeVisible();
    expect(await expectFocusedIndex(media, "data-media-index")).toBe("0");

    await pressGamepad(gamepad, page, "dpadRight");
    expect(await expectFocusedIndex(media, "data-media-index")).toBe("1");

    await pressGamepad(gamepad, page, "leftBumper");
    await expect(root).toHaveAttribute("data-active-zone", "wheel");
    await expect((await expectRovingSelection(gameWheel))).toHaveAccessibleName("夏日列车");
  });

  test("top navigation selected item and DOM focus stay on the same search control", async ({
    appPage: page,
    gamepad,
  }) => {
    await connectGamepad(gamepad, page);
    await pressGamepad(gamepad, page, "b");
    await expect(bigPicture(page)).toHaveAttribute("data-active-zone", "top-nav");
    await expect(page.getByRole("navigation", { name: "大屏分类" }).getByRole("button", { name: "游戏", exact: true })).toBeFocused();

    await pressGamepad(gamepad, page, "dpadRight");
    await pressGamepad(gamepad, page, "dpadRight");
    const searchTrigger = page.getByRole("button", { name: "搜索游戏" });
    await expect(searchTrigger).toHaveAttribute("tabindex", "0");
    await expect(searchTrigger).toBeFocused();
  });

  test("search and virtual keyboard own the gamepad, synchronize results, then exit", async ({
    appPage: page,
    gamepad,
  }) => {
    const root = bigPicture(page);
    const gameWheel = wheel(page);
    await connectGamepad(gamepad, page);
    await pressGamepad(gamepad, page, "dpadDown");
    await expect((await expectRovingSelection(gameWheel))).toHaveAccessibleName("夏日列车");

    const searchDialog = await openSearchWithGamepad(page, gamepad);
    const searchInput = searchDialog.locator("#bp-search-input");
    await expect(searchInput).toBeFocused();
    await expect(searchDialog).toHaveAttribute("data-focus-zone", "search");

    await pressGamepad(gamepad, page, "dpadDown");
    const keyboard = searchDialog.getByRole("application", { name: "屏幕键盘" });
    const qKey = keyboard.getByRole("button", { name: "Q", exact: true });
    await expect(searchDialog).toHaveAttribute("data-focus-zone", "keyboard");
    await expect(qKey).toBeFocused();
    await expect(qKey).toHaveAttribute("aria-pressed", "true");

    // The keyboard overlay owns the handler: moving within it must not change the underlying wheel.
    await expect(gameWheel.locator('[aria-selected="true"]')).toHaveAccessibleName("夏日列车");
    await pressGamepad(gamepad, page, "a");
    await expect(searchInput).toHaveValue("Q");
    await pressGamepad(gamepad, page, "x");
    await expect(searchInput).toHaveValue("");

    await pressGamepad(gamepad, page, "dpadDown");
    await pressGamepad(gamepad, page, "dpadDown");
    await pressGamepad(gamepad, page, "dpadRight");
    const xKey = keyboard.getByRole("button", { name: "X", exact: true });
    await expect(xKey).toBeFocused();
    await pressGamepad(gamepad, page, "a");
    await expect(searchInput).toHaveValue("X");

    await pressGamepad(gamepad, page, "dpadUp");
    await pressGamepad(gamepad, page, "dpadUp");
    await pressGamepad(gamepad, page, "dpadUp");
    await expect(searchDialog).toHaveAttribute("data-focus-zone", "search");
    const results = searchDialog.getByRole("listbox", { name: "搜索结果" });
    let selectedResult = await expectRovingSelection(results);
    await expect(selectedResult).toHaveAccessibleName(/星海回声$/);

    await pressGamepad(gamepad, page, "dpadRight");
    selectedResult = await expectRovingSelection(results);
    await expect(selectedResult).toHaveAccessibleName(/夏日列车$/);
    await expect(gameWheel.locator('[aria-selected="true"]')).toHaveAccessibleName("夏日列车");

    await pressGamepad(gamepad, page, "a");
    await expect(searchDialog).toBeHidden();
    await expect(root).toHaveAttribute("data-active-zone", "wheel");
    await expect((await expectRovingSelection(gameWheel))).toHaveAccessibleName("夏日列车");

    await pressGamepad(gamepad, page, "b");
    await expect(root).toHaveAttribute("data-active-zone", "top-nav");
    await pressGamepad(gamepad, page, "b");
    await expect(root).toBeHidden();
    await expect(page.getByTestId("switch-home")).toBeVisible();
  });
});

