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
  import Icon from "./Icon.svelte";
  import { Button, Card, Chart, EmptyState, Skeleton, StatBlock, Tag } from "./ui";
  import {
    buildCompletionDoughnutData,
    buildMonthlyTrendData,
    buildStatusDistributionData,
    commonChartOptions,
    doughnutOptions,
    statusBarOptions,
  } from "../utils/chart";

  let data = $state<DashboardData | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let heroNumEl = $state<HTMLSpanElement>();
  let doneNumEl = $state<HTMLSpanElement>();
  const statusLabels: Record<string, string> = {
    not_started: "未开始",
    playing: "进行中",
    completed: "已通关",
    dropped: "搁置",
    on_hold: "暂停",
    plan_to_play: "计划玩",
    replaying: "重温中",
  };

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
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
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

<section class="stats-page aura-page" data-aura-echo="STATISTICS">
  <header class="aura-head">
    <div>
      <span class="aura-kicker">Library Pulse</span>
      <h1 class="aura-title">统计</h1>
      <p>Collection scale, completion state, play rhythm, and recent sessions.</p>
    </div>
  </header>

  {#if loading}
    <div class="loading-stack">
      <Skeleton variant="stat" count={4} />
      <Skeleton variant="block" count={2} />
    </div>
  {:else if error}
    <div class="inline-error" role="alert">
      <Icon name="x" size={16} />
      <span>加载失败：{error}</span>
      <Button variant="ghost" size="sm" class="retry-btn" press={loadDashboard}>
        <Icon name="refresh" size={15} />
        <span>重试</span>
      </Button>
    </div>
  {:else if data}
    <div class="bento">
      <Card class="metric-card hero">
        <span class="label">游戏总数</span>
        <span class="value hero-value aura-num" bind:this={heroNumEl}>{data.total_games}</span>
        <span class="hint">完成率 <span class="aura-num">{data.completion_rate}%</span></span>
      </Card>

      <StatBlock label="已通关" value={data.completed_games} class="stat-cell" />

      <StatBlock label="总时长" value={data.playtime_hours.toFixed(0)} unit="h" class="stat-cell" />

      <StatBlock
        label="磁盘占用"
        value={data.disk_usage_gb.toFixed(1)}
        unit="GB"
        hint="后台缓存统计，重新进入页面后更新"
        class="stat-cell"
      />

      <Card class="metric-card donut-card">
        <span class="label">完成率</span>
        <div class="donut">
          <Chart type="doughnut" data={buildCompletionDoughnutData(data.completion_rate)} options={doughnutOptions} />
          <span class="donut-num aura-num">{data.completion_rate}%</span>
        </div>
      </Card>

      <Card class="metric-card wide">
        <span class="label">状态分布</span>
        <div class="status-chart">
          <Chart
            type="bar"
            data={buildStatusDistributionData(toStatusCountItems(data.completion_distribution), statusLabels)}
            options={statusBarOptions}
          />
        </div>
      </Card>

      <Card class="metric-card">
        <span class="label">热门标签</span>
        <div class="tag-cloud">
          {#each toCountItems(data.top_tags).slice(0, 10) as tag}
            <Tag class="tag-chip">
              {tag.name}<small class="aura-num">{tag.count}</small>
            </Tag>
          {/each}
        </div>
      </Card>

      <Card class="metric-card">
        <span class="label">数据覆盖</span>
        <div class="flat-list">
          <div class="list-row">
            <span>已安装</span>
            <span class="mono muted aura-num">{data.installed_games} / {data.total_games}</span>
          </div>
          <div class="list-row">
            <span>元数据刮削</span>
            <span class="mono muted aura-num">{data.scrape_coverage.toFixed(1)}%</span>
          </div>
        </div>
      </Card>

      <Card class="metric-card wide">
        <span class="label">月度趋势</span>
        {#if (data.monthly_heatmap ?? []).length > 0}
          <div class="trend-chart">
            <Chart
              type="line"
              data={buildMonthlyTrendData(data.monthly_heatmap)}
              options={commonChartOptions}
            />
          </div>
        {:else}
          <EmptyState title="暂无数据" />
        {/if}
      </Card>

      <Card class="metric-card wide">
        <span class="label">最近游玩</span>
        {#if data.recent_games.length > 0}
          <div class="sessions">
            {#each data.recent_games.slice(0, 5) as game, index}
              <div class="session-row">
                <span class="game-name">{game}</span>
                <span class="mono muted aura-num">RECENT</span>
                <span class="mono aura-num">#{index + 1}</span>
              </div>
            {/each}
          </div>
        {:else}
          <EmptyState title="暂无记录" />
        {/if}
      </Card>

      <Card class="metric-card wide">
        <span class="label">智能合集</span>
        <div class="collection-grid">
          {#each toCollectionCountItems(data.collections).slice(0, 8) as collection}
            <Card
              class="collection-action"
              padding="none"
              hoverable
              onclick={() => handleCollectionClick(collection)}
              ariaLabel={collection.name}
            >
              <span class="collection-count aura-num">{collection.count}</span>
              <span class="collection-name">{collection.name}</span>
            </Card>
          {/each}
        </div>
      </Card>
    </div>
  {:else}
    <div class="empty-panel">
      <EmptyState title="暂无统计数据" description="导入游戏并游玩后，这里将展示统计仪表盘。" />
    </div>
  {/if}
</section>

<style>
  .stats-page {
    min-width: 0;
    height: 100%;
    padding: 24px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    background: var(--aura-bg, var(--bg-void));
  }

  .aura-head {
    min-width: 0;
    width: min(1180px, 100%);
    padding: 18px 20px;
  }

  .aura-head > div {
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
    color: var(--text-primary);
    font-size: clamp(24px, 2.2vw, 32px);
    font-weight: 760;
    line-height: 1.12;
  }

  .aura-head p {
    color: var(--text-secondary);
    line-height: 1.55;
  }

  .bento {
    min-width: 0;
    width: min(1180px, 100%);
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(0, 1fr) minmax(0, 1fr);
    grid-auto-flow: dense;
    gap: 14px;
  }

  :global(.ui-card.metric-card),
  :global(.ui-stat.stat-cell),
  .inline-error,
  .empty-panel {
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
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
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
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
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

  .inline-error {
    min-height: 54px;
    padding: 12px 14px;
    display: flex;
    align-items: center;
    gap: 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
    color: var(--color-error);
  }

  .inline-error span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .inline-error :global(.ui-button.retry-btn) {
    margin-left: auto;
  }

  .empty-panel {
    min-height: 220px;
    display: grid;
    place-items: center;
  }

  .loading-stack {
    min-width: 0;
    display: grid;
    gap: 14px;
  }

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
    .stats-page {
      padding: 18px;
    }

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

    .inline-error {
      align-items: stretch;
      flex-direction: column;
    }

    .inline-error :global(.ui-button.retry-btn) {
      width: 100%;
      justify-content: center;
      margin-left: 0;
    }
  }
</style>
