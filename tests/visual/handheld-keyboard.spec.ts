import { DEFAULT_APP_STATE, expect, test } from "./fixtures";

const handheldState = {
  ...DEFAULT_APP_STATE,
  settings: {
    ...DEFAULT_APP_STATE.settings,
    startup_mode: "fullscreen",
    theme: "cinema",
  },
};

test.describe("Handheld mode on-screen keyboard", () => {
  test.use({ appState: handheldState });

  test("1280x800 掌机视口下输入框聚焦自动弹出屏幕键盘并可输入/关闭", async ({ appPage: page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    // 掌机自动判定生效（横屏 && 高≤800 && 宽≤1920）
    await expect.poll(() => page.evaluate(() => document.documentElement.dataset.handheld)).toBe("true");

    const input = page.locator('input[type="search"]').first();
    await input.focus();
    const keyboard = page.getByTestId("handheld-keyboard");
    await expect(keyboard).toBeVisible();

    // 点击键帽输入，焦点落到键盘内不应关闭
    await page.getByRole("button", { name: "Q", exact: true }).click();
    await expect(input).toHaveValue("Q");
    await expect(keyboard).toBeVisible();

    // 关闭后出现重开按钮，点击可重新聚焦输入并弹出
    await page.getByRole("button", { name: "关闭屏幕键盘" }).click();
    await expect(keyboard).toBeHidden();
    const reopen = page.getByRole("button", { name: "打开屏幕键盘" });
    await expect(reopen).toBeVisible();
    await reopen.click();
    await expect(keyboard).toBeVisible();
  });

  test("1440x900 桌面视口不弹键盘（掌机未生效）", async ({ appPage: page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    const input = page.locator('input[type="search"]').first();
    await input.focus();
    await expect(page.getByTestId("handheld-keyboard")).toHaveCount(0);
    await page.waitForTimeout(250);
    await expect(page.getByTestId("handheld-keyboard")).toHaveCount(0);
  });
});
