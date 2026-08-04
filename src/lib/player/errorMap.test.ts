import { describe, expect, it } from "vitest";
import { buildErrorLog, classifyPlaybackError, formatFailureRecord, mergeFailureContext } from "./errorMap";

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

describe("formatFailureRecord", () => {
  it("渲染失败链路行：attempt / why / raw / httpStatus", () => {
    expect(
      formatFailureRecord({ why: "hls network", raw: "Request failed with status 403", httpStatus: 403, attempt: 1 }),
    ).toBe("[尝试 1] hls network: Request failed with status 403 (HTTP 403)");
    expect(
      formatFailureRecord({ why: "video error", raw: { code: 2, message: "MEDIA_ERR_NETWORK" }, attempt: 2 }),
    ).toBe("[尝试 2] video error: MEDIA_ERR_NETWORK");
    // raw 缺省时省略冒号后的详情
    expect(formatFailureRecord({ why: "timeout", attempt: 1 })).toBe("[尝试 1] timeout");
  });
});

describe("mergeFailureContext", () => {
  it("单次失败保持 classifyPlaybackError 原始 detail（不包装成多行）", () => {
    const final = classifyPlaybackError(new Error("boom"), 403);
    const merged = mergeFailureContext(final, [{ why: "hls network", raw: new Error("boom"), httpStatus: 403, attempt: 1 }]);
    expect(merged).toEqual(final);
  });

  it("多次失败合并完整链路：kind/httpStatus 取最终失败，detail 携带全部中间失败", () => {
    const final = classifyPlaybackError({ code: 2, message: "MEDIA_ERR_NETWORK" }, undefined);
    const merged = mergeFailureContext(final, [
      { why: "hls network", raw: "Request failed with status 403", httpStatus: 403, attempt: 1 },
      { why: "video error", raw: { code: 2, message: "MEDIA_ERR_NETWORK" }, attempt: 2 },
    ]);
    expect(merged.kind).toBe("NETWORK");
    expect(merged.httpStatus).toBeUndefined();
    expect(merged.detail).toContain("[尝试 1] hls network: Request failed with status 403 (HTTP 403)");
    expect(merged.detail).toContain("[尝试 2] video error: MEDIA_ERR_NETWORK");
  });

  it("合并结果可被 buildErrorLog 携带（复制日志含完整链路）", () => {
    const final = classifyPlaybackError("denied", 403);
    const merged = mergeFailureContext(final, [
      { why: "hls network", raw: "fetch aborted", attempt: 1 },
      { why: "hls network", raw: "Request failed with status 403", httpStatus: 403, attempt: 2 },
    ]);
    const log = buildErrorLog(merged);
    expect(log).toContain("kind: HTTP_FORBIDDEN");
    expect(log).toContain("httpStatus: 403");
    expect(log).toContain("detail: [尝试 1] hls network: fetch aborted");
    expect(log).toContain("[尝试 2] hls network: Request failed with status 403 (HTTP 403)");
  });
});
