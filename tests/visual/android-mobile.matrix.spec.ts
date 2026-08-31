import { expect, test } from "@playwright/test";

const mobileViewports = [
  { name: "portrait-360x800", width: 360, height: 800, nav: "bottom" },
  { name: "portrait-412x915", width: 412, height: 915, nav: "bottom" },
  { name: "landscape-800x360", width: 800, height: 360, nav: "rail" },
  { name: "landscape-915x412", width: 915, height: 412, nav: "rail" },
] as const;

for (const viewport of mobileViewports) {
  test(`Android shell ${viewport.name}`, async ({ page }) => {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    await page.goto("/?skip_wizard&platform=android");
    // Android 首页是唯一的全屏掌机中心；移动主导航出现在其余浏览页面。
    await expect(page.getByTestId("handheld-page")).toBeVisible();
    await expect(page.locator(".mobile-bottom-nav, .mobile-rail")).toHaveCount(0);
    await page.goto("/?skip_wizard&platform=android#game-library");
    const nav = viewport.nav === "bottom" ? page.locator(".mobile-bottom-nav") : page.locator(".mobile-rail");
    await expect(nav).toBeVisible();
    await expect(nav.getByText("游戏", { exact: true })).toBeVisible();
    await expect(page.getByTestId("main-content")).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(1);
    for (const route of ["anime", "comic"] as const) {
      await page.goto(`/?skip_wizard&platform=android#${route}`);
      const routeView = page.locator(`[data-route-view="${route}"]`).last();
      await expect(routeView).toBeVisible();
      const routeOverflow = await routeView.evaluate((element) => element.scrollWidth - element.clientWidth);
      expect(routeOverflow, `${route} should fit ${viewport.name}`).toBeLessThanOrEqual(1);
    }
  });
}

test("Android legacy handheld route redirects to the unified home", async ({ page }) => {
  await page.setViewportSize({ width: 808, height: 454 });
  await page.goto("/?skip_wizard&platform=android#handheld");
  await expect(page).toHaveURL(/#home$/);
  await expect(page.getByTestId("handheld-page").last()).toBeVisible();
});

test("Android home exposes the PSP-style XMB focus surfaces", async ({ page }) => {
  await page.setViewportSize({ width: 808, height: 454 });
  await page.goto("/?skip_wizard&platform=android#home");

  await expect(page.locator(".hh-xmb-page")).toBeVisible();
  await expect(page.locator(".hh-xmb-category")).toHaveCount(5);
  await expect(page.locator(".hh-xmb-category.active")).toHaveCount(1);
  await expect(page.locator(".hh-xmb-hints")).toBeVisible();
  await expect(page.locator("[data-testid='handheld-xmb-welcome'], [data-testid='handheld-xmb-selection'], [data-testid='handheld-game-wheel'], .hh-state-panel")).toHaveCount(1);
  await expect.poll(() => page.evaluate(() => document.activeElement?.classList.contains("hh-xmb-category"))).toBe(true);

  await page.keyboard.press("ArrowRight");
  await expect(page.locator(".hh-xmb-category.active")).toHaveCount(1);
  await expect(page.locator(".hh-xmb-category.active .hh-system-label")).toBeVisible();
});

test("Android game channel exposes the second-level emulator platform strip", async ({ page }) => {
  await page.setViewportSize({ width: 808, height: 454 });
  await page.goto("/?skip_wizard&platform=android#home");
  await page.locator(".hh-xmb-category").filter({ hasText: "游戏" }).click();
  await expect(page.getByTestId("handheld-game-platforms")).toBeVisible();
  await expect(page.getByTestId("handheld-game-platforms").locator("[data-platform-index]").first()).toBeVisible();
  await expect(page.getByTestId("handheld-game-wheel")).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("[data-testid='handheld-game-wheel'] [data-game-index]").first()).toBeVisible();
  await page.keyboard.press("PageDown");
  await expect(page.locator("[data-testid='handheld-game-platforms'] button.active")).toHaveCount(1);
});

test("Android XMB channel strip hides after interaction and can be restored", async ({ page }) => {
  await page.setViewportSize({ width: 808, height: 454 });
  await page.goto("/?skip_wizard&platform=android#home");

  await page.locator(".hh-xmb-category").filter({ hasText: "漫画" }).click();
  await expect(page.locator(".hh-xmb-category")).toHaveCount(5);
  await expect(page.locator(".hh-xmb-category")).toHaveCount(0, { timeout: 3_800 });
  await expect(page.getByRole("button", { name: "显示掌机频道" })).toBeVisible();
  await page.getByRole("button", { name: "显示掌机频道" }).click();
  await expect(page.locator(".hh-xmb-category")).toHaveCount(5);
});

test("Android home keeps the game library and emulator import as first-class actions", async ({ page }) => {
  await page.setViewportSize({ width: 808, height: 454 });
  await page.goto("/?skip_wizard&platform=android#home");

  await expect(page.getByTestId("handheld-game-library-entry")).toBeVisible();
  await expect(page.getByTestId("handheld-emulator-import-entry")).toBeVisible();

  await page.getByTestId("handheld-game-library-entry").click();
  await expect(page).toHaveURL(/#game-library$/);
  await page.goto("/?skip_wizard&platform=android#home");
  await page.getByTestId("handheld-emulator-import-entry").click();
  await expect(page).toHaveURL(/#handheld-import$/);
});

test("Android settings hides desktop platform integrations", async ({ page }) => {
  await page.setViewportSize({ width: 412, height: 915 });
  await page.goto("/?skip_wizard&platform=android#settings");
  await expect(page.locator('[data-route-view="settings"]').last()).toBeVisible();
  await expect(page.getByTestId("android-settings-center")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("设备偏好", { exact: true })).toBeVisible();
  await expect(page.getByText("Steam / Epic 导入", { exact: true })).toHaveCount(0);
  await expect(page.getByText("沉浸式系统栏", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "竖屏", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "横屏", exact: true })).toBeVisible();
});

test("Android media browse shell keeps a touch back affordance", async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 360 });
  await page.goto("/?skip_wizard&platform=android#anime");
  await expect(page.getByTestId("handheld-media-shell")).toBeVisible();
  const backDock = page.locator(".hms-back");
  await expect(backDock).toBeVisible({ timeout: 5_000 });
  const rect = await backDock.boundingBox();
  expect(rect).not.toBeNull();
  expect(rect?.x).toBeGreaterThanOrEqual(0);
  expect(rect?.y).toBeGreaterThanOrEqual(0);
  await backDock.click();
  await expect(page).toHaveURL(/#home$/);
});

test("Android handheld terminal matrix stays within the viewport", async ({ page }) => {
  for (const size of [
    { width: 808, height: 454 },
    { width: 960, height: 540 },
    { width: 1280, height: 720 },
  ]) {
    await page.setViewportSize(size);
    for (const route of ["home", "game-library", "anime", "comic", "novel", "settings"] as const) {
      await page.goto(`/?skip_wizard&platform=android#${route}`);
      const routeView = page.locator(`[data-route-view="${route}"]`).last();
      await expect(routeView).toBeVisible();
      const metrics = await routeView.evaluate((element) => ({
        horizontalOverflow: element.scrollWidth - element.clientWidth,
        verticalOverflow: element.scrollHeight - element.clientHeight,
        rawTypeError: element.textContent?.includes("TypeError") ?? false,
      }));
      expect(metrics.horizontalOverflow, `${route} overflows at ${size.width}x${size.height}`).toBeLessThanOrEqual(1);
      expect(metrics.rawTypeError, `${route} exposes a raw TypeError at ${size.width}x${size.height}`).toBe(false);
    }
  }
});
