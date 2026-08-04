<script lang="ts">
  // 源健康状态徽标（spec task-02 Step 7.2，纯展示组件）。
  import type { HealthStatus } from "../api/rules";

  let {
    status,
    latencyMs = null,
    lastError = null,
    size = "md",
  }: {
    status: HealthStatus;
    latencyMs?: number | null;
    lastError?: string | null;
    size?: "sm" | "md";
  } = $props();

  const LABELS: Record<HealthStatus, { text: string; cls: string }> = {
    Healthy: { text: "可用", cls: "healthy" },
    Degraded: { text: "波动", cls: "degraded" },
    Abnormal: { text: "异常", cls: "abnormal" },
    Unknown: { text: "未知", cls: "unknown" },
  };

  const label = $derived(LABELS[status] ?? LABELS.Unknown);
  // Abnormal 时 title 提示 lastError（tooltip 用原生 title 即可）。
  const tip = $derived(status === "Abnormal" && lastError ? lastError : undefined);
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
