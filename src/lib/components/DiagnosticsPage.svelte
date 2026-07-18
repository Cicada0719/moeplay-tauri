<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, StatBlock } from "./ui";
  import { i18n } from "../stores/i18n.svelte";
  import {
    exportDiagnosticsZip,
    getMigrationStatus,
    getPerformanceSnapshot,
    runDiagnostics,
    type DiagnosticsReport,
    type MigrationInfo,
    type PerformanceSnapshot,
  } from "../api";
  import { PageShell, PageHeader, StateBoundary, type ViewState } from "./ui-v2";

  let report = $state<DiagnosticsReport | null>(null);
  let perf = $state<PerformanceSnapshot | null>(null);
  let migrations = $state<MigrationInfo[]>([]);
  let exported = $state("");
  let loading = $state(true);
  let exporting = $state(false);
  let error = $state<string | null>(null);
  // 导出失败与页级加载错误分离：原实现共用 error，report 就绪后导出失败会被吞掉。
  let exportError = $state<string | null>(null);

  // 三态统一：加载 / 错误 / 就绪收敛到 StateBoundary。
  const viewState = $derived<ViewState>(
    !report && error ? "error" : !report ? "loading" : "ready",
  );

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
    exportError = null;
    try {
      exported = await exportDiagnosticsZip();
    } catch (e) {
      exportError = String(e);
    } finally {
      exporting = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<PageShell as="div" width="full" scrollable={false} class="diagnostics-v2-shell" labelledBy="diagnostics-page-title" ariaLabel={i18n.t("diagnostics.title")}>
  <div class="dg">
    <div class="v2-grain dg-grain" aria-hidden="true"></div>

    <PageHeader
      id="diagnostics-page-title"
      class="dg-header"
      eyebrow="診断 / DIAGNOSTICS"
      title={i18n.t("diagnostics.title")}
      description={i18n.t("diagnostics.subtitle")}
    >
      {#snippet actions()}
        <Button variant="primary" press={exportBundle} loading={exporting} disabled={exporting}>
          <Icon name="download" size={16} />
          <span>{i18n.t("diagnostics.export")}</span>
        </Button>
      {/snippet}
    </PageHeader>

    <main class="dg-content">
      <StateBoundary
        state={viewState}
        onRetry={load}
        retryLabel={i18n.t("button.retry")}
        title={i18n.t("diagnostics.error_title")}
        description={error ?? undefined}
        loadingRows={4}
      >
        {#if report}
          {#if exportError}
            <div class="dg-banner" role="alert">{i18n.t("diagnostics.export_failed", { error: exportError })}</div>
          {/if}

          <div class="dg-stat-grid" aria-label={i18n.t("diagnostics.overview_aria")}>
            <StatBlock class="stat-tile" label={i18n.t("diagnostics.stat_system")} value={report.system_info.os} />
            <StatBlock class="stat-tile" label={i18n.t("diagnostics.stat_le")} value={report.system_info.locale_emulator_installed ? i18n.t("diagnostics.le_installed") : i18n.t("diagnostics.le_missing")} />
            <StatBlock class="stat-tile" label={i18n.t("diagnostics.stat_games")} value={perf?.game_count ?? 0} />
            <StatBlock class="stat-tile" label={i18n.t("diagnostics.stat_cache")} value={Math.round((perf?.cache_size_bytes ?? 0) / 1024 / 1024)} unit="MB" />
          </div>

          <div class="dg-panels">
            <Card class="panel">
              <div class="panel-head">
                <h2>{i18n.t("diagnostics.panel_issues")}</h2>
                <span class="mono">{report.issues.length}</span>
              </div>
              {#if report.issues.length}
                <div class="row-list">
                  {#each report.issues as issue}
                    <article class="data-row">
                      <strong class="status-cell mono"><span class="status-dot stopped"></span>{issue.severity}</strong>
                      <span>{issue.message}</span>
                    </article>
                  {/each}
                </div>
              {:else}
                <EmptyState title={i18n.t("diagnostics.issues_empty")} />
              {/if}
            </Card>

            <Card class="panel">
              <div class="panel-head">
                <h2>{i18n.t("diagnostics.panel_migrations")}</h2>
                <span class="mono">{migrations.length}</span>
              </div>
              {#if migrations.length}
                <div class="row-list">
                  {#each migrations as migration}
                    <article class="data-row">
                      <strong class="status-cell mono"><span class="status-dot" class:running={migration.applied} class:stopped={!migration.applied}></span>v{migration.version}</strong>
                      <span>{migration.applied ? i18n.t("diagnostics.migration_applied") : i18n.t("diagnostics.migration_pending")} · {migration.description}</span>
                    </article>
                  {/each}
                </div>
              {:else}
                <EmptyState title={i18n.t("diagnostics.migrations_empty")} />
              {/if}
            </Card>
          </div>

          <Card class="panel">
            <div class="panel-head">
              <h2>{i18n.t("diagnostics.panel_log")}</h2>
              <span class="mono">{new Date(perf?.timestamp ?? Date.now()).toLocaleTimeString(i18n.locale, { hour: "2-digit", minute: "2-digit" })}</span>
            </div>
            <div class="log-well" aria-label={i18n.t("diagnostics.log_aria")}>
              <code>os={report.system_info.os} arch={report.system_info.arch} memory={report.system_info.memory_gb}GB</code>
              <code>db={report.app_info.database_size_mb}MB games={perf?.game_count ?? report.app_info.game_count} cache={Math.round((perf?.cache_size_bytes ?? 0) / 1024 / 1024)}MB</code>
              <code>scrapers={(report.app_info.scrape_sources ?? []).join(",") || "none"}</code>
              <code>export={exported || "not_run"}</code>
            </div>
          </Card>

          {#if exported}
            <p class="dg-exported">
              <Icon name="check" size={16} />
              <span>{i18n.t("diagnostics.exported", { path: exported })}</span>
            </p>
          {/if}
        {/if}
      </StateBoundary>
    </main>
  </div>
</PageShell>

<style>
  :global(.diagnostics-v2-shell) { height: 100%; }
  :global(.diagnostics-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .dg {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .dg-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.dg-header) {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: 26px 28px 14px;
    flex-shrink: 0;
  }

  .dg-content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: 0 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    scroll-behavior: smooth;
  }

  h2,
  p {
    margin: 0;
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

  .dg-banner {
    padding: 12px 14px;
    border: 1px solid color-mix(in srgb, var(--danger, #ef4444) 45%, transparent);
    border-radius: 8px;
    background: color-mix(in srgb, var(--danger, #ef4444) 10%, transparent);
    color: var(--text-primary);
    font-size: 13px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .dg-stat-grid,
  .dg-panels {
    min-width: 0;
    display: grid;
    gap: 14px;
  }

  .dg-stat-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .dg-panels {
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

  .panel-head span {
    min-width: 0;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.2;
    letter-spacing: 0;
  }

  .mono {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .row-list {
    min-width: 0;
    display: grid;
    margin-top: 12px;
    overflow: hidden;
  }

  .data-row {
    min-width: 0;
    padding: 12px 0;
    display: grid;
    grid-template-columns: minmax(72px, 0.25fr) minmax(0, 1fr);
    gap: 12px;
    border-bottom: 1px solid var(--border);
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-inset, var(--bg-base));
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

  .dg-exported {
    padding: 12px 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
    color: var(--text-primary);
  }

  .dg-exported span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  @media (max-width: 900px) {
    .dg-stat-grid,
    .dg-panels {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 560px) {
    .dg-content { padding: 0 16px 36px; }
    :global(.dg-header) { padding: 20px 16px 12px; }

    .data-row {
      grid-template-columns: 1fr;
      gap: 4px;
    }
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .dg-content { scroll-behavior: auto; }
  }
  :global([data-motion="reduce"]) .dg-content { scroll-behavior: auto; }
</style>
