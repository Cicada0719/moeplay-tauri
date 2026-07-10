import type {
  ApplyImportResponse,
  ImportDecision,
  ImportPreview,
  LibraryApi,
  LibraryHealthSnapshot,
  PreviewImportRequest,
} from "./contracts";
import { createRequestGate } from "./request-gate";

export interface LibraryFeatureState {
  previewGeneration: number;
  applyGeneration: number;
  healthGeneration: number;
  isPreviewing: boolean;
  isApplying: boolean;
  isLoadingHealth: boolean;
  preview: ImportPreview | null;
  applyResult: ApplyImportResponse | null;
  health: LibraryHealthSnapshot | null;
  error: string | null;
}

export interface LibraryFeatureStore {
  getSnapshot(): LibraryFeatureState;
  subscribe(listener: (state: LibraryFeatureState) => void): () => void;
  preview(request: PreviewImportRequest): Promise<void>;
  apply(decisions: ImportDecision[], idempotencyKey: string): Promise<void>;
  loadHealth(): Promise<void>;
  cancelPreview(): void;
  cancelApply(): void;
  cancelHealth(): void;
  cancelAll(): void;
  clear(): void;
}

const initialState = (): LibraryFeatureState => ({
  previewGeneration: 0,
  applyGeneration: 0,
  healthGeneration: 0,
  isPreviewing: false,
  isApplying: false,
  isLoadingHealth: false,
  preview: null,
  applyResult: null,
  health: null,
  error: null,
});

export function createLibraryFeatureStore(api: LibraryApi): LibraryFeatureStore {
  let state = initialState();
  const listeners = new Set<(state: LibraryFeatureState) => void>();
  const previewGate = createRequestGate();
  const applyGate = createRequestGate();
  const healthGate = createRequestGate();

  const patch = (next: Partial<LibraryFeatureState>) => {
    state = { ...state, ...next };
    listeners.forEach((listener) => listener(state));
  };

  return {
    getSnapshot: () => state,
    subscribe(listener) {
      listeners.add(listener);
      listener(state);
      return () => listeners.delete(listener);
    },
    async preview(request) {
      const lease = previewGate.begin();
      patch({
        previewGeneration: lease.generation,
        isPreviewing: true,
        applyResult: null,
        error: null,
      });
      try {
        const preview = await api.preview(request, lease.signal);
        if (previewGate.isCurrent(lease.generation)) patch({ preview });
      } catch (error) {
        if (previewGate.isCurrent(lease.generation)) patch({ error: errorMessage(error) });
      } finally {
        if (previewGate.isCurrent(lease.generation)) patch({ isPreviewing: false });
      }
    },
    async apply(decisions, idempotencyKey) {
      const preview = state.preview;
      if (!preview) {
        patch({ error: "Import preview is required before apply." });
        return;
      }
      const lease = applyGate.begin();
      patch({
        applyGeneration: lease.generation,
        isApplying: true,
        error: null,
      });
      try {
        const applyResult = await api.apply(
          { preview, decisions, idempotencyKey },
          lease.signal,
        );
        if (applyGate.isCurrent(lease.generation)) patch({ applyResult });
      } catch (error) {
        if (applyGate.isCurrent(lease.generation)) patch({ error: errorMessage(error) });
      } finally {
        if (applyGate.isCurrent(lease.generation)) patch({ isApplying: false });
      }
    },
    async loadHealth() {
      const lease = healthGate.begin();
      patch({
        healthGeneration: lease.generation,
        isLoadingHealth: true,
        error: null,
      });
      try {
        const health = await api.health(lease.signal);
        if (healthGate.isCurrent(lease.generation)) patch({ health });
      } catch (error) {
        if (healthGate.isCurrent(lease.generation)) patch({ error: errorMessage(error) });
      } finally {
        if (healthGate.isCurrent(lease.generation)) patch({ isLoadingHealth: false });
      }
    },
    cancelPreview() {
      previewGate.cancel();
      patch({ previewGeneration: previewGate.currentGeneration(), isPreviewing: false });
    },
    cancelApply() {
      applyGate.cancel();
      patch({ applyGeneration: applyGate.currentGeneration(), isApplying: false });
    },
    cancelHealth() {
      healthGate.cancel();
      patch({ healthGeneration: healthGate.currentGeneration(), isLoadingHealth: false });
    },
    cancelAll() {
      previewGate.cancel();
      applyGate.cancel();
      healthGate.cancel();
      patch({
        previewGeneration: previewGate.currentGeneration(),
        applyGeneration: applyGate.currentGeneration(),
        healthGeneration: healthGate.currentGeneration(),
        isPreviewing: false,
        isApplying: false,
        isLoadingHealth: false,
      });
    },
    clear() {
      previewGate.cancel();
      applyGate.cancel();
      healthGate.cancel();
      state = {
        ...initialState(),
        previewGeneration: previewGate.currentGeneration(),
        applyGeneration: applyGate.currentGeneration(),
        healthGeneration: healthGate.currentGeneration(),
      };
      listeners.forEach((listener) => listener(state));
    },
  };
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}
