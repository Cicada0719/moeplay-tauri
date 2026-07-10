import type { Locator, Page } from "@playwright/test";
import {
  DEFAULT_APP_STATE,
  MOCK_GAMES,
  expect,
  test,
  type GamepadButtonName,
  type GamepadController,
} from "./fixtures";

const responsiveState = {
  ...DEFAULT_APP_STATE,
  settings: {
    ...DEFAULT_APP_STATE.settings,
    startup_mode: "big-picture",
    theme: "cinema",
  },
  games: MOCK_GAMES.map((game, index) => ({
    ...game,
    metadata: {
      ...game.metadata,
      background: `C:\\FixtureArt\\responsive-${index + 1}.jpg`,
    },
  })),
};

test.use({ appState: responsiveState });

const bigPicture = (page: Page) => page.locator('section[aria-label="大屏模式"]');

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
  await page.waitForTimeout(80);
}

async function expectNoRootOverflow(page: Page): Promise<void> {
  const metrics = await page.evaluate(() => ({
    innerWidth: window.innerWidth,
    innerHeight: window.innerHeight,
    htmlWidth: document.documentElement.scrollWidth,
    htmlHeight: document.documentElement.scrollHeight,
    bodyWidth: document.body.scrollWidth,
    bodyHeight: document.body.scrollHeight,
  }));

  expect(metrics.htmlWidth).toBeLessThanOrEqual(metrics.innerWidth + 1);
  expect(metrics.bodyWidth).toBeLessThanOrEqual(metrics.innerWidth + 1);
  expect(metrics.htmlHeight).toBeLessThanOrEqual(metrics.innerHeight + 1);
  expect(metrics.bodyHeight).toBeLessThanOrEqual(metrics.innerHeight + 1);
}

async function expectInsideViewport(locator: Locator, page: Page, label: string): Promise<void> {
  await expect(locator, `${label} should be visible`).toBeVisible();
  const [box, viewport] = await Promise.all([
    locator.boundingBox(),
    page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight })),
  ]);
  expect(box, `${label} should have a layout box`).not.toBeNull();
  if (!box) return;
  expect(box.x, `${label} left edge`).toBeGreaterThanOrEqual(-1);
  expect(box.y, `${label} top edge`).toBeGreaterThanOrEqual(-1);
  expect(box.x + box.width, `${label} right edge`).toBeLessThanOrEqual(viewport.width + 1);
  expect(box.y + box.height, `${label} bottom edge`).toBeLessThanOrEqual(viewport.height + 1);
}

async function expectHomeLayout(page: Page): Promise<void> {
  const root = bigPicture(page);
  await expect(root).toBeVisible();
  await expect(root).toHaveAttribute("data-active-zone", "wheel");
  await expectNoRootOverflow(page);

  const selected = page.getByRole("listbox", { name: "大屏游戏列表" }).locator('[aria-selected="true"]');
  await expect(selected).toHaveCount(1);
  await expect(selected).toBeFocused();
  await expect(selected).toHaveAttribute("tabindex", "0");

  await expectInsideViewport(page.locator('[data-focus-zone="wheel"]'), page, "game wheel");
  await expectInsideViewport(page.locator(".bp-top"), page, "couch header");
  await expectInsideViewport(page.locator('[data-focus-zone="hero"]'), page, "hero content");
  await expectInsideViewport(page.locator(".bp-hints"), page, "gamepad hints");
}

async function expectNoRunningAnimations(page: Page): Promise<void> {
  await expect.poll(() => page.evaluate(() => document.getAnimations()
    .filter((animation) => {
      if (animation.playState !== "running" && animation.playState !== "pending") return false;
      const endTime = animation.effect?.getComputedTiming().endTime;
      // Ignore decorative infinite loops; this assertion targets finite transitions stuck mid-state.
      return typeof endTime === "number" && Number.isFinite(endTime);
    })
    .length)).toBe(0);
}

