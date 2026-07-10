import { afterEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "./core";
import { secretDelete, secretSet, secretStatus } from "./secrets";

describe("SecretStore API contract", () => {
  afterEach(() => clearMockInvokeHandler());

  it("returns status only and never echoes a sentinel secret", async () => {
    const sentinel = "SENTINEL_FRONTEND_SECRET_MUST_NOT_RETURN";
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    setMockInvokeHandler((command, args) => {
      calls.push({ command, args });
      return { kind: "ai_api_key", configured: command !== "secret_delete" };
    });

    const stored = await secretSet("ai_api_key", sentinel, "https://api.example.com/v1/chat");
    const status = await secretStatus("ai_api_key", "https://api.example.com/v1/chat");
    const deleted = await secretDelete("ai_api_key", "https://api.example.com/v1/chat");

    expect(stored).toEqual({ kind: "ai_api_key", configured: true });
    expect(status).toEqual({ kind: "ai_api_key", configured: true });
    expect(deleted).toEqual({ kind: "ai_api_key", configured: false });
    expect(JSON.stringify([stored, status, deleted])).not.toContain(sentinel);
    expect(calls[0]).toEqual({
      command: "secret_set",
      args: {
        kind: "ai_api_key",
        origin: "https://api.example.com/v1/chat",
        secret: sentinel,
      },
    });
  });
});
