import { expect, test } from "@playwright/test";

test.describe("FOAM-inspired activity archive", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?skip_wizard#records");
    await expect(page.getByTestId("app-shell")).toBeVisible();
  });

  test("opens in Archive mode and keeps Index management available", async ({ page }) => {
    await expect(page.getByRole("group", { name: "记录页视图" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "ACTIVITY ARCHIVE" })).toBeVisible();
    await expect(page.getByTestId("records-archive")).toBeVisible();

    await page.getByRole("button", { name: "INDEX / 管理" }).click();
    await expect(page.getByRole("heading", { name: "活动索引" })).toBeVisible();
    await expect(page.getByRole("region", { name: "Activity v2 活动记录" })).toBeVisible();
  });

  test("uses the full-width archive canvas without a side rail", async ({ page }) => {
    await expect(page.getByTestId("system-dock")).toHaveCount(0);
    const box = await page.getByTestId("records-archive").boundingBox();
    expect(box?.x).toBe(0);
    expect(box?.width).toBe(1440);
  });
});
