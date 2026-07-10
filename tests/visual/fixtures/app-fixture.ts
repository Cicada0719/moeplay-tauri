import { test as base, expect, type Page } from "@playwright/test";
import {
  DEFAULT_APP_STATE,
  type MockAppState,
} from "./mock-app-state";
import {
  installDeterministicEnvironment,
  waitForUiReady,
} from "./deterministic";
import {
  GamepadController,
  installGamepadMock,
} from "./gamepad";

interface MoePlayFixtures {
  appState: MockAppState;
  appPage: Page;
  gamepad: GamepadController;
}

export const test = base.extend<MoePlayFixtures>({
  appState: [{ ...DEFAULT_APP_STATE }, { option: true }],

  page: async ({ page, appState }, use) => {
    await installDeterministicEnvironment(page, appState);
    await installGamepadMock(page);
    await use(page);
  },

  appPage: async ({ page }, use) => {
    await page.goto("/?skip_wizard", { waitUntil: "domcontentloaded" });
    await waitForUiReady(page);
    await use(page);
  },

  gamepad: async ({ page }, use) => {
    await use(new GamepadController(page));
  },
});

export { expect };
export * from "./deterministic";
export * from "./gamepad";
export * from "./mock-app-state";
