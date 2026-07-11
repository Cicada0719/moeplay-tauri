import AxeBuilder from "@axe-core/playwright";
import {
  EMPTY_APP_STATE,
  expect,
  test,
} from "./fixtures/app-fixture";

test.use({ appState: EMPTY_APP_STATE });

test.describe("accessibility infrastructure", () => {
  test("deterministic app shell has no serious or critical axe violations", async ({ appPage }) => {
    await expect(appPage.getByTestId("app-shell")).toBeVisible();
    await expect(appPage.locator("html")).toHaveAttribute("data-ui-ready", "true");

    const results = await new AxeBuilder({ page: appPage })
      .include('nav[aria-label="主要模块"]')
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

