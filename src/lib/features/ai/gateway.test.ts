import { describe, expect, it } from "vitest";
import {
  AiCancellation,
  AiGatewayError,
  BudgetLedger,
  FixedWindowRateLimiter,
  OllamaAdapter,
  OpenAiCompatibleAdapter,
  PromptRegistry,
  authorizeEndpointBinding,
  authorizeProviderFallback,
  buildLibraryChangeSetPreview,
  redactHeaders,
  redactText,
  redactUrl,
  validateEndpoint,
  type AiCapabilities,
  type AiProviderConfig,
  type LibraryCleanupInput,
  type StructuredRequest,
} from "./index";

const capabilities = (local: boolean): AiCapabilities => ({
  structuredOutput: true,
  jsonMode: true,
  streaming: false,
  vision: false,
  local,
  maxContextTokens: 32768,
});

const openAiConfig = (baseUrl: string): AiProviderConfig => ({
  id: "openai-fixture",
  kind: "openai_compatible",
  displayName: "Fixture OpenAI",
  baseUrl,
  model: "fixture-model",
  secretConfigured: true,
  capabilities: capabilities(false),
  enabled: true,
});

const ollamaConfig = (baseUrl: string): AiProviderConfig => ({
  id: "ollama-fixture",
  kind: "ollama",
  displayName: "Fixture Ollama",
  baseUrl,
  model: "qwen2.5:7b",
  secretConfigured: false,
  capabilities: capabilities(true),
  enabled: true,
});

const request: StructuredRequest = {
  taskId: "task-fixture",
  promptId: "library_cleanup",
  promptVersion: "1.0.0",
  schemaId: "library_cleanup.change_set",
  model: "fixture-model",
  messages: [
    { role: "system", content: "Return JSON only." },
    { role: "user", content: "Fixture request" },
  ],
  temperature: 0.1,
  maxOutputTokens: 800,
};

const libraryInput: LibraryCleanupInput = {
  games: [
    { id: "game-1", title: " fixture game ", tags: ["relaxing"], metadata: {} },
    { id: "game-2", title: "Fixture Game", tags: [], metadata: {} },
  ],
};

const validLibraryContent = JSON.stringify({
  summary: "Normalize one title and flag a duplicate candidate.",
  confidence: 0.91,
  operations: [
    { type: "set_field", gameId: "game-1", field: "title", value: "Fixture Game", reason: "Normalize title." },
    { type: "possible_duplicate", gameIds: ["game-1", "game-2"], reason: "Metadata overlaps." },
  ],
});

const validFilterContent = `\`\`\`json
${JSON.stringify({
  kind: "game",
  filters: [
    { field: "lastPlayedAt", op: "is_null" },
    { field: "contentRating", op: "eq", value: "all_ages" },
    { field: "estimatedHours", op: "lte", value: 10 },
  ],
  sort: [{ field: "userAffinity", direction: "desc" }],
  explanation: "Unplayed all-ages games under ten hours.",
})}
\`\`\``;

