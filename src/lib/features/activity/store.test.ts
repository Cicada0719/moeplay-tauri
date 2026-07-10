import { describe, expect, it } from "vitest";
import { createActivityStore } from "./store";
import type { ActivityApi, ActivityEventsResponse, ActivitySummary, ContinueCandidate } from "./contracts";

function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; }
const summary: ActivitySummary = { eventCount: 0, exactDurationSeconds: 0, estimatedDurationSeconds: 0, progressOnlyCount: 0, days: [] };
const page = (id: string): ActivityEventsResponse => ({ events: [{ id, resourceKind: "game", resourceId: "g", eventType: "progressed", startedAt: "2026-01-01T00:00:00Z", endedAt: null, durationSeconds: null, providerId: null, payload: {}, durationQuality: "progress_only", sourceLegacyId: null }], nextCursor: null });

describe("ActivityStore", () => {
  it("ignores a stale timeline completion after a newer filter generation", async () => {
    const first = deferred<ActivityEventsResponse>(); const second = deferred<ActivityEventsResponse>(); let calls = 0;
    const api: ActivityApi = { events: async () => (++calls === 1 ? first.promise : second.promise), summary: async () => summary, continueCandidates: async () => [], editEvent: async () => { throw new Error("unused"); }, deleteEvent: async () => false, exportEvents: async () => "" };
    const store = createActivityStore(api); const oldLoad = store.load({ resourceKind: "game" }); const newLoad = store.load({ resourceKind: "anime" });
    second.resolve(page("new")); await newLoad; first.resolve(page("old")); await oldLoad;
    expect(store.getSnapshot().events.map((event) => event.id)).toEqual(["new"]); expect(store.getSnapshot().filters.resourceKind).toBe("anime");
  });
  it("cancels the continue rail generation and leaves the late result invisible", async () => {
    const result = deferred<ContinueCandidate[]>(); const api: ActivityApi = { events: async () => page("unused"), summary: async () => summary, continueCandidates: async () => result.promise, editEvent: async () => { throw new Error("unused"); }, deleteEvent: async () => false, exportEvents: async () => "" };
    const store = createActivityStore(api); const loading = store.loadContinue(); store.cancelContinue(); result.resolve([{ resourceKind: "comic", resourceId: "c", providerId: null, title: "Late", artworkUrl: null, position: {}, updatedAt: "2026-01-01T00:00:00Z", completed: false, durationQuality: "progress_only", exactDurationSeconds: null, estimatedDurationSeconds: null }]); await loading;
    expect(store.getSnapshot().continueCandidates).toEqual([]); expect(store.getSnapshot().isLoadingContinue).toBe(false);
  });
});