test.describe("Big Picture responsive matrix", () => {
  test("1080p keeps home and media focus targets inside the couch safe area", async ({
    appPage: page,
    gamepad,
  }, testInfo) => {
    test.skip(testInfo.project.name !== "couch-1080p", "covered by the couch-1080p project");
    await expectHomeLayout(page);

    await connectGamepad(gamepad, page);
    await pressGamepad(gamepad, page, "rightBumper");
    const media = page.locator('[data-focus-zone="media"][data-active="true"]');
    await expectInsideViewport(media, page, "media surface");
    const focusedPanel = media.locator('[data-media-index][tabindex="0"]');
    await expect(focusedPanel).toHaveCount(1);
    await expect(focusedPanel).toBeFocused();
    await expectInsideViewport(focusedPanel, page, "focused media panel");
    await expectNoRootOverflow(page);
  });

  test("4K keeps the wheel, hero and hints bounded without root overflow", async ({ appPage: page }, testInfo) => {
    test.skip(testInfo.project.name !== "couch-4k", "covered by the couch-4k project");
    await expectHomeLayout(page);

    const viewport = page.viewportSize();
    expect(viewport).toEqual({ width: 3840, height: 2160 });
    const heroTitle = page.locator(".bp-title");
    await expect(heroTitle).toBeVisible();
    await expectInsideViewport(heroTitle, page, "4K hero title");
  });

  test("1280x720 low-height mode leaves the hero actions and footer reachable", async ({ appPage: page }, testInfo) => {
    test.skip(testInfo.project.name !== "low-height", "covered by the low-height project");
    await expectHomeLayout(page);

    const viewport = page.viewportSize();
    expect(viewport).toEqual({ width: 1280, height: 720 });
    await expectInsideViewport(page.getByRole("toolbar", { name: "游戏操作" }), page, "hero actions");
    await expectInsideViewport(page.locator(".bp-pos"), page, "wheel position indicator");
  });

  test("21:9 ultrawide viewport preserves bounded content and focus", async ({ appPage: page }, testInfo) => {
    test.skip(testInfo.project.name !== "desktop-standard", "run the explicit ultrawide viewport once");
    await page.setViewportSize({ width: 3440, height: 1440 });
    await expectHomeLayout(page);

    expect(page.viewportSize()).toEqual({ width: 3440, height: 1440 });
    const layout = page.locator(".bp-layout");
    await expectInsideViewport(layout, page, "21:9 layout");
  });

  test("reduced motion swaps backgrounds without a cross-fade intermediate", async ({
    appPage: page,
    gamepad,
  }, testInfo) => {
    test.skip(testInfo.project.name !== "reduced-motion", "covered by the reduced-motion project");
    await expectHomeLayout(page);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect.poll(() => page.evaluate(() => matchMedia("(prefers-reduced-motion: reduce)").matches)).toBe(true);
    await connectGamepad(gamepad, page);

    await pressGamepad(gamepad, page, "dpadDown");
    await expect(page.locator(".bp-bg-layer-prev")).toHaveCount(0);
    await expect(page.locator(".bp-bg-layer-current")).not.toHaveClass(/fade-in/);
    await expectNoRunningAnimations(page);
  });

  test("reduced motion opens detail in its settled final state", async ({
    appPage: page,
    gamepad,
  }, testInfo) => {
    test.skip(testInfo.project.name !== "reduced-motion", "covered by the reduced-motion project");
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect.poll(() => page.evaluate(() => matchMedia("(prefers-reduced-motion: reduce)").matches)).toBe(true);
    await connectGamepad(gamepad, page);
    await pressGamepad(gamepad, page, "dpadDown");
    await pressGamepad(gamepad, page, "y");

    const detail = page.getByRole("dialog", { name: "夏日列车" });
    await expect(detail).toBeVisible();
    const detailStyle = await detail.evaluate((node) => {
      const style = getComputedStyle(node);
      return { opacity: style.opacity, transform: style.transform };
    });
    expect(detailStyle.opacity).toBe("1");
    expect(["none", "matrix(1, 0, 0, 1, 0, 0)"]).toContain(detailStyle.transform);
    await expectNoRunningAnimations(page);
  });

  test("reduced motion opens search in its settled final state", async ({
    appPage: page,
    gamepad,
  }, testInfo) => {
    test.skip(testInfo.project.name !== "reduced-motion", "covered by the reduced-motion project");
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect.poll(() => page.evaluate(() => matchMedia("(prefers-reduced-motion: reduce)").matches)).toBe(true);
    await connectGamepad(gamepad, page);

    await pressGamepad(gamepad, page, "b");
    await pressGamepad(gamepad, page, "dpadRight");
    await pressGamepad(gamepad, page, "dpadRight");
    await pressGamepad(gamepad, page, "a");

    const search = page.getByRole("dialog", { name: "搜索游戏" });
    const overlay = page.locator(".bps-overlay");
    await expect(search).toBeVisible();
    await expect(overlay).toHaveCSS("opacity", "1");
    await expectNoRunningAnimations(page);
  });
});
