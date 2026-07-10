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
    begin() {
      controller?.abort();
      controller = new AbortController();
      generation += 1;
      return { generation, signal: controller.signal };
    },
    cancel() {
      controller?.abort();
      controller = null;
      generation += 1;
    },
    isCurrent(candidate) {
      return candidate === generation;
    },
    currentGeneration() {
      return generation;
    },
  };
}
