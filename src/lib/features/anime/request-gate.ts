/**
 * Pure request-generation guard shared by the feature store. Each new request
 * revokes the previous generation. Consumers must check `isCurrent` before a
 * late completion writes state.
 */
export interface RequestLease {
  generation: number;
  signal: AbortSignal;
}

export interface RequestGate {
  begin(): RequestLease;
  cancel(): void;
  isCurrent(generation: number): boolean;
  currentGeneration(): number;
}

export function createRequestGate(): RequestGate {
  let generation = 0;
  let controller: AbortController | null = null;

  return {
    begin(): RequestLease {
      controller?.abort();
      controller = new AbortController();
      generation += 1;
      return { generation, signal: controller.signal };
    },
    cancel(): void {
      controller?.abort();
      controller = null;
      generation += 1;
    },
    isCurrent(candidate: number): boolean {
      return candidate === generation;
    },
    currentGeneration(): number {
      return generation;
    },
  };
}
