import type {
  ActivityApi, ActivityError, ActivityEventPatch, ActivityEventView, ActivityExportFormat, ActivityFilters,
  ActivitySummary, ContinueCandidate, ContinueQuery,
} from "./contracts";
import { createRequestGate } from "./request-gate";

export interface ActivityStoreState {
  filters: ActivityFilters;
  events: ActivityEventView[];
  summary: ActivitySummary | null;
  continueCandidates: ContinueCandidate[];
  nextCursor: { startedAt: string; id: string } | null;
  timelineGeneration: number;
  continueGeneration: number;
  isLoadingTimeline: boolean;
  isLoadingMore: boolean;
  isLoadingContinue: boolean;
  error: ActivityError | null;
}
export interface ActivityStore {
  getSnapshot(): ActivityStoreState;
  subscribe(listener: (state: ActivityStoreState) => void): () => void;
  load(filters?: ActivityFilters): Promise<void>;
  loadMore(): Promise<void>;
  loadContinue(query?: ContinueQuery): Promise<void>;
  edit(id: string, patch: ActivityEventPatch): Promise<ActivityEventView | null>;
  remove(id: string): Promise<boolean>;
  export(format: ActivityExportFormat, path: string): Promise<string | null>;
  setFilters(filters: ActivityFilters): void;
  cancelTimeline(): void;
  cancelContinue(): void;
}

const initial = (): ActivityStoreState => ({
  filters: {}, events: [], summary: null, continueCandidates: [], nextCursor: null,
  timelineGeneration: 0, continueGeneration: 0, isLoadingTimeline: false, isLoadingMore: false,
  isLoadingContinue: false, error: null,
});
function toError(error: unknown, operation: string): ActivityError {
  if (error instanceof DOMException && error.name === "AbortError") return { operation, message: "request cancelled", cancelled: true };
  return { operation, message: error instanceof Error ? error.message : "activity request failed", cancelled: false };
}
function clone(state: ActivityStoreState): ActivityStoreState { return { ...state, filters: { ...state.filters }, events: [...state.events], continueCandidates: [...state.continueCandidates], summary: state.summary ? { ...state.summary, days: [...state.summary.days] } : null }; }

export function createActivityStore(api: ActivityApi): ActivityStore {
  let state = initial(); const listeners = new Set<(state: ActivityStoreState) => void>();
  const timelineGate = createRequestGate(); const continueGate = createRequestGate(); const mutationGate = createRequestGate();
  const publish = () => { const snapshot = clone(state); listeners.forEach((listener) => listener(snapshot)); };
  const patch = (value: Partial<ActivityStoreState>) => { state = { ...state, ...value }; publish(); };
  const refreshAfterMutation = async () => { await Promise.all([store.load(state.filters), store.loadContinue()]); };

  const store: ActivityStore = {
    getSnapshot: () => clone(state),
    subscribe(listener) { listeners.add(listener); listener(clone(state)); return () => listeners.delete(listener); },
    setFilters(filters) { patch({ filters: { ...filters } }); },
    async load(filters = state.filters) {
      const lease = timelineGate.begin(); patch({ filters: { ...filters }, timelineGeneration: lease.generation, isLoadingTimeline: true, isLoadingMore: false, error: null });
      try {
        const [events, summary] = await Promise.all([api.events({ filters, limit: 50 }, lease.signal), api.summary(filters, lease.signal)]);
        if (!timelineGate.isCurrent(lease.generation)) return;
        patch({ events: events.events, nextCursor: events.nextCursor, summary, isLoadingTimeline: false });
      } catch (error) {
        if (!timelineGate.isCurrent(lease.generation)) return;
        patch({ isLoadingTimeline: false, error: toError(error, "timeline") });
      }
    },
    async loadMore() {
      if (!state.nextCursor || state.isLoadingTimeline || state.isLoadingMore) return;
      const lease = timelineGate.begin(); const cursor = state.nextCursor; patch({ timelineGeneration: lease.generation, isLoadingMore: true, error: null });
      try {
        const events = await api.events({ filters: state.filters, cursor, limit: 50 }, lease.signal);
        if (!timelineGate.isCurrent(lease.generation)) return;
        patch({ events: [...state.events, ...events.events], nextCursor: events.nextCursor, isLoadingMore: false });
      } catch (error) {
        if (!timelineGate.isCurrent(lease.generation)) return;
        patch({ isLoadingMore: false, error: toError(error, "timeline_more") });
      }
    },
    async loadContinue(query = {}) {
      const lease = continueGate.begin(); patch({ continueGeneration: lease.generation, isLoadingContinue: true, error: null });
      try {
        const continueCandidates = await api.continueCandidates(query, lease.signal);
        if (continueGate.isCurrent(lease.generation)) patch({ continueCandidates, isLoadingContinue: false });
      } catch (error) {
        if (continueGate.isCurrent(lease.generation)) patch({ isLoadingContinue: false, error: toError(error, "continue") });
      }
    },
    async edit(id, editPatch) {
      const lease = mutationGate.begin();
      try { const updated = await api.editEvent(id, editPatch, lease.signal); if (!mutationGate.isCurrent(lease.generation)) return null; await refreshAfterMutation(); return updated; }
      catch (error) { if (mutationGate.isCurrent(lease.generation)) patch({ error: toError(error, "edit") }); return null; }
    },
    async remove(id) {
      const lease = mutationGate.begin();
      try { const deleted = await api.deleteEvent(id, lease.signal); if (!mutationGate.isCurrent(lease.generation)) return false; if (deleted) await refreshAfterMutation(); return deleted; }
      catch (error) { if (mutationGate.isCurrent(lease.generation)) patch({ error: toError(error, "delete") }); return false; }
    },
    async export(format, path) {
      const lease = mutationGate.begin();
      try { const result = await api.exportEvents(state.filters, format, path, lease.signal); return mutationGate.isCurrent(lease.generation) ? result : null; }
      catch (error) { if (mutationGate.isCurrent(lease.generation)) patch({ error: toError(error, "export") }); return null; }
    },
    cancelTimeline() { timelineGate.cancel(); patch({ timelineGeneration: timelineGate.currentGeneration(), isLoadingTimeline: false, isLoadingMore: false }); },
    cancelContinue() { continueGate.cancel(); patch({ continueGeneration: continueGate.currentGeneration(), isLoadingContinue: false }); },
  };
  return store;
}
