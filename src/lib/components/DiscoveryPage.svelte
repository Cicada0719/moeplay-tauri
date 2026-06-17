<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Skeleton from "./Skeleton.svelte";
  import Icon from "./Icon.svelte";
  import SakuraParticles from "./SakuraParticles.svelte";
  import CachedImage from "./CachedImage.svelte";
  import {
    getRecommendations,
    getSmartCollections,
    getDashboardData,
    scrapeGames,
    openUrl,
    buildSourceUrl,
    type Collection,
    type ScrapeResult,
  } from "../api";
  import DiscoveryDetail from "./DiscoveryDetail.svelte";

  const tabs = ["搜索", "推荐", "开发商", "标签", "年份", "评分"];
  let active = $state("搜索");
  let query = $state("");
  let results = $state<ScrapeResult[]>([]);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let selectedResult = $state<ScrapeResult | null>(null);
  let recommendations = $state<{ name: string; score: number; reasons?: string[] }[]>([]);
  let collections = $state<Collection[]>([]);
  let loadError = $state<string | null>(null);

  // Facet data
  let topDevs = $state<{ name: string; count: number }[]>([]);
  let topTags = $state<{ name: string; count: number }[]>([]);

  async function search() {
    if (!query.trim()) return;
    searching = true;
    searchError = null;
    try {
      results = await scrapeGames(query, true, true, {
        kungal: true, steam: true, pcgw: true, erogamescape: true, ymgal: true, touchgal: true,
      });
    } catch (e) {
      searchError = String(e);
      console.error("Search failed:", e);
    } finally {
      searching = false;
    }
  }

  function handleCardDblClick(r: ScrapeResult) {
    selectedResult = r;
  }

  async function loadRecommendations() {
    try {
      const [recs, cols, dash] = await Promise.all([
        getRecommendations(null, 12),
        getSmartCollections(),
        getDashboardData(),
      ]);
      recommendations = (recs ?? []) as { name: string; score: number; reasons?: string[] }[];
      collections = cols ?? [];
      topDevs = (dash as any)?.top_developers ?? [];
      topTags = (dash as any)?.top_tags ?? [];
      loadError = null;
    } catch (e) {
      loadError = String(e);
      console.error("Discovery mount failed:", e);
    }
  }

  onMount(() => {
    void loadRecommendations();
  });
</script>

