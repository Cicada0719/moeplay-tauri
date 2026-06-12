<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import Skeleton from "./Skeleton.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Icon from "./Icon.svelte";

  interface DashboardData {
    total_games: number;
    completed_games: number;
    total_playtime_hours: number;
    completion_rate: number;
    disk_usage_gb: number;
    top_tags: { name: string; count: number }[];
    top_developers: { name: string; count: number }[];
    monthly_heatmap: { month: string; hours: number }[];
    recent_sessions: { game_name: string; hours: number; date: string }[];
    collection_counts: { id: string; name: string; count: number }[];
    status_distribution: { status: string; count: number }[];
  }

  let data = $state<DashboardData | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let heroNumEl = $state<HTMLSpanElement>();
  let doneNumEl = $state<HTMLSpanElement>();
  let hoursNumEl = $state<HTMLSpanElement>();
  let donutSvg = $state<SVGCircleElement>();

  const statusLabels: Record<string, string> = {
    not_started: "未开始",
    playing: "进行中",
    completed: "已通关",
    dropped: "搁置",
    on_hold: "暂停",
    plan_to_play: "计划玩",
    replaying: "重温中",
  };

  const donutR = 42;
  const donutCirc = 2 * Math.PI * donutR;
  const chartW = 320;
  const chartH = 92;

  async function loadDashboard() {
    loading = true;
    error = null;
    try {
      data = await invoke<DashboardData>("get_dashboard_data");
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function handleCollectionClick(c: { id: string; name: string; count: number }) {
    const name = c.name.toLowerCase();
    if (name.includes("未玩") || name.includes("计划")) gameStore.filterTag = "未玩";
    else if (name.includes("通关") || name.includes("完成")) gameStore.filterTag = "已通关";
    else if (name.includes("收藏")) gameStore.filterTag = "收藏";
    else gameStore.filterTag = c.name;
    uiStore.currentView = "home";
  }

  function statusPct(status: string): number {
    if (!data || data.total_games <= 0) return 0;
    const entry = data.status_distribution.find((s) => s.status === status);
    return entry ? Math.round((entry.count / data.total_games) * 100) : 0;
  }

  function trendPoints(items: { hours: number }[]): string {
    if (items.length === 0) return "";
    const max = Math.max(1, ...items.map((item) => item.hours));
    return items.map((item, index) => {
      const x = items.length === 1 ? chartW / 2 : (index / (items.length - 1)) * chartW;
      const y = chartH - (item.hours / max) * (chartH - 10) - 5;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(" ");
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

      const hours = { v: 0 };
      gsap.to(hours, {
        v: d.total_playtime_hours,
        duration: 1.1,
        ease: "power2.out",
        delay: 0.25,
        onUpdate: () => {
          if (hoursNumEl) hoursNumEl.textContent = Math.round(hours.v).toString();
        },
      });

      if (donutSvg) {
        const target = donutCirc * (1 - Math.min(1, (d.completion_rate || 0) / 100));
        gsap.fromTo(
          donutSvg,
          { strokeDashoffset: donutCirc },
          {
            strokeDashoffset: target,
            duration: 1.3,
            ease: "power3.out",
            delay: 0.2,
          },
        );
      }
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
      <Skeleton variant="stats" count={4} />
      <Skeleton variant="card" count={2} />
    </div>
  {:else if error}
    <div class="inline-error" role="alert">
      <Icon name="x" size={16} />
      <span>加载失败：{error}</span>
      <button onclick={loadDashboard}>
        <Icon name="refresh" size={15} />
        <span>重试</span>
      </button>
    </div>
  {:else if data}
    <div class="bento">
      <article class="metric-card hero">
        <span class="label">游戏总数</span>
        <span class="value hero-value aura-num" bind:this={heroNumEl}>{data.total_games}</span>
        <span class="hint">完成率 <span class="aura-num">{data.completion_rate}%</span></span>
      </article>

      <article class="metric-card">
        <span class="label">已通关</span>
        <span class="value mid aura-num" bind:this={doneNumEl}>{data.completed_games}</span>
      </article>

      <article class="metric-card">
        <span class="label">总时长</span>
        <span class="value mid aura-num"><span bind:this={hoursNumEl}>{data.total_playtime_hours.toFixed(0)}</span><small>h</small></span>
      </article>

      <article class="metric-card">
        <span class="label">磁盘占用</span>
        <span class="value mid aura-num">{data.disk_usage_gb.toFixed(1)}<small>GB</small></span>
      </article>

      <article class="metric-card donut-card">
        <span class="label">完成率</span>
        <div class="donut">
          <svg width="108" height="108" viewBox="0 0 108 108" aria-hidden="true">
            <circle cx="54" cy="54" r={donutR} fill="none" stroke="var(--border)" stroke-width="8" />
            <circle
              bind:this={donutSvg}
              cx="54"
              cy="54"
              r={donutR}
              fill="none"
              stroke="var(--aura-data-a)"
              stroke-width="8"
              stroke-linecap="round"
              stroke-dasharray={donutCirc}
              stroke-dashoffset={donutCirc * (1 - Math.min(1, (data.completion_rate || 0) / 100))}
              transform="rotate(-90 54 54)"
            />
          </svg>
          <span class="donut-num aura-num">{data.completion_rate}%</span>
        </div>
      </article>

      <article class="metric-card wide">
        <span class="label">状态分布</span>
        <div class="status-bars">
          {#each Object.entries(statusLabels) as [key, label]}
            {@const pct = statusPct(key)}
            <div class="status-row" title="{label}: {pct}%">
              <span class="status-label">{label}</span>
              <div class="status-track"><div class="status-fill" style="width:{pct}%"></div></div>
              <span class="status-pct aura-num">{pct}%</span>
            </div>
          {/each}
        </div>
      </article>

      <article class="metric-card">
        <span class="label">热门标签</span>
        <div class="tag-cloud">
          {#each (data.top_tags ?? []).slice(0, 10) as tag}
            <span class="tag-chip">
              {tag.name}<small class="aura-num">{tag.count}</small>
            </span>
          {/each}
        </div>
      </article>

      <article class="metric-card">
        <span class="label">开发商</span>
        <div class="flat-list">
          {#each (data.top_developers ?? []).slice(0, 10) as developer}
            <div class="list-row">
              <span>{developer.name}</span>
              <span class="mono muted aura-num">{developer.count}</span>
            </div>
          {/each}
        </div>
      </article>

      <article class="metric-card wide">
        <span class="label">月度热力图</span>
        {#if (data.monthly_heatmap ?? []).length > 0}
          <svg class="trend-line" viewBox={`0 0 ${chartW} ${chartH}`} aria-label="月度趋势">
            <polyline class="trend-area" points={`${trendPoints(data.monthly_heatmap.slice(-12))} ${chartW},${chartH} 0,${chartH}`} />
            <polyline class="trend-stroke" points={trendPoints(data.monthly_heatmap.slice(-12))} />
          </svg>
          <div class="heatmap">
            {#each data.monthly_heatmap.slice(-12) as month}
              <div class="heat-cell" title="{month.month}: {month.hours}h" style="--heat:{Math.min(1, month.hours * 0.06)}">
                <span><span class="aura-num">{month.month.slice(-2)}</span>月</span>
                <strong class="aura-num">{month.hours}h</strong>
              </div>
            {/each}
          </div>
        {:else}
          <EmptyState title="暂无数据" />
        {/if}
      </article>

      <article class="metric-card wide">
        <span class="label">最近游玩</span>
        {#if (data.recent_sessions ?? []).length > 0}
          <div class="sessions">
            {#each data.recent_sessions.slice(0, 5) as session}
              <div class="session-row">
                <span class="game-name">{session.game_name}</span>
                <span class="mono muted aura-num">{session.date}</span>
                <span class="mono aura-num">{session.hours.toFixed(1)}h</span>
              </div>
            {/each}
          </div>
        {:else}
          <EmptyState title="暂无记录" />
        {/if}
      </article>

      <article class="metric-card wide">
        <span class="label">智能合集</span>
        <div class="collection-grid">
          {#each (data.collection_counts ?? []).slice(0, 8) as collection}
            <button class="collection-action" onclick={() => handleCollectionClick(collection)}>
              <span class="collection-count aura-num">{collection.count}</span>
              <span class="collection-name">{collection.name}</span>
            </button>
          {/each}
        </div>
      </article>
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

  .metric-card,
  .inline-error,
  .empty-panel {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
    box-shadow: none;
  }

  .metric-card {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 132px;
  }

  .metric-card.wide {
    grid-column: span 2;
  }

  .metric-card.hero {
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

  .value.mid {
    font-size: 24px;
  }

  .value.hero-value {
    font-size: clamp(44px, 7vw, 68px);
  }

  .value small {
    color: var(--text-muted);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 650;
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

  .donut-card {
    align-items: center;
  }

  .donut {
    position: relative;
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

  .status-bars,
  .flat-list,
  .sessions {
    min-width: 0;
    display: grid;
    gap: 8px;
  }

  .status-row {
    min-width: 0;
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) 40px;
    gap: 10px;
    align-items: center;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .status-track {
    min-width: 0;
    height: 6px;
    border-radius: 999px;
    background: var(--aura-inset);
    overflow: hidden;
  }

  .status-fill {
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--aura-data-a), var(--aura-data-b));
    transition: width 0.6s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .status-pct {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .tag-cloud {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .tag-chip {
    min-width: 0;
    max-width: 100%;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 5px 9px;
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    color: var(--text-secondary);
    background: transparent;
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .tag-chip small {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
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

  .heatmap {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(64px, 1fr));
    gap: 8px;
  }

  .trend-line {
    width: 100%;
    min-height: 82px;
    display: block;
    overflow: visible;
  }

  .trend-stroke,
  .trend-area {
    fill: none;
    vector-effect: non-scaling-stroke;
  }

  .trend-stroke {
    stroke: var(--aura-data-1);
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
    filter: drop-shadow(0 10px 18px rgba(232, 85, 127, 0.18));
  }

  .trend-area {
    fill: var(--aura-data-3);
    opacity: 0.34;
  }

  .heat-cell {
    min-width: 0;
    min-height: 54px;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px;
    display: grid;
    place-items: center;
    gap: 3px;
    color: var(--text-secondary);
    background:
      linear-gradient(180deg, var(--aura-data-3), transparent),
      var(--aura-inset);
    box-shadow: inset 0 -3px 0 var(--aura-data-2);
    font-size: 12px;
  }

  .heat-cell strong {
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 12px;
  }

  .collection-grid {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }

  .collection-action {
    min-width: 0;
    min-height: 58px;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 9px;
    display: grid;
    place-items: center;
    gap: 3px;
    color: var(--text-secondary);
    background: transparent;
    font: inherit;
    cursor: pointer;
    transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease, transform 0.16s ease;
  }

  .collection-action:hover {
    border-color: var(--border-hover);
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .collection-action:active {
    transform: translateY(1px);
  }

  .collection-action:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
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
    color: var(--color-error);
  }

  .inline-error span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .inline-error button {
    min-height: 34px;
    margin-left: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0 12px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--text-secondary);
    background: transparent;
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }

  .inline-error button:hover {
    border-color: var(--border-hover);
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .inline-error button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
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

  @media (prefers-reduced-motion: reduce) {
    .status-fill,
    .collection-action,
    .inline-error button {
      transition: none;
    }
  }

  @media (max-width: 900px) {
    .bento {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .metric-card.wide,
    .metric-card.hero {
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

    .metric-card,
    .metric-card.wide,
    .metric-card.hero {
      grid-column: 1;
    }

    .session-row,
    .status-row {
      grid-template-columns: minmax(0, 1fr);
    }

    .status-pct {
      text-align: left;
    }

    .inline-error {
      align-items: stretch;
      flex-direction: column;
    }

    .inline-error button {
      width: 100%;
      justify-content: center;
      margin-left: 0;
    }
  }
</style>
