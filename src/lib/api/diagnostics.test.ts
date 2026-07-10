import { afterEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "./core";
import { exportDiagnosticsZip } from "./index";

afterEach(() => clearMockInvokeHandler());

describe("diagnostics export boundary", () => {
  it("uses the redacted diagnostic bundle command instead of database export", async () => {
    setMockInvokeHandler((command, args) => {
      expect(command).toBe("export_diagnostics_zip");
      expect(args).toBeUndefined();
      return "C:/Documents/moeplay_diagnostics.zip";
    });
    await expect(exportDiagnosticsZip()).resolves.toContain("moeplay_diagnostics.zip");
  });
});
