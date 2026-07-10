<script lang="ts">
  import { onMount } from "svelte";
  import Button from "../ui/Button.svelte";
  import Icon from "../Icon.svelte";
  import { GenerationGuard, getAiClient, isAbortError, isAiUnavailableError, type AiClient } from "../../features/ai";
  import type { AiStatusSnapshot, AiTaskRecord } from "../../features/ai/types";

  let { client = getAiClient() }: { client?: AiClient } = $props();

  let snapshot = $state<AiStatusSnapshot | null>(null);
  let tasks = $state<AiTaskRecord[]>([]);
  let phase = $state<"idle" | "loading" | "ready" | "offline" | "error" | "cancelled">("idle");
  let errorMessage = $state("");
  let cancelling = $state<string | null>(null);
  const guard = new GenerationGuard();

  const providerCount = $derived(snapshot?.providers.length ?? 0);
  const healthyCount = $derived(snapshot?.providers.filter((provider) => provider.health === "healthy").length ?? 0);

  function healthLabel(health: string) {
    return ({ healthy: "可用", degraded: "受限", offline: "离线", disabled: "已关闭", unknown: "未知" } as Record<string, string>)[health] ?? health;
  }

  function taskLabel(status: string) {
    return ({ queued: "排队", running: "运行中", succeeded: "已完成", failed: "失败", cancelled: "已取消" } as Record<string, string>)[status] ?? status;
  }

  function formatDuration(task: AiTaskRecord) {
    if (task.durationMs == null) return "—";
    if (task.durationMs < 1000) return `${task.durationMs} ms`;
    return `${(task.durationMs / 1000).toFixed(1)} s`;
  }

  async function refresh() {
    const request = guard.begin();
    phase = "loading";
    errorMessage = "";
    try {
      const [nextSnapshot, nextTasks] = await Promise.all([
        client.getStatus(request.signal),
        client.listTasks(12, request.signal),
      ]);
      if (!guard.isCurrent(request.generation)) return;
      snapshot = nextSnapshot;
      tasks = nextTasks;
      phase = nextSnapshot.availability === "offline" || nextSnapshot.availability === "disabled" ? "offline" : "ready";
    } catch (error) {
      if (!guard.isCurrent(request.generation) || isAbortError(error)) return;
      snapshot = null;
      tasks = [];
      phase = isAiUnavailableError(error) ? "offline" : "error";
      errorMessage = phase === "offline" ? "AI 服务当前不可用；本地搜索、筛选和推荐仍可继续使用。" : "无法读取 AI 状态，请稍后重试。";
    }
  }

  function cancelRefresh() {
    guard.cancel();
    phase = "cancelled";
    errorMessage = "已取消本次状态刷新。";
  }

  async function cancelTask(taskId: string) {
    cancelling = taskId;
    try {
      await client.cancelTask(taskId);
      tasks = tasks.map((task) => task.id === taskId ? { ...task, status: "cancelled" } : task);
    } catch {
      errorMessage = "任务取消请求失败，任务状态未更改。";
    } finally {
      cancelling = null;
    }
  }

  onMount(() => {
    void refresh();
    return () => guard.cancel();
  });
</script>

