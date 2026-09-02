/**
 * Convert recommendation failures into copy suitable for the product UI.
 * Transport and parser details stay in the diagnostic log instead of leaking
 * request URLs or native error payloads into the handheld banner.
 */
export function friendlyRecommendationError(error: unknown, fallback: string): string {
  const raw = error instanceof Error ? error.message : String(error ?? "");
  const normalized = raw.trim();
  if (!normalized) return fallback;

  if (/typeerror|not iterable|cannot read properties|undefined is not/i.test(normalized)) {
    return `${fallback}（源数据格式暂不可用）`;
  }
  if (/error sending request|error trying to connect|connection reset|connection refused|network|fetch failed|timed? out|dns|http request/i.test(normalized)) {
    return "推荐源暂时无法连接，请检查网络或稍后重试";
  }
  if (/\b(401|403)\b|unauthori[sz]ed|forbidden|access token/i.test(normalized)) {
    return "推荐源需要授权，请检查来源设置";
  }
  return normalized.length > 160 ? `${normalized.slice(0, 157)}…` : normalized;
}
