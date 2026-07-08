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
  import { Button, Card, EmptyState, Input, SegmentControl, Tag } from "./ui";

  let url = $state("");
  let filename = $state("");
  let downloads = $state<DownloadTask[]>([]);
  let animeDownloads = $state<AnimeDownloadTask[]>([]);
  let activeTab = $state<"general" | "anime">("general");
  let loading = $state(false);
  let urlBox = $state<HTMLDivElement>();

  async function refresh() {
    downloads = await getDownloads();
    try { animeDownloads = await animeGetDownloads(); } catch { animeDownloads = []; }
  }

  async function start() {
    if (!url.trim()) return;
    loading = true;
    try {
      await downloadStart(url, filename || url.split("/").pop() || "download.bin");
      url = "";
      filename = "";
      await refresh();
    } finally { loading = false; }
  }

  function focusUrlInput() {
    urlBox?.querySelector<HTMLInputElement>("input")?.focus();
  }

  onMount(() => {
    refresh();
    const id = window.setInterval(refresh, 1200);
    return () => window.clearInterval(id);
  });

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
      Downloading: "active", Completed: "done", Failed: "fail",
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

  const activeCount = $derived(downloads.filter(d => d.status === "Downloading").length);
  const doneCount = $derived(downloads.filter(d => d.status === "Completed").length);
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
      <Button variant="ghost" size="sm" press={() => downloadClearFinished().then(refresh)} title="清除已完成">
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
    <Button variant="primary" press={start} loading={loading} disabled={loading}>
      {loading ? "添加中..." : "添加下载"}
    </Button>
  </Card>

  <Card class="panel aura-panel" padding="none">
    {#if downloads.length}
      <div class="downloads" role="list">
        {#each downloads as task}
          <article class="task {statusClass(task.status)}" role="listitem">
            <div class="task-head">
              <strong class="task-fname">{task.filename}</strong>
              <Tag variant="neutral" size="sm" class="status-badge {statusClass(task.status)}">{statusLabel(task.status)}</Tag>
            </div>

            <div class="bar-wrap">
              <div class="bar aura-track" style="--p:{Math.min(1, Math.max(0, task.progress || 0))}"></div>
            </div>

            <div class="task-meta">
              <span class="size-info">
                {formatFileSize(task.downloaded_size)} / {formatFileSize(task.total_size || 0)}
                <span class="pct aura-num">({Math.round(task.progress * 100)}%)</span>
              </span>
              <span class="speed-info aura-num">
                {#if task.status === "Downloading"}
                  <Icon name="download" size={12} /> {speedStr(task.speed)}
                  {#if etaStr(task)}
                    <span class="eta aura-num">剩余 {etaStr(task)}</span>
                  {/if}
                {/if}
              </span>
            </div>

            {#if task.error}
              <div class="task-error">{task.error}</div>
            {/if}

            <div class="task-actions">
              {#if task.status === "Downloading"}
                <Button variant="ghost" size="sm" press={() => downloadPause(task.id)}><Icon name="chevronDown" size={14} /> 暂停</Button>
              {/if}
              {#if task.status === "Paused"}
                <Button variant="ghost" size="sm" press={() => downloadResume(task.id)}><Icon name="play" size={14} /> 继续</Button>
              {/if}
              {#if task.status === "Failed"}
                <Button variant="ghost" size="sm" press={() => downloadRetry(task.id)}><Icon name="refresh" size={14} /> 重试</Button>
              {/if}
              {#if task.status !== "Downloading"}
                <Button variant="ghost" size="sm" class="danger" press={() => downloadRemove(task.id).then(refresh)}><Icon name="trash" size={14} /> 移除</Button>
              {/if}
              {#if task.status === "Downloading"}
                <Button variant="ghost" size="sm" class="danger" press={() => downloadCancel(task.id)}><Icon name="x" size={14} /> 取消</Button>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {:else}
      <EmptyState
        title="暂无下载任务"
        description="粘贴资源链接开始下载。支持断点续传与限速。"
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
