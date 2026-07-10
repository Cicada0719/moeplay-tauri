import type {
  NaturalLanguageFilterDsl,
  NormalizedChangeSetPreview,
  StructuredFilterFallback,
  ValidatedRecommendationExplanation,
  ValidationResult,
} from "./types";
import type { AiChangeSetPreview } from "./change-set";
import type { LibraryOperation, ResourceFilterKind } from "./contracts";

const MAX_FILTERS = 24;
const MAX_SORTS = 4;
const MAX_OPERATIONS = 100;
const allowedKeys = (value: Record<string, unknown>, keys: string[]) =>
  Object.keys(value).every((key) => keys.includes(key));

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function nonEmptyText(value: unknown, max: number): value is string {
  return typeof value === "string" && value.trim().length > 0 && [...value.trim()].length <= max;
}

function finiteNumber(value: unknown, min: number, max: number): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= min && value <= max;
}

function filterRule(kind: ResourceFilterKind, field: string, op: string): "none" | "string" | "strings" | "number" | "boolean" | "rating" | null {
  if (
    (kind === "game" && field === "lastPlayedAt" && op === "is_null") ||
    (kind === "anime" && field === "lastWatchedAt" && op === "is_null") ||
    (kind === "comic" && field === "lastReadAt" && op === "is_null")
  ) return "none";
  if (field === "title" && op === "contains") return "string";
  if (field === "tags" && (op === "contains_any" || op === "contains_all")) return "strings";
  if ((kind === "anime" || kind === "comic") && field === "genres" && (op === "contains_any" || op === "contains_all")) return "strings";
  if (kind === "game" && field === "contentRating" && op === "eq") return "rating";
  if (
    (kind === "game" && ["estimatedHours", "userAffinity"].includes(field) && ["lte", "gte"].includes(op)) ||
    (kind === "anime" && ["episodeCount", "userAffinity"].includes(field) && ["lte", "gte"].includes(op)) ||
    (kind === "comic" && ["chapterCount", "userAffinity"].includes(field) && ["lte", "gte"].includes(op))
  ) return "number";
  if (["favorite", "completed"].includes(field) && op === "eq") return "boolean";
  return null;
}

function validSortField(kind: ResourceFilterKind, field: string) {
  const common = ["title", "userAffinity", "addedAt"];
  const byKind = {
    game: ["lastPlayedAt", "estimatedHours"],
    anime: ["lastWatchedAt", "episodeCount"],
    comic: ["lastReadAt", "chapterCount"],
  } as const;
  return common.includes(field) || byKind[kind].includes(field as never);
}

