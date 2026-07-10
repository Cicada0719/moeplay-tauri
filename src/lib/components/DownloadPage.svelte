<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import {
    downloadCancel,
    downloadPause,
    downloadResume,
    downloadStart,
    downloadRetry,
    downloadRemove,
    downloadClearFinished,
    formatFileSize,
    getDownloads,
    type DownloadTask,
    animeGetDownloads,
    animeCancelDownload,
    animePauseDownload,
    animeResumeDownload,
    animeRemoveDownload,
    animeClearFinishedDownloads,
    animeOpenDownloadFolder,
    type AnimeDownloadTask,
  } from "../api";
  import {
    createJobsStore,
    persistentDownloadsApi,
    type Job,
  } from "../features/jobs";
  import { Button, Card, EmptyState, Input, SegmentControl, Tag } from "./ui";

  type DownloadEvidence = {
    accepted: boolean;
    reason?: string | null;
    availableBytes?: number | null;
    requiredBytes?: number | null;
    quotaBytes?: number | null;
    quotaRemainingBytes?: number | null;
  };
  type DownloadProjection = DownloadTask & {
    recovered?: boolean;
    resumable?: boolean;
    retryable?: boolean;
    quota_bytes?: number | null;
    preflight?: DownloadEvidence | null;
  };
  type DownloadRow = { id: string; job?: Job; task?: DownloadProjection };

  const jobsStore = createJobsStore();
  let url = $state("");
  let filename = $state("");
  let quotaGb = $state("");
  let downloads = $state<DownloadProjection[]>([]);
  let jobs = $state<Job[]>([]);
  let jobsError = $state<string | null>(null);
  let animeDownloads = $state<AnimeDownloadTask[]>([]);
  let activeTab = $state<"general" | "anime">("general");
  let loading = $state(false);
  let startError = $state<string | null>(null);
  let urlBox = $state<HTMLDivElement>();

  const unsubscribeJobs = jobsStore.subscribe((snapshot) => {
    jobs = snapshot.jobs;
    jobsError = snapshot.error;
  });

  function buildRows(allJobs: Job[], legacy: DownloadProjection[]): DownloadRow[] {
    const persistent = allJobs.filter((job) => job.kind === "download");
    const persistentIds = new Set(persistent.map((job) => job.id));
    return [
      ...persistent.map((job) => ({
        id: job.id,
        job,
        task: legacy.find((task) => task.id === job.id),
      })),
      ...legacy
        .filter((task) => !persistentIds.has(task.id))
        .map((task) => ({ id: task.id, task })),
    ];
  }

  const generalRows = $derived(buildRows(jobs, downloads));

  async function refreshLegacy() {
    try { downloads = await getDownloads() as DownloadProjection[]; } catch { downloads = []; }
    try { animeDownloads = await animeGetDownloads(); } catch { animeDownloads = []; }
  }

  async function refresh() {
    await Promise.all([refreshLegacy(), jobsStore.load()]);
  }

  function requestedQuotaBytes(): number | undefined {
    if (!quotaGb.trim()) return undefined;
    const parsed = Number(quotaGb);
    if (!Number.isFinite(parsed) || parsed <= 0) return undefined;
    return Math.round(parsed * 1024 * 1024 * 1024);
  }

  async function start() {
    if (!url.trim()) return;
    loading = true;
    startError = null;
    const targetFilename = filename || url.split("/").pop() || "download.bin";
    try {
      if (jobsError) {
        await downloadStart(url, targetFilename);
      } else {
        await persistentDownloadsApi.start({
          url,
          filename: targetFilename,
          quotaBytes: requestedQuotaBytes(),
        });
      }
      url = "";
      filename = "";
      await refresh();
    } catch (error) {
      startError = error instanceof Error ? error.message : String(error);
    } finally {
      loading = false;
    }
  }

  async function act(row: DownloadRow, action: "pause" | "resume" | "retry" | "cancel") {
    if (row.job) {
      if (action === "pause") await jobsStore.pause(row.id);
      if (action === "resume") await jobsStore.resume(row.id);
      if (action === "retry") await jobsStore.retry(row.id);
      if (action === "cancel") await jobsStore.cancel(row.id);
    } else {
      if (action === "pause") await downloadPause(row.id);
      if (action === "resume") await downloadResume(row.id);
      if (action === "retry") await downloadRetry(row.id);
      if (action === "cancel") await downloadCancel(row.id);
    }
    await refreshLegacy();
  }

  async function removeRow(row: DownloadRow) {
    await downloadRemove(row.id);
    await refresh();
  }

  async function clearGeneralFinished() {
    await downloadClearFinished();
    await refresh();
  }

  function focusUrlInput() {
    urlBox?.querySelector<HTMLInputElement>("input")?.focus();
  }

  onMount(() => {
    void refresh();
    const id = window.setInterval(refresh, 1200);
    return () => {
      window.clearInterval(id);
      unsubscribeJobs();
    };
  });

  function rowStatus(row: DownloadRow): string {
    if (row.job) {
      if (row.job.recovered) return "已恢复（可继续）";
      const labels: Record<Job["status"], string> = {
        queued: "等待中", running: "下载中", paused: "已暂停",
        succeeded: "已完成", failed: "失败", cancelled: "已取消",
      };
      return labels[row.job.status];
    }
    return statusLabel(row.task?.status ?? "Pending");
  }

  function rowStatusClass(row: DownloadRow): string {
    if (row.job) {
      if (row.job.recovered || row.job.status === "paused") return "paused";
      if (row.job.status === "running") return "active";
      if (row.job.status === "succeeded") return "done";
      if (row.job.status === "failed" || row.job.status === "cancelled") return "fail";
      return "";
    }
    return statusClass(row.task?.status ?? "Pending");
  }

  function rowProgress(row: DownloadRow): number {
    return Math.min(1, Math.max(0, row.job?.progress ?? row.task?.progress ?? 0));
  }

  function statusLabel(s: string): string {
    const m: Record<string,string> = {
      Pending: "等待中", Downloading: "下载中", Paused: "已暂停",
      Completed: "已完成", Failed: "失败", Extracting: "解压中",
      Importing: "导入中", Cancelled: "已取消",
    };
    return m[s] ?? s;
  }

  function statusClass(s: string): string {
    const m: Record<string,string> = {
      Downloading: "active", Completed: "done", Failed: "fail", Cancelled: "fail",
      Paused: "paused", Extracting: "active", Importing: "active",
    };
    return m[s] ?? "";
  }

  function speedStr(bytesPerSec: number): string {
    if (!bytesPerSec || bytesPerSec < 1024) return "0 KB/s";
    if (bytesPerSec < 1048576) return (bytesPerSec / 1024).toFixed(1) + " KB/s";
    return (bytesPerSec / 1048576).toFixed(1) + " MB/s";
  }

  function animeStatusLabel(s: string): string {
    const m: Record<string,string> = {
      Pending: "等待中", Parsing: "解析中", Downloading: "下载中",
      Merging: "合并中", Completed: "已完成", Failed: "失败",
      Paused: "已暂停", Cancelled: "已取消",
    };
    return m[s] ?? s;
  }

  function animeStatusClass(s: string): string {
    const m: Record<string,string> = {
      Downloading: "active", Parsing: "active", Merging: "active",
      Completed: "done", Failed: "fail", Paused: "paused",
    };
    return m[s] ?? "";
  }

  function etaStr(task: DownloadTask): string {
    if (!task.speed || !task.total_size || task.progress >= 1) return "";
    const remaining = task.total_size - task.downloaded_size;
    const sec = Math.round(remaining / task.speed);
    if (sec < 60) return `${sec}s`;
    if (sec < 3600) return `${Math.floor(sec / 60)}m`;
    return `${(sec / 3600).toFixed(1)}h`;
  }

  const activeCount = $derived(generalRows.filter(row => row.job?.status === "running" || (!row.job && row.task?.status === "Downloading")).length);
  const doneCount = $derived(generalRows.filter(row => row.job?.status === "succeeded" || (!row.job && row.task?.status === "Completed")).length);
  const animeActiveCount = $derived(animeDownloads.filter(d => d.status === "Downloading" || d.status === "Parsing" || d.status === "Merging").length);
  const animeDoneCount = $derived(animeDownloads.filter(d => d.status === "Completed").length);
  const tabs = $derived([
    { value: "general", label: "通用下载" },
    { value: "anime", label: animeDownloads.length > 0 ? `番剧下载 (${animeDownloads.length})` : "番剧下载" },
  ]);
