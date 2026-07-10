import type { SnapshotDiff } from "../../api";

export interface SnapshotDiffSummary {
  changedFiles: number;
  totalCompared: number;
  destructive: boolean;
}

export function summarizeSnapshotDiff(diff: SnapshotDiff): SnapshotDiffSummary {
  const changedFiles = diff.added.length + diff.removed.length + diff.changed.length;
  return {
    changedFiles,
    totalCompared: changedFiles + diff.unchanged,
    destructive: diff.removed.length > 0 || diff.changed.length > 0,
  };
}