export function validateFilterDslResult(
  input: unknown,
  expectedKind: ResourceFilterKind,
): ValidationResult<NaturalLanguageFilterDsl> {
  const errors: string[] = [];
  if (!record(input) || !allowedKeys(input, ["kind", "filters", "sort", "explanation"])) {
    return { ok: false, errors: ["结果不是受支持的本地筛选 DSL。"] };
  }
  if (input.kind !== expectedKind) errors.push("资源类型与当前筛选范围不一致。");
  if (!Array.isArray(input.filters) || input.filters.length > MAX_FILTERS) {
    errors.push(`筛选条件必须是数组且不超过 ${MAX_FILTERS} 条。`);
  }
  if (!Array.isArray(input.sort) || input.sort.length > MAX_SORTS) {
    errors.push(`排序条件必须是数组且不超过 ${MAX_SORTS} 条。`);
  }
  if (!nonEmptyText(input.explanation, 2000)) errors.push("筛选解释缺失或过长。");

  const filters = Array.isArray(input.filters) ? input.filters : [];
  filters.forEach((clause, index) => {
    if (!record(clause) || !allowedKeys(clause, ["field", "op", "value"]) || typeof clause.field !== "string" || typeof clause.op !== "string") {
      errors.push(`第 ${index + 1} 条筛选结构无效。`);
      return;
    }
    if (/sql|script|command/i.test(clause.field) || /sql|script|command/i.test(clause.op)) {
      errors.push(`第 ${index + 1} 条筛选包含禁止的执行语义。`);
      return;
    }
    const rule = filterRule(expectedKind, clause.field, clause.op);
    if (!rule) {
      errors.push(`第 ${index + 1} 条筛选字段或操作符不在白名单中。`);
      return;
    }
    const value = clause.value;
    if (rule === "none" && value !== undefined) errors.push(`第 ${index + 1} 条筛选不接受值。`);
    if (rule === "string" && !nonEmptyText(value, 200)) errors.push(`第 ${index + 1} 条筛选需要有效文本。`);
    if (rule === "strings" && (!Array.isArray(value) || value.length < 1 || value.length > 20 || value.some((entry) => !nonEmptyText(entry, 80)))) {
      errors.push(`第 ${index + 1} 条筛选需要 1 至 20 个有效文本值。`);
    }
    if (rule === "number" && !finiteNumber(value, -1_000_000, 1_000_000)) errors.push(`第 ${index + 1} 条筛选需要有界数字。`);
    if (rule === "boolean" && typeof value !== "boolean") errors.push(`第 ${index + 1} 条筛选需要布尔值。`);
    if (rule === "rating" && !["all_ages", "teen", "mature", "adult", "unknown"].includes(String(value))) {
      errors.push(`第 ${index + 1} 条筛选包含未知分级。`);
    }
  });

  const sort = Array.isArray(input.sort) ? input.sort : [];
  sort.forEach((clause, index) => {
    if (!record(clause) || !allowedKeys(clause, ["field", "direction"]) || typeof clause.field !== "string" || !validSortField(expectedKind, clause.field) || !["asc", "desc"].includes(String(clause.direction))) {
      errors.push(`第 ${index + 1} 条排序不在白名单中。`);
    }
  });

  if (errors.length) return { ok: false, errors };
  return { ok: true, value: input as unknown as NaturalLanguageFilterDsl };
}

export function buildStructuredFallback(input: StructuredFilterFallback): NaturalLanguageFilterDsl {
  const filters: NaturalLanguageFilterDsl["filters"] = [];
  if (input.keyword.trim()) filters.push({ field: "title", op: "contains", value: input.keyword.trim() });
  if (input.tag.trim()) filters.push({ field: "tags", op: "contains_any", value: [input.tag.trim()] });
  if (input.maxHours.trim()) {
    const hours = Number(input.maxHours);
    if (Number.isFinite(hours) && hours >= 0 && hours <= 10_000) filters.push({ field: "estimatedHours", op: "lte", value: hours });
  }
  if (input.unplayedOnly) filters.push({ field: "lastPlayedAt", op: "is_null" });
  if (input.contentRating !== "any") filters.push({ field: "contentRating", op: "eq", value: input.contentRating });
  const sortField = input.sort === "recent" ? "addedAt" : input.sort === "title" ? "title" : "userAffinity";
  return {
    kind: "game",
    filters,
    sort: [{ field: sortField, direction: input.sort === "title" ? "asc" : "desc" }],
    explanation: "由本地结构化筛选生成，未调用 AI。",
  };
}

function validateOperation(operation: unknown): operation is LibraryOperation {
  if (!record(operation) || typeof operation.type !== "string" || !nonEmptyText(operation.reason, 1000)) return false;
  if (operation.type === "set_field") {
    if (!allowedKeys(operation, ["type", "gameId", "field", "value", "reason"]) || !nonEmptyText(operation.gameId, 200) || typeof operation.field !== "string") return false;
    if (["title", "developer", "publisher"].includes(operation.field)) return nonEmptyText(operation.value, 200);
    if (operation.field === "description") return nonEmptyText(operation.value, 5000);
    if (operation.field === "contentRating") return ["all_ages", "teen", "mature", "adult", "unknown"].includes(String(operation.value));
    if (operation.field === "estimatedHours") return finiteNumber(operation.value, 0, 10_000);
    return false;
  }
  if (operation.type === "add_tag") {
    return allowedKeys(operation, ["type", "gameId", "tag", "reason"]) && nonEmptyText(operation.gameId, 200) && nonEmptyText(operation.tag, 80);
  }
  if (operation.type === "possible_duplicate") {
    return allowedKeys(operation, ["type", "gameIds", "reason"]) && Array.isArray(operation.gameIds) && operation.gameIds.length >= 2 && operation.gameIds.length <= 10 && new Set(operation.gameIds).size === operation.gameIds.length && operation.gameIds.every((id) => nonEmptyText(id, 200));
  }
  if (operation.type === "needs_review") {
    return allowedKeys(operation, ["type", "gameId", "reason"]) && nonEmptyText(operation.gameId, 200);
  }
  return false;
}

