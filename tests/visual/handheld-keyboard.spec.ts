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

  test("Android 掌机搜索保留可编辑输入框", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/?skip_wizard&platform=android#anime");
    const input = page.getByRole("searchbox", { name: "搜索番剧" });
    await input.fill("葬送的芙莉莲");
    await expect(input).toHaveValue("葬送的芙莉莲");
    await expect(input).toBeEditable();
  });

  test("Windows 小屏掌机布局使用系统输入，不弹应用键盘", async ({ appPage: page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await expect.poll(() => page.evaluate(() => document.documentElement.dataset.handheld)).toBe("true");
    const input = page.locator('input[type="search"]').first();
    await input.fill("电脑 中文输入");
    await expect(input).toHaveValue("电脑 中文输入");
    await expect(page.getByTestId("handheld-keyboard")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "打开屏幕键盘" })).toHaveCount(0);
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
