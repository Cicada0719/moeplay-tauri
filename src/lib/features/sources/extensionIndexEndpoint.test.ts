import { beforeEach, describe, expect, it } from "vitest";
import {
  EXTENSION_INDEX_ENDPOINT_STORAGE_KEY,
  getConfiguredExtensionIndexEndpoint,
  saveExtensionIndexEndpoint,
} from "./extensionIndexEndpoint";

describe("extension-index endpoint configuration", () => {
  beforeEach(() => localStorage.clear());

  it("persists only a credential-free controlled endpoint and removes unsafe stored values", () => {
    localStorage.setItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY, "https://token:secret@directory.example/index.json");
    expect(getConfiguredExtensionIndexEndpoint()).toBeNull();
    expect(localStorage.getItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY)).toBeNull();

    expect(() => saveExtensionIndexEndpoint("https://directory.example/index.json?token=secret")).toThrow("不含凭据");
    expect(localStorage.getItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY)).toBeNull();

    expect(saveExtensionIndexEndpoint("https://directory.example/index.json")).toBe("https://directory.example/index.json");
    expect(localStorage.getItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY)).toBe("https://directory.example/index.json");
  });
});