describe("AI gateway foundation", () => {
  it("enforces endpoint and origin-binding policy", () => {
    expect(validateEndpoint("https://api.example.test/v1", "openai_compatible").scope).toBe("remote");
    expect(() => validateEndpoint("http://api.example.test/v1", "openai_compatible")).toThrow(AiGatewayError);
    expect(() => validateEndpoint("https://user:pass@api.example.test/v1", "openai_compatible")).toThrow();
    expect(validateEndpoint("http://127.0.0.1:11434", "ollama").scope).toBe("loopback");
    expect(() => validateEndpoint("http://127.0.0.1:11434", "openai_compatible")).toThrow();

    const endpoint = validateEndpoint("https://api.example.test/v1", "openai_compatible");
    authorizeEndpointBinding({ providerId: "openai-fixture", boundOrigin: endpoint.origin }, endpoint);
    expect(() => authorizeEndpointBinding(
      { providerId: "openai-fixture", boundOrigin: endpoint.origin },
      validateEndpoint("https://other.example.test/v1", "openai_compatible"),
    )).toThrow();
  });

  it("builds secret-free OpenAI-compatible and Ollama transport DTOs", () => {
    expect(() => new OpenAiCompatibleAdapter(ollamaConfig("http://127.0.0.1:11434"))).toThrow();
    const openAi = new OpenAiCompatibleAdapter(openAiConfig("https://api.example.test/v1"));
    const openAiRequest = openAi.buildRequest(request);
    expect(openAiRequest.url).toBe("https://api.example.test/v1/chat/completions");
    expect(openAiRequest.credential).toBe("bearer_secret");
    expect(JSON.stringify(openAiRequest).toLowerCase()).not.toContain("authorization");

    expect(() => new OllamaAdapter(openAiConfig("https://api.example.test/v1"))).toThrow();
    const ollama = new OllamaAdapter(ollamaConfig("http://[::1]:11434"));
    const ollamaRequest = ollama.buildRequest(request);
    expect(ollamaRequest.url).toBe("http://[::1]:11434/api/chat");
    expect(ollamaRequest.credential).toBe("none");
  });

  it("requires valid schema/business output before creating an unselected preview", () => {
    const registry = new PromptRegistry();
    const validated = registry.validateLibraryCleanupResponse("1.0.0", validLibraryContent, libraryInput);
    const preview = buildLibraryChangeSetPreview("change-1", "task-fixture", validated);
    expect(preview.state).toBe("awaiting_confirmation");
    expect(preview.operations).toHaveLength(2);
    expect(preview.operations.every((operation) => !operation.selected)).toBe(true);

    expect(() => registry.validateLibraryCleanupResponse("1.0.0", "not json", libraryInput)).toThrow();
    expect(() => registry.validateLibraryCleanupResponse("1.0.0", JSON.stringify({
      summary: "Unsafe",
      confidence: 1,
      operations: [{ type: "needs_review", gameId: "hallucinated", reason: "No" }],
    }), libraryInput)).toThrow(/outside the supplied context/);
  });

  it("compiles only whitelist DSL and rejects SQL-like fields", () => {
    const registry = new PromptRegistry();
    const validated = registry.validateNaturalLanguageFilterResponse("1.0.0", validFilterContent, "game");
    expect(validated.value.filters).toHaveLength(3);
    expect(() => registry.validateNaturalLanguageFilterResponse("1.0.0", JSON.stringify({
      kind: "game",
      filters: [{ field: "rawSql", op: "execute", value: "DELETE FROM games" }],
      sort: [],
      explanation: "Unsafe",
    }), "game")).toThrow();
  });

  it("provides deterministic budget, rate-limit, cancellation and fallback gates", () => {
    const budget = new BudgetLedger({
      monthlyHardLimitTokens: 1000,
      softWarningTokens: 800,
      perTaskLimitTokens: 600,
    }, 300);
    const reservation = budget.reserve(500);
    expect(budget.snapshot().softWarningReached).toBe(true);
    reservation.commit(450);
    expect(budget.snapshot().committedTokens).toBe(750);
    expect(() => budget.reserve(300)).toThrow();

    const limiter = new FixedWindowRateLimiter({ maxRequests: 2, windowMs: 1000 });
    limiter.check(10_000);
    limiter.check(10_001);
    try {
      limiter.check(10_100);
      throw new Error("expected rate limit");
    } catch (error) {
      expect(error).toBeInstanceOf(AiGatewayError);
      expect((error as AiGatewayError).retryAfterMs).toBe(900);
    }
    limiter.check(11_000);

    const cancellation = new AiCancellation();
    const guard = cancellation.guard();
    guard.ensureActive();
    cancellation.cancel();
    expect(guard.signal.aborted).toBe(true);
    expect(() => guard.ensureActive()).toThrow();

    expect(() => authorizeProviderFallback(true, false, "disabled")).toThrow();
    expect(() => authorizeProviderFallback(true, false, "same_scope_only")).toThrow();
    authorizeProviderFallback(true, false, "explicit_cross_scope");
  });

  it("redacts credentials and explicit sentinels", () => {
    const sentinel = "sentinel-secret-123";
    const redacted = redactHeaders({ Authorization: `Bearer ${sentinel}`, "x-api-key": sentinel });
    expect(JSON.stringify(redacted)).not.toContain(sentinel);
    expect(redactUrl("https://user:pass@example.test/v1?token=abc")).toBe("https://example.test/v1");
    expect(redactText(`failure ${sentinel}`, [sentinel])).not.toContain(sentinel);
    expect(redactText("request failed Bearer bearer-token-sentinel")).not.toContain("bearer-token-sentinel");
  });
});
