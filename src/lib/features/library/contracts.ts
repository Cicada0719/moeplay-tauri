export interface PlatformIdentity {
  source: string;
  id: string;
}

export interface GameIdentity {
  launchPath: string | null;
  platformId: PlatformIdentity | null;
  titleFingerprint: string;
}

export interface ImportSourceRecord {
  sourceRecordId: string;
  title: string;
  launchPath: string | null;
  installDir: string | null;
  platformId: PlatformIdentity | null;
  launchUri: string | null;
  fields: Record<string, unknown>;
}

export type ImportAction = "create" | "update" | "merge" | "conflict" | "ignore";
export type ImportReasonCode =
  | "new_identity"
  | "launch_path_match"
  | "platform_id_match"
  | "ambiguous_strong_identity"
  | "title_recall_only"
  | "no_launch_target"
  | "explicit_merge"
  | "explicit_ignore"
  | "stale_preview"
  | "invalid_decision";

export interface ImportReason {
  code: ImportReasonCode;
  message: string;
  recalledGameIds: string[];
}

export type IdentityMatchKind = "platform_id" | "launch_path" | "title_recall";

export interface IdentityMatch {
  gameId: string;
  gameTitle: string;
  kind: IdentityMatchKind;
  confidence: number;
}

export type FieldDiffDisposition =
  | "unchanged"
  | "fill_empty"
  | "replace_imported"
  | "preserve_user"
  | "conflict";

export interface FieldProvenance {
  gameId: string;
  field: string;
  source: string;
  sourceRecordId: string;
  importedAt: string;
  appliedValue: unknown;
  valueHash: string;
}

export interface FieldDiff {
  field: string;
  current: unknown;
  incoming: unknown;
  disposition: FieldDiffDisposition;
  willApply: boolean;
  currentProvenance: FieldProvenance | null;
  incomingSource: string;
}

export interface ImportCandidate {
  id: string;
  source: string;
  identity: GameIdentity;
  action: ImportAction;
  reason: ImportReason;
  matches: IdentityMatch[];
  targetGameId: string | null;
  fieldDiff: FieldDiff[];
  record: ImportSourceRecord;
}

export interface PreviewImportRequest {
  source: string;
  records: ImportSourceRecord[];
}

export interface ImportPreview {
  previewId: string;
  source: string;
  candidates: ImportCandidate[];
  createdAt: string;
  writeCount: number;
}

export interface ImportDecision {
  candidateId: string;
  action: ImportAction;
  targetGameId: string | null;
}

export interface ApplyImportRequest {
  preview: ImportPreview;
  decisions: ImportDecision[];
  idempotencyKey: string;
}

export type ApplyItemStatus =
  | "created"
  | "updated"
  | "merged"
  | "no_changes"
  | "ignored"
  | "conflict"
  | "failed"
  | "already_applied";

export interface ApplyItemResult {
  candidateId: string;
  itemIdempotencyKey: string;
  action: ImportAction;
  status: ApplyItemStatus;
  gameId: string | null;
  message: string;
  appliedFields: string[];
  preservedFields: string[];
}

export interface FieldProvenanceChange {
  itemIdempotencyKey: string;
  gameId: string;
  field: string;
  before: unknown;
  after: unknown;
  previous: FieldProvenance | null;
  current: FieldProvenance;
}

export interface ApplyImportResponse {
  jobId: string;
  idempotencyKey: string;
  replayed: boolean;
  results: ApplyItemResult[];
  provenanceChanges: FieldProvenanceChange[];
}

export type LibraryHealthState = "healthy" | "needs_attention" | "degraded";

export interface LibraryHealthIssue {
  code: string;
  severity: string;
  message: string;
  gameIds: string[];
}

export interface LibraryHealthSnapshot {
  state: LibraryHealthState;
  totalGames: number;
  missingLaunchTargets: number;
  duplicateIdentityGroups: number;
  titleRecallGroups: number;
  unresolvedImportConflicts: number;
  provenanceCoverage: number;
  issues: LibraryHealthIssue[];
}

/** Pure feature boundary. A Tauri adapter can implement this without coupling the store to invoke(). */
export interface LibraryApi {
  preview(request: PreviewImportRequest, signal: AbortSignal): Promise<ImportPreview>;
  apply(request: ApplyImportRequest, signal: AbortSignal): Promise<ApplyImportResponse>;
  health(signal: AbortSignal): Promise<LibraryHealthSnapshot>;
}