export function validateChangeSetPreview(input: unknown): ValidationResult<NormalizedChangeSetPreview> {
  if (!record(input) || !allowedKeys(input, ["id", "taskId", "summary", "confidence", "state", "operations"])) {
    return { ok: false, errors: ["整理结果不是受支持的变更集。"] };
  }
  const errors: string[] = [];
  if (!nonEmptyText(input.id, 200)) errors.push("变更集 ID 无效。");
  if (!nonEmptyText(input.taskId, 200)) errors.push("任务 ID 无效。");
  if (!nonEmptyText(input.summary, 1000)) errors.push("变更摘要无效。");
  if (!finiteNumber(input.confidence, 0, 1)) errors.push("置信度必须位于 0 到 1。 ");
  if (input.state !== "awaiting_confirmation") errors.push("只有等待确认的变更集可以预览。");
  if (!Array.isArray(input.operations) || input.operations.length > MAX_OPERATIONS) errors.push(`变更操作必须是数组且不超过 ${MAX_OPERATIONS} 条。`);
  const operations = Array.isArray(input.operations) ? input.operations : [];
  operations.forEach((entry, index) => {
    if (!record(entry) || !allowedKeys(entry, ["id", "operation", "selected"]) || (entry.id !== undefined && !nonEmptyText(entry.id, 200)) || typeof entry.selected !== "boolean" || !validateOperation(entry.operation)) {
      errors.push(`第 ${index + 1} 条变更操作无效。`);
    }
  });
  if (errors.length) return { ok: false, errors };
  const source = input as unknown as AiChangeSetPreview;
  const sourceOperations = input.operations as Array<AiChangeSetPreview["operations"][number] & { id?: string }>;
  return {
    ok: true,
    value: {
      ...source,
      operations: sourceOperations.map((entry, index) => ({
        ...entry,
        id: entry.id || `operation-${index}`,
        selected: false,
      })),
    },
  };
}

export function validateRecommendationExplanations(
  input: unknown,
  allowedIds: readonly string[],
): ValidationResult<ValidatedRecommendationExplanation[]> {
  if (!Array.isArray(input) || input.length > allowedIds.length) return { ok: false, errors: ["推荐解释结果结构无效。"] };
  const allowed = new Set(allowedIds);
  const seen = new Set<string>();
  const errors: string[] = [];
  const value: ValidatedRecommendationExplanation[] = [];
  input.forEach((entry, index) => {
    if (!record(entry) || !allowedKeys(entry, ["resourceId", "explanation"]) || !nonEmptyText(entry.resourceId, 200) || !allowed.has(entry.resourceId) || seen.has(entry.resourceId) || !nonEmptyText(entry.explanation, 1200)) {
      errors.push(`第 ${index + 1} 条推荐解释无效或引用了候选集外资源。`);
      return;
    }
    seen.add(entry.resourceId);
    value.push({ resourceId: entry.resourceId, explanation: entry.explanation });
  });
  return errors.length ? { ok: false, errors } : { ok: true, value };
}

export function serializeFilterDsl(dsl: NaturalLanguageFilterDsl): string {
  return JSON.stringify(dsl, null, 2);
}

export function operationTitle(operation: LibraryOperation): string {
  if (operation.type === "set_field") return `更新字段 · ${operation.field}`;
  if (operation.type === "add_tag") return `添加标签 · ${operation.tag}`;
  if (operation.type === "possible_duplicate") return `标记可能重复 · ${operation.gameIds.length} 项`;
  return "标记为需要人工检查";
}

export function operationTarget(operation: LibraryOperation): string {
  return operation.type === "possible_duplicate" ? operation.gameIds.join("、") : operation.gameId;
}



