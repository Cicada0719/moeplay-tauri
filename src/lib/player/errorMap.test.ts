import { describe, expect, it } from "vitest";
import { buildErrorLog, classifyPlaybackError } from "./errorMap";

describe("classifyPlaybackError", () => {
  it("HTTP 401/403 → HTTP_FORBIDDEN", () => {
    const err = classifyPlaybackError("denied", 403);
    expect(err.kind).toBe("HTTP_FORBIDDEN");
    expect(err.message).toContain("拒绝访问");
    expect(err.httpStatus).toBe(403);
    expect(classifyPlaybackError("denied", 401).kind).toBe("HTTP_FORBIDDEN");
  });

  it("其他 4xx/5xx → HTTP_ERROR（含状态码文案）", () => {
    const err = classifyPlaybackError("boom", 503);
    expect(err.kind).toBe("HTTP_ERROR");
    expect(err.message).toContain("503");
    expect(err.httpStatus).toBe(503);
  });

  it("空地址 → PARSE_EMPTY", () => {
    expect(classifyPlaybackError("", undefined).kind).toBe("PARSE_EMPTY");
    expect(classifyPlaybackError(null, undefined).kind).toBe("PARSE_EMPTY");
    expect(classifyPlaybackError(undefined, undefined).kind).toBe("PARSE_EMPTY");
    expect(classifyPlaybackError({ kind: "PARSE_EMPTY" }, undefined).kind).toBe("PARSE_EMPTY");
  });

  it("MEDIA_ERR_NETWORK → NETWORK", () => {
    const err = classifyPlaybackError({ code: 2, message: "MEDIA_ERR_NETWORK" }, undefined);
    expect(err.kind).toBe("NETWORK");
    expect(err.message).toContain("网络连接失败");
  });

  it("MEDIA_ERR_DECODE / SRC_NOT_SUPPORTED → MEDIA_DECODE", () => {
    expect(classifyPlaybackError({ code: 3, message: "MEDIA_ERR_DECODE" }, undefined).kind).toBe("MEDIA_DECODE");
    expect(classifyPlaybackError({ code: 4, message: "MEDIA_ERR_SRC_NOT_SUPPORTED" }, undefined).kind).toBe("MEDIA_DECODE");
  });

  it("未知错误 → HTTP_ERROR 兜底", () => {
    const err = classifyPlaybackError(new Error("boom"), undefined);
    expect(err.kind).toBe("HTTP_ERROR");
  });

  it("非数字 httpStatus（如字符串 '403'）不被误判为 HTTP_FORBIDDEN/HTTP_ERROR", () => {
    const err = classifyPlaybackError("denied", "403" as unknown as number);
    expect(err.kind).toBe("HTTP_ERROR");
    expect(err.httpStatus).toBeUndefined();
  });
});

describe("buildErrorLog", () => {
  it("包含 detail 与 httpStatus", () => {
    const log = buildErrorLog({
      kind: "HTTP_FORBIDDEN",
      message: "该源拒绝访问",
      detail: "Request failed with status 403",
      httpStatus: 403,
      url: "https://example.com/play",
      occurredAt: 0,
    });
    expect(log).toContain("kind: HTTP_FORBIDDEN");
    expect(log).toContain("httpStatus: 403");
    expect(log).toContain("detail: Request failed with status 403");
    expect(log).toContain("url: https://example.com/play");
  });
});
