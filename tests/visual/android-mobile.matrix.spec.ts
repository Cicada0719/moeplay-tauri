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
    const nav = viewport.nav === "bottom" ? page.locator(".mobile-bottom-nav") : page.locator(".mobile-rail");
    await expect(nav).toBeVisible();
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

test("Android settings hides desktop platform integrations", async ({ page }) => {
  await page.setViewportSize({ width: 412, height: 915 });
  await page.goto("/?skip_wizard&platform=android#settings");
  await expect(page.locator('[data-route-view="settings"]').last()).toBeVisible();
  await expect(page.getByText("手机版只管理游戏资料与同步数据", { exact: false })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("Steam / Epic 导入", { exact: true })).toHaveCount(0);
  await expect(page.getByText("自动", { exact: true })).toBeVisible();
  await expect(page.getByText("竖屏", { exact: true })).toBeVisible();
  await expect(page.getByText("横屏", { exact: true })).toBeVisible();
});
