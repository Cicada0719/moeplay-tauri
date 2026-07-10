import type { ActivitySummary } from "../../features/activity";

export interface ActivityDurationBuckets {
  exactSeconds: number;
  estimatedSeconds: number;
  progressOnlyEvents: number;
}

export function splitActivityDurations(summary: ActivitySummary | null | undefined): ActivityDurationBuckets {
  return {
    exactSeconds: summary?.exactDurationSeconds ?? 0,
    estimatedSeconds: summary?.estimatedDurationSeconds ?? 0,
    progressOnlyEvents: summary?.progressOnlyCount ?? 0,
  };
}
