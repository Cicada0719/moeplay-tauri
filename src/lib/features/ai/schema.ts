import {
  AiGatewayError,
  type FilterClause,
  type LibraryChangeSetOutput,
  type LibraryCleanupInput,
  type LibraryOperation,
  type NaturalLanguageFilterOutput,
  type ResourceFilterKind,
  type SortClause,
} from "./contracts";

export const MAX_STRUCTURED_OUTPUT_BYTES = 262_144;
export const MAX_LIBRARY_OPERATIONS = 100;
export const MAX_FILTER_CLAUSES = 24;
export const MAX_SORT_CLAUSES = 4;

const validatedLibraryBrand: unique symbol = Symbol("validated-library-change-set");
const validatedFilterBrand: unique symbol = Symbol("validated-natural-filter");

export interface ValidatedLibraryChangeSet {
  readonly value: LibraryChangeSetOutput;
  readonly [validatedLibraryBrand]: true;
}

export interface ValidatedNaturalLanguageFilter {
  readonly value: NaturalLanguageFilterOutput;
  readonly [validatedFilterBrand]: true;
}

function invalid(message: string): never {
  throw new AiGatewayError("invalid_output", message);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, required: string[], optional: string[] = []): void {
  const allowed = new Set([...required, ...optional]);
  if (required.some((key) => !(key in value)) || Object.keys(value).some((key) => !allowed.has(key))) {
    invalid("structured output contained missing or unknown fields");
  }
}

function text(value: unknown, field: string, max: number): string {
  if (typeof value !== "string" || value.trim().length === 0 || [...value.trim()].length > max) {
    invalid(`${field} must contain between 1 and ${max} characters`);
  }
  return value;
}

export function parseJsonDocument(content: string): unknown {
  if (new TextEncoder().encode(content).byteLength > MAX_STRUCTURED_OUTPUT_BYTES) {
    invalid("structured AI output exceeded the size limit");
  }
  const trimmed = content.trim();
  if (!trimmed) invalid("structured AI output was empty");
  let document = trimmed;
  if (trimmed.startsWith("```")) {
    const firstNewline = trimmed.indexOf("\n");
    if (firstNewline < 0 || !trimmed.endsWith("```")) {
      invalid("structured AI output contained an invalid fenced block");
    }
    const marker = trimmed.slice(0, firstNewline).toLowerCase();
    if (marker !== "```" && marker !== "```json") {
      invalid("structured AI output used an unsupported fenced block");
    }
    document = trimmed.slice(firstNewline + 1, -3).trim();
    if (document.includes("```")) invalid("structured AI output contained multiple fenced blocks");
  }
  try {
    return JSON.parse(document);
  } catch {
    invalid("structured AI output was not a valid JSON document");
  }
}

function requireAllowedId(id: unknown, allowedIds: Set<string>): string {
  if (typeof id !== "string" || !allowedIds.has(id)) {
    invalid("library cleanup referenced an ID outside the supplied context");
  }
  return id;
}

function validateSetField(field: string, value: unknown): void {
  if (["title", "developer", "publisher"].includes(field)) {
    text(value, field, 200);
    return;
  }
  if (field === "description") {
    text(value, field, 5000);
    return;
  }
  if (field === "contentRating") {
    if (!["all_ages", "teen", "mature", "adult", "unknown"].includes(String(value))) {
      invalid("contentRating is not an allowed value");
    }
    return;
  }
  if (field === "estimatedHours") {
    if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 10_000) {
      invalid("estimatedHours must be a number between 0 and 10000");
    }
    return;
  }
  invalid("library cleanup attempted to change a non-whitelisted field");
}

function validateLibraryOperation(value: unknown, allowedIds: Set<string>): LibraryOperation {
  if (!isRecord(value) || typeof value.type !== "string") invalid("library operation is invalid");
  switch (value.type) {
    case "set_field": {
      exactKeys(value, ["type", "gameId", "field", "value", "reason"]);
      const gameId = requireAllowedId(value.gameId, allowedIds);
      const field = text(value.field, "field", 80);
      const reason = text(value.reason, "reason", 1000);
      validateSetField(field, value.value);
      return { type: "set_field", gameId, field, value: value.value, reason };
    }
    case "add_tag": {
      exactKeys(value, ["type", "gameId", "tag", "reason"]);
      return {
        type: "add_tag",
        gameId: requireAllowedId(value.gameId, allowedIds),
        tag: text(value.tag, "tag", 80),
        reason: text(value.reason, "reason", 1000),
      };
    }
    case "possible_duplicate": {
      exactKeys(value, ["type", "gameIds", "reason"]);
      if (!Array.isArray(value.gameIds) || value.gameIds.length < 2 || value.gameIds.length > 10) {
        invalid("possible_duplicate must contain between 2 and 10 IDs");
      }
      const gameIds = value.gameIds.map((id) => requireAllowedId(id, allowedIds));
      if (new Set(gameIds).size !== gameIds.length) invalid("possible_duplicate IDs must be unique");
      return { type: "possible_duplicate", gameIds, reason: text(value.reason, "reason", 1000) };
    }
    case "needs_review": {
      exactKeys(value, ["type", "gameId", "reason"]);
      return {
        type: "needs_review",
        gameId: requireAllowedId(value.gameId, allowedIds),
        reason: text(value.reason, "reason", 1000),
      };
    }
    default:
      invalid("library operation type is not whitelisted");
  }
}

