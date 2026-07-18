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
    animeOpenDownloadFolder,
    type AnimeDownloadTask,
  } from "../api";
  import {
    createJobsStore,
    persistentDownloadsApi,
    type Job,
  } from "../features/jobs";
  import { i18n } from "../stores/i18n.svelte";
  import { Button, Card, Input, SegmentControl, Tag } from "./ui";
  import { PageShell, PageHeader, FilterBar, AsyncState, type ViewState } from "./ui-v2";

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
  let initialLoading = $state(true);

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
    await Promise.allSettled([downloadClearFinished(), jobsStore.clearFinished()]);
    await refresh();
  }

  function focusUrlInput() {
    urlBox?.querySelector<HTMLInputElement>("input")?.focus();
  }

  onMount(() => {
    void (async () => {
      await refresh();
      initialLoading = false;
    })();
    const id = window.setInterval(refresh, 1200);
    return () => {
      window.clearInterval(id);
      unsubscribeJobs();
    };
  });

  const jobStatusLabels = $derived<Record<Job["status"], string>>({
    queued: i18n.t("download.status.pending"),
    running: i18n.t("download.status.downloading"),
    paused: i18n.t("download.status.paused"),
    succeeded: i18n.t("download.status.completed"),
    failed: i18n.t("download.status.failed"),
    cancelled: i18n.t("download.status.cancelled"),
  });

  function rowStatus(row: DownloadRow): string {
    if (row.job) {
      if (row.job.recovered) return i18n.t("downloads.status_recovered");
      return jobStatusLabels[row.job.status];
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
    const m: Record<string, string> = {
      Pending: i18n.t("download.status.pending"),
      Downloading: i18n.t("download.status.downloading"),
      Paused: i18n.t("download.status.paused"),
      Completed: i18n.t("download.status.completed"),
      Failed: i18n.t("download.status.failed"),
      Extracting: i18n.t("download.status.extracting"),
      Importing: i18n.t("download.status.importing"),
      Cancelled: i18n.t("download.status.cancelled"),
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
    const m: Record<string, string> = {
      Pending: i18n.t("download.status.pending"),
      Parsing: i18n.t("download.status.parsing"),
      Downloading: i18n.t("download.status.downloading"),
      Merging: i18n.t("download.status.merging"),
      Completed: i18n.t("download.status.completed"),
      Failed: i18n.t("download.status.failed"),
      Paused: i18n.t("download.status.paused"),
      Cancelled: i18n.t("download.status.cancelled"),
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
  const tabs = $derived([
    { value: "general", label: i18n.t("downloads.tab_general") },
    { value: "anime", label: animeDownloads.length > 0 ? i18n.t("downloads.tab_anime_count", { count: animeDownloads.length }) : i18n.t("downloads.tab_anime") },
  ]);

  // 三态统一：首次加载 / 空列表 / 就绪 收敛到 AsyncState。
  const generalViewState = $derived<ViewState>(
    initialLoading && generalRows.length === 0 ? "loading" : generalRows.length ? "ready" : "empty",
  );
  const animeViewState = $derived<ViewState>(
    initialLoading && animeDownloads.length === 0 ? "loading" : animeDownloads.length ? "ready" : "empty",
  );
</script>

<PageShell as="div" width="full" scrollable={false} class="downloads-v2-shell" labelledBy="downloads-page-title" ariaLabel={i18n.t("downloads.title")}>
  <div class="dl">
    <div class="v2-grain dl-grain" aria-hidden="true"></div>

    <PageHeader
      id="downloads-page-title"
      class="dl-header"
      eyebrow="ダウンロード / DOWNLOADS"
      title={i18n.t("downloads.title")}
      description={i18n.t("downloads.subtitle")}
    >
      {#snippet actions()}
        <div class="dl-head-actions">
          {#if activeCount > 0}
            <Tag variant="accent" size="sm">{i18n.t("downloads.tag_active", { count: activeCount })}</Tag>
          {/if}
          {#if doneCount > 0}
            <Tag variant="neutral" size="sm">{i18n.t("downloads.tag_done", { count: doneCount })}</Tag>
          {/if}
          {#if animeActiveCount > 0}
            <Tag variant="accent" size="sm">{i18n.t("downloads.tag_anime_active", { count: animeActiveCount })}</Tag>
          {/if}
          <Button variant="ghost" size="sm" press={clearGeneralFinished} title={i18n.t("downloads.clear_finished_title")}>
            <Icon name="trash" size={14} /> {i18n.t("downloads.clear_finished")}
          </Button>
        </div>
      {/snippet}
    </PageHeader>

    <div class="dl-tabs">
      <FilterBar label={i18n.t("downloads.tabs_aria")}>
        <SegmentControl options={tabs} value={activeTab} onChange={(v) => activeTab = v as "general" | "anime"} size="sm" />
      </FilterBar>
    </div>

    <main class="dl-content">
      {#if activeTab === "general"}
        <FilterBar label={i18n.t("downloads.addbar_aria")} class="dl-addbar">
          <div class="search-box" bind:this={urlBox}>
            <Icon name="download" size={16} />
            <Input bind:value={url} placeholder={i18n.t("download.url_placeholder")} onkeydown={(e) => e.key === "Enter" && start()} class="url-input" ariaLabel={i18n.t("downloads.url_aria")} />
          </div>
          <Input bind:value={filename} placeholder={i18n.t("download.filename_placeholder")} class="fname-input" ariaLabel={i18n.t("downloads.filename_aria")} />
          <Input bind:value={quotaGb} placeholder={i18n.t("downloads.quota_placeholder")} class="quota-input" ariaLabel={i18n.t("downloads.quota_aria")} />
          {#snippet actions()}
            <Button variant="primary" press={start} loading={loading} disabled={loading}>
              {loading ? i18n.t("downloads.adding") : i18n.t("download.add")}
            </Button>
          {/snippet}
        </FilterBar>
        {#if startError}<div class="task-error" role="alert">{startError}</div>{/if}
        {#if jobsError}<div class="legacy-note">{i18n.t("downloads.jobs_fallback")}</div>{/if}

        {#if generalViewState === "ready"}
          <Card class="panel dl-panel" padding="none">
            <div class="downloads" role="list">
              {#each generalRows as row (row.id)}
                <article class="task {rowStatusClass(row)}" role="listitem">
                  <div class="task-head">
                    <strong class="task-fname">{row.task?.filename ?? row.job?.title ?? i18n.t("downloads.task_default")}</strong>
                    <div class="task-badges">
                      {#if row.job?.recovered}<Tag variant="muted" size="sm">{i18n.t("downloads.badge_recovered")}</Tag>{/if}
                      <Tag variant="neutral" size="sm" class="status-badge {rowStatusClass(row)}">{rowStatus(row)}</Tag>
                    </div>
                  </div>

                  <div class="bar-wrap">
                    <div class="bar" style="--p:{rowProgress(row)}"></div>
                  </div>

                  <div class="task-meta">
                    <span class="size-info">
                      {formatFileSize(row.task?.downloaded_size ?? 0)} / {formatFileSize(row.task?.total_size ?? 0)}
                      <span class="pct">({Math.round(rowProgress(row) * 100)}%)</span>
                    </span>
                    <span class="speed-info">
                      {#if row.job?.pausable || (!row.job && row.task?.status === "Downloading")}
                        <Icon name="download" size={12} /> {speedStr(row.task?.speed ?? 0)}
                        {#if row.task && etaStr(row.task)}
                          <span class="eta">{i18n.t("downloads.eta_left", { eta: etaStr(row.task) })}</span>
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
                        {i18n.t("downloads.preflight_ok")}
                        {#if row.task.preflight.requiredBytes != null} · {i18n.t("downloads.preflight_required", { size: formatFileSize(row.task.preflight.requiredBytes) })}{/if}
                        {#if row.task.preflight.availableBytes != null} · {i18n.t("downloads.preflight_available", { size: formatFileSize(row.task.preflight.availableBytes) })}{/if}
                        {#if row.task.preflight.quotaBytes != null} · {i18n.t("downloads.preflight_quota", { size: formatFileSize(row.task.preflight.quotaBytes) })}{/if}
                      {:else}
                        {row.task.preflight.reason ?? i18n.t("downloads.preflight_fail")}
                      {/if}
                    </div>
                  {/if}
                  {#if row.task?.error}
                    <div class="task-error">{row.task.error}</div>
                  {/if}

                  <div class="task-actions">
                    {#if row.job?.status === "running" || (!row.job && row.task?.status === "Downloading")}
                      <Button variant="ghost" size="sm" press={() => act(row, "pause")}><Icon name="chevronDown" size={14} /> {i18n.t("downloads.action_pause")}</Button>
                    {/if}
                    {#if (row.job?.status === "paused" && row.job.resumable) || (!row.job && row.task?.status === "Paused")}
                      <Button variant="ghost" size="sm" press={() => act(row, "resume")}><Icon name="play" size={14} /> {row.job?.recovered ? i18n.t("downloads.action_resume_recovered") : i18n.t("downloads.action_resume")}</Button>
                    {/if}
                    {#if (row.job?.retryable && (row.job.status === "failed" || row.job.status === "paused")) || (!row.job && row.task?.status === "Failed")}
                      <Button variant="ghost" size="sm" press={() => act(row, "retry")}><Icon name="refresh" size={14} /> {i18n.t("button.retry")}</Button>
                    {/if}
                    {#if row.job?.cancellable || (!row.job && row.task?.status === "Downloading")}
                      <Button variant="ghost" size="sm" class="danger" press={() => act(row, "cancel")}><Icon name="x" size={14} /> {i18n.t("button.cancel")}</Button>
                    {/if}
                    {#if row.job?.status === "paused" || row.job?.status === "failed" || row.job?.status === "succeeded" || row.job?.status === "cancelled" || (!row.job && row.task?.status !== "Downloading")}
                      <Button variant="ghost" size="sm" class="danger" press={() => removeRow(row)}><Icon name="trash" size={14} /> {i18n.t("downloads.action_remove")}</Button>
                    {/if}
                  </div>
                </article>
              {/each}
            </div>
          </Card>
        {:else}
          <AsyncState
            state={generalViewState}
            title={generalViewState === "empty" ? i18n.t("empty.no_downloads") : undefined}
            description={generalViewState === "empty" ? i18n.t("downloads.empty_desc") : undefined}
            primaryAction={generalViewState === "empty" ? { label: i18n.t("downloads.empty_action"), onSelect: focusUrlInput } : undefined}
            loadingRows={4}
          />
        {/if}
      {:else}
        {#if animeViewState === "ready"}
          <Card class="panel dl-panel" padding="none">
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
                    <div class="bar" style="--p:{Math.min(1, Math.max(0, task.progress || 0))}"></div>
                  </div>

                  <div class="task-meta">
                    <span class="size-info">
                      {#if task.is_m3u8 && task.total_segments > 0}
                        {i18n.t("downloads.segments", { done: task.downloaded_segments, total: task.total_segments })}
                      {:else}
                        {formatFileSize(task.downloaded_size)} / {formatFileSize(task.total_size || 0)}
                      {/if}
                      <span class="pct">({Math.round(task.progress * 100)}%)</span>
                    </span>
                    <span class="speed-info">
                      {#if task.status === "Downloading"}
                        <Icon name="download" size={12} /> {speedStr(task.speed)}
                      {/if}
                      {#if task.status === "Merging"}
                        <Icon name="download" size={12} /> {i18n.t("downloads.merging")}
                      {/if}
                    </span>
                  </div>

                  {#if task.error}
                    <div class="task-error">{task.error}</div>
                  {/if}

                  <div class="task-actions">
                    {#if task.status === "Downloading" || task.status === "Parsing"}
                      <Button variant="ghost" size="sm" press={() => animePauseDownload(task.id).then(refresh)}><Icon name="chevronDown" size={14} /> {i18n.t("downloads.action_pause")}</Button>
                    {/if}
                    {#if task.status === "Paused"}
                      <Button variant="ghost" size="sm" press={() => animeResumeDownload(task.id).then(refresh)}><Icon name="play" size={14} /> {i18n.t("downloads.action_resume")}</Button>
                    {/if}
                    {#if task.status === "Completed"}
                      <Button variant="ghost" size="sm" press={() => animeOpenDownloadFolder(task.id)}><Icon name="externalLink" size={14} /> {i18n.t("downloads.action_open_folder")}</Button>
                    {/if}
                    {#if task.status !== "Downloading" && task.status !== "Parsing" && task.status !== "Merging"}
                      <Button variant="ghost" size="sm" class="danger" press={() => animeRemoveDownload(task.id).then(refresh)}><Icon name="trash" size={14} /> {i18n.t("downloads.action_remove")}</Button>
                    {/if}
                    {#if task.status === "Downloading" || task.status === "Parsing" || task.status === "Paused"}
                      <Button variant="ghost" size="sm" class="danger" press={() => animeCancelDownload(task.id).then(refresh)}><Icon name="x" size={14} /> {i18n.t("button.cancel")}</Button>
                    {/if}
                  </div>
                </article>
              {/each}
            </div>
          </Card>
        {:else}
          <AsyncState
            state={animeViewState}
            title={animeViewState === "empty" ? i18n.t("downloads.anime_empty_title") : undefined}
            description={animeViewState === "empty" ? i18n.t("downloads.anime_empty_desc") : undefined}
            loadingRows={4}
          />
        {/if}
      {/if}
    </main>
  </div>
</PageShell>

<style>
  :global(.downloads-v2-shell) { height: 100%; }
  :global(.downloads-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .dl {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .dl-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.dl-header) {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto;
    padding: 26px 28px 14px;
    flex-shrink: 0;
  }
  .dl-head-actions { min-width: 0; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }

  .dl-tabs {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto 14px;
    padding: 0 28px;
  }
  .dl-content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto;
    padding: 0 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    scroll-behavior: smooth;
  }

  /* ── Add-download bar ── */
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

  :global(.ui-card.dl-panel) { padding: 0; overflow: hidden; }
  .downloads { display: flex; flex-direction: column; }

  .task {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-bottom: 1px solid var(--border);
    transition: background 0.16s ease, border-color 0.16s ease;
  }
  .task:last-child { border-bottom: 0; }
  .task:hover { background: rgba(255, 255, 255, 0.045); }
  .task.done { border-bottom-color: rgba(74, 222, 128, 0.28); }
  .task.fail { border-bottom-color: rgba(248, 113, 113, 0.28); }

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

  .bar-wrap { height: 6px; border-radius: 3px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .bar {
    width: 100%; height: 100%; border-radius: 3px; background: var(--accent);
    transform: scaleX(var(--p, 0));
    transform-origin: left center;
    transition: transform 0.4s cubic-bezier(0.22, 1, 0.36, 1);
    will-change: transform;
  }

  .task-meta { min-width: 0; display: flex; justify-content: space-between; gap: 10px; font-size: 0.75rem; }
  .size-info { min-width: 0; color: var(--text-secondary); display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .pct { color: var(--accent); font-weight: 600; }
  .speed-info { min-width: 0; display: flex; align-items: center; gap: 8px; color: var(--text-muted); flex-wrap: wrap; justify-content: flex-end; }
  .eta { color: var(--text-muted); font-size: 0.7rem; }
  .speed-info, .eta, .pct { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }

  .task-error { font-size: 0.75rem; color: var(--color-error); padding: 6px 10px; border-radius: var(--radius-sm); background: rgba(239,68,68,0.08); }

  .task-actions { display: flex; gap: 6px; flex-wrap: wrap; }
  :global(.ui-button.danger:hover) { border-color: var(--color-error); color: var(--color-error); }

  /* ── Responsive ── */
  @media (max-width: 700px) {
    .dl-content { padding: 0 16px 36px; }
    .dl-tabs { padding: 0 16px; }
    :global(.dl-header) { padding: 20px 16px 12px; }

    .dl-head-actions { justify-content: flex-start; }

    .search-box,
    :global(.ui-input.fname-input),
    :global(.ui-input.quota-input) {
      width: 100%;
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

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .bar, .task { transition: none; }
    .dl-content { scroll-behavior: auto; }
  }
  :global([data-motion="reduce"]) .bar,
  :global([data-motion="reduce"]) .task { transition: none; }
  :global([data-motion="reduce"]) .dl-content { scroll-behavior: auto; }
</style>
