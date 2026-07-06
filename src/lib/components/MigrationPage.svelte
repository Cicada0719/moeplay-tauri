<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { join, localDataDir } from "@tauri-apps/api/path";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { pickDirectory, migrateFromCsharp, verifyMigrationIds, openPath, type MigrationReport, type MigrationVerifyReport } from "../api";
  import Icon from "./Icon.svelte";
  import Button from "./ui/Button.svelte";

  type MigrationProgress = {
    stage: string;
    current: number;
    total: number;
    message: string;
  };

  let sourceDir = $state("");
  let migrating = $state(false);
  let report = $state<MigrationReport | null>(null);
  let verify = $state<MigrationVerifyReport | null>(null);
  let error = $state("");
  let autoDetected = $state(false);
  let progress = $state<MigrationProgress | null>(null);
  const progressPct = $derived(progress && progress.total > 0 ? Math.min(100, Math.round((progress.current / progress.total) * 100)) : 0);

  onMount(() => {
    localDataDir()
      .then((dir) => join(dir, "MoeGameSetup", "library"))
      .then((dir) => {
        if (!sourceDir) {
          sourceDir = dir;
          autoDetected = true;
        }
      })
      .catch(() => {});

    const unlisten = listen<MigrationProgress>("migration-progress", (event) => {
      progress = event.payload;
    });

    return () => {
      unlisten.then((off) => off()).catch(() => {});
    };
  });

  async function selectSource() {
    const dir = await pickDirectory().catch(() => "");
    if (dir) {
      sourceDir = dir;
      autoDetected = false;
    }
  }

  async function doMigrate() {
    if (!sourceDir) return;
    migrating = true;
    error = "";
    report = null;
    verify = null;
    progress = null;
    try {
      report = await migrateFromCsharp(sourceDir);
      if (report.total_found > 0) {
        verify = await verifyMigrationIds(report.total_found, report.source_ids);
        await gameStore.load();
      }
    } catch (e) {
      error = String(e);
    } finally {
      migrating = false;
      progress = null;
    }
  }

  function close() {
    uiStore.currentView = "home";
  }

  async function openBackup() {
    if (!report?.backup_dir) return;
    try {
      await openPath(report.backup_dir);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="overlay aura-page" data-aura-echo="MIGRATION" role="dialog" tabindex="-1" aria-label="数据迁移" onkeydown={(e) => { if (e.key === "Escape") close(); }}>
  <div class="dialog aura-panel aura-bevel">
    <header class="aura-head">
      <div>
        <p class="aura-kicker">Migration</p>
        <h2 class="aura-title"><Icon name="save" size={20} /> 从旧版萌游迁移数据</h2>
        <p>读取旧版游戏库、元数据、标签和存档信息，并导入到新库。</p>
      </div>
      <button class="close" onclick={close} aria-label="关闭"><Icon name="x" size={18} /></button>
    </header>

    <p class="desc">选择 C# 版萌游的数据目录（或导出的 JSON 文件），迁移完成后会刷新本机游戏库并生成校验结果。</p>

    {#if !sourceDir}
      <Button onclick={selectSource}>
        <Icon name="folder" size={16} /> 选择旧版数据目录
      </Button>
    {:else}
      <div class="path"><Icon name="folder" size={14} /> {sourceDir}</div>
      {#if autoDetected}
        <p class="hint">已自动填入本机 MoeGameSetup 旧版库目录。</p>
      {/if}
      <div class="actions">
        <Button variant="ghost" onclick={() => sourceDir = ""}>重新选择</Button>
        <Button onclick={doMigrate} disabled={migrating}>
          {migrating ? "迁移中..." : "开始迁移"}
        </Button>
      </div>
    {/if}

    {#if error}
      <div class="error-msg"><Icon name="x" size={14} /> {error}</div>
    {/if}

    <!-- Progress -->
    {#if migrating}
      <section class="migration-progress" aria-label="迁移进度">
        <div class="progress-meta">
          <span>{progress?.stage ?? "准备迁移"}</span>
          <strong class="aura-num">
            {#if progress}
              {progress.current}/{progress.total}
            {:else}
              --
            {/if}
          </strong>
        </div>
        <div class="progress-bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={progressPct}>
          <div class="fill" class:animate={!progress || progress.total === 0} style={`--p: ${progress ? progressPct / 100 : 1}`}></div>
        </div>
        <p class="progress-text">{progress?.message ?? "正在导入游戏、复制封面和映射元数据..."}</p>
      </section>
    {/if}

    <!-- Report -->
    {#if report}
      <div class="report aura-card">
        <h3>迁移完成</h3>
        <div class="stats">
          <span>来源：{report.source_label}</span>
          <span>发现游戏：<strong class="aura-num">{report.total_found}</strong></span>
          <span>导入：<strong class="aura-num">{report.imported}</strong></span>
          <span>更新：<strong class="aura-num">{report.updated}</strong></span>
          <span>跳过：<strong class="aura-num">{report.skipped}</strong></span>
          <span>封面已复制：<strong class="aura-num">{report.media_copied}</strong></span>
          <span>封面缺失：<strong class="aura-num">{report.media_missing}</strong></span>
          <span>耗时：<strong class="aura-num">{report.duration_secs.toFixed(1)}</strong> 秒</span>
        </div>
        {#if report.backup_dir}
          <div class="backup-row">
            <span>备份：{report.backup_dir}</span>
            <Button variant="ghost" onclick={openBackup}>打开备份</Button>
          </div>
        {/if}
        {#if report.errors.length > 0}
          <details class="errors">
            <summary>{report.errors.length} 个问题</summary>
            {#each report.errors as e}
              <p>{e}</p>
            {/each}
          </details>
        {/if}
      </div>
    {/if}

    <!-- Verify -->
    {#if verify}
      <div class="report aura-card">
        <h3>校验结果</h3>
        <div class="stats">
          <span>期望数量：<strong class="aura-num">{verify.expected_count}</strong></span>
          <span>库内总数：<strong class="aura-num">{verify.actual_count}</strong></span>
          <span>本次匹配：<strong class="aura-num">{verify.matched_count}</strong></span>
          <span>缺失：<strong class="aura-num">{verify.missing_count}</strong></span>
          <span class={verify.count_match ? "ok" : "warn"}>
            {verify.count_match ? "迁移完整" : "需要检查"}
          </span>
          <span>有封面：<strong class="aura-num">{verify.with_cover}</strong> (<strong class="aura-num">{verify.cover_rate.toFixed(1)}%</strong>)</span>
        </div>
        {#if verify.issues.length > 0}
          <details class="errors">
            <summary>{verify.issues.length} 条建议</summary>
            {#each verify.issues as i}
              <p>{i}</p>
            {/each}
          </details>
        {/if}
      </div>
    {/if}

    {#if report}
      <Button onclick={close}>完成</Button>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0; z-index: 180;
    display: flex; align-items: center; justify-content: center;
    background: rgba(0,0,0,0.55);
  }
  .dialog {
    width: 540px; max-width: 94vw; max-height: 85vh; overflow-y: auto;
    padding: 28px; display: flex; flex-direction: column; gap: 16px;
    position: relative;
    clip-path: polygon(0 0, calc(100% - 30px) 0, 100% 30px, 100% 100%, 0 100%);
  }
  .dialog::before {
    content: "";
    position: absolute;
    right: 0;
    top: 0;
    width: 30px;
    height: 30px;
    border-left: 1px solid var(--aura-border-strong);
    background: var(--accent-lo);
    pointer-events: none;
  }
  header { display: flex; justify-content: space-between; align-items: center; }
  .aura-head { align-items: flex-start; gap: 16px; }
  .aura-kicker {
    margin: 0 0 6px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--accent);
    text-transform: uppercase;
  }
  .aura-title { margin: 0; }
  .aura-head p { margin: 6px 0 0; color: var(--text-secondary); font-size: 0.82rem; line-height: 1.5; }
  h2 { font-size: 1.15rem; font-weight: 650; display: flex; align-items: center; gap: 8px; color: var(--text-primary); }
  .close { background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 4px; border-radius: var(--radius-sm); display: flex; }
  .close:hover { color: var(--text-primary); background: var(--bg-hover); }
  .desc { color: var(--text-secondary); font-size: 0.88rem; line-height: 1.6; }

  .path {
    padding: 10px 14px; border-radius: var(--radius-md); background: var(--bg-secondary);
    font-size: 0.82rem; color: var(--text-secondary); border: 1px solid var(--border);
    display: flex; align-items: center; gap: 8px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  .actions { display: flex; gap: 10px; justify-content: flex-end; }

  .error-msg {
    padding: 10px 14px; border-radius: var(--radius-md);
    background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.25);
    color: var(--color-error); font-size: 0.85rem; display: flex; align-items: center; gap: 6px;
  }

  .migration-progress {
    display: flex; flex-direction: column; gap: 9px;
    padding: 12px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--aura-inset);
  }
  .progress-meta { display: flex; align-items: center; justify-content: space-between; gap: 12px; color: var(--text-secondary); font-size: 0.8rem; }
  .progress-meta strong { color: var(--accent); }
  .progress-bar { height: 6px; border-radius: 3px; background: var(--aura-track); overflow: hidden; }
  .progress-bar .fill {
    width: 100%;
    height: 100%;
    border-radius: 3px;
    background: var(--accent);
    transform: scaleX(var(--p, 0));
    transform-origin: left center;
    transition: transform 0.24s ease;
    will-change: transform;
  }
  .progress-bar .fill.animate { animation: shimmer 1.5s ease-in-out infinite; }
  @keyframes shimmer {
    0% { opacity: 0.4; } 50% { opacity: 1; } 100% { opacity: 0.4; }
  }
  .progress-text { font-size: 0.82rem; color: var(--text-muted); text-align: left; }

  .report { padding: 16px; display: flex; flex-direction: column; gap: 10px; }
  .report h3 { font-size: 0.95rem; color: var(--accent); }
  .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; font-size: 0.82rem; color: var(--text-secondary); }
  .stats span { min-width: 0; overflow-wrap: anywhere; }
  .stats strong { color: var(--text-primary); font-weight: 650; }
  .stats .ok { color: var(--color-success); }
  .stats .warn { color: var(--color-warning); }
  .backup-row {
    display: flex; align-items: center; justify-content: space-between; gap: 10px;
    color: var(--text-muted); font-size: 0.78rem;
  }
  .backup-row span { min-width: 0; overflow-wrap: anywhere; }
  .errors { max-height: 120px; overflow-y: auto; margin-top: 4px; }
  .errors summary { cursor: pointer; font-size: 0.8rem; color: var(--text-muted); }
  .errors p { font-size: 0.78rem; color: var(--text-muted); padding: 2px 0; }
</style>
