<script lang="ts">
  // 源健康状态徽标（spec task-02 Step 7.2，纯展示组件）。
  import type { HealthErrorKind, HealthStatus, LastKnownHealth } from "../api/rules";

  let {
    status,
    latencyMs = null,
    lastError = null,
    stage = null,
    errorKind = null,
    httpStatus = null,
    checkedAt = null,
    lastKnown = null,
    size = "md",
  }: {
    status: HealthStatus;
    latencyMs?: number | null;
    lastError?: string | null;
    stage?: string | null;
    errorKind?: HealthErrorKind | null;
    httpStatus?: number | null;
    checkedAt?: number | null;
    lastKnown?: LastKnownHealth | null;
    size?: "sm" | "md";
  } = $props();

  const LABELS: Record<HealthStatus, { text: string; cls: string }> = {
    Healthy: { text: "可用", cls: "healthy" },
    Degraded: { text: "波动", cls: "degraded" },
    Abnormal: { text: "异常", cls: "abnormal" },
    Unknown: { text: "未知", cls: "unknown" },
  };

  const label = $derived(LABELS[status] ?? LABELS.Unknown);
  const ERROR_LABELS: Record<HealthErrorKind, string> = {
    network: "网络",
    http: "HTTP",
    "tls-dns": "TLS/DNS",
    timeout: "超时",
    challenge: "验证页",
    script: "脚本",
    empty: "空结果",
    cancelled: "已取消",
    unknown: "未知错误",
  };
  // 健康状态是最近一次探测的观测，不宣称第三方源永久可用。
  const tip = $derived(
    status === "Healthy" && !stage && !errorKind && httpStatus == null && !lastKnown
      ? undefined
      : !stage && !errorKind && httpStatus == null && !lastKnown && lastError
        ? lastError
        : [
          `最近检查：${label.text}`,
          stage ? `阶段：${stage}` : "",
          errorKind ? `错误：${ERROR_LABELS[errorKind]}` : "",
          httpStatus != null ? `HTTP ${httpStatus}` : "",
          checkedAt != null ? `时间：${new Date(checkedAt * 1000).toLocaleString()}` : "",
          lastError ?? "",
          lastKnown ? `上次已知：${new Date(lastKnown.checkedAt * 1000).toLocaleString()}` : "",
        ]
          .filter(Boolean)
          .join("；"),
  );
</script>

<span
  class="source-health source-health--{label.cls} source-health--{size}"
  data-testid="source-health"
  data-status={status}
  title={tip}
  role="img"
  aria-label={label.text}
>
  <span class="source-health__dot" aria-hidden="true"></span>
  <span class="source-health__text">{label.text}</span>
  {#if size === "md" && latencyMs != null}
    <span class="source-health__latency" data-testid="source-health-latency">{latencyMs}ms</span>
  {/if}
  {#if size === "md" && errorKind}
    <span class="source-health__error-kind" data-testid="source-health-error-kind">{ERROR_LABELS[errorKind]}</span>
  {/if}
</span>

<style>
  .source-health {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary, inherit);
    font-size: 11px;
    font-weight: 600;
    line-height: 1.4;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .source-health--sm {
    padding: 1px 6px;
    font-size: 10px;
  }
  .source-health__dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-dim, #888);
    flex-shrink: 0;
  }
  .source-health--healthy .source-health__dot {
    background: var(--color-success, #22c55e);
    box-shadow: 0 0 8px var(--color-success, #22c55e);
  }
  .source-health--degraded .source-health__dot {
    background: #eab308;
    box-shadow: 0 0 8px rgba(234, 179, 8, 0.6);
  }
  .source-health--abnormal .source-health__dot {
    background: var(--color-error, #ef4444);
    box-shadow: 0 0 8px var(--color-error, #ef4444);
  }
  .source-health--unknown .source-health__dot {
    background: var(--text-dim, #888);
  }
  .source-health__latency {
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }
</style>
