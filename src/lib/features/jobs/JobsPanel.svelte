<script lang="ts">
  import { onMount } from "svelte";
  import Button from "../../components/ui/Button.svelte";
  import Icon from "../../components/Icon.svelte";
  import Tag from "../../components/ui/Tag.svelte";
  import { createJobsStore, jobPercent, redactTaskText, type JobsStore, type Job } from ".";
  import TaskDetailDrawer from "./TaskDetailDrawer.svelte";

  type Filter = "all" | "active" | "failed" | "completed";
  type JobCapabilities = Job & {
    cancellable?: boolean;
    pausable?: boolean;
  };

  export let store: JobsStore = createJobsStore();
  export let compact = false;

  let jobs: Job[] = [];
  let loading = false;
  let error: string | null = null;
  let lastLoadedAt: number | null = null;
  let filter: Filter = "all";
  let actionId: string | null = null;
  let announcement = "";
  let selectedJob: Job | null = null;

  const unsubscribe = store.subscribe((snapshot) => {
    jobs = snapshot.jobs;
    loading = snapshot.loading;
    error = snapshot.error;
    lastLoadedAt = snapshot.lastLoadedAt;
  });

  $: activeCount = jobs.filter((job) => ["queued", "running", "paused"].includes(job.status)).length;
  $: failedCount = jobs.filter((job) => job.status === "failed").length;
  $: completedCount = jobs.filter((job) => ["succeeded", "cancelled"].includes(job.status)).length;
  $: if (selectedJob) {
    const updated = jobs.find((job) => job.id === selectedJob?.id);
    if (updated && updated !== selectedJob) selectedJob = updated;
  }
  $: filteredJobs = jobs.filter((job) => {
    if (filter === "active") return ["queued", "running", "paused"].includes(job.status);
    if (filter === "failed") return job.status === "failed";
    if (filter === "completed") return ["succeeded", "cancelled"].includes(job.status);
    return true;
  });

  onMount(() => {
    void store.load();
    const timer = window.setInterval(() => void store.load(), 2500);
    return () => {
      window.clearInterval(timer);
      unsubscribe();
    };
  });

  const statusLabel: Record<Job["status"], string> = {
    queued: "排队中",
    running: "运行中",
    paused: "已暂停",
    succeeded: "已完成",
    failed: "失败",
    cancelled: "已取消",
  };

  const kindLabel: Record<string, string> = {
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

  function safeText(value: unknown, maxLength = 512): string {
    return redactTaskText(value, maxLength);
  }

  function openDetails(job: Job) {
    selectedJob = job;
  }

  function canCancel(job: JobCapabilities): boolean {
    return job.cancellable ?? ["queued", "running", "paused"].includes(job.status);
  }

  function canPause(job: JobCapabilities): boolean {
    return job.pausable ?? (job.kind === "download" && job.status === "running");
  }

  async function runAction(job: Job, label: string, action: () => Promise<void>) {
    actionId = job.id;
    announcement = `${safeText(job.title)}：正在${label}`;
    try {
      await action();
      announcement = `${safeText(job.title)}：${label}请求已完成`;
    } finally {
      actionId = null;
    }
  }

  function formattedTime(value: string): string {
    const timestamp = Date.parse(value);
    if (!Number.isFinite(timestamp)) return "未知时间";
    return new Intl.DateTimeFormat("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(timestamp);
  }
</script>

<section class:compact class="jobs" aria-label="后台任务" data-testid="task-center-list">
  <div class="jobs__toolbar">
    <div class="jobs__filters" role="group" aria-label="任务筛选">
      <Tag active={filter === "all"} onclick={() => filter = "all"}>全部 {jobs.length}</Tag>
      <Tag active={filter === "active"} onclick={() => filter = "active"}>进行中 {activeCount}</Tag>
      <Tag active={filter === "failed"} onclick={() => filter = "failed"}>失败 {failedCount}</Tag>
      <Tag active={filter === "completed"} onclick={() => filter = "completed"}>已结束 {completedCount}</Tag>
    </div>
    <div class="jobs__toolbar-actions">
      {#if lastLoadedAt}<span class="last-sync">更新于 {new Date(lastLoadedAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}</span>{/if}
      <Button variant="quiet" size="sm" loading={loading} press={() => store.load()} ariaLabel="刷新任务">
        <Icon name="refresh" size={15} />刷新
      </Button>
      <Button variant="quiet" size="sm" disabled={loading || completedCount + failedCount === 0} press={() => store.clearFinished()} ariaLabel="清理已结束任务">
        <Icon name="trash" size={15} />清理
      </Button>
    </div>
  </div>

  <p class="sr-only" aria-live="polite">{announcement}</p>

  {#if error}
    <div class="jobs__error" role="alert">
      <span>任务状态暂时无法同步，已保留上次结果。</span>
      <Button variant="quiet" size="sm" press={() => store.load()}>重试</Button>
    </div>
  {/if}

  {#if loading && jobs.length === 0}
    <div class="jobs__empty" aria-busy="true">
      <span class="spinner" aria-hidden="true"></span>
      <p>正在加载任务…</p>
    </div>
  {:else if filteredJobs.length === 0}
    <div class="jobs__empty">
      <Icon name="list" size={28} />
      <strong>{jobs.length === 0 ? "暂无后台任务" : "当前筛选下没有任务"}</strong>
      <p>{jobs.length === 0 ? "下载、导入、刮削、来源验证和 AI 操作会显示在这里。" : "切换筛选条件查看其他状态。"}</p>
    </div>
  {:else}
    <ul class="jobs__list" aria-label="任务列表">
      {#each filteredJobs as job (job.id)}
        <li class="job" data-status={job.status} data-kind={job.kind}>
          <div class="job__main">
            <div class="job__title-row">
              <span class="job__kind">{kindLabel[job.kind] ?? "后台"}</span>
              <strong>{safeText(job.title)}</strong>
              {#if job.recovered}<span class="job__recovered">从上次会话恢复</span>{/if}
            </div>
            <div class="job__meta">
              <span class="job__status">{statusLabel[job.status]}</span>
              <span>{formattedTime(job.updated_at)}</span>
              {#if job.status === "running" || job.status === "paused" || job.status === "queued"}
                <span>{jobPercent(job)}%</span>
              {/if}
            </div>
            {#if safeText(job.message, 240)}<p class="job__message">{safeText(job.message, 240)}</p>{/if}
            {#if ["queued", "running", "paused"].includes(job.status)}
              <progress max="1" value={job.progress} aria-label={`${safeText(job.title)}进度`}>{jobPercent(job)}%</progress>
            {/if}
          </div>

          <div class="job__actions" aria-label={`${safeText(job.title)}操作`}>
            <Button variant="quiet" size="sm" press={() => openDetails(job)} ariaLabel={`查看 ${safeText(job.title)} 详情`}>详情</Button>
            {#if canPause(job)}
              <Button variant="quiet" size="sm" disabled={actionId === job.id} press={() => runAction(job, "暂停", () => store.pause(job.id))}>暂停</Button>
            {/if}
            {#if job.status === "paused" && job.resumable}
              <Button variant="secondary" size="sm" disabled={actionId === job.id} press={() => runAction(job, "继续", () => store.resume(job.id))}>继续</Button>
            {/if}
            {#if (job.status === "failed" || job.status === "paused") && job.retryable}
              <Button variant="secondary" size="sm" disabled={actionId === job.id} press={() => runAction(job, "重试", () => store.retry(job.id))}>重试</Button>
            {/if}
            {#if canCancel(job)}
              <Button variant="quiet" size="sm" disabled={actionId === job.id} press={() => runAction(job, "取消", () => store.cancel(job.id))}>取消</Button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<TaskDetailDrawer open={selectedJob !== null} job={selectedJob} {store} onClose={() => selectedJob = null} />

<style>
  .jobs { min-width: 0; display: grid; gap: 14px; }
  .jobs__toolbar { min-width: 0; display: flex; align-items: center; justify-content: space-between; gap: 14px; }
  .jobs__filters, .jobs__toolbar-actions, .job__title-row, .job__meta, .job__actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .jobs__toolbar-actions { justify-content: flex-end; }
  .last-sync { color: var(--text-muted); font-size: 11px; }
  .jobs__error { padding: 10px 12px; border: 1px solid color-mix(in srgb, var(--color-error, #f87171) 35%, var(--border)); border-radius: 8px; display: flex; align-items: center; justify-content: space-between; gap: 12px; color: var(--color-error, #f87171); background: color-mix(in srgb, var(--color-error, #f87171) 7%, var(--bg-card)); }
  .jobs__list { margin: 0; padding: 0; list-style: none; display: grid; gap: 8px; }
  .job { min-width: 0; padding: 14px; border: 1px solid var(--border); border-radius: 8px; display: flex; align-items: center; justify-content: space-between; gap: 18px; background: var(--bg-card); }
  .job[data-status="running"] { border-color: color-mix(in srgb, var(--accent) 36%, var(--border)); }
  .job[data-status="failed"] { border-color: color-mix(in srgb, var(--color-error, #f87171) 38%, var(--border)); }
  .job__main { min-width: 0; flex: 1; display: grid; gap: 8px; }
  .job__title-row strong { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-primary); }
  .job__kind, .job__status, .job__recovered { width: max-content; padding: 3px 7px; border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted); font-size: 10px; font-weight: 700; }
  .job__recovered { color: var(--accent); border-color: var(--accent-ring); }
  .job__meta { color: var(--text-muted); font-size: 11px; }
  .job__message { margin: 0; color: var(--text-secondary); font-size: 12px; line-height: 1.5; overflow-wrap: anywhere; }
  progress { width: 100%; height: 5px; border: 0; border-radius: 999px; overflow: hidden; accent-color: var(--accent); }
  progress::-webkit-progress-bar { background: var(--bg-elevated); }
  progress::-webkit-progress-value { background: var(--accent); border-radius: inherit; }
  .job__actions { flex: 0 0 auto; justify-content: flex-end; }
  .jobs__empty { min-height: 210px; padding: 28px; border: 1px dashed var(--border); border-radius: 8px; display: grid; place-items: center; align-content: center; gap: 8px; text-align: center; color: var(--text-muted); background: color-mix(in srgb, var(--bg-card) 72%, transparent); }
  .jobs__empty strong { color: var(--text-primary); }
  .jobs__empty p { max-width: 460px; margin: 0; font-size: 12px; line-height: 1.6; }
  .spinner { width: 24px; height: 24px; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: spin .7s linear infinite; }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
  .compact .jobs__toolbar { align-items: flex-start; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 760px) {
    .jobs__toolbar, .job { align-items: stretch; flex-direction: column; }
    .jobs__toolbar-actions, .job__actions { justify-content: flex-start; }
  }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>
