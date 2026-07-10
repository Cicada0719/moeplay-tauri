import { invokeCmd } from "../../api/core";
import type { AppTask, Game, Settings } from "../../api/types";
import type {
  AiCapabilities,
  LibraryChangeSetOutput,
  LibraryOperation,
  ResourceFilterKind,
} from "./contracts";
import type {
  AiRecommendationRequest,
  AiRecommendationResult,
  AiStatusSnapshot,
  AiTaskRecord,
  ApplyChangeSetRequest,
  ApplyChangeSetResult,
  CompileFilterRequest,
  CompileFilterResult,
  LibraryCleanupPreviewRequest,
  UndoChangeSetResult,
} from "./types";
import type { AiChangeSetPreview } from "./change-set";

export interface AiClient {
  getStatus(signal?: AbortSignal): Promise<AiStatusSnapshot>;
  listTasks(limit?: number, signal?: AbortSignal): Promise<AiTaskRecord[]>;
  compileFilter(request: CompileFilterRequest, signal?: AbortSignal): Promise<CompileFilterResult>;
  recommend(request: AiRecommendationRequest, signal?: AbortSignal): Promise<AiRecommendationResult>;
  previewLibraryCleanup(request: LibraryCleanupPreviewRequest, signal?: AbortSignal): Promise<AiChangeSetPreview>;
  cancelTask(taskId: string): Promise<void>;
  applyChangeSet(request: ApplyChangeSetRequest): Promise<ApplyChangeSetResult>;
  undoChangeSet(undoToken: string): Promise<UndoChangeSetResult>;
}

type BackendProviderKind = "open_ai_compatible" | "ollama" | "mock";
type BackendTaskStatus = "queued" | "running" | "paused" | "succeeded" | "failed" | "cancelled";

interface BackendProviderSpec {
  id: string;
  kind: BackendProviderKind;
  displayName: string;
  baseUrl: string;
  model: string;
  enabled: boolean;
  maxContextTokens?: number | null;
}

interface BackendProviderStatus {
  id: string;
  kind: BackendProviderKind;
  displayName: string;
  model: string;
  enabled: boolean;
  credentialConfigured: boolean;
  ready: boolean;
  issue?: string | null;
  capabilities: AiCapabilities;
}

interface BackendBudgetSnapshot {
  committedTokens: number;
  reservedTokens: number;
  softWarningReached: boolean;
}

interface BackendTaskStatusDto {
  id: string;
  kind: string;
  status: BackendTaskStatus;
  progress: number;
  createdAt: string;
  updatedAt: string;
  message?: string | null;
  resultAvailable: boolean;
}

interface BackendExecutionResult {
  taskId: string;
  providerId: string;
  model: string;
  promptId: string;
  promptVersion: string;
  schemaId: string;
  estimatedChargedTokens: number;
  result:
    | { type: "library_cleanup"; changeSet: LibraryChangeSetOutput }
    | { type: "natural_language_filter"; filter: unknown }
    | { type: "recommendation"; recommendation: { recommendations: Array<{ resourceId: string; reason: string }> } };
}

interface BackendTaskResultDto {
  task: BackendTaskStatusDto;
  result?: BackendExecutionResult | null;
  error?: { kind: string; message: string; retryable: boolean } | null;
}

interface StoredChangeSet {
  changeSet: AiChangeSetPreview;
  provenance: {
    providerId: string;
    model: string;
    promptId: string;
    promptVersion: string;
  };
}

interface BackendApplyResult {
  status: "applied" | "no_changes";
  changeSetId: string;
  selectedOperationCount: number;
  changedFieldCount: number;
  undoId?: string | null;
}

interface BackendUndoResult {
  status: "undone" | "already_undone";
  undoId: string;
  changeSetId: string;
}

const POLL_INTERVAL_MS = 150;
const TASK_TIMEOUT_MS = 180_000;

function abortError(): DOMException {
  return new DOMException("Aborted", "AbortError");
}

function assertNotAborted(signal?: AbortSignal): void {
  if (signal?.aborted) throw abortError();
}

function delay(ms: number, signal?: AbortSignal): Promise<void> {
  assertNotAborted(signal);
  return new Promise((resolve, reject) => {
    const timer = setTimeout(resolve, ms);
    const onAbort = () => {
      clearTimeout(timer);
      reject(abortError());
    };
    signal?.addEventListener("abort", onAbort, { once: true });
    if (signal) {
      setTimeout(() => signal.removeEventListener("abort", onAbort), ms + 1);
    }
  });
}

