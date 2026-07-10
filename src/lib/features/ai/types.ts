import type {
  AiCapabilities,
  LibraryOperation,
  NaturalLanguageFilterOutput,
  ResourceFilterKind,
} from "./contracts";
import type { AiChangeSetPreview } from "./change-set";

export type AiAvailability = "ready" | "degraded" | "offline" | "disabled";
export type AiHealthState = "healthy" | "degraded" | "offline" | "disabled" | "unknown";
export type AiTaskStatus = "queued" | "running" | "paused" | "succeeded" | "failed" | "cancelled";
export type AiBackendErrorKind =
  | "not_configured"
  | "auth"
  | "rate_limited"
  | "budget_exceeded"
  | "timeout"
  | "invalid_output"
  | "provider_unavailable"
  | "cancelled"
  | "policy_rejected";

/** Safe DTO only: secrets, raw prompts, raw responses and authorization data are intentionally absent. */
export interface AiProviderStatus {
  id: string;
  displayName: string;
  kind: "openai_compatible" | "ollama" | "mock" | string;
  model: string;
  enabled: boolean;
  secretConfigured: boolean;
  health: AiHealthState;
  errorKind?: AiBackendErrorKind | string;
  capabilities: AiCapabilities;
}

/** Safe task history summary. Never add prompt, response, headers, endpoint credentials or local paths. */
export interface AiTaskRecord {
  id: string;
  useCase: string;
  providerId: string;
  model: string;
  promptVersion: string;
  status: AiTaskStatus;
  createdAt: string;
  completedAt?: string | null;
  durationMs?: number | null;
  inputSummary: Record<string, number | string | boolean>;
  outputSchema: string;
  tokenEstimate?: number | null;
  errorKind?: string | null;
}

export interface AiStatusSnapshot {
  availability: AiAvailability;
  providers: AiProviderStatus[];
  activeTaskCount: number;
  dailyTokenEstimate?: number | null;
  dailyBudgetEstimate?: number | null;
  updatedAt: string;
}

export type NaturalLanguageFilterDsl = NaturalLanguageFilterOutput;

export interface CompileFilterRequest {
  query: string;
  kind: ResourceFilterKind;
  generation: number;
}

export interface CompileFilterResult {
  taskId: string;
  generation: number;
  schemaId: "natural_language_filter.dsl" | string;
  schemaVersion: string;
  dsl: unknown;
}

export interface StructuredFilterFallback {
  keyword: string;
  tag: string;
  maxHours: string;
  unplayedOnly: boolean;
  contentRating: "any" | "all_ages" | "teen" | "mature" | "adult" | "unknown";
  sort: "affinity" | "recent" | "title";
}

export interface NormalizedPreviewOperation {
  id: string;
  operation: LibraryOperation;
  selected: false;
}

export interface NormalizedChangeSetPreview extends Omit<AiChangeSetPreview, "operations"> {
  operations: NormalizedPreviewOperation[];
}

export interface LibraryCleanupPreviewRequest {
  scope: "game_library";
  limit: number;
  generation: number;
}

/** Compatibility request used by the existing confirmation component. */
export interface ApplyChangeSetRequest {
  changeSetId: string;
  selectedOperationIndexes: number[];
  confirmed: true;
}

export interface ApplyChangeSetResult {
  changeSetId: string;
  state: "applied";
  appliedOperationCount: number;
  /** Empty when the backend reports a no-op and therefore creates no undo record. */
  undoToken: string;
}

export interface UndoChangeSetResult {
  changeSetId: string;
  state: "reverted";
}

export interface AiRecommendationRequest {
  kind: ResourceFilterKind;
  candidateIds: string[];
  limit: number;
  generation: number;
}

export interface AiRecommendationResult {
  taskId: string;
  generation: number;
  explanations: unknown;
}

export interface ValidatedRecommendationExplanation {
  resourceId: string;
  explanation: string;
}

export interface ValidationSuccess<T> {
  ok: true;
  value: T;
}

export interface ValidationFailure {
  ok: false;
  errors: string[];
}

export type ValidationResult<T> = ValidationSuccess<T> | ValidationFailure;

