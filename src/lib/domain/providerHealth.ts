import type { ProviderHealth, ProviderHealthState } from "./contracts";

export interface ProviderScoreOptions {
  latencyBudgetMs?: number;
  failurePenalty?: number;
  consecutiveFailurePenalty?: number;
}

export function providerHealthState(health: ProviderHealth, now = Date.now()): ProviderHealthState {
  if (health.state === "disabled") return "disabled";
  if (health.circuitOpenUntil) {
    const until = Date.parse(health.circuitOpenUntil);
    if (Number.isFinite(until) && until > now) return "open_circuit";
  }
  if (health.consecutiveFailures >= 3 || health.state === "degraded") return "degraded";
  if (health.successCount > 0 && health.consecutiveFailures === 0) return "healthy";
  return health.state;
}

export function providerHealthScore(
  health: ProviderHealth,
  options: ProviderScoreOptions = {},
): number {
  const state = providerHealthState(health);
  if (state === "disabled" || state === "open_circuit") return Number.NEGATIVE_INFINITY;

  const latencyBudget = Math.max(1, options.latencyBudgetMs ?? 4_000);
  const failurePenalty = options.failurePenalty ?? 12;
  const consecutivePenalty = options.consecutiveFailurePenalty ?? 18;
  const total = health.successCount + health.failureCount;
  const successRate = total === 0 ? 0.5 : health.successCount / total;
  const latency = health.latencyMsEma ?? latencyBudget / 2;
  const latencyScore = Math.max(0, 1 - latency / latencyBudget);

  return (
    successRate * 70
    + latencyScore * 30
    - health.failureCount * failurePenalty / Math.max(1, total)
    - health.consecutiveFailures * consecutivePenalty
  );
}

export function sortProviderHealth<T extends ProviderHealth>(items: readonly T[]): T[] {
  return [...items].sort((a, b) => {
    const scoreDiff = providerHealthScore(b) - providerHealthScore(a);
    if (Number.isFinite(scoreDiff) && scoreDiff !== 0) return scoreDiff;
    return a.providerId.localeCompare(b.providerId);
  });
}
