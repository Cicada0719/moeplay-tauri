import type { Page } from "@playwright/test";
import {
  FIXED_NOW,
  FIXED_RANDOM_SEED,
  type MockAppState,
} from "./mock-app-state";

export interface InvocationRecord {
  command: string;
  args?: Record<string, unknown>;
}

export async function installDeterministicEnvironment(
  page: Page,
  appState: MockAppState,
): Promise<void> {
  await page.clock.install({ time: new Date(FIXED_NOW) });
  await page.clock.resume();

  await page.addInitScript(
    ({ state, seed }) => {
      let randomState = seed >>> 0;
      Math.random = () => {
        randomState = (Math.imul(randomState, 1_664_525) + 1_013_904_223) >>> 0;
        return randomState / 0x1_0000_0000;
      };

      const invocations: Array<{ command: string; args?: Record<string, unknown> }> = [];
      const commandResults = state.commandResults ?? {};
      const clone = <T>(value: T): T => {
        if (value === undefined || value === null) return value;
        return structuredClone(value);
      };

      for (const [key, value] of Object.entries(state.localStorage ?? {})) {
        localStorage.setItem(key, value);
      }

      const invoke = async (command: string, args?: Record<string, unknown>) => {
        invocations.push({ command, args: clone(args) });
        if (Object.prototype.hasOwnProperty.call(commandResults, command)) {
          return clone(commandResults[command]);
        }
        if (command === "get_settings") return clone(state.settings);
        if (command === "update_settings") return clone(args?.settings ?? state.settings);
        if (command === "get_games" || command === "search_games") return clone(state.games);
        if (command.startsWith("plugin:event|")) return 1;
        if (command.startsWith("plugin:window|is_fullscreen")) return false;
        if (command.startsWith("plugin:window|")) return null;
        if (command.startsWith("plugin:updater|")) return null;
        if (command === "frontend_log") return null;
        return null;
      };

      Object.defineProperty(window, "__MOEPLAY_TEST__", {
        configurable: true,
        value: {
          deterministic: true,
          invocations,
          uiReady: false,
        },
      });
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        configurable: true,
        value: {
          metadata: { currentWindow: { label: "main" } },
          invoke,
          transformCallback: () => 1,
          unregisterCallback: () => {},
          convertFileSrc: (filePath: string) => `asset://localhost/${filePath}`,
        },
      });

      const style = document.createElement("style");
      style.dataset.moeplayTestStyle = "deterministic";
      style.textContent = `
        *, *::before, *::after {
          animation-delay: 0s !important;
          animation-duration: 0s !important;
          transition-delay: 0s !important;
          transition-duration: 0s !important;
          scroll-behavior: auto !important;
        }
      `;
      document.documentElement.append(style);
    },
    { state: appState, seed: FIXED_RANDOM_SEED },
  );
}

export async function waitForUiReady(page: Page): Promise<void> {
  const shell = page.locator('[data-testid="app-shell"]');
  await shell.waitFor({ state: "visible" });
  await page.evaluate(async () => {
    if (document.fonts) await document.fonts.ready;
  });
  await page.waitForTimeout(50);
  await page.evaluate(() => {
    const currentShell = document.querySelector('[data-testid="app-shell"]');
    if (!currentShell) throw new Error("App shell disappeared before UI readiness");
    currentShell.setAttribute("data-ui-ready", "true");
    document.documentElement.setAttribute("data-ui-ready", "true");
    const testApi = (window as unknown as {
      __MOEPLAY_TEST__?: { uiReady: boolean };
    }).__MOEPLAY_TEST__;
    if (testApi) testApi.uiReady = true;
  });
  await page.locator('[data-testid="app-shell"][data-ui-ready="true"]').waitFor({
    state: "visible",
  });
}

export async function getInvocationLog(page: Page): Promise<InvocationRecord[]> {
  return page.evaluate(() => {
    const records = (window as unknown as {
      __MOEPLAY_TEST__?: { invocations?: InvocationRecord[] };
    }).__MOEPLAY_TEST__?.invocations;
    return structuredClone(records ?? []);
  });
}
