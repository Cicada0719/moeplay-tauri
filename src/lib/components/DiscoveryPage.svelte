<script lang="ts">
  import { onMount } from "svelte";
  import EmptyState from "./ui/EmptyState.svelte";
  import Icon from "./Icon.svelte";
  import SakuraParticles from "./SakuraParticles.svelte";
  import Card from "./ui/Card.svelte";
  import Tag from "./ui/Tag.svelte";
  import Button from "./ui/Button.svelte";
  import SearchInput from "./ui/SearchInput.svelte";
  import SegmentControl from "./ui/SegmentControl.svelte";
  import Rail from "./ui/Rail.svelte";
  import DiscoveryCard from "./DiscoveryCard.svelte";
  import {
    getRecommendations,
    getSmartCollections,
    getDashboardData,
    scrapeGames,
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
      const resp = await scrapeGames(query, true, true, {
        kungal: true, steam: true, pcgw: true, erogamescape: true, ymgal: true, touchgal: true,
      });
      results = resp.results;
    } catch (e) {
      searchError = String(e);
      console.error("Search failed:", e);
    } finally {
      searching = false;
    }
  }

  function handleCardSelect(r: ScrapeResult) {
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
      <Button variant="ghost" size="sm" press={loadRecommendations}>重试</Button>
    </div>
  {/if}

  <nav class="tabs">
    <SegmentControl
      options={tabs.map(t => ({ value: t, label: t }))}
      value={active}
      onChange={(v) => active = v}
      size="sm"
    />
  </nav>

  {#if active === "搜索"}
    {#if selectedResult}
      <DiscoveryDetail result={selectedResult} onClose={() => selectedResult = null} />
    {:else}
    <div class="toolbar">
      <SearchInput bind:value={query} placeholder="搜索游戏、开发商或 Steam ID" class="search-field" />
      <Button variant="primary" press={search} disabled={searching}>
        {searching ? "搜索中..." : "搜索"}
      </Button>
    </div>

    {#if searching}
      <Rail title="搜索中" loading skeletonCount={6} itemWidth="clamp(220px, 29vw, 340px)">
        {#each [] as _}{/each}
      </Rail>
    {:else if results.length}
      <Rail title="搜索结果" itemWidth="clamp(220px, 29vw, 340px)">
        {#each results as r}
          <DiscoveryCard result={r} onSelect={handleCardSelect} />
        {/each}
      </Rail>
    {:else if searchError}
      <EmptyState title="搜索失败" description={searchError} action={{ label: "重试", onclick: search }} />
    {:else}
      <EmptyState title="等待搜索" description="并发查询 Bangumi / VNDB / Kungal / Steam / PCGW / Ymgal 等" />
    {/if}
    {/if}

  {:else if active === "推荐"}
    <div class="grid compact">
      {#each recommendations as item}
        <Card class="rec-card" padding="md" hoverable>
          <strong>{item.name}</strong>
          <div class="rec-row">
            <Tag variant="accent" size="sm">推荐分 {item.score}</Tag>
          </div>
          <p>{item.reasons?.join(" / ")}</p>
        </Card>
      {:else}
        <EmptyState title="暂无推荐" description="添加更多游戏后 AI 会生成个性化推荐" />
      {/each}
    </div>

  {:else if active === "开发商"}
    <div class="grid compact">
      {#each topDevs.slice(0, 20) as d}
        <Card class="facet-card" padding="md" hoverable>
          <strong>{d.name}</strong>
          <Tag variant="muted" size="sm">{d.count} 款</Tag>
        </Card>
      {:else}
        <EmptyState title="暂无数据" />
      {/each}
    </div>

  {:else if active === "标签"}
    <div class="grid compact">
      {#each topTags.slice(0, 30) as t}
        <Card class="facet-card" padding="md" hoverable>
          <strong>{t.name}</strong>
          <Tag variant="muted" size="sm">{t.count} 次</Tag>
        </Card>
      {:else}
        <EmptyState title="暂无数据" />
      {/each}
    </div>

  {:else if active === "年份" || active === "评分"}
    <div class="grid compact">
      {#each collections.filter(c => c.id?.includes(active === "年份" ? "year" : "rating") || c.id?.includes(active === "年份" ? "month" : "score")).slice(0, 12) as c}
        <Card class="coll-card" padding="md" hoverable>
          <strong>{c.name}</strong>
          <Tag variant="muted" size="sm">{c.game_count} 款</Tag>
          <p>{c.description}</p>
        </Card>
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

  .tabs { display: flex; gap: 6px; flex-wrap: wrap; }

  .toolbar { display: flex; gap: 10px; }
  :global(.search-field) { flex: 1; }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 14px; }
  .grid.compact { grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); }

  :global(.rec-card), :global(.facet-card), :global(.coll-card) {
    display: flex; flex-direction: column; gap: 8px;
  }
  .rec-row { display: flex; align-items: center; gap: 10px; }
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
</style>
