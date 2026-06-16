<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import EmptyState from "./EmptyState.svelte";
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

  let url = $state("");
  let filename = $state("");
  let downloads = $state<DownloadTask[]>([]);
  let animeDownloads = $state<AnimeDownloadTask[]>([]);
  let activeTab = $state<"general" | "anime">("general");
  let loading = $state(false);
  let urlInput = $state<HTMLInputElement>();

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
    urlInput?.focus();
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
</script>

<section class="page aura-page" data-aura-echo="DOWNLOADS">
  <header class="page-head aura-head">
    <div>
      <span class="aura-kicker">Transfer Queue</span>
      <h1 class="aura-title">资源下载</h1>
      <p>流式下载 · 暂停续传 · 自动解压导入</p>
    </div>
    <div class="head-actions">
      {#if activeCount > 0}
        <span class="pill active">{activeCount} 下载中</span>
      {/if}
      {#if doneCount > 0}
        <span class="pill done">{doneCount} 已完成</span>
      {/if}
      {#if animeActiveCount > 0}
        <span class="pill active">番剧 {animeActiveCount} 下载中</span>
      {/if}
      <button class="ghost" onclick={() => downloadClearFinished().then(refresh)} title="清除已完成">
        <Icon name="trash" size={14} /> 清除
      </button>
    </div>
  </header>

  <div class="tabs">
    <button class="tab-btn" class:active={activeTab === "general"} onclick={() => activeTab = "general"}>
      <Icon name="download" size={14} /> 通用下载
    </button>
    <button class="tab-btn" class:active={activeTab === "anime"} onclick={() => activeTab = "anime"}>
      <span class="tab-icon">▶</span> 番剧下载
      {#if animeDownloads.length > 0}
        <span class="tab-badge">{animeDownloads.length}</span>
      {/if}
    </button>
  </div>

  {#if activeTab === "general"}
  <div class="toolbar">
    <div class="search-box">
      <Icon name="download" size={16} />
      <input bind:this={urlInput} bind:value={url} placeholder="粘贴下载 URL 或磁力链接" onkeydown={(e) => e.key === "Enter" && start()} />
    </div>
    <input bind:value={filename} placeholder="文件名（可选）" class="fname-input" />
    <button class="primary" onclick={start} disabled={loading}>
      {loading ? "添加中..." : "添加下载"}
    </button>
  </div>

  <section class="panel aura-panel">
    {#if downloads.length}
      <div class="downloads">
        {#each downloads as task}
          <article class="task" class:done={task.status === "Completed"} class:fail={task.status === "Failed"}>
            <div class="task-head">
              <strong class="task-fname">{task.filename}</strong>
              <span class="status-badge {statusClass(task.status)}">{statusLabel(task.status)}</span>
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
                <button class="act" onclick={() => downloadPause(task.id)}><Icon name="chevronDown" size={14} /> 暂停</button>
              {/if}
              {#if task.status === "Paused"}
                <button class="act" onclick={() => downloadResume(task.id)}><Icon name="play" size={14} /> 继续</button>
              {/if}
              {#if task.status === "Failed"}
                <button class="act" onclick={() => downloadRetry(task.id)}><Icon name="refresh" size={14} /> 重试</button>
              {/if}
              {#if task.status !== "Downloading"}
                <button class="act danger" onclick={() => downloadRemove(task.id).then(refresh)}><Icon name="trash" size={14} /> 移除</button>
              {/if}
              {#if task.status === "Downloading"}
                <button class="act danger" onclick={() => downloadCancel(task.id)}><Icon name="x" size={14} /> 取消</button>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {:else}
      <EmptyState
        title="暂无下载任务"
        description="粘贴资源链接开始下载。支持断点续传与限速。"
        actionLabel="添加资源链接"
        onAction={focusUrlInput}
      />
    {/if}
  </section>
  {:else}
  <!-- 番剧下载 Tab -->
  <div class="panel aura-panel">
    {#if animeDownloads.length}
      <div class="downloads">
        {#each animeDownloads as task}
          <article class="task" class:done={task.status === "Completed"} class:fail={task.status === "Failed"}>
            <div class="task-head">
              <div class="task-info">
                <strong class="task-fname">{task.episode_name || task.filename}</strong>
                {#if task.anime_name}
                  <span class="task-anime-name">{task.anime_name}</span>
                {/if}
              </div>
              <div class="task-badges">
                {#if task.is_m3u8}
                  <span class="badge m3u8">HLS</span>
                {/if}
                <span class="status-badge {animeStatusClass(task.status)}">{animeStatusLabel(task.status)}</span>
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
                <button class="act" onclick={() => animePauseDownload(task.id).then(refresh)}><Icon name="chevronDown" size={14} /> 暂停</button>
              {/if}
              {#if task.status === "Paused"}
                <button class="act" onclick={() => animeResumeDownload(task.id).then(refresh)}><Icon name="play" size={14} /> 继续</button>
              {/if}
              {#if task.status === "Completed"}
                <button class="act" onclick={() => animeOpenDownloadFolder(task.id)}><Icon name="externalLink" size={14} /> 打开目录</button>
              {/if}
              {#if task.status !== "Downloading" && task.status !== "Parsing" && task.status !== "Merging"}
                <button class="act danger" onclick={() => animeRemoveDownload(task.id).then(refresh)}><Icon name="trash" size={14} /> 移除</button>
              {/if}
              {#if task.status === "Downloading" || task.status === "Parsing" || task.status === "Paused"}
                <button class="act danger" onclick={() => animeCancelDownload(task.id).then(refresh)}><Icon name="x" size={14} /> 取消</button>
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
  </div>
  {/if}
</section>

<style>
  .page { min-width: 0; padding: 24px; overflow-y: auto; height: 100%; display: flex; flex-direction: column; gap: 18px; }
  .page-head { min-width: 0; display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  h1 { font-size: 1.5rem; font-weight: 700; color: var(--text-primary); }
  .page-head p { color: var(--text-secondary); font-size: 0.85rem; margin-top: 2px; }
  .head-actions { min-width: 0; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .pill { padding: 4px 12px; border-radius: var(--radius-full); font-size: 0.75rem; font-weight: 600; }
  .pill.active { background: var(--accent-lo); color: var(--accent); }
  .pill.done { background: rgba(34,197,94,0.12); color: var(--color-success); }
  .ghost {
    display: inline-flex; align-items: center; gap: 4px; padding: 6px 12px;
    border: 1px solid var(--border); border-radius: var(--radius-full);
    background: transparent; color: var(--text-secondary); cursor: pointer; font-size: 0.8rem;
  }
  .ghost:hover { border-color: var(--accent); color: var(--text-primary); }

  .tabs {
    display: flex;
    gap: 4px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px;
  }
  .tab-btn {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }
  .tab-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .tab-btn.active {
    background: var(--accent-lo);
    color: var(--accent);
    font-weight: 600;
  }
  .tab-icon { font-size: 11px; }
  .tab-badge {
    padding: 1px 7px;
    border-radius: 10px;
    font-size: 0.7rem;
    font-weight: 700;
    background: var(--accent);
    color: #fff;
  }

  .task-info { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .task-anime-name { font-size: 0.75rem; color: var(--text-muted); }
  .task-badges { display: flex; align-items: center; gap: 6px; }
  .badge {
    padding: 1px 8px;
    border-radius: var(--radius-full);
    font-size: 0.65rem;
    font-weight: 700;
    background: rgba(99,102,241,0.15);
    color: #818cf8;
  }

  .toolbar { min-width: 0; display: flex; gap: 10px; align-items: center; }
  .search-box {
    min-width: 0; flex: 1; display: flex; align-items: center; gap: 8px;
    background: var(--bg-card); border: 1px solid var(--border);
    border-radius: var(--radius-full); padding: 10px 18px; color: var(--text-muted);
  }
  .search-box:focus-within { border-color: var(--accent); }
  .search-box input {
    min-width: 0; flex: 1; border: none; background: transparent; color: var(--text-primary);
    font-size: 0.9rem; outline: none; font-family: var(--font-ui);
  }
  .fname-input {
    min-width: 0; width: 180px; padding: 10px 14px; border-radius: var(--radius-md);
    background: var(--bg-card); border: 1px solid var(--border); color: var(--text-primary);
    font-size: 0.85rem; font-family: var(--font-mono);
  }
  .fname-input:focus { outline: none; border-color: var(--accent); }
  .primary {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 10px 22px; border: none; border-radius: var(--radius-full);
    background: var(--accent); color: #fff; font-weight: 600; cursor: pointer;
    font-size: 0.9rem; white-space: nowrap; transition: opacity 0.2s;
  }
  .primary:hover:not(:disabled) { opacity: 0.85; }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }

  .panel { flex: 1; overflow-y: auto; }
  .downloads { display: flex; flex-direction: column; gap: 10px; }

  .task {
    padding: 16px 18px; display: flex; flex-direction: column; gap: 10px;
    transition: border-color 0.2s;
  }
  .task.done { border-color: rgba(34,197,94,0.25); }
  .task.fail { border-color: rgba(239,68,68,0.25); }

  .task-head { min-width: 0; display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .task-fname { min-width: 0; font-size: 0.9rem; font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .status-badge {
    padding: 2px 10px; border-radius: var(--radius-full); font-size: 0.7rem; font-weight: 600; white-space: nowrap;
    background: var(--bg-hover); color: var(--text-secondary);
  }
  .status-badge.active { background: var(--accent-lo); color: var(--accent); }
  .status-badge.done { background: rgba(34,197,94,0.12); color: var(--color-success); }
  .status-badge.fail { background: rgba(239,68,68,0.12); color: var(--color-error); }
  .status-badge.paused { background: rgba(245,158,11,0.12); color: var(--color-warning); }

  .bar-wrap { height: 6px; border-radius: 3px; background: var(--bg-hover); overflow: hidden; }
  .bar { height: 100%; border-radius: 3px; background: var(--accent); transition: width 0.4s ease; }

  .task-meta { min-width: 0; display: flex; justify-content: space-between; gap: 10px; font-size: 0.75rem; }
  .size-info { min-width: 0; color: var(--text-secondary); display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .pct { font-family: var(--font-mono); color: var(--accent); font-weight: 600; }
  .speed-info { min-width: 0; display: flex; align-items: center; gap: 8px; color: var(--text-muted); flex-wrap: wrap; justify-content: flex-end; }
  .eta { color: var(--text-muted); font-size: 0.7rem; }

  .task-error { font-size: 0.75rem; color: var(--color-error); padding: 6px 10px; border-radius: var(--radius-sm); background: rgba(239,68,68,0.08); }

  .task-actions { display: flex; gap: 6px; flex-wrap: wrap; }
  .act {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 5px 12px; border: 1px solid var(--border); border-radius: var(--radius-full);
    background: transparent; color: var(--text-secondary); cursor: pointer; font-size: 0.75rem;
    transition: all 0.15s;
  }
  .act:hover { border-color: var(--accent); color: var(--text-primary); }
  .act.danger:hover { border-color: var(--color-error); color: var(--color-error); }

  @media (max-width: 700px) {
    .page {
      padding: 18px;
    }

    .page-head,
    .toolbar {
      align-items: stretch;
      flex-direction: column;
    }

    .head-actions {
      justify-content: flex-start;
    }

    .search-box,
    .fname-input,
    .primary {
      width: 100%;
    }

    .primary {
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

  .page {
    position: relative;
    isolation: isolate;
    min-width: 0;
    --aura-track: rgba(255, 255, 255, 0.08);
    background: var(--bg-void);
    color: var(--text-primary);
  }
  .page-head,
  .toolbar,
  .task,
  .panel {
    border: 1px solid var(--border);
    background: var(--bg-card);
    box-shadow: var(--shadow-xs);
  }
  .page-head {
    padding: 18px 20px;
    border-radius: 8px;
  }
  .toolbar {
    padding: 12px;
    border-radius: 8px;
  }
  .panel {
    border-radius: 8px;
    padding: 10px;
  }
  .task {
    border-radius: 8px;
  }
  .search-box,
  .fname-input {
    border-radius: 8px;
    background: var(--bg-deep);
    border-color: var(--border);
  }

  .aura-head {
    align-items: center;
  }

  .aura-head > div:first-child {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .aura-kicker {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.2;
  }

  .aura-title,
  .aura-head p {
    margin: 0;
  }

  .aura-title {
    font-size: clamp(24px, 2.2vw, 32px);
    font-weight: 760;
    line-height: 1.12;
  }

  .panel {
    padding: 0;
    overflow: hidden;
  }

  .downloads {
    gap: 0;
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
