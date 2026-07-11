<script lang="ts">
  import Drawer from "../../components/ui-v2/Drawer.svelte";
  import type { Job, TaskDetail, TaskDetailApi, TaskEvent, TaskEventLevel } from "./contracts";
  import type { JobsStore } from "./store";
  import { tauriTaskDetailApi } from "./api";
  import { mergeTaskEvents } from "./contracts";
  import { redactTaskCode, redactTaskText } from "./redaction";

  type EventFilter = "all" | TaskEventLevel;

  export let open = false;
  export let job: Job | null = null;
  export let store: JobsStore;
  export let detailApi: TaskDetailApi = tauriTaskDetailApi;
  export let onClose: () => void;
  export let pageSize = 50;

  let detail: TaskDetail | null = null;
  let events: TaskEvent[] = [];
  let loading = false;
  let refreshing = false;
  let loadingMore = false;
  let error: string | null = null;
  let eventFilter: EventFilter = "all";
  let actionName: string | null = null;
  let announcement = "";
  let requestedId: string | null = null;
  let requestVersion = 0;
  let hasMoreEvents = false;

  const eventFilters: ReadonlyArray<{ value: EventFilter; label: string }> = [
    { value: "all", label: "全部" },
    { value: "info", label: "信息" },
    { value: "warn", label: "警告" },
    { value: "error", label: "错误" },
  ];

  const statusLabel: Record<Job["status"], string> = {
    queued: "排队中",
    running: "运行中",
    paused: "已暂停",
    succeeded: "已完成",
    failed: "失败",
    cancelled: "已取消",
  };

  const kindLabel: Record<Job["kind"], string> = {
    download: "下载",
    import: "导入",
    scrape: "刮削",
    provider_verify: "来源验证",
    ai: "AI",
    backup: "备份",
    restore: "恢复",
    diagnostics: "诊断",
    update: "更新",
    generic: "后台",
  };

  const operationLabel: Record<NonNullable<TaskDetail["operation"]>["kind"], string> = {
    import: "导入",
    scrape: "刮削",
    provider_verify: "来源验证",
    backup: "备份",
    restore: "恢复",
    diagnostics_export: "导出诊断",
    update_check: "检查更新",
  };

  $: currentJob = job ?? detail?.job ?? null;
  $: safeTitle = redactTaskText(currentJob?.title ?? "任务");
  $: visibleEvents = eventFilter === "all" ? events : events.filter((event) => event.level === eventFilter);
  $: eventCursor = events.at(-1)?.sequence ?? null;
  $: resolvedPageSize = Math.min(200, Math.max(1, Math.trunc(pageSize)));
  $: hasMore = hasMoreEvents && !loadingMore;

  $: if (open && job?.id && requestedId !== job.id) {
    requestedId = job.id;
    void loadInitial(job.id);
  }

  $: if (!open && requestedId !== null) {
    requestedId = null;
    requestVersion += 1;
  }

  function errorMessage(value: unknown): string {
    return value instanceof Error ? value.message : String(value);
  }

  function formatTime(value: string): string {
    const timestamp = Date.parse(value);
    if (!Number.isFinite(timestamp)) return "未知时间";
    return new Intl.DateTimeFormat("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    }).format(timestamp);
  }

  function isCurrentRequest(id: string, version: number): boolean {
    return open && requestedId === id && requestVersion === version;
  }

  function mergePage(incoming: readonly TaskEvent[], replace = false) {
    events = replace ? mergeTaskEvents([], incoming) : mergeTaskEvents(events, incoming);
  }

  async function loadInitial(id: string) {
    const version = ++requestVersion;
    loading = true;
    error = null;
    detail = null;
    events = [];
    hasMoreEvents = false;
    try {
      const [nextDetail, page] = await Promise.all([
        detailApi.getTaskDetail(id),
        detailApi.getTaskEvents(id, { limit: resolvedPageSize }),
      ]);
      if (!isCurrentRequest(id, version)) return;
      detail = nextDetail;
      mergePage(page.events, true);
      hasMoreEvents = page.hasMore;
      announcement = page.events.length > 0 ? `已加载 ${page.events.length} 条任务事件。` : "任务详情已加载，暂无事件。";
    } catch (reason) {
      if (!isCurrentRequest(id, version)) return;
      error = `无法加载任务详情：${redactTaskText(errorMessage(reason), 180)}`;
      announcement = "任务详情加载失败。";
    } finally {
      if (isCurrentRequest(id, version)) loading = false;
    }
  }

  /** Polls only events after the rendered sequence; it never clears the timeline. */
  async function refresh() {
    const id = job?.id;
    if (!id || refreshing || loadingMore) return;
    const version = ++requestVersion;
    refreshing = true;
    error = null;
    try {
      const [nextDetail, page] = await Promise.all([
        detailApi.getTaskDetail(id),
        detailApi.getTaskEvents(id, {
          ...(eventCursor ? { afterSequence: eventCursor } : {}),
          limit: resolvedPageSize,
        }),
      ]);
      if (!isCurrentRequest(id, version)) return;
      detail = nextDetail;
      mergePage(page.events);
      hasMoreEvents = page.hasMore;
      announcement = page.events.length > 0 ? `任务事件已更新，新增 ${page.events.length} 条。` : "任务详情已刷新，没有新事件。";
    } catch (reason) {
      if (!isCurrentRequest(id, version)) return;
      error = `无法刷新任务详情：${redactTaskText(errorMessage(reason), 180)}`;
      announcement = "任务详情刷新失败，已保留当前事件。";
    } finally {
      if (isCurrentRequest(id, version)) refreshing = false;
    }
  }

  async function loadMore() {
    const id = job?.id;
    if (!id || !eventCursor || loadingMore || loading || refreshing) return;
    const version = ++requestVersion;
    loadingMore = true;
    error = null;
    try {
      const page = await detailApi.getTaskEvents(id, { afterSequence: eventCursor, limit: resolvedPageSize });
      if (!isCurrentRequest(id, version)) return;
      mergePage(page.events);
      hasMoreEvents = page.hasMore;
      announcement = page.events.length > 0 ? `已加载 ${page.events.length} 条后续任务事件。` : "没有更多任务事件。";
    } catch (reason) {
      if (!isCurrentRequest(id, version)) return;
      error = `无法加载后续事件：${redactTaskText(errorMessage(reason), 180)}`;
      announcement = "加载后续任务事件失败，已保留当前事件。";
    } finally {
      if (isCurrentRequest(id, version)) loadingMore = false;
    }
  }

  function retryLoad() {
    if (events.length > 0) {
      void refresh();
    } else if (job) {
      void loadInitial(job.id);
    }
  }

  function canPause(candidate: Job): boolean {
    return candidate.status === "running" && candidate.pausable;
  }

  function canResume(candidate: Job): boolean {
    return candidate.status === "paused" && candidate.resumable;
  }

  function canRetry(candidate: Job): boolean {
    return (candidate.status === "failed" || candidate.status === "paused") && candidate.retryable;
  }

  function canCancel(candidate: Job): boolean {
    return ["queued", "running", "paused"].includes(candidate.status) && candidate.cancellable;
  }

  async function runAction(label: string, action: () => Promise<void>) {
    if (!currentJob || actionName) return;
    actionName = label;
    announcement = `${safeTitle}：正在${label}。`;
    try {
      await action();
      announcement = `${safeTitle}：${label}请求已完成。`;
      await refresh();
    } catch (reason) {
      error = `无法${label}任务：${redactTaskText(errorMessage(reason), 180)}`;
      announcement = `${safeTitle}：${label}失败。`;
    } finally {
      actionName = null;
    }
  }
</script>

<Drawer
  {open}
  title="任务详情"
  description={safeTitle}
  side="right"
  size="lg"
  onClose={onClose}
  initialFocus="auto"
  returnFocus
  class="task-detail-drawer"
>
  <div class="task-detail" data-testid="task-detail-drawer" aria-busy={loading}>
    <p class="sr-only" aria-live="polite" aria-atomic="true">{announcement}</p>

    {#if currentJob}
      <section class="task-summary" aria-label="任务摘要">
        <div class="task-summary__heading">
          <div>
            <p class="eyebrow">{kindLabel[currentJob.kind]}</p>
            <h3>{safeTitle}</h3>
          </div>
          <span class="status" data-status={currentJob.status}>{statusLabel[currentJob.status]}</span>
        </div>
        {#if currentJob.message}<p class="summary-message">{redactTaskText(currentJob.message)}</p>{/if}
        {#if currentJob.errorKind}<p class="error-code"><span>错误代码</span><code>{redactTaskCode(currentJob.errorKind)}</code></p>{/if}
        <dl class="task-meta">
          <div><dt>任务 ID</dt><dd>{redactTaskText(currentJob.id)}</dd></div>
          <div><dt>创建时间</dt><dd>{formatTime(currentJob.createdAt)}</dd></div>
          <div><dt>最近更新</dt><dd>{formatTime(currentJob.updatedAt)}</dd></div>
          {#if currentJob.source}
            <div><dt>来源区域</dt><dd>{redactTaskText(currentJob.source.area)}</dd></div>
            {#if currentJob.source.label}<div><dt>来源</dt><dd>{redactTaskText(currentJob.source.label)}</dd></div>{/if}
            {#if currentJob.source.entityId}<div><dt>引用 ID</dt><dd>{redactTaskText(currentJob.source.entityId)}</dd></div>{/if}
          {/if}
        </dl>
        {#if detail?.operation}
          <div class="operation" aria-label="任务操作元数据">
            <strong>{operationLabel[detail.operation.kind]}</strong>
            {#if detail.operation.fields.length > 0}
              <dl>
                {#each detail.operation.fields as field (field.label)}
                  <div><dt>{field.label}</dt><dd>{redactTaskText(field.value)}</dd></div>
                {/each}
              </dl>
            {/if}
          </div>
        {/if}
      </section>
    {/if}

    <section class="timeline-section" aria-labelledby="task-event-timeline-title">
      <div class="timeline-section__heading">
        <div>
          <p class="eyebrow">Event timeline</p>
          <h3 id="task-event-timeline-title">任务事件</h3>
        </div>
        <button class="refresh-button" type="button" onclick={refresh} disabled={loading || refreshing || loadingMore} aria-label="刷新任务详情">
          {refreshing ? "刷新中…" : "刷新"}
        </button>
      </div>

      <div class="event-filters" role="group" aria-label="事件级别筛选">
        {#each eventFilters as filter (filter.value)}
          <button type="button" aria-pressed={eventFilter === filter.value} onclick={() => eventFilter = filter.value}>{filter.label}</button>
        {/each}
      </div>

      {#if error}
        <div class="drawer-error" role="alert">
          <span>{error}</span>
          <button type="button" onclick={retryLoad}>重试</button>
        </div>
      {/if}

      {#if loading}
        <div class="timeline-state" role="status"><span class="spinner" aria-hidden="true"></span>正在加载任务事件…</div>
      {:else if visibleEvents.length === 0}
        <div class="timeline-state" data-testid="task-events-empty">
          <strong>{events.length === 0 ? "暂无任务事件" : "没有匹配的任务事件"}</strong>
          <span>{events.length === 0 ? "该任务尚未记录可展示的进度或错误事件。" : "请调整事件级别筛选。"}</span>
        </div>
      {:else}
        <ol class="timeline" aria-label="任务事件时间线">
          {#each visibleEvents as event (event.sequence)}
            <li data-level={event.level}>
              <div class="timeline__marker" aria-hidden="true"></div>
              <div class="timeline__content">
                <div class="timeline__topline">
                  <time datetime={event.createdAt}>{formatTime(event.createdAt)}</time>
                  <code>{redactTaskCode(event.code)}</code>
                </div>
                <p>{redactTaskText(event.message, 1024)}</p>
                {#if event.progress !== undefined}<span class="event-progress">进度 {Math.round(event.progress * 100)}%</span>{/if}
              </div>
            </li>
          {/each}
        </ol>
        {#if hasMore}
          <button class="load-more" type="button" onclick={loadMore} disabled={loadingMore}>
            {loadingMore ? "加载中…" : "加载后续事件"}
          </button>
        {/if}
      {/if}
    </section>
  </div>

  {#snippet footer()}
    <div class="task-actions" aria-label={`${safeTitle}操作`}>
      {#if currentJob && canPause(currentJob)}
        <button type="button" onclick={() => runAction("暂停", () => store.pause(currentJob!.id))} disabled={actionName !== null}>暂停</button>
      {/if}
      {#if currentJob && canResume(currentJob)}
        <button type="button" onclick={() => runAction("继续", () => store.resume(currentJob!.id))} disabled={actionName !== null}>继续</button>
      {/if}
      {#if currentJob && canRetry(currentJob)}
        <button class="secondary" type="button" onclick={() => runAction("重试", () => store.retry(currentJob!.id))} disabled={actionName !== null}>重试</button>
      {/if}
      {#if currentJob && canCancel(currentJob)}
        <button class="danger" type="button" onclick={() => runAction("取消", () => store.cancel(currentJob!.id))} disabled={actionName !== null}>取消</button>
      {/if}
      {#if !currentJob || (!canPause(currentJob) && !canResume(currentJob) && !canRetry(currentJob) && !canCancel(currentJob))}
        <span class="no-actions">当前状态没有可用操作</span>
      {/if}
    </div>
  {/snippet}
</Drawer>

<style>
  .task-detail { min-width: 0; display: grid; gap: 1.25rem; color: var(--text-secondary); }
  .task-summary, .timeline-section { min-width: 0; display: grid; gap: .85rem; }
  .task-summary__heading, .timeline-section__heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
  .eyebrow { margin: 0 0 .25rem; color: var(--accent); font: 700 10px/1 var(--font-mono, monospace); letter-spacing: .12em; text-transform: uppercase; }
  h3 { margin: 0; color: var(--text-primary); font-size: 1rem; line-height: 1.3; overflow-wrap: anywhere; }
  .status { flex: 0 0 auto; padding: .25rem .48rem; border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted); font: 700 10px/1 var(--font-mono, monospace); }
  .status[data-status="running"], .status[data-status="queued"] { border-color: color-mix(in srgb, var(--accent) 52%, var(--border)); color: var(--accent); }
  .status[data-status="failed"] { border-color: color-mix(in srgb, var(--color-error, #f87171) 52%, var(--border)); color: var(--color-error, #f87171); }
  .status[data-status="succeeded"] { color: var(--color-success, #4ade80); }
  .summary-message { margin: 0; font-size: .8125rem; line-height: 1.55; overflow-wrap: anywhere; }
  .error-code { margin: 0; display: flex; align-items: center; gap: .5rem; color: var(--color-error, #f87171); font-size: .75rem; }
  .error-code code, .timeline code { overflow-wrap: anywhere; font: 700 .7rem/1.3 var(--font-mono, monospace); }
  .task-meta, .operation dl { margin: 0; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .6rem; }
  .task-meta div, .operation dl div { min-width: 0; padding: .58rem .65rem; border: 1px solid var(--border); border-radius: .45rem; background: color-mix(in srgb, var(--bg-elevated) 65%, transparent); }
  dt { color: var(--text-muted); font-size: .66rem; line-height: 1.2; }
  dd { margin: .26rem 0 0; color: var(--text-primary); font: 600 .75rem/1.4 var(--font-mono, monospace); overflow-wrap: anywhere; }
  .operation { display: grid; gap: .55rem; padding: .75rem; border-left: 2px solid var(--accent); background: color-mix(in srgb, var(--accent) 7%, transparent); }
  .operation strong { color: var(--text-primary); font-size: .8rem; }
  .refresh-button, .event-filters button, .drawer-error button, .load-more, .task-actions button { min-height: 2rem; border: 1px solid var(--border); border-radius: .42rem; background: var(--bg-elevated); color: var(--text-secondary); font: 650 .75rem/1 var(--font-ui, sans-serif); cursor: pointer; }
  .refresh-button { padding: 0 .65rem; }
  .event-filters { display: flex; flex-wrap: wrap; gap: .4rem; }
  .event-filters button { padding: 0 .65rem; }
  .event-filters button[aria-pressed="true"] { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 15%, var(--bg-elevated)); color: var(--text-primary); }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  button:disabled { cursor: wait; opacity: .62; }
  .drawer-error { display: flex; align-items: center; justify-content: space-between; gap: .75rem; padding: .7rem; border: 1px solid color-mix(in srgb, var(--color-error, #f87171) 45%, var(--border)); border-radius: .45rem; color: var(--color-error, #f87171); background: color-mix(in srgb, var(--color-error, #f87171) 8%, transparent); font-size: .75rem; line-height: 1.45; }
  .drawer-error button { flex: 0 0 auto; padding: 0 .55rem; }
  .timeline-state { min-height: 9rem; padding: 1rem; border: 1px dashed var(--border); border-radius: .5rem; display: grid; place-items: center; align-content: center; gap: .6rem; color: var(--text-muted); font-size: .78rem; text-align: center; }
  .timeline-state strong { color: var(--text-primary); }
  .timeline { margin: 0; padding: 0; list-style: none; display: grid; gap: 0; }
  .timeline li { position: relative; min-width: 0; display: grid; grid-template-columns: 1rem minmax(0, 1fr); gap: .7rem; padding: 0 0 1rem; }
  .timeline li:not(:last-child)::before { position: absolute; top: .75rem; bottom: 0; left: .46rem; width: 1px; background: var(--border); content: ""; }
  .timeline__marker { width: .65rem; height: .65rem; margin-top: .25rem; border: 2px solid var(--bg-card); border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 1px var(--accent-ring, var(--accent)); }
  .timeline li[data-level="warn"] .timeline__marker { background: var(--color-warning, #fbbf24); box-shadow: 0 0 0 1px var(--color-warning, #fbbf24); }
  .timeline li[data-level="error"] .timeline__marker { background: var(--color-error, #f87171); box-shadow: 0 0 0 1px var(--color-error, #f87171); }
  .timeline__content { min-width: 0; padding: .6rem .7rem; border: 1px solid var(--border); border-radius: .45rem; background: var(--bg-card); }
  .timeline__topline { display: flex; align-items: center; justify-content: space-between; gap: .6rem; color: var(--text-muted); font-size: .68rem; }
  .timeline__topline time { flex: 0 0 auto; }
  .timeline__topline code { color: var(--text-secondary); text-align: right; }
  .timeline p { margin: .45rem 0 0; color: var(--text-secondary); font-size: .78rem; line-height: 1.5; overflow-wrap: anywhere; }
  .event-progress { display: inline-block; margin-top: .45rem; color: var(--accent); font: 700 .68rem/1 var(--font-mono, monospace); }
  .load-more { width: 100%; padding: 0 .75rem; }
  .task-actions { width: 100%; display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: .5rem; }
  .task-actions button { padding: 0 .75rem; color: var(--text-primary); }
  .task-actions .secondary { border-color: color-mix(in srgb, var(--accent) 55%, var(--border)); }
  .task-actions .danger { border-color: color-mix(in srgb, var(--color-error, #f87171) 55%, var(--border)); color: var(--color-error, #f87171); }
  .no-actions { color: var(--text-muted); font-size: .75rem; }
  .spinner { width: 1.15rem; height: 1.15rem; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: task-spin .7s linear infinite; }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
  @keyframes task-spin { to { transform: rotate(360deg); } }
  @media (max-width: 32rem) { .task-meta, .operation dl { grid-template-columns: 1fr; } .task-summary__heading { align-items: stretch; flex-direction: column; } .status { align-self: flex-start; } }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>
