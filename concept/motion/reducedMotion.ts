export type ReducedMotionListener = (reduced: boolean) => void;

export interface ReducedMotionPreference {
  readonly reduced: boolean;
  subscribe(listener: ReducedMotionListener): () => void;
  dispose(): void;
}

export function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function createReducedMotionPreference(): ReducedMotionPreference {
  const query = typeof matchMedia === "function"
    ? matchMedia("(prefers-reduced-motion: reduce)")
    : null;
  const listeners = new Set<ReducedMotionListener>();
  let reduced = query?.matches ?? false;
  let disposed = false;

  const notify = (event: MediaQueryListEvent) => {
    reduced = event.matches;
    for (const listener of listeners) listener(reduced);
  };

  query?.addEventListener("change", notify);

  return {
    get reduced() {
      return reduced;
    },
    subscribe(listener) {
      if (disposed) {
        listener(reduced);
        return () => undefined;
      }
      listeners.add(listener);
      listener(reduced);
      return () => listeners.delete(listener);
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      query?.removeEventListener("change", notify);
      listeners.clear();
    },
  };
}
