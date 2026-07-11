import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "../../api/core";
import { tauriSourceCenterApi } from "./api";
import { EXTENSION_INDEX_ENDPOINT_STORAGE_KEY } from "./extensionIndexEndpoint";

describe("tauriSourceCenterApi extension-index bridge", () => {
  beforeEach(() => {
    localStorage.clear();
    clearMockInvokeHandler();
  });

  afterEach(() => clearMockInvokeHandler());

  it("does not invoke an extension-directory command without an explicit controlled endpoint", async () => {
    const invoke = vi.fn();
    setMockInvokeHandler(invoke);

    await expect(tauriSourceCenterApi.getExtensionIndexSnapshot(null)).resolves.toBeNull();
    await expect(tauriSourceCenterApi.refreshExtensionIndex("https://token@example.test/index.json?apiKey=secret")).resolves.toBeNull();

    expect(invoke).not.toHaveBeenCalled();
    expect(localStorage.getItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY)).toBeNull();
  });

  it("passes the user-controlled endpoint to the real command and normalizes its refresh response", async () => {
    const invoke = vi.fn((command: string) => {
      if (command === "refresh_extension_index") {
        return { refresh: { state: "offline_snapshot", warningCode: "extension_index_network_failed", snapshot: { entries: [{ id: "reader", name: "Reader" }], fetchedAt: "2026-07-11T00:00:00Z", expiresAt: "2026-07-12T00:00:00Z" } } };
      }
      return null;
    });
    setMockInvokeHandler(invoke);

    await expect(tauriSourceCenterApi.refreshExtensionIndex("https://directory.example/extensions.json")).resolves.toEqual({
      entries: [{ id: "reader", name: "Reader" }],
      fetchedAt: "2026-07-11T00:00:00Z",
      expiresAt: "2026-07-12T00:00:00Z",
      isOfflineSnapshot: true,
      lastError: "extension_index_network_failed",
    });

    expect(invoke).toHaveBeenCalledWith("refresh_extension_index", {
      endpoint: "https://directory.example/extensions.json",
      force: true,
    });
  });
});