<section class="status-center" aria-labelledby="ai-status-title">
  <div class="section-head">
    <div>
      <p class="eyebrow">AI STATUS CENTER</p>
      <h2 id="ai-status-title">AI 状态中心</h2>
      <p class="description">仅展示 Provider 能力、健康状态和脱敏任务摘要，不保存 prompt 或 response 正文。</p>
    </div>
    <div class="actions">
      {#if phase === "loading"}
        <Button variant="ghost" size="sm" press={cancelRefresh}>取消</Button>
      {:else}
        <Button variant="secondary" size="sm" press={refresh}>刷新状态</Button>
      {/if}
    </div>
  </div>

  {#if phase === "loading"}
    <div class="state-banner" aria-live="polite"><span class="spinner"></span>正在读取安全状态摘要…</div>
  {:else if phase === "offline" || phase === "error" || phase === "cancelled"}
    <div class:offline={phase === "offline"} class:error={phase === "error"} class="state-banner" role="status">
      <Icon name={phase === "offline" ? "cloudOff" : phase === "error" ? "x" : "square"} size={15} />
      <span>{errorMessage}</span>
    </div>
  {/if}

  {#if snapshot}
    <div class="summary-grid" aria-label="AI 概览">
      <div><span>Provider</span><strong>{healthyCount}/{providerCount}</strong><small>当前可用</small></div>
      <div><span>活动任务</span><strong>{snapshot.activeTaskCount}</strong><small>不含任务正文</small></div>
      <div><span>Token 估算</span><strong>{snapshot.dailyTokenEstimate ?? "—"}</strong><small>今日累计估算</small></div>
      <div><span>预算估算</span><strong>{snapshot.dailyBudgetEstimate ?? "—"}</strong><small>由后端策略提供</small></div>
    </div>

    <div class="provider-grid">
      {#each snapshot.providers as provider (provider.id)}
        <article class="provider-card">
          <div class="provider-top">
            <div>
              <strong>{provider.displayName}</strong>
              <span>{provider.model || "未选择模型"}</span>
            </div>
            <span class="health health-{provider.health}">{healthLabel(provider.health)}</span>
          </div>
          <div class="capabilities" aria-label={`${provider.displayName} 能力`}>
            <span class:enabled={provider.capabilities.structuredOutput}>结构化输出</span>
            <span class:enabled={provider.capabilities.jsonMode}>JSON</span>
            <span class:enabled={provider.capabilities.streaming}>流式</span>
            <span class:enabled={provider.capabilities.vision}>视觉</span>
            <span class:enabled={provider.capabilities.local}>本地</span>
          </div>
          <p>{provider.secretConfigured ? "凭据已配置（内容不可见）" : provider.capabilities.local ? "本地 Provider 无需远端凭据" : "未配置凭据"}</p>
        </article>
      {:else}
        <p class="empty-copy">尚未配置可公开展示的 Provider。</p>
      {/each}
    </div>
  {/if}

  <div class="history-head">
    <div>
      <h3>最近任务</h3>
      <p>仅保留用途、状态、耗时、Token 估算和错误分类。</p>
    </div>
    <span>{tasks.length} 条</span>
  </div>

  <div class="task-list">
    {#each tasks as task (task.id)}
      <article class="task-row">
        <div class="task-main">
          <span class="task-status status-{task.status}">{taskLabel(task.status)}</span>
          <div>
            <strong>{task.useCase}</strong>
            <p>{task.providerId} · {task.model} · schema {task.outputSchema}</p>
          </div>
        </div>
        <div class="task-meta">
          <span>{formatDuration(task)}</span>
          <span>{task.tokenEstimate ?? "—"} tokens</span>
          {#if task.errorKind}<span class="error-kind">{task.errorKind}</span>{/if}
          {#if task.status === "queued" || task.status === "running"}
            <Button variant="quiet" size="sm" disabled={cancelling === task.id} press={() => cancelTask(task.id)}>取消任务</Button>
          {/if}
        </div>
      </article>
    {:else}
      <p class="empty-copy">暂无可展示的 AI 任务历史。</p>
    {/each}
  </div>
</section>

<style>
  .status-center { display: grid; gap: 16px; }
  .section-head, .history-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .eyebrow { margin: 0 0 5px; color: var(--accent); font-family: var(--font-mono); font-size: 10px; font-weight: 700; letter-spacing: .13em; }
  h2, h3, p { margin: 0; }
  h2 { color: var(--text-primary); font-size: 1.12rem; }
  h3 { color: var(--text-primary); font-size: .9rem; }
  .description, .history-head p { max-width: 68ch; margin-top: 5px; color: var(--text-secondary); font-size: 12px; line-height: 1.55; }
  .actions { flex-shrink: 0; }
  .state-banner { min-height: 42px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; display: flex; align-items: center; gap: 9px; background: var(--bg-elev); color: var(--text-secondary); font-size: 12px; }
  .state-banner.offline { border-color: color-mix(in srgb, var(--color-warning, #fbbf24) 35%, var(--border)); color: var(--color-warning, #fbbf24); }
  .state-banner.error { border-color: color-mix(in srgb, var(--color-error, #f87171) 35%, var(--border)); color: var(--color-error, #f87171); }
  .spinner { width: 14px; height: 14px; border: 2px solid currentColor; border-right-color: transparent; border-radius: 50%; animation: spin .8s linear infinite; }
  .summary-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
  .summary-grid > div { padding: 13px; border: 1px solid var(--border); border-radius: 8px; display: grid; gap: 3px; background: var(--bg-deep); }
  .summary-grid span, .summary-grid small { color: var(--text-muted); font-size: 10px; }
  .summary-grid strong { color: var(--text-primary); font-family: var(--font-mono); font-size: 1.15rem; }
  .provider-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: 10px; }
  .provider-card { padding: 14px; border: 1px solid var(--border); border-radius: 8px; display: grid; gap: 12px; background: var(--bg-card); }
  .provider-top { display: flex; justify-content: space-between; gap: 12px; }
  .provider-top > div { min-width: 0; display: grid; gap: 3px; }
  .provider-top strong { overflow: hidden; color: var(--text-primary); font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .provider-top div > span, .provider-card p { color: var(--text-muted); font-size: 10px; }
  .health, .task-status { width: max-content; height: 22px; padding: 0 8px; border: 1px solid var(--border); border-radius: 999px; display: inline-flex; align-items: center; color: var(--text-muted); font-size: 10px; font-weight: 700; }
  .health-healthy, .status-succeeded { border-color: color-mix(in srgb, var(--color-success, #4ade80) 35%, transparent); color: var(--color-success, #4ade80); }
  .health-degraded, .status-queued { border-color: color-mix(in srgb, var(--color-warning, #fbbf24) 35%, transparent); color: var(--color-warning, #fbbf24); }
  .health-offline, .status-failed { border-color: color-mix(in srgb, var(--color-error, #f87171) 35%, transparent); color: var(--color-error, #f87171); }
  .status-running { border-color: color-mix(in srgb, var(--accent) 35%, transparent); color: var(--accent); }
  .capabilities { display: flex; flex-wrap: wrap; gap: 6px; }
  .capabilities span { padding: 4px 7px; border-radius: 5px; background: var(--bg-elev); color: var(--text-muted); font-size: 9px; }
  .capabilities span.enabled { color: var(--text-primary); }
  .history-head { padding-top: 3px; }
  .history-head > span { color: var(--text-muted); font-family: var(--font-mono); font-size: 11px; }
  .task-list { border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
  .task-row { min-height: 62px; padding: 10px 12px; display: flex; align-items: center; justify-content: space-between; gap: 14px; background: var(--bg-card); }
  .task-row + .task-row { border-top: 1px solid var(--border); }
  .task-main, .task-meta { display: flex; align-items: center; gap: 10px; }
  .task-main { min-width: 0; }
  .task-main > div { min-width: 0; }
  .task-main strong { color: var(--text-primary); font-size: 12px; }
  .task-main p { margin-top: 3px; overflow: hidden; color: var(--text-muted); font-family: var(--font-mono); font-size: 9px; text-overflow: ellipsis; white-space: nowrap; }
  .task-meta { flex-wrap: wrap; justify-content: flex-end; color: var(--text-muted); font-family: var(--font-mono); font-size: 9px; }
  .error-kind { color: var(--color-error, #f87171); }
  .empty-copy { padding: 18px; color: var(--text-muted); font-size: 12px; text-align: center; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 800px) { .summary-grid { grid-template-columns: repeat(2, 1fr); } .task-row { align-items: flex-start; flex-direction: column; } .task-meta { justify-content: flex-start; } }
  @media (max-width: 560px) { .section-head { flex-direction: column; } .summary-grid { grid-template-columns: 1fr; } }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>
