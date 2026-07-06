import { test, expect } from "@playwright/test";

test.describe("App smoke", () => {
  test("app shell renders", async ({ page }) => {
    await page.goto("/?skip_wizard");
    await expect(page.getByTestId("app-shell")).toBeVisible();
    await expect(page.getByTestId("main-content")).toBeVisible();
  });
});
