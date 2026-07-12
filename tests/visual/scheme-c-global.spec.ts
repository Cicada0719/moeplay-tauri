import { expect, test } from "@playwright/test";

test.describe("Scheme C global product shell", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?skip_wizard#home");
    await expect(page.getByTestId("app-shell")).toBeVisible();
  });

  test("uses the full viewport without a persistent side rail", async ({ page }) => {
    const mainBox = await page.getByTestId("main-content").boundingBox();
    await expect(page.getByTestId("system-dock")).toHaveCount(0);
    expect(mainBox?.x).toBe(0);
    expect(mainBox?.width).toBe(1440);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(1440);
  });

  test("keeps navigation and display controls in the top bar", async ({ page }) => {
    await expect(page.getByRole("link", { name: "MoePlay，跳到主要内容" })).toBeVisible();
    await expect(page.getByRole("button", { name: "打开番剧" })).toHaveCount(1);
    await expect(page.getByRole("button", { name: "进入应用全屏" })).toBeVisible();
    await expect(page.getByRole("button", { name: "进入大屏模式" })).toBeVisible();
  });

  test("assigns fixed visual languages to the three content modules", async ({ page }) => {
    await expect(page.getByTestId("switch-home")).toHaveAttribute("data-module-style", "cinematic");
    await page.getByRole("button", { name: "打开番剧" }).click();
    await expect(page.locator('[data-route-view="anime"]')).toHaveAttribute("data-module-style", "editorial");
    await page.getByRole("button", { name: "打开漫画" }).click();
    await expect(page.locator('[data-route-view="comic"]')).toHaveAttribute("data-module-style", "kinetic");
  });

  test("does not expose a global director style selector", async ({ page }) => {
    await expect(page.getByRole("group", { name: "导演风格" })).toHaveCount(0);
    await expect(page.getByText("游戏模块固定导演语言")).toBeVisible();
  });
});
