export class GenerationGuard {
  #generation = 0;
  #controller: AbortController | null = null;

  begin(): { generation: number; signal: AbortSignal } {
    this.#controller?.abort();
    this.#controller = new AbortController();
    this.#generation += 1;
    return { generation: this.#generation, signal: this.#controller.signal };
  }

  isCurrent(generation: number): boolean {
    return generation === this.#generation && !this.#controller?.signal.aborted;
  }

  cancel(): void {
    this.#controller?.abort();
    this.#generation += 1;
  }

  get current(): number {
    return this.#generation;
  }
}

export function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}
