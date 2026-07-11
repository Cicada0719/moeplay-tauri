import { describe, expect, it } from "vitest";
import {
  SOURCE_PRIORITY_MAX,
  SOURCE_PRIORITY_MIN,
  normalizeSourceDescriptor,
} from "./contracts";

describe("Source Center Rust DTO normalization", () => {
  it("maps serialized Rust health, runtime, auth, and capability values explicitly", () => {
    const source = normalizeSourceDescriptor({
      providerId: "runtime-reader",
      mediaType: "external_runtime",
      name: "Runtime Reader",
      capabilities: ["children", "resolve", "verify", "not_a_capability"],
      priority: -10_000,
      health: { state: "open_circuit", latencyMs: 31, consecutiveFailures: 3, lastErrorKind: "network" },
      authState: "missing",
      runtimeState: "deferred",
    });

    expect(source).toMatchObject({
      providerId: "runtime-reader",
      mediaType: "external_runtime",
      capabilities: ["children", "resolve", "verify"],
      priority: SOURCE_PRIORITY_MIN,
      health: { state: "open_circuit", latencyMs: 31, consecutiveFailures: 3 },
      authState: "missing",
      runtimeState: "deferred",
    });
    expect(source.recentFailures).toEqual([{ code: "network", message: "来源验证失败" }]);
  });

  it("preserves the full signed priority range while bounding malformed values", () => {
    expect(normalizeSourceDescriptor({ priority: -10_000 }).priority).toBe(SOURCE_PRIORITY_MIN);
    expect(normalizeSourceDescriptor({ priority: 10_000, runtimeState: "available" }).priority).toBe(SOURCE_PRIORITY_MAX);
    expect(normalizeSourceDescriptor({ priority: -12_000 }).priority).toBe(SOURCE_PRIORITY_MIN);
    expect(normalizeSourceDescriptor({ priority: 12_000 }).priority).toBe(SOURCE_PRIORITY_MAX);
    expect(normalizeSourceDescriptor({ runtimeState: "available" }).runtimeState).toBe("available");
  });
});