function providerFromSettings(settings: Settings): BackendProviderSpec {
  const baseUrl = settings.ai_api_url.trim();
  const isOllama = /(?:localhost|127\.0\.0\.1|\[::1\]):11434(?:\/|$)/i.test(baseUrl) || /\/api\/chat\/?$/i.test(baseUrl);
  return {
    id: isOllama ? "settings-ollama" : "settings-openai-compatible",
    kind: isOllama ? "ollama" : "open_ai_compatible",
    displayName: isOllama ? "Ollama（设置）" : "OpenAI Compatible（设置）",
    baseUrl,
    model: settings.ai_model.trim(),
    enabled: settings.ai_enabled,
  };
}

async function loadProvider(): Promise<BackendProviderSpec> {
  const settings = await invokeCmd<Settings>("get_settings");
  return providerFromSettings(settings);
}

function startRequest(provider: BackendProviderSpec, task: Record<string, unknown>): Record<string, unknown> {
  return {
    providers: [provider],
    primaryProviderId: provider.id,
    fallbackProviderId: null,
    fallbackAuthorization: "disabled",
    task,
  };
}

async function waitForTask(taskId: string, signal?: AbortSignal): Promise<BackendExecutionResult> {
  const started = Date.now();
  let cancellationSent = false;
  const cancelOnAbort = () => {
    if (cancellationSent) return;
    cancellationSent = true;
    void invokeCmd("ai_v2_cancel_task", { taskId }).catch(() => undefined);
  };
  signal?.addEventListener("abort", cancelOnAbort, { once: true });
  try {
    while (Date.now() - started < TASK_TIMEOUT_MS) {
      assertNotAborted(signal);
      const status = await invokeCmd<BackendTaskStatusDto>("ai_v2_task_status", { taskId });
      if (status.status === "succeeded" || status.status === "failed" || status.status === "cancelled") {
        const outcome = await invokeCmd<BackendTaskResultDto>("ai_v2_task_result", { taskId });
        if (outcome.result) return outcome.result;
        if (outcome.error) throw new Error(`${outcome.error.kind}: ${outcome.error.message}`);
        throw new Error(`AI task ended without a retained result (${status.status})`);
      }
      await delay(POLL_INTERVAL_MS, signal);
    }
    await invokeCmd("ai_v2_cancel_task", { taskId }).catch(() => undefined);
    throw new Error("AI task timed out");
  } finally {
    signal?.removeEventListener("abort", cancelOnAbort);
  }
}

async function runStructuredTask(
  task: Record<string, unknown>,
  signal?: AbortSignal,
): Promise<BackendExecutionResult> {
  assertNotAborted(signal);
  const provider = await loadProvider();
  if (!provider.enabled) throw new Error("AI is disabled");
  if (!provider.baseUrl || !provider.model) throw new Error("AI provider is not configured");
  const started = await invokeCmd<BackendTaskStatusDto>("ai_v2_start_structured_task", {
    request: startRequest(provider, task),
  });
  return waitForTask(started.id, signal);
}

function safeGameMetadata(game: Game): Record<string, unknown> {
  return {
    developer: game.developer ?? game.metadata?.developer ?? null,
    publisher: game.publisher ?? game.metadata?.publisher ?? null,
    releaseYear: game.release_year ?? game.metadata?.release_year ?? null,
    rating: game.rating ?? game.metadata?.vndb_rating ?? game.metadata?.bangumi_rating ?? null,
    favorite: game.favorite,
    hidden: game.hidden,
    playTimeSeconds: game.play_time_seconds,
    lastPlayed: game.last_played ?? null,
  };
}

function gameContext(game: Game) {
  return {
    id: game.id,
    title: game.name,
    description: game.description ?? null,
    tags: game.tags ?? [],
    metadata: safeGameMetadata(game),
  };
}

function recommendationSignals(game: Game): string[] {
  const signals: string[] = [];
  if (game.favorite) signals.push("favorite");
  if (!game.last_played) signals.push("unplayed");
  if (game.play_time_seconds > 0) signals.push("played_before");
  for (const tag of (game.tags ?? []).slice(0, 8)) signals.push(`tag:${tag}`);
  return signals;
}

