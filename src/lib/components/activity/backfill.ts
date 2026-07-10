import { invokeCmd } from "../../api/core";

export type BackfillInvoker = () => Promise<unknown>;

export function shouldFallbackActivityV2(operation: string | undefined): boolean {
  return operation === "backfill" || operation === "timeline" || operation === "timeline_more" || operation === "continue";
}

let backfillPromise: Promise<unknown> | null = null;

/**
 * Run the legacy game-session backfill once per renderer lifetime.
 * The native command is idempotent; the shared promise also prevents duplicate
 * calls when the dashboard is mounted more than once during navigation.
 */
export function backfillLegacyGameActivityOnce(invoke: BackfillInvoker = () => invokeCmd("backfill_legacy_game_activity")): Promise<unknown> {
  if (!backfillPromise) {
    backfillPromise = Promise.resolve().then(invoke);
  }
  return backfillPromise;
}

/** Test-only reset for the module-level once gate. */
export function resetBackfillLegacyGameActivityForTests(): void {
  backfillPromise = null;
}
