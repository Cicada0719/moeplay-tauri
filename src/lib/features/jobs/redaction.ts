/**
 * Defense in depth for task text. The backend persists only sanitized values,
 * but Task Center must also remain safe with legacy jobs and future adapters.
 */
const URL_PATTERN = /\b(?:https?|wss?):\/\/[^\s<>'"`]+/gi;
const HEADER_SECRET_PATTERN = /\b(authorization|proxy-authorization|x-api-key|api[_ -]?key|access[_ -]?token|refresh[_ -]?token|token|password|passwd|secret|cookie)\b\s*([:=])\s*(?:Bearer\s+)?[^\s,;)}\]]+/gi;
const JSON_SECRET_PATTERN = /(["']?(?:authorization|proxy-authorization|x-api-key|api[_ -]?key|access[_ -]?token|refresh[_ -]?token|token|password|passwd|secret|cookie)["']?\s*:\s*)(["'])[^"']*\2/gi;
const COMMAND_SECRET_PATTERN = /(--(?:token|api[_-]?key|password|secret)|-(?:p|k))\s*(?:=|\s)\s*([^\s]+)/gi;
const LONG_PAYLOAD_PATTERN = /\b(prompt|response|messages?|completion|chat[_ -]?history|request[_ -]?body)\b/i;

export function redactTaskText(value: unknown, maxLength = 512): string {
  if (typeof value !== "string") return "";
  const normalized = value.replace(/\s+/g, " ").trim();
  if (!normalized) return "";
  if (normalized.length > 2048 && LONG_PAYLOAD_PATTERN.test(normalized)) return "[已隐藏长内容]";

  const redacted = normalized
    .replace(URL_PATTERN, "[已隐藏链接]")
    .replace(JSON_SECRET_PATTERN, "$1$2[已隐藏]$2")
    .replace(HEADER_SECRET_PATTERN, "$1$2[已隐藏]")
    .replace(COMMAND_SECRET_PATTERN, "$1=[已隐藏]");

  return redacted.length > maxLength ? `${redacted.slice(0, Math.max(0, maxLength - 1))}…` : redacted;
}

export function redactTaskCode(value: unknown): string {
  const safe = redactTaskText(value, 128);
  return safe || "event";
}
