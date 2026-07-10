const sensitiveHeaders = new Set([
  "authorization",
  "proxy-authorization",
  "x-api-key",
  "api-key",
  "cookie",
  "set-cookie",
]);

export function redactHeaders(headers: Record<string, string>): Record<string, string> {
  return Object.fromEntries(
    Object.entries(headers).map(([name, value]) => [
      name,
      sensitiveHeaders.has(name.toLowerCase()) ? "[REDACTED]" : redactText(value),
    ]),
  );
}

export function redactUrl(raw: string): string {
  try {
    const url = new URL(raw);
    url.username = "";
    url.password = "";
    url.search = "";
    url.hash = "";
    return url.toString();
  } catch {
    return "[INVALID_URL]";
  }
}

export function redactText(input: string, sentinels: string[] = []): string {
  let output = input;
  for (const sentinel of sentinels.filter(Boolean)) output = output.split(sentinel).join("[REDACTED]");
  let redactNext = false;
  return output
    .split(/(\s+)/)
    .map((part) => {
      if (/^\s+$/.test(part)) return part;
      if (redactNext) {
        redactNext = false;
        return "[REDACTED]";
      }
      const lower = part.toLowerCase();
      if (lower === "bearer" || lower.endsWith("=bearer") || lower.endsWith(":bearer")) {
        redactNext = true;
        return "[REDACTED]";
      }
      return /^(bearer[-_]|sk-|key-)/i.test(part) ? "[REDACTED]" : part;
    })
    .join("");
}

export function redactJson(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(redactJson);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => {
        const lower = key.toLowerCase();
        const sensitive = ["secret", "token", "password", "api_key", "apikey", "authorization", "cookie"]
          .some((marker) => lower.includes(marker));
        return [key, sensitive ? "[REDACTED]" : redactJson(entry)];
      }),
    );
  }
  return typeof value === "string" ? redactText(value) : value;
}
