import { describe, expect, it } from "vitest";
import { canRetryProviderError, isTerminalJob, type ResolvedTarget } from "./contracts";

describe("domain contracts", () => {
  it("treats only final job states as terminal", () => {
    expect(isTerminalJob("queued")).toBe(false);
    expect(isTerminalJob("paused")).toBe(false);
    expect(isTerminalJob("succeeded")).toBe(true);
    expect(isTerminalJob("cancelled")).toBe(true);
  });

  it("does not retry policy, auth, DRM or cancellation errors", () => {
    expect(canRetryProviderError({ kind: "timeout", message: "timeout", retryable: true })).toBe(true);
    expect(canRetryProviderError({ kind: "unsupported_drm", message: "drm", retryable: true })).toBe(false);
    expect(canRetryProviderError({ kind: "auth_required", message: "auth", retryable: true })).toBe(false);
  });

  it("keeps resolved targets explicitly discriminated", () => {
    const target: ResolvedTarget = {
      mode: "unsupported",
      reason: "protected",
      errorKind: "unsupported_drm",
    };
    expect(target.mode).toBe("unsupported");
  });
});