function mapTaskStatus(status: string): AiTaskRecord["status"] {
  if (status === "succeeded" || status === "completed") return "succeeded";
  if (status === "pending") return "queued";
  if (status === "cancelled") return "cancelled";
  if (status === "failed") return "failed";
  return status === "running" ? "running" : "queued";
}

function useCaseFromKind(kind: string): string {
  if (kind.endsWith("library_cleanup")) return "library_cleanup";
  if (kind.endsWith("natural_language_filter")) return "natural_language_filter";
  if (kind.endsWith("recommendation")) return "recommendation";
  return kind;
}

function schemaFromKind(kind: string): string {
  if (kind.endsWith("library_cleanup")) return "library_change_set.v1";
  if (kind.endsWith("natural_language_filter")) return "natural_language_filter.dsl";
  if (kind.endsWith("recommendation")) return "recommendation.explanations";
  return "unknown";
}

function createPreview(execution: BackendExecutionResult, changeSet: LibraryChangeSetOutput): AiChangeSetPreview {
  return {
    id: `${execution.taskId}-changes`,
    taskId: execution.taskId,
    summary: changeSet.summary,
    confidence: changeSet.confidence,
    state: "awaiting_confirmation",
    operations: changeSet.operations.map((operation) => ({ operation, selected: false })),
  };
}

function encodeUndoToken(undoId: string, changeSetId: string): string {
  return JSON.stringify({ undoId, changeSetId });
}

function decodeUndoToken(token: string): { undoId: string; changeSetId: string } {
  const decoded: unknown = JSON.parse(token);
  if (!decoded || typeof decoded !== "object") throw new Error("invalid undo token");
  const record = decoded as Record<string, unknown>;
  if (typeof record.undoId !== "string" || typeof record.changeSetId !== "string") {
    throw new Error("invalid undo token");
  }
  return { undoId: record.undoId, changeSetId: record.changeSetId };
}

