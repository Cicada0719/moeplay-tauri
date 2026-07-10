import type { LibraryOperation } from "./contracts";
import type { ValidatedLibraryChangeSet } from "./schema";

export type ChangeSetState = "awaiting_confirmation" | "applied" | "rejected";

export interface PreviewOperation {
  operation: LibraryOperation;
  selected: boolean;
}

export interface AiChangeSetPreview {
  id: string;
  taskId: string;
  summary: string;
  confidence: number;
  state: ChangeSetState;
  operations: PreviewOperation[];
}

/** Only branded, validated output can enter the confirmation workflow. */
export function buildLibraryChangeSetPreview(
  id: string,
  taskId: string,
  validated: ValidatedLibraryChangeSet,
): AiChangeSetPreview {
  return {
    id,
    taskId,
    summary: validated.value.summary,
    confidence: validated.value.confidence,
    state: "awaiting_confirmation",
    operations: validated.value.operations.map((operation) => ({ operation, selected: false })),
  };
}
