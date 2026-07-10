import { afterEach, describe, expect, it } from "vitest";
import { backfillLegacyGameActivityOnce, resetBackfillLegacyGameActivityForTests, shouldFallbackActivityV2 } from "./backfill";
import { splitActivityDurations } from "./metrics";
import { createActivityStore } from "../../features/activity/store";
import type { ActivityApi, ActivityEventsRequest, ActivityEventsResponse, ActivitySummary, ContinueCandidate } from "../../features/activity/contracts";

const summary: ActivitySummary = {
  eventCount: 3,
  exactDurationSeconds: 3600,
  estimatedDurationSeconds: 900,
  progressOnlyCount: 1,
  days: [],
};

function event(id: string, kind: "game" | "anime" | "comic" = "game") {
  return {
    id,
    resourceKind: kind,
    resourceId: `${kind}-1`,
    eventType: "progressed" as const,
    startedAt: `2026-01-0${id === "one" ? "1" : "2"}T00:00:00Z`,
    endedAt: null,
    durationSeconds: id === "one" ? 3600 : null,
    providerId: null,
    payload: {},
    durationQuality: id === "one" ? "exact" as const : "progress_only" as const,
    sourceLegacyId: null,
  };
}

const emptyContinue: ContinueCandidate[] = [];

function apiFor(events: (request: ActivityEventsRequest) => Promise<ActivityEventsResponse>): ActivityApi {
  return {
    events,
    summary: async () => summary,
    continueCandidates: async () => emptyContinue,
    editEvent: async () => event("edited"),
    deleteEvent: async () => true,
    exportEvents: async (_filters, format, path) => `${path}.${format}`,
  };
}

afterEach(() => resetBackfillLegacyGameActivityForTests());

describe("Activity v2 dashboard helpers", () => {
  it("runs legacy backfill only once and shares the in-flight promise", async () => {
    let calls = 0;
    const invoke = async () => { calls += 1; return { created: 2 }; };
    const [first, second] = await Promise.all([
      backfillLegacyGameActivityOnce(invoke),
      backfillLegacyGameActivityOnce(invoke),
    ]);
    expect(calls).toBe(1);
    expect(first).toEqual({ created: 2 });
    expect(second).toEqual({ created: 2 });
  });

  it("keeps exact, estimated, and progress-only buckets separate", () => {
    expect(splitActivityDurations(summary)).toEqual({ exactSeconds: 3600, estimatedSeconds: 900, progressOnlyEvents: 1 });
    expect(splitActivityDurations(summary).exactSeconds + splitActivityDurations(summary).estimatedSeconds).toBe(4500);
    expect(splitActivityDurations(summary).progressOnlyEvents).toBe(1);
  });

  it("marks data-load failures for old aggregate fallback but not mutations", () => {
    expect(shouldFallbackActivityV2("backfill")).toBe(true);
    expect(shouldFallbackActivityV2("timeline")).toBe(true);
    expect(shouldFallbackActivityV2("continue")).toBe(true);
    expect(shouldFallbackActivityV2("edit")).toBe(false);
    expect(shouldFallbackActivityV2("export")).toBe(false);
  });

  it("passes filters, appends the next page, and keeps the cursor contract", async () => {
    const requests: ActivityEventsRequest[] = [];
    const api = apiFor(async (request) => {
      requests.push(request);
      if (!request.cursor) return { events: [event("one", "anime")], nextCursor: { startedAt: event("one").startedAt, id: "one" } };
      return { events: [event("two", "comic")], nextCursor: null };
    });
    const store = createActivityStore(api);

    await store.load({ resourceKind: "anime", eventType: "progressed" });
    await store.loadMore();

    expect(requests[0].filters).toEqual({ resourceKind: "anime", eventType: "progressed" });
    expect(requests[0].limit).toBe(50);
    expect(requests[1].cursor).toEqual({ startedAt: event("one").startedAt, id: "one" });
    expect(requests[1].filters).toEqual({ resourceKind: "anime", eventType: "progressed" });
    expect(store.getSnapshot().events.map((item) => item.id)).toEqual(["one", "two"]);
    expect(store.getSnapshot().nextCursor).toBeNull();
  });

  it("does not let a late filtered response overwrite a newer response", async () => {
    let resolveOld!: (value: ActivityEventsResponse) => void;
    let resolveNew!: (value: ActivityEventsResponse) => void;
    const old = new Promise<ActivityEventsResponse>((resolve) => { resolveOld = resolve; });
    const newer = new Promise<ActivityEventsResponse>((resolve) => { resolveNew = resolve; });
    let calls = 0;
    const api = apiFor(async () => ++calls === 1 ? old : newer);
    const store = createActivityStore(api);
    const oldLoad = store.load({ resourceKind: "game" });
    const newLoad = store.load({ resourceKind: "comic" });
    resolveNew({ events: [event("new", "comic")], nextCursor: null });
    await newLoad;
    resolveOld({ events: [event("old", "game")], nextCursor: null });
    await oldLoad;
    expect(store.getSnapshot().events.map((item) => item.id)).toEqual(["new"]);
    expect(store.getSnapshot().filters.resourceKind).toBe("comic");
  });
});
