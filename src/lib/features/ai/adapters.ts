import {
  AiGatewayError,
  type AiProviderConfig,
  type StructuredRequest,
  type StructuredResponse,
} from "./contracts";
import { validateEndpoint, type ValidatedEndpoint } from "./endpoint-policy";

export type CredentialRequirement = "none" | "bearer_secret";

/** Secret-free transport DTO; authorization is injected by the native transport. */
export interface AdapterHttpRequest {
  method: "POST";
  url: string;
  headers: Record<string, string>;
  body: unknown;
  credential: CredentialRequirement;
}

export interface AdapterHttpResponse {
  status: number;
  body: string;
}

const MAX_RESPONSE_BYTES = 1_048_576;

function completionUrl(base: string, suffix: "openai" | "ollama"): string {
  const url = new URL(base);
  const path = url.pathname.replace(/\/+$/, "");
  if (suffix === "openai") {
    url.pathname = path.endsWith("/chat/completions")
      ? path
      : path.endsWith("/v1")
        ? `${path}/chat/completions`
        : `${path}/v1/chat/completions`;
  } else {
    url.pathname = path.endsWith("/api/chat") ? path : `${path}/api/chat`;
  }
  return url.toString();
}

function classifyStatus(status: number): void {
  if (status >= 200 && status <= 299) return;
  if (status === 401 || status === 403) {
    throw new AiGatewayError("auth", "AI provider authentication failed");
  }
  if (status === 408 || status === 504) {
    throw new AiGatewayError("timeout", "AI provider request timed out", true);
  }
  if (status === 429) {
    throw new AiGatewayError("rate_limited", "AI provider rate limited the request", true);
  }
  if (status >= 500) {
    throw new AiGatewayError("provider_unavailable", "AI provider is temporarily unavailable", true);
  }
  throw new AiGatewayError(
    "provider_unavailable",
    `AI provider rejected the request with status ${status}`,
  );
}

function parseBody(body: string): Record<string, unknown> {
  if (new TextEncoder().encode(body).byteLength > MAX_RESPONSE_BYTES) {
    throw new AiGatewayError("invalid_output", "AI provider response exceeded the size limit");
  }
  try {
    const value: unknown = JSON.parse(body);
    if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error();
    return value as Record<string, unknown>;
  } catch {
    throw new AiGatewayError("invalid_output", "AI provider returned invalid JSON");
  }
}

function optionalCount(value: unknown): number | null {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : null;
}

export class OpenAiCompatibleAdapter {
  readonly endpoint: ValidatedEndpoint;

  constructor(readonly config: AiProviderConfig) {
    if (config.kind !== "openai_compatible") {
      throw new AiGatewayError("policy_rejected", "OpenAI-compatible adapter requires an openai_compatible provider config");
    }
    this.endpoint = validateEndpoint(config.baseUrl, config.kind);
  }

  buildRequest(request: StructuredRequest): AdapterHttpRequest {
    const body: Record<string, unknown> = {
      model: request.model,
      messages: request.messages,
      temperature: request.temperature,
      max_tokens: request.maxOutputTokens,
    };
    if (this.config.capabilities.jsonMode) body.response_format = { type: "json_object" };
    return {
      method: "POST",
      url: completionUrl(this.endpoint.url, "openai"),
      headers: { accept: "application/json", "content-type": "application/json" },
      body,
      credential: "bearer_secret",
    };
  }

  parseResponse(response: AdapterHttpResponse): StructuredResponse {
    classifyStatus(response.status);
    const value = parseBody(response.body);
    const choices = Array.isArray(value.choices) ? value.choices : [];
    const choice = choices[0] as Record<string, unknown> | undefined;
    const message = choice?.message as Record<string, unknown> | undefined;
    if (typeof message?.content !== "string" || !message.content.trim()) {
      throw new AiGatewayError("invalid_output", "AI provider response did not include text content");
    }
    const usage = value.usage as Record<string, unknown> | undefined;
    return {
      providerId: this.config.id,
      model: typeof value.model === "string" ? value.model : this.config.model,
      content: message.content,
      usage: {
        inputTokens: optionalCount(usage?.prompt_tokens),
        outputTokens: optionalCount(usage?.completion_tokens),
      },
      finishReason: typeof choice?.finish_reason === "string" ? choice.finish_reason : null,
    };
  }
}

export class OllamaAdapter {
  readonly endpoint: ValidatedEndpoint;

  constructor(readonly config: AiProviderConfig) {
    if (config.kind !== "ollama") {
      throw new AiGatewayError("policy_rejected", "Ollama adapter requires an ollama provider config");
    }
    this.endpoint = validateEndpoint(config.baseUrl, config.kind);
  }

  buildRequest(request: StructuredRequest): AdapterHttpRequest {
    return {
      method: "POST",
      url: completionUrl(this.endpoint.url, "ollama"),
      headers: { accept: "application/json", "content-type": "application/json" },
      body: {
        model: request.model,
        messages: request.messages,
        stream: false,
        format: "json",
        options: { temperature: request.temperature, num_predict: request.maxOutputTokens },
      },
      credential: "none",
    };
  }

  parseResponse(response: AdapterHttpResponse): StructuredResponse {
    classifyStatus(response.status);
    const value = parseBody(response.body);
    const message = value.message as Record<string, unknown> | undefined;
    if (typeof message?.content !== "string" || !message.content.trim()) {
      throw new AiGatewayError("invalid_output", "Ollama response did not include text content");
    }
    return {
      providerId: this.config.id,
      model: typeof value.model === "string" ? value.model : this.config.model,
      content: message.content,
      usage: {
        inputTokens: optionalCount(value.prompt_eval_count),
        outputTokens: optionalCount(value.eval_count),
      },
      finishReason: typeof value.done_reason === "string" ? value.done_reason : null,
    };
  }
}
