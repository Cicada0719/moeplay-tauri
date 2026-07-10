export type ActivityResourceKind = "game" | "anime" | "comic";
export type ActivityEventType = "started" | "progressed" | "completed" | "rated" | "favorited" | "imported" | "failed";
export type DurationQuality = "exact" | "estimated" | "progress_only";

export interface ActivityFilters {
  resourceKind?: ActivityResourceKind | null;
  resourceId?: string | null;
  eventType?: ActivityEventType | null;
  startedAtFrom?: string | null;
  startedAtTo?: string | null;
}
export interface ActivityCursor { startedAt: string; id: string; }
export interface ActivityEvent {
  id: string; resourceKind: ActivityResourceKind; resourceId: string; eventType: ActivityEventType;
  startedAt: string; endedAt: string | null; durationSeconds: number | null; providerId: string | null; payload: unknown;
}
export interface ActivityEventView extends ActivityEvent { durationQuality: DurationQuality; sourceLegacyId: string | null; }
export interface ActivityEventsRequest { filters: ActivityFilters; cursor?: ActivityCursor | null; limit?: number | null; }
export interface ActivityEventsResponse { events: ActivityEventView[]; nextCursor: ActivityCursor | null; }
export interface ActivityDaySummary { day: string; eventCount: number; exactDurationSeconds: number; estimatedDurationSeconds: number; progressOnlyCount: number; }
export interface ActivitySummary { eventCount: number; exactDurationSeconds: number; estimatedDurationSeconds: number; progressOnlyCount: number; days: ActivityDaySummary[]; }
export interface ActivityEventPatch {
  eventType?: ActivityEventType; startedAt?: string; endedAt?: string | null; durationSeconds?: number | null;
  providerId?: string | null; payload?: unknown; durationQuality?: DurationQuality;
}
export interface ContinueCandidate {
  resourceKind: ActivityResourceKind; resourceId: string; providerId: string | null; title: string; artworkUrl: string | null;
  position: unknown; updatedAt: string; completed: boolean; durationQuality: DurationQuality;
  exactDurationSeconds: number | null; estimatedDurationSeconds: number | null;
}
export interface ContinueQuery { limit?: number; includeCompleted?: boolean; }
export type ActivityExportFormat = "json" | "csv";
export interface ActivityApi {
  events(request: ActivityEventsRequest, signal: AbortSignal): Promise<ActivityEventsResponse>;
  summary(filters: ActivityFilters, signal: AbortSignal): Promise<ActivitySummary>;
  continueCandidates(query: ContinueQuery, signal: AbortSignal): Promise<ContinueCandidate[]>;
  editEvent(id: string, patch: ActivityEventPatch, signal: AbortSignal): Promise<ActivityEventView>;
  deleteEvent(id: string, signal: AbortSignal): Promise<boolean>;
  exportEvents(filters: ActivityFilters, format: ActivityExportFormat, path: string, signal: AbortSignal): Promise<string>;
}
export interface ActivityError { operation: string; message: string; cancelled: boolean; }