</script>

<section class="page aura-page" data-aura-echo="DOWNLOADS">
  <Card class="page-head aura-head" padding="md">
    <div>
      <span class="aura-kicker">Transfer Queue</span>
      <h1 class="aura-title">资源下载</h1>
      <p>流式下载 · 暂停续传 · 自动解压导入</p>
    </div>
    <div class="head-actions">
      {#if activeCount > 0}
        <Tag variant="accent" size="sm">{activeCount} 下载中</Tag>
      {/if}
      {#if doneCount > 0}
        <Tag variant="neutral" size="sm">{doneCount} 已完成</Tag>
      {/if}
      {#if animeActiveCount > 0}
        <Tag variant="accent" size="sm">番剧 {animeActiveCount} 下载中</Tag>
      {/if}
      <Button variant="ghost" size="sm" press={clearGeneralFinished} title="清除已完成">
        <Icon name="trash" size={14} /> 清除
      </Button>
    </div>
  </Card>

  <SegmentControl options={tabs} value={activeTab} onChange={(v) => activeTab = v as "general" | "anime"} size="sm" />

  {#if activeTab === "general"}
  <Card class="toolbar" padding="sm">
    <div class="search-box" bind:this={urlBox}>
      <Icon name="download" size={16} />
      <Input bind:value={url} placeholder="粘贴下载 URL 或磁力链接" onkeydown={(e) => e.key === "Enter" && start()} class="url-input" ariaLabel="下载链接" />
    </div>
    <Input bind:value={filename} placeholder="文件名（可选）" class="fname-input" ariaLabel="文件名" />
    <Input bind:value={quotaGb} placeholder="配额 GB（可选）" class="quota-input" ariaLabel="下载目录配额（GB）" />
    <Button variant="primary" press={start} loading={loading} disabled={loading}>
      {loading ? "添加中..." : "添加下载"}
    </Button>
  </Card>
  {#if startError}<div class="task-error" role="alert">{startError}</div>{/if}
  {#if jobsError}<div class="legacy-note">后台任务控制面暂不可用，正在保留旧版下载列表回退。</div>{/if}

  <Card class="panel aura-panel" padding="none">
    {#if generalRows.length}
      <div class="downloads" role="list">
        {#each generalRows as row (row.id)}
          <article class="task {rowStatusClass(row)}" role="listitem" data-job-state={row.job?.status ?? row.task?.status}>
            <div class="task-head">
              <strong class="task-fname">{row.task?.filename ?? row.job?.title ?? "下载任务"}</strong>
              <div class="task-badges">
                {#if row.job?.recovered}<Tag variant="muted" size="sm">重启恢复</Tag>{/if}
                <Tag variant="neutral" size="sm" class="status-badge {rowStatusClass(row)}">{rowStatus(row)}</Tag>
              </div>
            </div>

            <div class="bar-wrap">
              <div class="bar aura-track" style="--p:{rowProgress(row)}"></div>
            </div>

            <div class="task-meta">
              <span class="size-info">
                {formatFileSize(row.task?.downloaded_size ?? 0)} / {formatFileSize(row.task?.total_size ?? 0)}
                <span class="pct aura-num">({Math.round(rowProgress(row) * 100)}%)</span>
              </span>
              <span class="speed-info aura-num">
                {#if row.job?.status === "running" || (!row.job && row.task?.status === "Downloading")}
                  <Icon name="download" size={12} /> {speedStr(row.task?.speed ?? 0)}
                  {#if row.task && etaStr(row.task)}
                    <span class="eta aura-num">剩余 {etaStr(row.task)}</span>
                  {/if}
                {/if}
              </span>
            </div>

            {#if row.job?.message}
              <div class="task-message">{row.job.message}</div>
            {/if}
            {#if row.task?.preflight}
              <div class:fail={!row.task.preflight.accepted} class="task-evidence">
                {#if row.task.preflight.accepted}
                  空间预检通过
                  {#if row.task.preflight.requiredBytes != null} · 需 {formatFileSize(row.task.preflight.requiredBytes)}{/if}
                  {#if row.task.preflight.availableBytes != null} · 可用 {formatFileSize(row.task.preflight.availableBytes)}{/if}
                  {#if row.task.preflight.quotaBytes != null} · 配额 {formatFileSize(row.task.preflight.quotaBytes)}{/if}
                {:else}
                  {row.task.preflight.reason ?? "磁盘空间或配额预检失败"}
                {/if}
              </div>
            {/if}
            {#if row.task?.error}
              <div class="task-error">{row.task.error}</div>
            {/if}

            <div class="task-actions">
              {#if row.job?.status === "running" || (!row.job && row.task?.status === "Downloading")}
                <Button variant="ghost" size="sm" press={() => act(row, "pause")}><Icon name="chevronDown" size={14} /> 暂停</Button>
              {/if}
              {#if row.job?.status === "paused" || (!row.job && row.task?.status === "Paused")}
                <Button variant="ghost" size="sm" press={() => act(row, "resume")}><Icon name="play" size={14} /> {row.job?.recovered ? "继续恢复" : "继续"}</Button>
              {/if}
              {#if row.job?.status === "failed" || (row.job?.status === "paused" && row.job.retryable) || (!row.job && row.task?.status === "Failed")}
                <Button variant="ghost" size="sm" press={() => act(row, "retry")}><Icon name="refresh" size={14} /> 重试</Button>
              {/if}
              {#if row.job?.status === "queued" || row.job?.status === "running" || row.job?.status === "paused" || (!row.job && row.task?.status === "Downloading")}
                <Button variant="ghost" size="sm" class="danger" press={() => act(row, "cancel")}><Icon name="x" size={14} /> 取消</Button>
              {/if}
              {#if row.job?.status === "paused" || row.job?.status === "failed" || row.job?.status === "succeeded" || row.job?.status === "cancelled" || (!row.job && row.task?.status !== "Downloading")}
                <Button variant="ghost" size="sm" class="danger" press={() => removeRow(row)}><Icon name="trash" size={14} /> 移除</Button>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {:else}
      <EmptyState
        title="暂无下载任务"
        description="粘贴资源链接开始下载。支持持久恢复、断点续传、磁盘空间与配额预检。"
        action={{ label: "添加资源链接", onclick: focusUrlInput }}
      />
    {/if}
  </Card>
  {:else}
  <!-- 番剧下载 Tab -->
  <Card class="panel aura-panel" padding="none">
    {#if animeDownloads.length}
      <div class="downloads" role="list">
        {#each animeDownloads as task}
          <article class="task {animeStatusClass(task.status)}" role="listitem">
            <div class="task-head">
              <div class="task-info">
                <strong class="task-fname">{task.episode_name || task.filename}</strong>
                {#if task.anime_name}
                  <span class="task-anime-name">{task.anime_name}</span>
                {/if}
              </div>
              <div class="task-badges">
                {#if task.is_m3u8}
                  <Tag variant="muted" size="sm" class="m3u8">HLS</Tag>
                {/if}
                <Tag variant="neutral" size="sm" class="status-badge {animeStatusClass(task.status)}">{animeStatusLabel(task.status)}</Tag>
              </div>
            </div>

            <div class="bar-wrap">
              <div class="bar aura-track" style="--p:{Math.min(1, Math.max(0, task.progress || 0))}"></div>
            </div>

            <div class="task-meta">
              <span class="size-info">
                {#if task.is_m3u8 && task.total_segments > 0}
                  分片 {task.downloaded_segments}/{task.total_segments}
                {:else}
                  {formatFileSize(task.downloaded_size)} / {formatFileSize(task.total_size || 0)}
                {/if}
                <span class="pct aura-num">({Math.round(task.progress * 100)}%)</span>
              </span>
              <span class="speed-info aura-num">
                {#if task.status === "Downloading"}
                  <Icon name="download" size={12} /> {speedStr(task.speed)}
                {/if}
                {#if task.status === "Merging"}
                  <Icon name="download" size={12} /> 合并分片中...
                {/if}
              </span>
            </div>

            {#if task.error}
              <div class="task-error">{task.error}</div>
            {/if}

            <div class="task-actions">
              {#if task.status === "Downloading" || task.status === "Parsing"}
                <Button variant="ghost" size="sm" press={() => animePauseDownload(task.id).then(refresh)}><Icon name="chevronDown" size={14} /> 暂停</Button>
              {/if}
              {#if task.status === "Paused"}
                <Button variant="ghost" size="sm" press={() => animeResumeDownload(task.id).then(refresh)}><Icon name="play" size={14} /> 继续</Button>
              {/if}
              {#if task.status === "Completed"}
                <Button variant="ghost" size="sm" press={() => animeOpenDownloadFolder(task.id)}><Icon name="externalLink" size={14} /> 打开目录</Button>
              {/if}
              {#if task.status !== "Downloading" && task.status !== "Parsing" && task.status !== "Merging"}
                <Button variant="ghost" size="sm" class="danger" press={() => animeRemoveDownload(task.id).then(refresh)}><Icon name="trash" size={14} /> 移除</Button>
              {/if}
              {#if task.status === "Downloading" || task.status === "Parsing" || task.status === "Paused"}
                <Button variant="ghost" size="sm" class="danger" press={() => animeCancelDownload(task.id).then(refresh)}><Icon name="x" size={14} /> 取消</Button>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {:else}
      <EmptyState
        title="暂无番剧下载"
        description="在播放器中点击「下载」按钮即可下载当前剧集。支持 m3u8/HLS 分片下载。"
      />
    {/if}
  </Card>
  {/if}
</section>

<style>
  .page {
    position: relative;
    isolation: isolate;
    min-width: 0;
    padding: 24px;
    overflow-y: auto;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 18px;
    --aura-track: rgba(255, 255, 255, 0.08);
    background: var(--bg-void);
    color: var(--text-primary);
  }

  :global(.page-head.aura-head) {
    min-width: 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 18px 20px;
  }
  :global(.page-head.aura-head) > div:first-child {
    min-width: 0;
    display: grid;
    gap: 4px;
  }
  :global(.page-head.aura-head) p {
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-top: 2px;
  }
  h1 { font-size: 1.5rem; font-weight: 700; color: var(--text-primary); }
  .head-actions { min-width: 0; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }

  :global(.toolbar) {
    min-width: 0;
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 12px;
  }
  .search-box {
    position: relative;
    min-width: 0; flex: 1; display: flex; align-items: center; gap: 8px;
    color: var(--text-muted);
  }
  .search-box > :global(.icon) {
    position: absolute;
    left: 14px;
    pointer-events: none;
  }
  :global(.ui-input.url-input) { padding-left: 36px; }
  :global(.ui-input.fname-input) { width: 180px; }
  :global(.ui-input.quota-input) { width: 150px; }
  .legacy-note,
  .task-message,
  .task-evidence {
    font-size: 0.75rem;
    color: var(--text-secondary);
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background: rgba(255,255,255,0.045);
  }
  .legacy-note { color: var(--color-warning); }
  .task-evidence.fail { color: var(--color-error); background: rgba(239,68,68,0.08); }

  :global(.panel.aura-panel) {
    flex: 1;
    overflow-y: auto;
    padding: 0;
  }
  .downloads { display: flex; flex-direction: column; }

  .task {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: border-color 0.2s;
  }
  .task.done { border-color: rgba(34,197,94,0.25); }
  .task.fail { border-color: rgba(239,68,68,0.25); }

  .task-head { min-width: 0; display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .task-fname { min-width: 0; font-size: 0.9rem; font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  :global(.ui-tag.status-badge.active) { color: var(--accent); background: var(--accent-lo); border-color: transparent; }
  :global(.ui-tag.status-badge.done) { color: var(--color-success); background: rgba(34,197,94,0.12); border-color: transparent; }
  :global(.ui-tag.status-badge.fail) { color: var(--color-error); background: rgba(239,68,68,0.12); border-color: transparent; }
  :global(.ui-tag.status-badge.paused) { color: var(--color-warning); background: rgba(245,158,11,0.12); border-color: transparent; }

  .task-info { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .task-anime-name { font-size: 0.75rem; color: var(--text-muted); }
  .task-badges { display: flex; align-items: center; gap: 6px; }
  :global(.ui-tag.m3u8) {
    background: rgba(99,102,241,0.15);
    color: #818cf8;
    border-color: transparent;
  }

  .bar-wrap { height: 6px; border-radius: 3px; background: var(--bg-hover); overflow: hidden; }
  .bar { height: 100%; border-radius: 3px; background: var(--accent); transition: width 0.4s ease; }

  .task-meta { min-width: 0; display: flex; justify-content: space-between; gap: 10px; font-size: 0.75rem; }
  .size-info { min-width: 0; color: var(--text-secondary); display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .pct { font-family: var(--font-mono); color: var(--accent); font-weight: 600; }
  .speed-info { min-width: 0; display: flex; align-items: center; gap: 8px; color: var(--text-muted); flex-wrap: wrap; justify-content: flex-end; }
  .eta { color: var(--text-muted); font-size: 0.7rem; }

  .task-error { font-size: 0.75rem; color: var(--color-error); padding: 6px 10px; border-radius: var(--radius-sm); background: rgba(239,68,68,0.08); }

  .task-actions { display: flex; gap: 6px; flex-wrap: wrap; }
  :global(.ui-button.danger:hover) { border-color: var(--color-error); color: var(--color-error); }

  @media (max-width: 700px) {
    .page {
      padding: 18px;
    }

    :global(.page-head.aura-head),
    :global(.toolbar) {
      align-items: stretch;
      flex-direction: column;
    }

    .head-actions {
      justify-content: flex-start;
    }

    .search-box,
    :global(.ui-input.fname-input),
    .toolbar :global(.ui-button) {
      width: 100%;
    }

    .toolbar :global(.ui-button) {
      justify-content: center;
    }

    .task-head,
    .task-meta {
      align-items: flex-start;
      flex-direction: column;
    }

    .speed-info {
      justify-content: flex-start;
    }
  }

  .aura-kicker {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.2;
  }

  .aura-title {
    margin: 0;
  }
  :global(.page-head.aura-head) p {
    margin: 0;
  }

  .aura-title {
    font-size: clamp(24px, 2.2vw, 32px);
    font-weight: 760;
    line-height: 1.12;
  }

  .aura-page .task {
    border: 0;
    border-bottom: 1px solid var(--aura-border);
    border-radius: 0;
    padding: 16px 18px;
    background: transparent;
    box-shadow: none;
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    transition: background 0.16s ease, border-color 0.16s ease;
  }

  .aura-page .task:last-child {
    border-bottom: 0;
  }

  .aura-page .task:hover {
    background: rgba(255, 255, 255, 0.045);
  }

  .aura-page .task.done {
    border-bottom-color: rgba(74, 222, 128, 0.28);
  }

  .aura-page .task.fail {
    border-bottom-color: rgba(248, 113, 113, 0.28);
  }

  .bar-wrap {
    background: var(--aura-track);
  }

  .bar {
    width: 100%;
    transform: scaleX(var(--p, 0));
    transform-origin: left center;
    transition: transform 0.4s cubic-bezier(0.22, 1, 0.36, 1);
    will-change: transform;
  }

  .speed-info,
  .eta,
  .pct {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }
</style>
