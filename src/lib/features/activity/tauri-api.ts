import { invokeCmd } from "../../api/core";
import type {
  ActivityApi, ActivityEventPatch, ActivityEventsRequest, ActivityEventsResponse,
  ActivityExportFormat, ActivityFilters, ActivitySummary, ActivityEventView,
  ContinueCandidate, ContinueQuery,
} from "./contracts";

function ensureActive(signal: AbortSignal): void {
  if (signal.aborted) throw new DOMException("request cancelled", "AbortError");
}

export const tauriActivityApi: ActivityApi = {
  async events(request: ActivityEventsRequest, signal: AbortSignal): Promise<ActivityEventsResponse> {
    ensureActive(signal); const value = await invokeCmd<ActivityEventsResponse>("get_activity_events", { request }); ensureActive(signal); return value;
  },
  async summary(filters: ActivityFilters, signal: AbortSignal): Promise<ActivitySummary> {
    ensureActive(signal); const value = await invokeCmd<ActivitySummary>("get_activity_summary", { filters }); ensureActive(signal); return value;
  },
  async continueCandidates(query: ContinueQuery, signal: AbortSignal): Promise<ContinueCandidate[]> {
    ensureActive(signal); const value = await invokeCmd<ContinueCandidate[]>("get_continue_candidates", { query }); ensureActive(signal); return value;
  },
  async editEvent(id: string, patch: ActivityEventPatch, signal: AbortSignal): Promise<ActivityEventView> {
    ensureActive(signal); const value = await invokeCmd<ActivityEventView>("edit_activity_event", { id, patch }); ensureActive(signal); return value;
  },
  async deleteEvent(id: string, signal: AbortSignal): Promise<boolean> {
    ensureActive(signal); const value = await invokeCmd<boolean>("delete_activity_event", { id }); ensureActive(signal); return value;
  },
  async exportEvents(filters: ActivityFilters, format: ActivityExportFormat, path: string, signal: AbortSignal): Promise<string> {
    ensureActive(signal); const value = await invokeCmd<string>("export_activity_events", { filters, format, path }); ensureActive(signal); return value;
  },
};
