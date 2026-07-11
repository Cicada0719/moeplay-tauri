import { expect, test } from "@playwright/test";

const viewports = [
  { name: "900x600", width: 900, height: 600 },
  { name: "1200x800", width: 1200, height: 800 },
  { name: "1920x1080", width: 1920, height: 1080 },
  { name: "ultrawide", width: 2560, height: 1080 },
] as const;

const templates = ["cinematic", "editorial", "kinetic"] as const;
const modes = ["visual", "index", "scene"] as const;

for (const viewport of viewports) {
  test.describe(viewport.name, () => {
    test.use({ viewport: { width: viewport.width, height: viewport.height } });

    for (const template of templates) {
      test(`${template} 在三模式下保持关键导航可用`, async ({ page }) => {
        await page.goto("/concept/index.html");
        await page.getByTestId(`template-${template}`).click();

        for (const mode of modes) {
          await page.getByTestId(`mode-${mode}`).click();
          await expect(page.getByTestId("concept-stage")).toHaveAttribute("data-mode", mode);
          await expect(page.getByRole("navigation", { name: "媒体模块" })).toBeVisible();
          await expect(page.getByTestId(`mode-${mode}`)).toBeVisible();
          const bounds = await page.getByTestId("concept-stage").boundingBox();
          expect(bounds?.width).toBeGreaterThanOrEqual(Math.min(viewport.width, 900) - 2);
        }
      });
    }
  });
}

