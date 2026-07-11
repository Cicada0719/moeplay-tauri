import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

const CONCEPT_URL = "/concept/index.html";

async function openConcept(page: Page) {
  await page.goto(CONCEPT_URL);
  await expect(page.getByTestId("concept-stage")).toBeVisible();
  await expect(page.getByTestId("review-panel")).toBeVisible();
}

async function expectStageState(
  page: Page,
  attribute: "template" | "module" | "mode",
  value: string,
) {
  await expect(page.getByTestId("concept-stage")).toHaveAttribute(`data-${attribute}`, value);
}

test.describe("MoePlay concept review contract", () => {
  test.beforeEach(async ({ page }) => {
    await openConcept(page);
  });

  test("模板切换同步评审按钮和舞台状态", async ({ page }) => {
    const cases = ["cinematic", "editorial", "kinetic"] as const;

    for (const template of cases) {
      const control = page.getByTestId(`template-${template}`);
      await control.click();
      await expect(control).toHaveAttribute("aria-checked", "true");
      await expect(page.getByTestId("review-template-state")).toHaveText(template);
      await expectStageState(page, "template", template);
    }
  });

  test("视觉、索引、场景三模式保持互斥并更新内容舞台", async ({ page }) => {
    const modes = ["visual", "index", "scene"] as const;

    for (const mode of modes) {
      await page.getByTestId(`mode-${mode}`).click();
      await expect(page.getByTestId(`mode-${mode}`)).toHaveAttribute("aria-checked", "true");
      await expect(page.getByTestId("review-mode-state")).toHaveText(mode);
      await expectStageState(page, "mode", mode);
      await expect(page.locator('[role="radio"][data-testid^="mode-"][aria-checked="true"]')).toHaveCount(1);
    }
  });

  test("打开详情后可返回来源内容并恢复焦点", async ({ page }) => {
    const source = page.locator('[data-testid="content-item"]').first();
    const sourceId = await source.getAttribute("data-content-id");
    expect(sourceId).toBeTruthy();

    await source.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("concept-detail")).toBeVisible();
    await expect(page.getByTestId("concept-detail")).toHaveAttribute("data-content-id", sourceId!);

    await page.getByTestId("detail-back").click();
    await expect(page.getByTestId("concept-detail")).toBeHidden();
    await expect(page.locator(`[data-testid="content-item"][data-content-id="${sourceId}"]`)).toBeFocused();
  });

  test("键盘可导航、确认、返回并切换模式", async ({ page }) => {
    const stage = page.getByTestId("concept-stage");
    await stage.focus();

    const selectedBefore = await stage.getAttribute("data-selected-id");
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => stage.getAttribute("data-selected-id")).not.toBe(selectedBefore);

    await page.keyboard.press("Enter");
    await expect(page.getByTestId("concept-detail")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("concept-detail")).toBeHidden();

    await expectStageState(page, "mode", "visual");
    await stage.focus();
    await page.keyboard.press("Shift+ArrowRight");
    await expectStageState(page, "mode", "index");
    await expect(page.getByTestId("input-guide")).toContainText("键盘 / 手柄");
  });



  test("模块分别记忆模式与焦点选择，切换后可恢复", async ({ page }) => {
    await page.getByTestId("mode-index").click();
    const gameStage = page.getByTestId("concept-stage");
    const gameSelected = await gameStage.getAttribute("data-selected-id");

    await page.getByTestId("module-anime").click();
    await page.getByTestId("mode-scene").click();
    await gameStage.focus();
    await page.keyboard.press("ArrowDown");
    const animeSelected = await gameStage.getAttribute("data-selected-id");

    await page.getByTestId("module-games").click();
    await expectStageState(page, "mode", "index");
    await expect(gameStage).toHaveAttribute("data-selected-id", gameSelected!);

    await page.getByTestId("module-anime").click();
    await expectStageState(page, "mode", "scene");
    await expect(gameStage).toHaveAttribute("data-selected-id", animeSelected!);
  });
  test("Kinetic Scene 使用 WebGL 媒体舞台并在 reduced quality 下完整降级", async ({ page }) => {
    await page.getByTestId("template-kinetic").click();
    await page.getByTestId("mode-scene").click();
    await expect(page.getByTestId("webgl-media-stage")).toBeVisible();

    await page.getByTestId("quality-reduced").click();
    await page.getByTestId("mode-visual").click();
    await page.getByTestId("mode-scene").click();
    await expect(page.getByTestId("webgl-media-stage")).toHaveAttribute("data-fallback", "true");
    await expect(page.getByTestId("webgl-media-stage").locator(".media-stage-fallback")).toBeVisible();
  });
  test("评审质量、声音、模块和 viewport 控制回写状态", async ({ page }) => {
    await page.getByTestId("module-anime").click();
    await expectStageState(page, "module", "anime");

    await page.getByTestId("quality-reduced").click();
    await expect(page.getByTestId("review-quality-state")).toHaveText("reduced");
    await expect(page.getByTestId("concept-stage")).toHaveAttribute("data-quality", "reduced");

    const sound = page.getByTestId("sound-toggle");
    const initialSound = await sound.getAttribute("aria-checked");
    await sound.click();
    await expect(sound).not.toHaveAttribute("aria-checked", initialSound!);

    await page.getByTestId("viewport-compact").click();
    await expect(page.getByTestId("review-viewport-state")).toHaveText("compact");
    await expect(page.getByTestId("concept-viewport")).toHaveAttribute("data-viewport", "compact");
  });
});

test.describe("MoePlay concept motion and accessibility", () => {
  test("系统 reduced motion 偏好禁用非必要动画", async ({ browser }) => {
    const context = await browser.newContext({
      reducedMotion: "reduce",
      viewport: { width: 1440, height: 900 },
      colorScheme: "dark",
    });
    const page = await context.newPage();
    await openConcept(page);

    await expect(page.getByTestId("concept-stage")).toHaveAttribute("data-reduced-motion", "true");
    const offenders = await page.locator("[data-motion]").evaluateAll((nodes) =>
      nodes
        .filter((node) => {
          const style = getComputedStyle(node);
          const durations = `${style.animationDuration},${style.transitionDuration}`
            .split(",")
            .map((value) => Number.parseFloat(value) || 0);
          return durations.some((duration) => duration > 0.01);
        })
        .map((node) => node.getAttribute("data-testid") ?? node.tagName),
    );
    expect(offenders).toEqual([]);
    await context.close();
  });

  test("概念站无 serious 或 critical 基础可访问性违规", async ({ page }) => {
    await openConcept(page);

    await expect(page.getByRole("main")).toBeVisible();
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(page.getByLabel("概念评审控制台")).toBeVisible();

    const results = await new AxeBuilder({ page })
      .include("body")
      .disableRules(["color-contrast"])
      .analyze();
    const blocking = results.violations.filter(
      ({ impact }) => impact === "serious" || impact === "critical",
    );

    expect(
      blocking.map(({ id, impact, help, nodes }) => ({
        id,
        impact,
        help,
        targets: nodes.map((node) => node.target),
      })),
    ).toEqual([]);
  });
});