// Native AI v2 command DTOs. These mirror Rust serde names exactly.
export interface AiV2ProviderSpec {
  id: string;
  kind: "openai_compatible" | "ollama" | "mock";
  displayName: string;
  baseUrl: string;
  model: string;
  enabled: boolean;
  maxContextTokens?: number | null;
}

export interface AiV2ProviderStatusDto {
  id: string;
  kind: AiV2ProviderSpec["kind"];
  displayName: string;
  model: string;
  endpointOrigin: string;
  endpointScope: "loopback" | "remote";
  enabled: boolean;
  credentialConfigured: boolean;
  ready: boolean;
  issue: AiBackendErrorKind | null;
  capabilities: AiCapabilities;
}

export interface AiV2BudgetSnapshotDto {
  committedTokens: number;
  reservedTokens: number;
  softWarningReached: boolean;
}

export interface AiV2TaskStatusDto {
  id: string;
  kind: string;
  status: AiTaskStatus;
  progress: number;
  createdAt: string;
  updatedAt: string;
  message: string | null;
  resultAvailable: boolean;
}

export interface AiV2ErrorDto {
  kind: AiBackendErrorKind;
  message: string;
  retryable: boolean;
  retryAfterMs: number | null;
}

export interface AiV2LibraryGameContext {
  id: string;
  title: string;
  description?: string | null;
  tags: string[];
  metadata: Record<string, unknown>;
}

export interface AiV2RecommendationCandidate {
  id: string;
  title: string;
  kind: ResourceFilterKind;
  available: boolean;
  estimatedMinutes?: number | null;
  signals: string[];
}

export type AiV2StructuredTaskInput =
  | { type: "library_cleanup"; input: { games: AiV2LibraryGameContext[] } }
  | { type: "natural_language_filter"; query: string; kind: ResourceFilterKind }
  | {
      type: "recommendation";
      input: {
        candidates: AiV2RecommendationCandidate[];
        excludedIds: string[];
        limit: number;
        request?: string | null;
      };
    };

export interface AiV2TaskStartSpec {
  providers: AiV2ProviderSpec[];
  primaryProviderId: string;
  fallbackProviderId?: string | null;
  fallbackAuthorization?: "disabled" | "same_scope_only" | "explicit_cross_scope";
  task: AiV2StructuredTaskInput;
}

export interface AiV2RecommendationOutput {
  summary: string;
  recommendations: Array<{
    resourceId: string;
    rank: number;
    reason: string;
    confidence: number;
    signals: string[];
  }>;
}

export type AiV2StructuredTaskResult =
  | { type: "library_cleanup"; changeSet: { summary: string; confidence: number; operations: LibraryOperation[] } }
  | { type: "natural_language_filter"; filter: NaturalLanguageFilterOutput }
  | { type: "recommendation"; recommendation: AiV2RecommendationOutput };

export interface AiV2ExecutionResultDto {
  taskId: string;
  providerId: string;
  model: string;
  promptId: string;
  promptVersion: string;
  schemaId: string;
  fallbackUsed: boolean;
  usage: { inputTokens: number | null; outputTokens: number | null };
  estimatedChargedTokens: number;
  result: AiV2StructuredTaskResult;
}

export interface AiV2TaskResultDto {
  task: AiV2TaskStatusDto;
  result: AiV2ExecutionResultDto | null;
  error: AiV2ErrorDto | null;
}

export interface AiChangeProvenanceDto {
  providerId: string;
  model: string;
  promptId: string;
  promptVersion: string;
}

export interface PreviewAiChangesResponseDto {
  changeSetId: string;
  taskId: string;
  operations: Array<{
    operationIndex: number;
    kind: string;
    gameIds: string[];
    field: string | null;
    before: unknown;
    after: unknown;
    reason: string;
    applicable: boolean;
  }>;
  writeCount: number;
}

export interface ApplyAiChangesResponseDto {
  status: "applied" | "no_changes";
  changeSetId: string;
  selectedOperationCount: number;
  changedFieldCount: number;
  undoId: string | null;
  appliedAt: string;
}

export interface UndoAiChangesResponseDto {
  status: "undone" | "already_undone";
  undoId: string;
  changeSetId: string;
  restoredFieldCount: number;
  undoneAt: string;
}
