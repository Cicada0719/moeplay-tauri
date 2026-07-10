import { describe, expect, it } from "vitest";
import type { ProviderHealth } from "./contracts";
import { providerHealthScore, providerHealthState, sortProviderHealth } from "./providerHealth";

function health(patch: Partial<ProviderHealth> = {}): ProviderHealth {
  return {
    providerId: "provider",
    operation: "search",
    state: "unknown",
    successCount: 0,
    failureCount: 0,
    consecutiveFailures: 0,
    ...patch,
  };
}

describe("provider health", () => {
  it("keeps an active circuit out of candidate selection", () => {
    const item = health({ circuitOpenUntil: "2099-01-01T00:00:00Z" });
    expect(providerHealthState(item)).toBe("open_circuit");
    expect(providerHealthScore(item)).toBe(Number.NEGATIVE_INFINITY);
  });

  it("prefers successful low-latency providers", () => {
    const fast = health({ providerId: "fast", successCount: 9, failureCount: 1, latencyMsEma: 300 });
    const slow = health({ providerId: "slow", successCount: 6, failureCount: 4, latencyMsEma: 3_000 });
    expect(sortProviderHealth([slow, fast]).map((item) => item.providerId)).toEqual(["fast", "slow"]);
  });

  it("degrades after repeated failures", () => {
    expect(providerHealthState(health({ consecutiveFailures: 3 }))).toBe("degraded");
  });
});
