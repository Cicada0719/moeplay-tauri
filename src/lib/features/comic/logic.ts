/** Monotonic cancellation generation used to ignore stale provider responses. */
export class RequestGeneration {
  #value = 0;
  get value(): number { return this.#value; }
  bump(): number { this.#value += 1; return this.#value; }
  isCurrent(generation: number): boolean { return generation === this.#value; }
}

export interface PageRetryPlan { page: number; attempt: number; maxAttempts: number; retryable: boolean; }

/** Pure page retry policy; callers decide when to schedule the next attempt. */
export function planPageRetry(page: number, failedAttempt: number, maxAttempts = 3, retryable = true): PageRetryPlan | undefined {
  const boundedMax = Math.max(1, Math.floor(maxAttempts));
  const nextAttempt = failedAttempt + 1;
  if (!retryable || nextAttempt >= boundedMax) return undefined;
  return { page, attempt: nextAttempt, maxAttempts: boundedMax, retryable };
}

export function pageRetryDelayMs(attempt: number, baseMs = 250, capMs = 4_000): number {
  const safeAttempt = Math.max(0, Math.floor(attempt));
  return Math.min(capMs, baseMs * 2 ** safeAttempt);
}

/** Returns neighbor pages only; current page is already visible. */
export function computePrefetchWindow(currentPage: number, totalPages: number, radius = 2): number[] {
  const total = Math.max(0, Math.floor(totalPages));
  if (total === 0) return [];
  const current = Math.min(total - 1, Math.max(0, Math.floor(currentPage)));
  const distance = Math.max(0, Math.floor(radius));
  const pages: number[] = [];
  for (let page = Math.max(0, current - distance); page <= Math.min(total - 1, current + distance); page += 1) {
    if (page !== current) pages.push(page);
  }
  return pages;
}

export function providerErrorMessage(error: unknown, fallback = "漫画源请求失败"): string {
  if (typeof error === "string" && error.trim()) return error;
  if (error && typeof error === "object") {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  return fallback;
}
