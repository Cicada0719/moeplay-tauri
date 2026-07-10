<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, StatBlock } from "./ui";
  import {
    exportDiagnosticsZip,
    getMigrationStatus,
    getPerformanceSnapshot,
    runDiagnostics,
    type DiagnosticsReport,
    type MigrationInfo,
    type PerformanceSnapshot,
  } from "../api";

  let report = $state<DiagnosticsReport | null>(null);
  let perf = $state<PerformanceSnapshot | null>(null);
  let migrations = $state<MigrationInfo[]>([]);
  let exported = $state("");
  let loading = $state(true);
  let exporting = $state(false);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      report = await runDiagnostics();
      perf = await getPerformanceSnapshot();
      migrations = await getMigrationStatus();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function exportBundle() {
    exporting = true;
    error = null;
    try {
      exported = await exportDiagnosticsZip();
    } catch (e) {
      error = String(e);
    } finally {
      exporting = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<section class="tool-page aura-page" data-aura-echo="DIAGNOSTICS">
  <header class="tool-head aura-head">
    <div class="head-copy">
      <span class="eyebrow aura-kicker">Diagnostics</span>
      <h1 class="aura-title">诊断</h1>
      <p>系统信息、迁移状态、性能快照和默认脱敏的诊断导出。</p>
    </div>
    <Button press={exportBundle} loading={exporting} disabled={exporting}>
      <Icon name="download" size={16} />
      <span>导出脱敏诊断包</span>
    </Button>
  </header>

  {#if report}
    <div class="stat-grid" aria-label="诊断概览">
      <StatBlock class="stat-tile" label="系统" value={report.system_info.os} />
      <StatBlock class="stat-tile" label="LE" value={report.system_info.locale_emulator_installed ? "已安装" : "未检测到"} />
      <StatBlock class="stat-tile" label="游戏数" value={perf?.game_count ?? 0} />
      <StatBlock class="stat-tile" label="缓存" value={Math.round((perf?.cache_size_bytes ?? 0) / 1024 / 1024)} unit="MB" />
    </div>

    <div class="content-grid">
      <Card class="panel aura-panel">
        <div class="panel-head">
          <h2>问题</h2>
          <span class="aura-num">{report.issues.length}</span>
        </div>
        {#if report.issues.length}
          <div class="row-list">
            {#each report.issues as issue}
              <article class="data-row">
                <strong class="status-cell aura-num"><span class="status-dot stopped"></span>{issue.severity}</strong>
                <span>{issue.message}</span>
              </article>
            {/each}
          </div>
        {:else}
          <EmptyState title="暂无诊断问题" />
        {/if}
      </Card>

      <Card class="panel aura-panel">
        <div class="panel-head">
          <h2>迁移</h2>
          <span class="aura-num">{migrations.length}</span>
        </div>
        {#if migrations.length}
          <div class="row-list">
            {#each migrations as migration}
              <article class="data-row">
                <strong class="status-cell aura-num"><span class="status-dot" class:running={migration.applied} class:stopped={!migration.applied}></span>v{migration.version}</strong>
                <span>{migration.applied ? "已应用" : "待应用"} · {migration.description}</span>
              </article>
            {/each}
          </div>
        {:else}
          <EmptyState title="暂无迁移记录" />
        {/if}
      </Card>
    </div>

    <Card class="panel aura-panel">
      <div class="panel-head">
        <h2>日志井</h2>
        <span class="aura-num">{new Date(perf?.timestamp ?? Date.now()).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}</span>
      </div>
      <div class="log-well aura-inset" aria-label="诊断日志井">
        <code>os={report.system_info.os} arch={report.system_info.arch} memory={report.system_info.memory_gb}GB</code>
        <code>db={report.app_info.database_size_mb}MB games={perf?.game_count ?? report.app_info.game_count} cache={Math.round((perf?.cache_size_bytes ?? 0) / 1024 / 1024)}MB</code>
        <code>scrapers={(report.app_info.scrape_sources ?? []).join(",") || "none"}</code>
        <code>export={exported || "not_run"}</code>
      </div>
    </Card>

    {#if exported}
      <p class="exported aura-inset">
        <Icon name="check" size={16} />
        <span>脱敏诊断包已导出：{exported}</span>
      </p>
    {/if}
  {:else if error}
    <Card class="panel aura-panel loading-panel">
      <EmptyState title="诊断加载失败" description={error ?? undefined} action={{ label: "重试", onclick: load }} />
    </Card>
  {:else}
    <Card class="panel aura-panel loading-panel">
      <EmptyState title="正在运行诊断" />
    </Card>
  {/if}
</section>

<style>
  .tool-page {
    min-width: 0;
    height: 100%;
    padding: 24px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    background: var(--bg-void, var(--bg-base));
  }

  .tool-head,
  .exported {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
    box-shadow: none;
  }

  .tool-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
  }

  .aura-head {
    align-items: center;
  }

  .head-copy {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .eyebrow,
  .panel-head span {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.2;
    letter-spacing: 0;
  }

  .aura-kicker {
    text-transform: none;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    color: var(--text-primary);
    font-size: 24px;
    font-weight: 750;
    line-height: 1.15;
    letter-spacing: 0;
  }

  .aura-title {
    font-size: clamp(24px, 2.2vw, 32px);
  }

  h2 {
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 700;
    line-height: 1.2;
    letter-spacing: 0;
  }

  p,
  .data-row span {
    color: var(--text-secondary);
    line-height: 1.55;
  }

  .stat-grid,
  .content-grid {
    min-width: 0;
    display: grid;
    gap: 12px;
  }

  .stat-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .content-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  :global(.ui-card.panel),
  :global(.ui-stat.stat-tile) {
    min-width: 0;
  }

  .panel-head {
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }

  .row-list {
    min-width: 0;
    display: grid;
    margin-top: 12px;
    overflow: hidden;
  }

  .aura-page .data-row {
    min-width: 0;
    padding: 12px 0;
    display: grid;
    grid-template-columns: minmax(72px, 0.25fr) minmax(0, 1fr);
    gap: 12px;
    border-bottom: 1px solid var(--border);
    border-top: 0;
    border-right: 0;
    border-left: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .data-row:last-child {
    border-bottom: 0;
    padding-bottom: 0;
  }

  .data-row strong {
    min-width: 0;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 700;
    overflow-wrap: anywhere;
  }

  .status-cell {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .status-dot {
    flex: 0 0 auto;
  }

  .data-row span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .log-well {
    margin-top: 12px;
    padding: 12px;
    display: grid;
    gap: 8px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    line-height: 1.55;
  }

  .log-well code {
    min-width: 0;
    color: inherit;
    font: inherit;
    overflow-wrap: anywhere;
  }

  .exported {
    padding: 12px 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
  }

  .exported span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  :global(.ui-card.loading-panel) {
    min-height: 180px;
    display: grid;
    place-items: center;
  }

  @media (max-width: 900px) {
    .stat-grid,
    .content-grid {
      grid-template-columns: 1fr;
    }

    .tool-head {
      flex-direction: column;
      align-items: stretch;
    }

    .tool-head :global(.ui-button) {
      width: 100%;
    }
  }

  @media (max-width: 560px) {
    .tool-page {
      padding: 18px;
    }

    .data-row {
      grid-template-columns: 1fr;
      gap: 4px;
    }
  }
</style>
