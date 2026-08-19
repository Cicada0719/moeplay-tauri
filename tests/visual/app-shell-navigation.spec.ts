import { expect, test } from "@playwright/test";

test.describe("P0-02 app shell navigation", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?skip_wizard");
    await expect(page.getByTestId("app-shell")).toBeVisible();
  });

  test("tools Drawer traps Escape and restores Dock focus", async ({ page }) => {
    const tools = page.getByRole("button", { name: "打开工具抽屉" });
    await tools.click();

    const drawer = page.getByRole("dialog", { name: "工具" });
    await expect(drawer).toBeVisible();
    await expect(tools).toHaveAttribute("aria-expanded", "true");
    await expect(drawer.locator("[data-tool-item]").first()).toBeFocused();

    await page.keyboard.press("Escape");
    await expect(drawer).toBeHidden();
    await expect(tools).toBeFocused();
    await expect(tools).toHaveAttribute("aria-expanded", "false");
  });

  test("primary Escape is stable, subview Escape returns, and current-page search stays scoped", async ({ page }) => {
    const animeDock = page.getByRole("button", { name: "打开番剧" });
    await animeDock.click();
    await expect(page).toHaveURL(/#anime$/);
    await expect(animeDock).toHaveAttribute("aria-current", "page");

    await page.keyboard.press("Escape");
    await expect(page).toHaveURL(/#anime$/);

    await page.keyboard.press("/");
    const animeRoot = page.locator("[data-route-view='anime']").last();
    const animeSearch = animeRoot.getByRole("searchbox", { name: "搜索" });
    await expect(animeSearch).toBeFocused();
    await expect(page).toHaveURL(/#anime$/);

    await animeRoot.focus();
    // 0.22.0：Ctrl/K 改为命令面板，聚焦当前页搜索框移至 Ctrl/Shift/K
    await page.keyboard.press("Control+Shift+K");
    await expect(animeSearch).toBeFocused();

    await page.getByRole("button", { name: "打开设置" }).click();
    await expect(page).toHaveURL(/#settings$/);
    await page.keyboard.press("Escape");
    await expect(page).toHaveURL(/#anime$/);
  });

  test("App route boundary does not create nested main landmarks", async ({ page }) => {
    await page.getByRole("button", { name: "打开漫画" }).click();
    await expect(page).toHaveURL(/#comic$/);
    expect(await page.locator("main main").count()).toBe(0);
    await expect(page.locator("[data-route-view='comic']")).toBeVisible();
  });
});
