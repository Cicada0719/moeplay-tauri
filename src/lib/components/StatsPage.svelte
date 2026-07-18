<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import {
    getDashboardData,
    toCollectionCountItems,
    toCountItems,
    toStatusCountItems,
    type CollectionCountItem,
    type DashboardData,
  } from "../api/dashboard";
  import { uiStore } from "../stores/ui.svelte";
  import { i18n } from "../stores/i18n.svelte";
  import { Card, Chart, EmptyState, StatBlock, Tag } from "./ui";
  import {
    buildCompletionDoughnutData,
    buildMonthlyTrendData,
    buildStatusDistributionData,
    commonChartOptions,
    doughnutOptions,
    statusBarOptions,
  } from "../utils/chart";
  import { PageShell, PageHeader, StateBoundary, type ViewState } from "./ui-v2";

  let data = $state<DashboardData | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let heroNumEl = $state<HTMLSpanElement>();
  let doneNumEl = $state<HTMLSpanElement>();
  const statusLabels = $derived<Record<string, string>>({
    not_started: i18n.t("stats.status_not_started"),
    playing: i18n.t("stats.status_playing"),
    completed: i18n.t("stats.status_completed"),
    dropped: i18n.t("stats.status_dropped"),
    on_hold: i18n.t("stats.status_on_hold"),
    plan_to_play: i18n.t("stats.status_plan_to_play"),
    replaying: i18n.t("stats.status_replaying"),
  });

  // 三态统一：加载 / 错误 / 空 / 就绪收敛到 StateBoundary。
  const viewState = $derived<ViewState>(
    loading ? "loading" : error ? "error" : data ? "ready" : "empty",
  );

  async function loadDashboard() {
    loading = true;
    error = null;
    try {
      data = await getDashboardData();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function handleCollectionClick(c: CollectionCountItem) {
    const name = c.name.toLowerCase();
    if (name.includes("未玩") || name.includes("计划")) gameStore.filterTag = "未玩";
    else if (name.includes("通关") || name.includes("完成")) gameStore.filterTag = "已通关";
    else if (name.includes("收藏")) gameStore.filterTag = "收藏";
    else gameStore.filterTag = c.name;
    uiStore.currentView = "home";
  }

  onMount(() => {
    void loadDashboard();
  });

  $effect(() => {
    if (!data || loading) return;
    const d = data;
    const reduce =
      window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches ||
      document.documentElement.dataset.motion === "reduce";
    if (reduce) return;

    const ctx = gsap.context(() => {
      const total = { v: 0 };
      gsap.to(total, {
        v: d.total_games,
        duration: 1.2,
        ease: "power2.out",
        onUpdate: () => {
          if (heroNumEl) heroNumEl.textContent = Math.round(total.v).toString();
        },
      });

      const completed = { v: 0 };
      gsap.to(completed, {
        v: d.completed_games,
        duration: 1.0,
        ease: "power2.out",
        delay: 0.15,
        onUpdate: () => {
          if (doneNumEl) doneNumEl.textContent = Math.round(completed.v).toString();
        },
      });
    });
    return () => ctx.revert();
  });
</script>

<PageShell as="div" width="full" scrollable={false} class="stats-v2-shell" labelledBy="stats-page-title" ariaLabel={i18n.t("stats.title")}>
  <div class="st">
    <div class="v2-grain st-grain" aria-hidden="true"></div>

    <PageHeader
      id="stats-page-title"
      class="st-header"
      eyebrow="統計 / STATS"
      title={i18n.t("stats.title")}
      description={i18n.t("stats.subtitle")}
    />

    <main class="st-content">
      <StateBoundary
        state={viewState}
        onRetry={loadDashboard}
        retryLabel={i18n.t("button.retry")}
        title={viewState === "error" ? i18n.t("stats.error_title") : i18n.t("stats.empty_title")}
        description={viewState === "error" ? (error ?? undefined) : i18n.t("stats.empty_desc")}
        loadingRows={5}
      >
        {#if data}
          <div class="bento">
            <Card class="metric-card hero">
              <span class="label">{i18n.t("stats.total_games")}</span>
              <span class="value hero-value" bind:this={heroNumEl}>{data.total_games}</span>
              <span class="hint">{i18n.t("stats.completion_rate")} <span class="mono">{data.completion_rate}%</span></span>
            </Card>

            <StatBlock label={i18n.t("stats.completed_games")} value={data.completed_games} class="stat-cell" />

            <StatBlock label={i18n.t("stats.total_hours")} value={data.playtime_hours.toFixed(0)} unit="h" class="stat-cell" />

            <StatBlock
              label={i18n.t("stats.disk_usage")}
              value={data.disk_usage_gb.toFixed(1)}
              unit="GB"
              hint={i18n.t("stats.disk_hint")}
              class="stat-cell"
            />

            <Card class="metric-card donut-card">
              <span class="label">{i18n.t("stats.completion_rate")}</span>
              <div class="donut">
                <Chart type="doughnut" data={buildCompletionDoughnutData(data.completion_rate)} options={doughnutOptions} />
                <span class="donut-num mono">{data.completion_rate}%</span>
              </div>
            </Card>

            <Card class="metric-card wide">
              <span class="label">{i18n.t("stats.status_dist")}</span>
              <div class="status-chart">
                <Chart
                  type="bar"
                  data={buildStatusDistributionData(toStatusCountItems(data.completion_distribution), statusLabels)}
                  options={statusBarOptions}
                />
              </div>
            </Card>

            <Card class="metric-card">
              <span class="label">{i18n.t("stats.top_tags")}</span>
              <div class="tag-cloud">
                {#each toCountItems(data.top_tags).slice(0, 10) as tag}
                  <Tag class="tag-chip">
                    {tag.name}<small class="mono">{tag.count}</small>
                  </Tag>
                {/each}
              </div>
            </Card>

            <Card class="metric-card">
              <span class="label">{i18n.t("stats.data_coverage")}</span>
              <div class="flat-list">
                <div class="list-row">
                  <span>{i18n.t("stats.installed")}</span>
                  <span class="mono muted">{data.installed_games} / {data.total_games}</span>
                </div>
                <div class="list-row">
                  <span>{i18n.t("stats.scraped")}</span>
                  <span class="mono muted">{data.scrape_coverage.toFixed(1)}%</span>
                </div>
              </div>
            </Card>

            <Card class="metric-card wide">
              <span class="label">{i18n.t("stats.monthly_heatmap")}</span>
              {#if (data.monthly_heatmap ?? []).length > 0}
                <div class="trend-chart">
                  <Chart
                    type="line"
                    data={buildMonthlyTrendData(data.monthly_heatmap)}
                    options={commonChartOptions}
                  />
                </div>
              {:else}
                <EmptyState title={i18n.t("stats.no_data")} />
              {/if}
            </Card>

            <Card class="metric-card wide">
              <span class="label">{i18n.t("stats.recent_games")}</span>
              {#if data.recent_games.length > 0}
                <div class="sessions">
                  {#each data.recent_games.slice(0, 5) as game, index}
                    <div class="session-row">
                      <span class="game-name">{game}</span>
                      <span class="mono muted">RECENT</span>
                      <span class="mono">#{index + 1}</span>
                    </div>
                  {/each}
                </div>
              {:else}
                <EmptyState title={i18n.t("stats.no_records")} />
              {/if}
            </Card>

            <Card class="metric-card wide">
              <span class="label">{i18n.t("stats.smart_collections")}</span>
              <div class="collection-grid">
                {#each toCollectionCountItems(data.collections).slice(0, 8) as collection}
                  <Card
                    class="collection-action"
                    padding="none"
                    hoverable
                    onclick={() => handleCollectionClick(collection)}
                    ariaLabel={collection.name}
                  >
                    <span class="collection-count mono">{collection.count}</span>
                    <span class="collection-name">{collection.name}</span>
                  </Card>
                {/each}
              </div>
            </Card>
          </div>
        {/if}
      </StateBoundary>
    </main>
  </div>
</PageShell>

<style>
  :global(.stats-v2-shell) { height: 100%; }
  :global(.stats-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .st {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .st-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.st-header) {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: 26px 28px 14px;
    flex-shrink: 0;
  }

  .st-content {
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

  .bento {
    min-width: 0;
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(0, 1fr) minmax(0, 1fr);
    grid-auto-flow: dense;
    gap: 14px;
  }

  :global(.ui-card.metric-card),
  :global(.ui-stat.stat-cell) {
    min-width: 0;
  }

  :global(.ui-card.metric-card) {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 132px;
  }

  :global(.ui-stat.stat-cell) {
    min-height: 132px;
  }

  :global(.ui-card.metric-card.wide) {
    grid-column: span 2;
  }

  :global(.ui-card.metric-card.hero) {
    grid-column: 1;
    grid-row: span 2;
    justify-content: center;
  }

  .label {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.2;
    letter-spacing: 0;
  }

  .value {
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 6px;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 32px;
    font-weight: 760;
    line-height: 1;
    overflow-wrap: anywhere;
  }

  .value.hero-value {
    font-size: clamp(44px, 7vw, 68px);
  }

  .hint,
  .muted {
    color: var(--text-muted);
  }

  .hint {
    font-size: 13px;
    line-height: 1.4;
  }

  .mono {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  :global(.ui-card.donut-card) {
    align-items: center;
  }

  .donut {
    position: relative;
    width: 108px;
    height: 108px;
    display: grid;
    place-items: center;
  }

  .donut-num {
    position: absolute;
    color: var(--text-primary);
    font-size: 18px;
    font-weight: 760;
  }

  .status-chart,
  .trend-chart {
    min-width: 0;
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .status-chart {
    height: 160px;
  }

  .trend-chart {
    height: 180px;
  }

  .flat-list,
  .sessions {
    min-width: 0;
    display: grid;
    gap: 8px;
  }

  .tag-cloud {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  :global(.tag-chip) {
    white-space: normal;
    overflow: visible;
    text-overflow: clip;
    overflow-wrap: anywhere;
  }

  .list-row,
  .session-row {
    min-width: 0;
    display: grid;
    gap: 10px;
    align-items: center;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .list-row {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .session-row {
    grid-template-columns: minmax(0, 1fr) 120px 58px;
  }

  .list-row span:first-child,
  .game-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .collection-grid {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }

  :global(.ui-card.collection-action) {
    min-width: 0;
    min-height: 58px;
    padding: 9px;
    display: grid;
    place-items: center;
    gap: 3px;
    color: var(--text-secondary);
    font: inherit;
  }

  :global(.ui-card.collection-action:hover) {
    color: var(--text-primary);
  }

  .collection-count {
    color: var(--text-primary);
    font-size: 18px;
    font-weight: 760;
  }

  .collection-name {
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  /* ── Responsive ── */
  @media (max-width: 900px) {
    .bento {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    :global(.ui-card.metric-card.wide),
    :global(.ui-card.metric-card.hero) {
      grid-column: span 2;
    }

    .collection-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 560px) {
    .st-content { padding: 0 16px 36px; }
    :global(.st-header) { padding: 20px 16px 12px; }

    .bento,
    .collection-grid {
      grid-template-columns: 1fr;
    }

    :global(.ui-card.metric-card),
    :global(.ui-card.metric-card.wide),
    :global(.ui-card.metric-card.hero),
    :global(.ui-stat.stat-cell) {
      grid-column: 1;
    }
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .st-content { scroll-behavior: auto; }
  }
  :global([data-motion="reduce"]) .st-content { scroll-behavior: auto; }
</style>