export function validateLibraryCleanup(
  raw: unknown,
  input: LibraryCleanupInput,
): ValidatedLibraryChangeSet {
  if (!isRecord(raw)) invalid("library cleanup output must be an object");
  exactKeys(raw, ["summary", "confidence", "operations"]);
  const summary = text(raw.summary, "summary", 1000);
  if (typeof raw.confidence !== "number" || raw.confidence < 0 || raw.confidence > 1) {
    invalid("library cleanup confidence must be between 0 and 1");
  }
  if (!Array.isArray(raw.operations) || raw.operations.length > MAX_LIBRARY_OPERATIONS) {
    invalid("library cleanup returned too many operations");
  }
  const allowedIds = new Set(input.games.map((game) => game.id));
  if (allowedIds.size === 0 && raw.operations.length > 0) {
    invalid("library cleanup cannot target an empty input set");
  }
  const value: LibraryChangeSetOutput = {
    summary,
    confidence: raw.confidence,
    operations: raw.operations.map((operation) => validateLibraryOperation(operation, allowedIds)),
  };
  return { value, [validatedLibraryBrand]: true };
}

type ValueRule = "none" | "string" | "string_array" | "number" | "boolean" | "content_rating";

function filterRule(kind: ResourceFilterKind, field: string, op: string): ValueRule | null {
  if (
    (kind === "game" && field === "lastPlayedAt" && op === "is_null") ||
    (kind === "anime" && field === "lastWatchedAt" && op === "is_null") ||
    (kind === "comic" && field === "lastReadAt" && op === "is_null")
  ) return "none";
  if (field === "title" && op === "contains") return "string";
  if (field === "tags" && (op === "contains_any" || op === "contains_all")) return "string_array";
  if ((kind === "anime" || kind === "comic") && field === "genres" && (op === "contains_any" || op === "contains_all")) return "string_array";
  if (kind === "game" && field === "contentRating" && op === "eq") return "content_rating";
  if (
    ((kind === "game" && ["estimatedHours", "userAffinity"].includes(field)) ||
      (kind === "anime" && ["episodeCount", "userAffinity"].includes(field)) ||
      (kind === "comic" && ["chapterCount", "userAffinity"].includes(field))) &&
    (op === "lte" || op === "gte")
  ) return "number";
  if ((field === "favorite" || field === "completed") && op === "eq") return "boolean";
  return null;
}

function validateFilterClause(kind: ResourceFilterKind, raw: unknown): FilterClause {
  if (!isRecord(raw)) invalid("filter clause must be an object");
  exactKeys(raw, ["field", "op"], ["value"]);
  const field = text(raw.field, "filter field", 80);
  const op = text(raw.op, "filter operator", 40);
  if (field.toLowerCase().includes("sql") || op.toLowerCase().includes("sql")) {
    invalid("filter DSL cannot contain SQL");
  }
  const rule = filterRule(kind, field, op);
  if (!rule) invalid("filter field/operator combination is not whitelisted");
  if (rule === "none" && "value" in raw) invalid("filter operator does not accept a value");
  if (rule === "string") text(raw.value, "filter value", 200);
  if (rule === "string_array") {
    if (!Array.isArray(raw.value) || raw.value.length === 0 || raw.value.length > 20) {
      invalid("filter string array must contain between 1 and 20 values");
    }
    raw.value.forEach((value) => text(value, "filter value", 80));
  }
  if (rule === "number" && (typeof raw.value !== "number" || !Number.isFinite(raw.value) || Math.abs(raw.value) > 1_000_000)) {
    invalid("filter value must be a bounded number");
  }
  if (rule === "boolean" && typeof raw.value !== "boolean") invalid("filter value must be a boolean");
  if (rule === "content_rating" && !["all_ages", "teen", "mature", "adult", "unknown"].includes(String(raw.value))) {
    invalid("content rating filter value is not whitelisted");
  }
  return "value" in raw ? { field, op, value: raw.value } : { field, op };
}

function validateSortClause(kind: ResourceFilterKind, raw: unknown): SortClause {
  if (!isRecord(raw)) invalid("sort clause must be an object");
  exactKeys(raw, ["field", "direction"]);
  const field = text(raw.field, "sort field", 80);
  const allowed = kind === "game"
    ? ["title", "lastPlayedAt", "estimatedHours", "userAffinity", "addedAt"]
    : kind === "anime"
      ? ["title", "lastWatchedAt", "episodeCount", "userAffinity", "addedAt"]
      : ["title", "lastReadAt", "chapterCount", "userAffinity", "addedAt"];
  if (!allowed.includes(field)) invalid("sort field is not whitelisted");
  if (raw.direction !== "asc" && raw.direction !== "desc") invalid("sort direction is invalid");
  return { field, direction: raw.direction };
}

export function validateNaturalLanguageFilter(
  raw: unknown,
  expectedKind: ResourceFilterKind,
): ValidatedNaturalLanguageFilter {
  if (!isRecord(raw)) invalid("natural-language filter output must be an object");
  exactKeys(raw, ["kind", "filters", "sort", "explanation"]);
  if (raw.kind !== expectedKind) invalid("filter resource kind did not match the requested kind");
  if (!Array.isArray(raw.filters) || raw.filters.length > MAX_FILTER_CLAUSES) {
    invalid("natural-language filter returned too many clauses");
  }
  if (!Array.isArray(raw.sort) || raw.sort.length > MAX_SORT_CLAUSES) {
    invalid("natural-language filter returned too many sort clauses");
  }
  const value: NaturalLanguageFilterOutput = {
    kind: expectedKind,
    filters: raw.filters.map((filter) => validateFilterClause(expectedKind, filter)),
    sort: raw.sort.map((sort) => validateSortClause(expectedKind, sort)),
    explanation: text(raw.explanation, "explanation", 1000),
  };
  return { value, [validatedFilterBrand]: true };
}