<section class="page aura-page" data-aura-echo="DISCOVERY">
  <SakuraParticles count={8} />

  <header class="aura-head">
    <div>
      <p class="aura-kicker">Discovery</p>
      <h1 class="aura-title">资源发现</h1>
      <p>搜索外部元数据、查看推荐和智能合集线索</p>
    </div>
    <div class="head-stats" aria-label="发现页概览">
      <span><strong class="aura-num">{recommendations.length}</strong> 推荐</span>
      <span><strong class="aura-num">{collections.length}</strong> 合集</span>
    </div>
  </header>

  {#if loadError}
    <div class="error-banner">
      <Icon name="x" size={14} />
      <span>推荐数据加载失败：{loadError}</span>
      <button class="retry-link" onclick={loadRecommendations}>重试</button>
    </div>
  {/if}

  <nav class="tabs">
    {#each tabs as tab}
      <button class:active={active === tab} onclick={() => active = tab}>{tab}</button>
    {/each}
  </nav>

  {#if active === "搜索"}
    {#if selectedResult}
      <DiscoveryDetail result={selectedResult} onClose={() => selectedResult = null} />
    {:else}
    <div class="toolbar">
      <div class="search-box">
        <Icon name="search" size={16} />
        <input bind:value={query} placeholder="搜索游戏、开发商或 Steam ID" onkeydown={(e) => e.key === "Enter" && search()} />
      </div>
      <button class="btn-search" onclick={search} disabled={searching}>
        {searching ? "搜索中..." : "搜索"}
      </button>
    </div>

    {#if searching}
      <div class="grid">
        <Skeleton variant="card" count={6} />
      </div>
    {:else if results.length}
      <div class="editorial-rail" role="list" aria-label="发现搜索结果">
        {#each results as r}
          {@const sourceUrl = buildSourceUrl(r)}
          <div class="result-card editorial-card" role="button" tabindex="0" onclick={() => selectedResult = r} onkeydown={(e) => { if (e.key === 'Enter') selectedResult = r; }}>
            {#if r.cover || r.background}
              <div class="cover-wrap">
                <CachedImage source={r.cover ?? r.background} cacheKey={`discovery:${r.source}:${r.source_id || r.title}`} alt={r.title} />
                <span class="source-badge">{r.source}</span>
                {#if sourceUrl}
                  <span class="url-hint" title="点击查看详情"><Icon name="arrowLeft" size={12} /></span>
                {/if}
              </div>
            {:else}
              <div class="cover-placeholder">
                <Icon name="gamepad" size={32} />
                <span class="source-badge">{r.source}</span>
                {#if sourceUrl}
                  <span class="url-hint" title="点击查看详情"><Icon name="arrowLeft" size={12} /></span>
                {/if}
              </div>
            {/if}
            <div class="meta">
              <strong class="title">{r.title}</strong>
              {#if r.release_year}
                <span class="year">{r.release_year}</span>
              {/if}
              {#if r.rating}
                <span class="rating"><Icon name="star" size={12} /> {r.rating.toFixed(1)}</span>
              {/if}
              {#if r.detail?.developer}
                <span class="dev text-muted">{r.detail.developer}</span>
              {/if}
              {#if r.description}
                <p class="desc">{r.description}</p>
              {:else}
                <p class="desc empty">暂无简介</p>
              {/if}
              {#if r.tags.length}
                <div class="tags">
                  {#each r.tags.slice(0, 6) as t}
                    <span class="tag">{t}</span>
                  {/each}
                  {#if r.tags.length > 6}
                    <span class="tag more">+{r.tags.length - 6}</span>
                  {/if}
                </div>
              {/if}
              {#if r.detail?.age_rating}
                <span class="age">{r.detail.age_rating}</span>
              {/if}
              {#if r.source === "kungal" || r.source === "touchgal"}
                <span class="dl-badge" title="可能有下载资源"><Icon name="download" size={10} /> 下载</span>
              {/if}
            </div>
            <div class="card-actions">
              <button class="act-btn primary" onclick={(e) => { e.stopPropagation(); selectedResult = r; }} title="查看详情">
                <Icon name="eye" size={14} /> 查看详情
              </button>
              {#if sourceUrl}
                <button class="act-btn" onclick={(e) => { e.stopPropagation(); openUrl(sourceUrl!); }} title="打开源站">
                  <Icon name="globe" size={14} />
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {:else if searchError}
      <EmptyState title="搜索失败" description={searchError} actionLabel="重试" onAction={search} />
    {:else}
      <EmptyState title="等待搜索" description="并发查询 Bangumi / VNDB / Kungal / Steam / PCGW / Ymgal 等" />
    {/if}
    {/if}

  {:else if active === "推荐"}
    <div class="grid compact">
      {#each recommendations as item}
        <article class="rec-card">
          <strong>{item.name}</strong>
          <div class="rec-row">
            <span class="score aura-num">推荐分 {item.score}</span>
          </div>
          <p>{item.reasons?.join(" / ")}</p>
        </article>
      {:else}
        <EmptyState title="暂无推荐" description="添加更多游戏后 AI 会生成个性化推荐" />
      {/each}
    </div>

  {:else if active === "开发商"}
    <div class="grid compact">
      {#each topDevs.slice(0, 20) as d}
        <article class="facet-card">
          <strong>{d.name}</strong>
          <span class="count aura-num">{d.count} 款</span>
        </article>
      {:else}
        <EmptyState title="暂无数据" />
      {/each}
    </div>

  {:else if active === "标签"}
    <div class="grid compact">
      {#each topTags.slice(0, 30) as t}
        <article class="facet-card">
          <strong>{t.name}</strong>
          <span class="count aura-num">{t.count} 次</span>
        </article>
      {:else}
        <EmptyState title="暂无数据" />
      {/each}
    </div>

  {:else if active === "年份" || active === "评分"}
    <div class="grid compact">
      {#each collections.filter(c => c.id?.includes(active === "年份" ? "year" : "rating") || c.id?.includes(active === "年份" ? "month" : "score")).slice(0, 12) as c}
        <article class="coll-card">
          <strong>{c.name}</strong>
          <span class="count aura-num">{c.game_count} 款</span>
          <p>{c.description}</p>
        </article>
      {:else}
        <EmptyState title="暂无数据" description="导入更多游戏后会自动生成合集" />
      {/each}
    </div>
  {/if}
</section>

<style>
  .page { padding: 24px; overflow-y: auto; height: 100%; display: flex; flex-direction: column; gap: 18px; }
  .page > :not(:global(.sakura-layer)) { position: relative; z-index: 1; }
  .page :global(.sakura-layer) { position: absolute; z-index: 0; opacity: 0.48; }
  .aura-head { display: flex; justify-content: space-between; gap: 16px; align-items: flex-end; }
  .aura-kicker {
    margin: 0 0 6px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--accent);
    text-transform: uppercase;
  }
  .aura-title { margin: 0; }
  .head-stats { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .head-stats span {
    display: inline-flex; align-items: baseline; gap: 5px;
    padding: 7px 10px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--bg-deep); color: var(--text-secondary); font-size: 0.76rem;
  }
  .head-stats strong { color: var(--text-primary); font-size: 0.95rem; }
  h1 { font-size: 1.5rem; font-weight: 700; color: var(--text-primary); }
  header p { color: var(--text-secondary); font-size: 0.85rem; margin-top: 2px; }

  .error-banner {
    display: flex; align-items: center; gap: 8px;
    padding: 10px 14px; border: 1px solid rgba(248,113,113,0.3); border-radius: 8px;
    background: rgba(248,113,113,0.08); color: #f87171; font-size: 13px;
  }
  .error-banner span { flex: 1; }
  .retry-link {
    padding: 4px 10px; border: 1px solid rgba(248,113,113,0.3); border-radius: 6px;
    background: transparent; color: #f87171; font-size: 12px; cursor: pointer;
  }
  .retry-link:hover { background: rgba(248,113,113,0.1); }

  .tabs { display: flex; gap: 6px; flex-wrap: wrap; }
  .tabs button {
    border: none; border-radius: var(--radius-full); padding: 8px 18px;
    color: var(--text-secondary); background: var(--bg-card); cursor: pointer;
    font-size: 0.85rem; transition: all 0.2s;
  }
  .tabs button:hover { color: var(--text-primary); }
  .tabs button.active { background: var(--accent); color: #fff; }

  .toolbar { display: flex; gap: 10px; }
  .search-box {
    flex: 1; display: flex; align-items: center; gap: 8px;
    background: var(--bg-card); border: 1px solid var(--border);
    border-radius: var(--radius-full); padding: 10px 18px; color: var(--text-muted);
  }
  .search-box:focus-within { border-color: var(--accent); }
  .search-box input {
    flex: 1; border: none; background: transparent; color: var(--text-primary);
    font-size: 0.9rem; outline: none;
  }
  .btn-search {
    border: none; border-radius: var(--radius-full); padding: 10px 24px;
    background: var(--accent); color: #fff; font-weight: 600; cursor: pointer;
    font-size: 0.9rem; transition: opacity 0.2s;
  }
  .btn-search:hover:not(:disabled) { opacity: 0.85; }
  .btn-search:disabled { opacity: 0.5; cursor: not-allowed; }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 14px; }
  .grid.compact { grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); }
  .editorial-rail {
    min-width: 0;
    width: 100%;
    max-width: 100%;
    box-sizing: border-box;
    contain: inline-size;
    display: flex;
    gap: 14px;
    overflow-x: auto;
    overscroll-behavior-inline: contain;
    scroll-snap-type: x proximity;
    padding: 2px 2px 10px;
  }

  /* Result card */
  .result-card { padding: 0; overflow: hidden; display: flex; flex-direction: column; cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; }
  .editorial-card {
    flex: 0 0 clamp(220px, 29vw, 340px);
    scroll-snap-align: start;
  }
  .result-card:hover { transform: translateY(-2px); box-shadow: var(--shadow-hover); }
  .cover-wrap { position: relative; aspect-ratio: 3/4; background: var(--aura-inset); }
  .cover-wrap :global(.cached-image) {
    width: 100%; aspect-ratio: 3/4; object-fit: cover; display: block;
  }
  .cover-placeholder {
    width: 100%; aspect-ratio: 3/4; display: flex; align-items: center; justify-content: center;
    background: var(--bg-hover); color: var(--text-muted); position: relative;
  }
  .source-badge {
    position: absolute; top: 8px; left: 8px;
    padding: 3px 8px; border-radius: var(--radius-full);
    background: rgba(0,0,0,0.6); color: #fff; font-size: 0.7rem;
    text-transform: uppercase; font-weight: 600; letter-spacing: 0;
  }
  .meta { padding: 14px; display: flex; flex-direction: column; gap: 6px; flex: 1; }
  .title { font-size: 0.95rem; font-weight: 600; color: var(--text-primary); line-height: 1.3; }
  .year { font-family: var(--font-mono); font-size: 0.75rem; color: var(--accent); }
  .rating { display: inline-flex; align-items: center; gap: 3px; font-size: 0.8rem; color: var(--color-warning); }
  .dev { font-size: 0.75rem; }
  .desc {
    font-size: 0.8rem; color: var(--text-secondary); line-height: 1.5;
    display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical;
    overflow: hidden; margin-top: 2px;
  }
  .desc.empty { font-style: italic; opacity: 0.5; }
  .tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: auto; padding-top: 6px; }
  .tag {
    padding: 2px 8px; border-radius: var(--radius-full);
    background: var(--bg-hover); color: var(--text-secondary); font-size: 0.7rem;
  }
  .tag.more { background: transparent; color: var(--text-muted); }
  .age {
    padding: 1px 8px; border-radius: var(--radius-full); font-size: 0.65rem; font-weight: 600;
    background: var(--accent-lo); color: var(--accent); align-self: flex-start;
  }
  .dl-badge {
    padding: 1px 8px; border-radius: var(--radius-full); font-size: 0.65rem; font-weight: 600;
    background: rgba(34,197,94,0.12); color: var(--color-success); align-self: flex-start;
    display: inline-flex; align-items: center; gap: 3px;
  }

  /* card actions bar */
  .card-actions {
    display: flex; gap: 6px; padding: 0 14px 12px 14px; margin-top: auto;
  }
  .act-btn {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 5px 12px; border: 1px solid var(--border); border-radius: var(--radius-full);
    background: var(--bg-hover); color: var(--text-secondary); cursor: pointer;
    font-size: 0.72rem; transition: all 0.15s;
  }
  .act-btn:hover { border-color: var(--accent); color: var(--text-primary); background: var(--accent-lo); }
  .act-btn.primary { border-color: var(--accent); color: var(--accent); }
  .act-btn.primary:hover { background: var(--accent); color: #fff; }

  .url-hint {
    position: absolute; top: 8px; right: 8px;
    width: 22px; height: 22px; border-radius: 50%;
    background: rgba(0,0,0,0.5); color: #fff; display: flex; align-items: center; justify-content: center;
    opacity: 0; transition: opacity 0.2s;
  }
  .cover-wrap:hover .url-hint, .cover-placeholder:hover .url-hint { opacity: 0.85; }

  .rec-card, .facet-card, .coll-card {
    padding: 16px; display: flex; flex-direction: column; gap: 8px;
  }
  .rec-row { display: flex; align-items: center; gap: 10px; }
  .score { font-family: var(--font-mono); font-size: 0.8rem; color: var(--accent); }
  .count { font-family: var(--font-mono); font-size: 0.8rem; color: var(--text-muted); }
  p { color: var(--text-secondary); font-size: 0.8rem; line-height: 1.4; }

  .page {
    position: relative;
    isolation: isolate;
    min-width: 0;
    background: var(--bg-void);
    color: var(--text-primary);
  }
  .page > header,
  .tabs,
  .toolbar {
    border: 1px solid var(--border);
    background: var(--bg-card);
    box-shadow: var(--shadow-xs);
    border-radius: 8px;
  }
  .page > header { padding: 18px 20px; }
  .tabs, .toolbar { padding: 10px; }
  .editorial-rail {
    border: 1px solid var(--border);
    background: rgba(7, 9, 15, 0.22);
    border-radius: 8px;
    padding: 12px;
  }
  .search-box,
  .tabs button,
  .result-card,
  .rec-card,
  .facet-card,
  .coll-card {
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg-card);
  }
  .tabs button.active,
  .btn-search {
    background: var(--accent);
  }
</style>
