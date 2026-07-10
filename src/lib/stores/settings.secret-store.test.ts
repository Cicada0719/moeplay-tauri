import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

type InvokeCall = { command: string; args?: Record<string, unknown> };

async function loadSettingsStore(
  handler: (command: string, args?: Record<string, unknown>) => unknown,
) {
  vi.resetModules();
  localStorage.setItem("moegame-startup-migrated-v1", "1");
  const core = await import("../api/core");
  core.setMockInvokeHandler(handler);
  return (await import("./settings.svelte")).settingsStore;
}

describe("settings secret sentinel contract", () => {
  beforeEach(() => localStorage.clear());
  afterEach(() => localStorage.clear());

  it("drops legacy plaintext fields returned by an old backend", async () => {
    const sentinel = "SENTINEL_LEGACY_SETTINGS_RESPONSE";
    const store = await loadSettingsStore((command) => {
      if (command === "get_settings") {
        return {
          theme: "dark",
          ai_api_key: sentinel,
          steam_api_key: sentinel,
        };
      }
      throw new Error(`unexpected command: ${command}`);
    });

    await store.load();

    expect("ai_api_key" in store.settings).toBe(false);
    expect("steam_api_key" in store.settings).toBe(false);
    expect(JSON.stringify(store.settings)).not.toContain(sentinel);
  });

  it("never sends legacy plaintext fields through update_settings", async () => {
    const sentinel = "SENTINEL_SETTINGS_UPDATE";
    const calls: InvokeCall[] = [];
    const store = await loadSettingsStore((command, args) => {
      calls.push({ command, args });
      if (command === "get_settings") return { theme: "dark" };
      if (command === "update_settings") return args?.settings;
      throw new Error(`unexpected command: ${command}`);
    });
    await store.load();

    await store.save({
      ...store.settings,
      ai_api_key: sentinel,
      steam_api_key: sentinel,
    } as typeof store.settings & { ai_api_key: string; steam_api_key: string });

    const update = calls.find(({ command }) => command === "update_settings");
    expect(update).toBeDefined();
    expect(update?.args?.settings).not.toHaveProperty("ai_api_key");
    expect(update?.args?.settings).not.toHaveProperty("steam_api_key");
    expect(JSON.stringify(update)).not.toContain(sentinel);
    expect("ai_api_key" in store.settings).toBe(false);
    expect("steam_api_key" in store.settings).toBe(false);
  });
});
