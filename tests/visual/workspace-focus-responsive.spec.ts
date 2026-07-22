import { expect, test } from "./fixtures";

async function navigateTo(page: import("@playwright/test").Page, view: string) {
  let target = page.locator(`[data-nav-view="${view}"]:visible`).first();
  if (await target.count() === 0) {
    await page.locator("#global-top-navigation-menu-toggle").click();
    target = page.locator(`[data-nav-view="${view}"]:visible`).first();
  }
  await target.click();
}

test.describe("workspace focus responsive shell", () => {
  test("keeps a visible recovery action and avoids horizontal clipping at high zoom-sized viewports", async ({ appPage: page }) => {
    await page.setViewportSize({ width: 520, height: 480 });

    const shell = page.getByTestId("app-shell");
    const main = page.getByTestId("main-content");
    const toggle = page.locator("[data-workspace-focus-toggle]");

    await expect(shell).toHaveAttribute("data-shell-mode", "standard");
    await expect(toggle).toBeVisible();
    await toggle.click();
    await expect(shell).toHaveAttribute("data-shell-mode", "focus");
    await expect(toggle).toHaveAttribute("data-state", "focus");
    await expect(toggle).toContainText("退出专注");

    const geometry = await page.evaluate(() => {
      const shellNode = document.querySelector('[data-testid="app-shell"]') as HTMLElement;
      const mainNode = document.querySelector('[data-testid="main-content"]') as HTMLElement;
      const toggleNode = document.querySelector('[data-workspace-focus-toggle]') as HTMLElement;
      const toggleRect = toggleNode.getBoundingClientRect();
      const mainRect = mainNode.getBoundingClientRect();
      return {
        shellOverflow: shellNode.scrollWidth - shellNode.clientWidth,
        mainOverflow: mainNode.scrollWidth - mainNode.clientWidth,
        mainRight: mainRect.right,
        toggleLeft: toggleRect.left,
        toggleRight: toggleRect.right,
        toggleBottom: toggleRect.bottom,
        viewportWidth: innerWidth,
        viewportHeight: innerHeight,
      };
    });

    expect(geometry.shellOverflow).toBeLessThanOrEqual(1);
    expect(geometry.mainOverflow).toBeLessThanOrEqual(1);
    expect(geometry.mainRight).toBeLessThanOrEqual(geometry.viewportWidth + 1);
    expect(geometry.toggleLeft).toBeGreaterThanOrEqual(0);
    expect(geometry.toggleRight).toBeLessThanOrEqual(geometry.viewportWidth);
    expect(geometry.toggleBottom).toBeLessThanOrEqual(geometry.viewportHeight);

    await toggle.click();
    await expect(shell).toHaveAttribute("data-shell-mode", "standard");
    await expect(page.locator(".mw-shell__topline")).toBeVisible();
  });

  test("navigation always leaves transient focus mode and does not hide controls on return", async ({ appPage: page }) => {
    const shell = page.getByTestId("app-shell");
    const toggle = page.locator("[data-workspace-focus-toggle]");

    await toggle.click();
    await expect(shell).toHaveAttribute("data-shell-mode", "focus");
    await navigateTo(page, "records");
    await expect(page.locator('[data-route-view="records"]')).toBeVisible();
    await expect(shell).toHaveAttribute("data-shell-mode", "standard");

    await navigateTo(page, "home");
    await expect(page.locator('[data-route-view="home"]')).toBeVisible();
    await expect(shell).toHaveAttribute("data-shell-mode", "standard");
    await expect(page.locator(".mw-shell__topline")).toBeVisible();
    await expect(page.locator("[data-workspace-focus-toggle]")).toContainText("专注模式");
  });
});
