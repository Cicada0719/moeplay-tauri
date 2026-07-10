import type { AiClient } from "./client";
import type {
  AiRecommendationResult,
  AiStatusSnapshot,
  AiTaskRecord,
  ApplyChangeSetResult,
  CompileFilterResult,
  UndoChangeSetResult,
} from "./types";
import type { AiChangeSetPreview } from "./change-set";

export interface MockAiClientOptions {
  status?: AiStatusSnapshot;
  tasks?: AiTaskRecord[];
  compileResult?: CompileFilterResult | ((request: Parameters<AiClient["compileFilter"]>[0]) => CompileFilterResult | Promise<CompileFilterResult>);
  recommendationResult?: AiRecommendationResult;
  cleanupPreview?: AiChangeSetPreview;
  latencyMs?: number;
  error?: unknown;
}

export interface MockAiClient extends AiClient {
  calls: {
    getStatus: number;
    listTasks: number;
    compileFilter: Parameters<AiClient["compileFilter"]>[0][];
    recommend: Parameters<AiClient["recommend"]>[0][];
    previewLibraryCleanup: Parameters<AiClient["previewLibraryCleanup"]>[0][];
    cancelTask: string[];
    applyChangeSet: Parameters<AiClient["applyChangeSet"]>[0][];
    undoChangeSet: string[];
  };
}

const defaultStatus: AiStatusSnapshot = {
  availability: "ready",
  activeTaskCount: 0,
  updatedAt: "2026-07-10T00:00:00.000Z",
  providers: [{
    id: "mock-local",
    displayName: "本地 Mock Provider",
    kind: "mock",
    model: "contract-fixture",
    enabled: true,
    secretConfigured: false,
    health: "healthy",
    capabilities: { structuredOutput: true, jsonMode: true, streaming: false, vision: false, local: true, maxContextTokens: 8192 },
  }],
};

function abortableDelay(ms: number, signal?: AbortSignal): Promise<void> {
  if (!ms) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const timer = setTimeout(resolve, ms);
    signal?.addEventListener("abort", () => {
      clearTimeout(timer);
      reject(new DOMException("Aborted", "AbortError"));
    }, { once: true });
  });
}

export function createMockAiClient(options: MockAiClientOptions = {}): MockAiClient {
  const calls: MockAiClient["calls"] = {
    getStatus: 0,
    listTasks: 0,
    compileFilter: [],
    recommend: [],
    previewLibraryCleanup: [],
    cancelTask: [],
    applyChangeSet: [],
    undoChangeSet: [],
  };
  const run = async <T>(value: T, signal?: AbortSignal) => {
    await abortableDelay(options.latencyMs ?? 0, signal);
    if (options.error) throw options.error;
    return value;
  };

  return {
    calls,
    async getStatus(signal) {
      calls.getStatus += 1;
      return run(options.status ?? defaultStatus, signal);
    },
    async listTasks(limit = 20, signal) {
      calls.listTasks += 1;
      return run((options.tasks ?? []).slice(0, limit), signal);
    },
    async compileFilter(request, signal) {
      calls.compileFilter.push(request);
      const configured = options.compileResult;
      const result = typeof configured === "function" ? await configured(request) : configured ?? {
        taskId: "mock-filter-task",
        generation: request.generation,
        schemaId: "natural_language_filter.dsl",
        schemaVersion: "1.0.0",
        dsl: { kind: request.kind, filters: [], sort: [], explanation: "Mock validated filter" },
      };
      return run(result, signal);
    },
    async recommend(request, signal) {
      calls.recommend.push(request);
      return run(options.recommendationResult ?? { taskId: "mock-recommend-task", generation: request.generation, explanations: [] }, signal);
    },
    async previewLibraryCleanup(request, signal) {
      calls.previewLibraryCleanup.push(request);
      return run(options.cleanupPreview ?? {
        id: "mock-change-set",
        taskId: "mock-cleanup-task",
        summary: "没有发现需要自动修改的项目。",
        confidence: 1,
        state: "awaiting_confirmation",
        operations: [],
      }, signal);
    },
    async cancelTask(taskId) {
      calls.cancelTask.push(taskId);
    },
    async applyChangeSet(request): Promise<ApplyChangeSetResult> {
      calls.applyChangeSet.push(request);
      if (!request.confirmed || request.selectedOperationIndexes.length === 0) throw new Error("confirmation_required");
      return { changeSetId: request.changeSetId, state: "applied", appliedOperationCount: request.selectedOperationIndexes.length, undoToken: "mock-undo-token" };
    },
    async undoChangeSet(undoToken): Promise<UndoChangeSetResult> {
      calls.undoChangeSet.push(undoToken);
      return { changeSetId: "mock-change-set", state: "reverted" };
    },
  };
}