export function createTauriAiClient(): AiClient {
  const retainedChangeSets = new Map<string, StoredChangeSet>();
  return {
    async getStatus(signal) {
      assertNotAborted(signal);
      const provider = await loadProvider();
      const [tasks, budget] = await Promise.all([
        invokeCmd<AppTask[]>("get_tasks"),
        invokeCmd<BackendBudgetSnapshot>("ai_v2_budget_status"),
      ]);
      assertNotAborted(signal);

      let mappedProvider: AiStatusSnapshot["providers"][number];
      if (!provider.enabled) {
        mappedProvider = {
          id: provider.id,
          displayName: provider.displayName,
          kind: provider.kind,
          model: provider.model,
          enabled: false,
          secretConfigured: false,
          health: "disabled",
          capabilities: {
            structuredOutput: true,
            jsonMode: true,
            streaming: false,
            vision: false,
            local: provider.kind === "ollama",
            maxContextTokens: null,
          },
        };
      } else {
        const status = await invokeCmd<BackendProviderStatus>("ai_v2_provider_status", { provider });
        mappedProvider = {
          id: status.id,
          displayName: status.displayName,
          kind: status.kind,
          model: status.model,
          enabled: status.enabled,
          secretConfigured: status.credentialConfigured,
          health: status.ready ? "healthy" : status.issue === "not_configured" ? "disabled" : "degraded",
          errorKind: status.issue ?? undefined,
          capabilities: status.capabilities,
        };
      }
      const activeTaskCount = tasks.filter((task) => task.kind.startsWith("ai_v2.") && ["queued", "pending", "running"].includes(task.status)).length;
      return {
        availability: !provider.enabled ? "disabled" : mappedProvider.health === "healthy" ? "ready" : "degraded",
        providers: [mappedProvider],
        activeTaskCount,
        dailyTokenEstimate: budget.committedTokens + budget.reservedTokens,
        dailyBudgetEstimate: 1_000_000,
        updatedAt: new Date().toISOString(),
      };
    },

    async listTasks(limit = 20, signal) {
      assertNotAborted(signal);
      const [tasks, provider] = await Promise.all([
        invokeCmd<AppTask[]>("get_tasks"),
        loadProvider(),
      ]);
      assertNotAborted(signal);
      return tasks
        .filter((task) => task.kind.startsWith("ai_v2."))
        .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
        .slice(0, Math.max(0, limit))
        .map((task) => ({
          id: task.id,
          useCase: useCaseFromKind(task.kind),
          providerId: provider.id,
          model: provider.model,
          promptVersion: "1.0.0",
          status: mapTaskStatus(task.status),
          createdAt: task.created_at,
          completedAt: ["succeeded", "completed", "failed", "cancelled"].includes(task.status) ? task.updated_at : null,
          durationMs: null,
          inputSummary: {},
          outputSchema: schemaFromKind(task.kind),
          tokenEstimate: null,
          errorKind: task.status === "failed" ? "provider_unavailable" : null,
        }));
    },

    async compileFilter(request, signal) {
      const execution = await runStructuredTask({
        type: "natural_language_filter",
        query: request.query,
        kind: request.kind,
      }, signal);
      if (execution.result.type !== "natural_language_filter") throw new Error("AI returned the wrong result type");
      return {
        taskId: execution.taskId,
        generation: request.generation,
        schemaId: execution.schemaId,
        schemaVersion: execution.promptVersion,
        dsl: execution.result.filter,
      };
    },

    async recommend(request, signal) {
      const games = await invokeCmd<Game[]>("get_games");
      assertNotAborted(signal);
      const byId = new Map(games.map((game) => [game.id, game]));
      const candidates = request.candidateIds.flatMap((id) => {
        const game = byId.get(id);
        if (!game) return [];
        return [{
          id: game.id,
          title: game.name,
          kind: request.kind,
          available: true,
          estimatedMinutes: game.play_time_seconds > 0 ? Math.ceil(game.play_time_seconds / 60) : null,
          signals: recommendationSignals(game),
        }];
      });
      const execution = await runStructuredTask({
        type: "recommendation",
        input: {
          candidates,
          excludedIds: [],
          limit: request.limit,
          request: null,
        },
      }, signal);
      if (execution.result.type !== "recommendation") throw new Error("AI returned the wrong result type");
      return {
        taskId: execution.taskId,
        generation: request.generation,
        explanations: execution.result.recommendation.recommendations.map((item) => ({
          resourceId: item.resourceId,
          explanation: item.reason,
        })),
      };
    },

    async previewLibraryCleanup(request, signal) {
      const games = (await invokeCmd<Game[]>("get_games")).slice(0, Math.max(0, request.limit));
      assertNotAborted(signal);
      const execution = await runStructuredTask({
        type: "library_cleanup",
        input: { games: games.map(gameContext) },
      }, signal);
      if (execution.result.type !== "library_cleanup") throw new Error("AI returned the wrong result type");
      const preview = createPreview(execution, execution.result.changeSet);
      await invokeCmd("ai_changes_preview", { request: { changeSet: preview } });
      retainedChangeSets.set(preview.id, {
        changeSet: preview,
        provenance: {
          providerId: execution.providerId,
          model: execution.model,
          promptId: execution.promptId,
          promptVersion: execution.promptVersion,
        },
      });
      return preview;
    },

    async cancelTask(taskId) {
      await invokeCmd("ai_v2_cancel_task", { taskId });
    },

    async applyChangeSet(request) {
      if (!request.confirmed) throw new Error("confirmation_required");
      const retained = retainedChangeSets.get(request.changeSetId);
      if (!retained) throw new Error("change set is not retained or was not validated in this session");
      const result = await invokeCmd<BackendApplyResult>("ai_changes_apply", {
        request: {
          changeSet: retained.changeSet,
          selectedOperationIndices: request.selectedOperationIndexes,
          provenance: retained.provenance,
        },
      });
      return {
        changeSetId: result.changeSetId,
        state: "applied",
        appliedOperationCount: result.changedFieldCount,
        undoToken: result.undoId ? encodeUndoToken(result.undoId, result.changeSetId) : "",
      };
    },

    async undoChangeSet(undoToken) {
      const request = decodeUndoToken(undoToken);
      const result = await invokeCmd<BackendUndoResult>("ai_changes_undo", { request });
      retainedChangeSets.delete(result.changeSetId);
      return { changeSetId: result.changeSetId, state: "reverted" };
    },
  };
}

let defaultClient: AiClient | null = null;
export function getAiClient(): AiClient {
  defaultClient ??= createTauriAiClient();
  return defaultClient;
}

export function isAiUnavailableError(error: unknown): boolean {
  const message = String(error).toLowerCase();
  return message.includes("not found") || message.includes("unknown command") || message.includes("offline") || message.includes("disabled") || message.includes("not configured") || message.includes("network") || message.includes("connection") || message.includes("failed to fetch");
}

