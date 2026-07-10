export type AiProviderKind = "openai_compatible" | "ollama" | "mock";

export interface AiCapabilities {
  structuredOutput: boolean;
  jsonMode: boolean;
  streaming: boolean;
  vision: boolean;
  local: boolean;
  maxContextTokens: number | null;
}

/** Public provider configuration. Secret values and secret references are excluded. */
export interface AiProviderConfig {
  id: string;
  kind: AiProviderKind;
  displayName: string;
  baseUrl: string;
  model: string;
  secretConfigured: boolean;
  capabilities: AiCapabilities;
  enabled: boolean;
}

export type AiMessageRole = "system" | "user" | "assistant";

export interface AiMessage {
  role: AiMessageRole;
  content: string;
}

export interface StructuredRequest {
  taskId: string;
  promptId: string;
  promptVersion: string;
  schemaId: string;
  model: string;
  messages: AiMessage[];
  temperature: number;
  maxOutputTokens: number;
}

export interface TokenUsage {
  inputTokens: number | null;
  outputTokens: number | null;
}

export interface StructuredResponse {
  providerId: string;
  model: string;
  content: string;
  usage: TokenUsage;
  finishReason: string | null;
}

export type AiErrorKind =
  | "not_configured"
  | "auth"
  | "rate_limited"
  | "budget_exceeded"
  | "timeout"
  | "invalid_output"
  | "provider_unavailable"
  | "cancelled"
  | "policy_rejected";

export class AiGatewayError extends Error {
  constructor(
    public readonly kind: AiErrorKind,
    message: string,
    public readonly retryable = false,
    public readonly retryAfterMs: number | null = null,
  ) {
    super(message);
    this.name = "AiGatewayError";
  }
}

export interface LibraryGameContext {
  id: string;
  title: string;
  description?: string | null;
  tags: string[];
  metadata: Record<string, unknown>;
}

export interface LibraryCleanupInput {
  games: LibraryGameContext[];
}

export type LibraryOperation =
  | { type: "set_field"; gameId: string; field: string; value: unknown; reason: string }
  | { type: "add_tag"; gameId: string; tag: string; reason: string }
  | { type: "possible_duplicate"; gameIds: string[]; reason: string }
  | { type: "needs_review"; gameId: string; reason: string };

export interface LibraryChangeSetOutput {
  summary: string;
  confidence: number;
  operations: LibraryOperation[];
}

export type ResourceFilterKind = "game" | "anime" | "comic";
export type SortDirection = "asc" | "desc";

export interface FilterClause {
  field: string;
  op: string;
  value?: unknown;
}

export interface SortClause {
  field: string;
  direction: SortDirection;
}

export interface NaturalLanguageFilterOutput {
  kind: ResourceFilterKind;
  filters: FilterClause[];
  sort: SortClause[];
  explanation: string;
}
