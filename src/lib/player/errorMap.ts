import type { PlayerError, PlayerErrorKind } from "../stores/player";

/**
 * 播放错误分类与用户提示文案映射（spec §3.6）。
 *
 * `classifyPlaybackError` 把来自 video 元素 / fetch / 解析链路的原始错误
 * 归约为 5 类 `PlayerErrorKind`，供 ErrorOverlay 展示结构化错误与操作入口。
 */

const MESSAGES: Record<PlayerErrorKind, string> = {
  NETWORK: "网络连接失败，请检查网络后重试",
  PARSE_EMPTY: "该源解析失败，建议切换源",
  HTTP_FORBIDDEN: "该源拒绝访问（防盗链），建议切换源",
  HTTP_ERROR: "播放地址返回错误",
  MEDIA_DECODE: "视频解码失败，建议切换源或画质",
};

export function messageForErrorKind(kind: PlayerErrorKind): string {
  return MESSAGES[kind];
}

export function httpErrorMessage(status?: number): string {
  return status === undefined ? MESSAGES.HTTP_ERROR : `播放地址返回错误（${status}）`;
}

interface MediaErrorLike {
  code?: number;
  message?: string;
}

function describeRaw(raw: unknown): string {
  if (raw instanceof Error) return raw.message;
  if (typeof raw === "string") return raw;
  if (raw && typeof raw === "object") {
    const message = (raw as MediaErrorLike).message;
    return message || JSON.stringify(raw);
  }
  return String(raw ?? "");
}

export function classifyPlaybackError(raw: unknown, httpStatus?: number): PlayerError {
  const detail = describeRaw(raw);
  const occurredAt = Date.now();
  // 严格类型校验：仅接受 number 状态码；字符串 '403' 等非数字按未知处理，避免误判为 HTTP_FORBIDDEN/HTTP_ERROR
  const status = typeof httpStatus === "number" && Number.isFinite(httpStatus) ? httpStatus : undefined;

  // HTTP 401/403 → 防盗链拒绝访问
  if (status === 401 || status === 403) {
    return { kind: "HTTP_FORBIDDEN", message: MESSAGES.HTTP_FORBIDDEN, detail, httpStatus: status, occurredAt };
  }
  // 其他 4xx/5xx
  if (typeof status === "number" && status >= 400 && status < 600) {
    return { kind: "HTTP_ERROR", message: httpErrorMessage(status), detail, httpStatus: status, occurredAt };
  }

  // 解析结果为空地址
  const isEmpty =
    raw == null ||
    (typeof raw === "string" && raw.trim() === "") ||
    (typeof raw === "object" && (raw as { kind?: string }).kind === "PARSE_EMPTY");
  if (isEmpty) {
    return { kind: "PARSE_EMPTY", message: MESSAGES.PARSE_EMPTY, detail, occurredAt };
  }

  const code = (raw as MediaErrorLike).code;
  const lower = detail.toLowerCase();

  // MEDIA_ERR_NETWORK(2) / fetch 超时
  if (
    code === 2 ||
    lower.includes("media_err_network") ||
    lower.includes("network") ||
    lower.includes("timeout") ||
    lower.includes("stalled") ||
    lower.includes("fetch")
  ) {
    return { kind: "NETWORK", message: MESSAGES.NETWORK, detail, occurredAt };
  }

  // MEDIA_ERR_DECODE(3) / MEDIA_ERR_SRC_NOT_SUPPORTED(4)
  if (
    code === 3 ||
    code === 4 ||
    lower.includes("media_err_decode") ||
    lower.includes("src_not_supported") ||
    lower.includes("decode")
  ) {
    return { kind: "MEDIA_DECODE", message: MESSAGES.MEDIA_DECODE, detail, occurredAt };
  }

  // 未知错误兜底
  return { kind: "HTTP_ERROR", message: httpErrorMessage(status), detail, httpStatus: status, occurredAt };
}

/** 复制日志：error.detail + 环境信息，供「复制日志」按钮写入剪贴板 */
export function buildErrorLog(error: PlayerError): string {
  const lines: Array<string | null> = [
    "[MoePlay Player Error]",
    `kind: ${error.kind}`,
    `message: ${error.message}`,
    error.httpStatus !== undefined ? `httpStatus: ${error.httpStatus}` : null,
    error.url ? `url: ${error.url}` : null,
    error.detail ? `detail: ${error.detail}` : null,
    `occurredAt: ${new Date(error.occurredAt).toISOString()}`,
  ];
  return lines.filter((line): line is string => line !== null).join("\n");
}
