import { AiGatewayError } from "./contracts";

export class AiCancellation {
  readonly controller = new AbortController();
  #generation = 0;

  guard(): AiCancellationGuard {
    return new AiCancellationGuard(this, this.#generation);
  }

  cancel(reason = "AI task was cancelled"): void {
    this.#generation += 1;
    this.controller.abort(reason);
  }

  currentGeneration(): number {
    return this.#generation;
  }
}

export class AiCancellationGuard {
  constructor(
    private readonly owner: AiCancellation,
    private readonly generation: number,
  ) {}

  get signal(): AbortSignal {
    return this.owner.controller.signal;
  }

  /** Check before send and before accepting any response or preview. */
  ensureActive(): void {
    if (this.signal.aborted || this.owner.currentGeneration() !== this.generation) {
      throw new AiGatewayError("cancelled", "AI task was cancelled");
    }
  }
}

export interface BudgetPolicy {
  monthlyHardLimitTokens: number;
  softWarningTokens: number;
  perTaskLimitTokens: number;
}

export interface BudgetSnapshot {
  committedTokens: number;
  reservedTokens: number;
  softWarningReached: boolean;
}

export class BudgetLedger {
  #reserved = 0;

  constructor(
    readonly policy: BudgetPolicy,
    private committed = 0,
  ) {}

  reserve(estimatedTokens: number): BudgetReservation {
    if (!Number.isSafeInteger(estimatedTokens) || estimatedTokens < 0 || estimatedTokens > this.policy.perTaskLimitTokens) {
      throw new AiGatewayError("budget_exceeded", "AI task token estimate exceeds the per-task limit");
    }
    if (this.committed + this.#reserved + estimatedTokens > this.policy.monthlyHardLimitTokens) {
      throw new AiGatewayError("budget_exceeded", "AI monthly token budget would be exceeded");
    }
    this.#reserved += estimatedTokens;
    return new BudgetReservation(this, estimatedTokens);
  }

  snapshot(): BudgetSnapshot {
    return {
      committedTokens: this.committed,
      reservedTokens: this.#reserved,
      softWarningReached: this.committed + this.#reserved >= this.policy.softWarningTokens,
    };
  }

  settle(reserved: number, actual: number): void {
    this.#reserved = Math.max(0, this.#reserved - reserved);
    if (!Number.isSafeInteger(actual) || actual < 0 || actual > this.policy.perTaskLimitTokens) {
      throw new AiGatewayError("budget_exceeded", "actual AI task usage exceeds the per-task limit");
    }
    if (this.committed + actual > this.policy.monthlyHardLimitTokens) {
      throw new AiGatewayError("budget_exceeded", "actual AI usage exceeds the monthly hard limit");
    }
    this.committed += actual;
  }

  release(reserved: number): void {
    this.#reserved = Math.max(0, this.#reserved - reserved);
  }
}

export class BudgetReservation {
  #settled = false;

  constructor(
    private readonly ledger: BudgetLedger,
    readonly reservedTokens: number,
  ) {}

  commit(actualTokens: number): void {
    if (this.#settled) throw new AiGatewayError("policy_rejected", "budget reservation is already settled");
    this.#settled = true;
    this.ledger.settle(this.reservedTokens, actualTokens);
  }

  release(): void {
    if (this.#settled) return;
    this.#settled = true;
    this.ledger.release(this.reservedTokens);
  }
}

export interface RateLimitPolicy {
  maxRequests: number;
  windowMs: number;
}

export class FixedWindowRateLimiter {
  #startedAtMs: number | null = null;
  #used = 0;

  constructor(readonly policy: RateLimitPolicy) {}

  /** Caller-supplied monotonic time keeps behavior deterministic. */
  check(nowMs: number): void {
    if (this.#startedAtMs === null || nowMs - this.#startedAtMs >= this.policy.windowMs) {
      this.#startedAtMs = nowMs;
      this.#used = 0;
    }
    if (this.#used >= this.policy.maxRequests) {
      const retryAfter = Math.max(0, this.policy.windowMs - (nowMs - this.#startedAtMs));
      throw new AiGatewayError("rate_limited", "AI request rate limit exceeded", true, retryAfter);
    }
    this.#used += 1;
  }
}

export type FallbackAuthorization = "disabled" | "same_scope_only" | "explicit_cross_scope";

export function authorizeProviderFallback(
  fromLocal: boolean,
  toLocal: boolean,
  authorization: FallbackAuthorization,
): void {
  if (authorization === "disabled") {
    throw new AiGatewayError("policy_rejected", "automatic cross-provider fallback is disabled");
  }
  if (fromLocal && !toLocal && authorization !== "explicit_cross_scope") {
    throw new AiGatewayError(
      "policy_rejected",
      "local-to-remote fallback requires explicit user authorization",
    );
  }
}
