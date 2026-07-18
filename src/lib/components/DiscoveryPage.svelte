<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import Card from "./ui/Card.svelte";
  import Tag from "./ui/Tag.svelte";
  import Button from "./ui/Button.svelte";
  import SearchInput from "./ui/SearchInput.svelte";
  import SegmentControl from "./ui/SegmentControl.svelte";
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
  import AiExperienceWorkbench from "./ai/AiExperienceWorkbench.svelte";
  import RecommendationExplanation from "./ai/RecommendationExplanation.svelte";
  import {
    GenerationGuard,
    getAiClient,
    isAbortError,
    isAiUnavailableError,
    validateRecommendationExplanations,
  } from "../features/ai";
  import type { NaturalLanguageFilterDsl, ValidatedRecommendationExplanation } from "../features/ai/types";
  import { platformStore } from "../platform";
  import { settingsStore } from "../stores/settings.svelte";
  import { i18n } from "../stores/i18n.svelte";
  import { PageShell, PageHeader, FilterBar, ContentGrid, StateBoundary, type ViewState } from "./ui-v2";

  type TabId = "search" | "recommend" | "ai" | "developers" | "tags" | "years" | "ratings";

  // 7 个 tab：稳定 id + i18n 文案键，切换语言时 SegmentControl 标签同步更新。
  const tabs: { id: TabId; labelKey: string }[] = [
    { id: "search", labelKey: "discovery.tab_search" },
    { id: "recommend", labelKey: "discovery.tab_recommend" },
    { id: "ai", labelKey: "discovery.tab_ai" },
    { id: "developers", labelKey: "discovery.tab_developers" },
    { id: "tags", labelKey: "discovery.tab_tags" },
    { id: "years", labelKey: "discovery.tab_years" },
    { id: "ratings", labelKey: "discovery.tab_ratings" },
  ];
  const tabOptions = $derived(tabs.map((tab) => ({ value: tab.id as string, label: i18n.t(tab.labelKey) })));

  let active = $state<TabId>("search");
  let query = $state("");
  let results = $state<ScrapeResult[]>([]);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let selectedResult = $state<ScrapeResult | null>(null);
  let recommendations = $state<{ game_id: string; name: string; score: number; reasons?: string[] }[]>([]);
  let collections = $state<Collection[]>([]);
  let facetLoading = $state(true);
  let loadError = $state<string | null>(null);
  let aiRecommendationState = $state<"idle" | "loading" | "ready" | "offline" | "error" | "cancelled">("idle");
  let aiExplanations = $state<Record<string, string>>({});
  let appliedFilterDsl = $state<NaturalLanguageFilterDsl | null>(null);
  const aiClient = getAiClient();
  const recommendationGuard = new GenerationGuard();

  // Facet data
  let topDevs = $state<{ name: string; count: number }[]>([]);
  let topTags = $state<{ name: string; count: number }[]>([]);
  const maxDeveloperCount = $derived(Math.max(1, ...topDevs.map((item) => item.count)));
  const maxTagCount = $derived(Math.max(1, ...topTags.map((item) => item.count)));

  function collectionMatches(collection: Collection, kind: "years" | "ratings"): boolean {
    const id = collection.id ?? "";
    return kind === "years" ? id.includes("year") || id.includes("month") : id.includes("rating") || id.includes("score");
  }
  const yearCollections = $derived(collections.filter((c) => collectionMatches(c, "years")).slice(0, 12));
  const ratingCollections = $derived(collections.filter((c) => collectionMatches(c, "ratings")).slice(0, 12));

  // 三态统一：各分区的 加载 / 错误 / 空 / 就绪 全部收敛到 StateBoundary。
  const searchViewState = $derived<ViewState>(
    searching ? "loading" : searchError ? "error" : results.length ? "ready" : "empty",
  );
  const recommendViewState = $derived<ViewState>(
    facetLoading ? "loading" : loadError ? "error" : recommendations.length ? "ready" : "empty",
  );
  const devsViewState = $derived<ViewState>(
    facetLoading ? "loading" : loadError ? "error" : topDevs.length ? "ready" : "empty",
  );
  const tagsViewState = $derived<ViewState>(
    facetLoading ? "loading" : loadError ? "error" : topTags.length ? "ready" : "empty",
  );
  const yearsViewState = $derived<ViewState>(
    facetLoading ? "loading" : loadError ? "error" : yearCollections.length ? "ready" : "empty",
  );
  const ratingsViewState = $derived<ViewState>(
    facetLoading ? "loading" : loadError ? "error" : ratingCollections.length ? "ready" : "empty",
  );

  const searchSourcesDesc = $derived(
    settingsStore.settings.getchu_enabled
      ? i18n.t("discovery.search_sources_getchu")
      : platformStore.capabilities.steamIntegration
        ? i18n.t("discovery.search_sources_steam")
        : i18n.t("discovery.search_sources"),
  );

  async function search() {
    if (!query.trim()) return;
    searching = true;
    searchError = null;
    try {
      const resp = await scrapeGames(query, true, true, {
        getchu: settingsStore.settings.getchu_enabled ?? false,
        kungal: true,
        steam: platformStore.capabilities.steamIntegration,
        pcgw: true,
        erogamescape: true,
        ymgal: true,
        touchgal: true,
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
    facetLoading = true;
    try {
      const [recs, cols, dash] = await Promise.all([
        getRecommendations(null, 12),
        getSmartCollections(),
        getDashboardData(),
      ]);
      recommendations = (recs ?? []) as { game_id: string; name: string; score: number; reasons?: string[] }[];
      void loadAiRecommendationExplanations(recommendations);
      collections = cols ?? [];
      topDevs = (dash as any)?.top_developers ?? [];
      topTags = (dash as any)?.top_tags ?? [];
      loadError = null;
    } catch (e) {
      loadError = String(e);
      console.error("Discovery mount failed:", e);
    } finally {
      facetLoading = false;
    }
  }


  async function loadAiRecommendationExplanations(items: { game_id: string }[]) {
    recommendationGuard.cancel();
    aiExplanations = {};
    if (!items.length) {
      aiRecommendationState = "idle";
      return;
    }
    const request = recommendationGuard.begin();
    aiRecommendationState = "loading";
    try {
      const result = await aiClient.recommend({
        kind: "game",
        candidateIds: items.map((item) => item.game_id),
        limit: items.length,
        generation: request.generation,
      }, request.signal);
      if (!recommendationGuard.isCurrent(request.generation) || result.generation !== request.generation) return;
      const validation = validateRecommendationExplanations(result.explanations, items.map((item) => item.game_id));
      if (!validation.ok) {
        aiRecommendationState = "error";
        return;
      }
      aiExplanations = Object.fromEntries(validation.value.map((item: ValidatedRecommendationExplanation) => [item.resourceId, item.explanation]));
      aiRecommendationState = "ready";
    } catch (error) {
      if (!recommendationGuard.isCurrent(request.generation) || isAbortError(error)) return;
      aiRecommendationState = isAiUnavailableError(error) ? "offline" : "error";
    }
  }

  function applyCompiledFilter(dsl: NaturalLanguageFilterDsl) {
    appliedFilterDsl = dsl;
  }

  onMount(() => {
    void loadRecommendations();
    return () => recommendationGuard.cancel();
  });
</script>

<PageShell as="div" width="full" scrollable={false} class="discovery-v2-shell" labelledBy="discovery-page-title" ariaLabel={i18n.t("discovery.title")}>
  <div class="dc">
    <div class="v2-grain dc-grain" aria-hidden="true"></div>

    <PageHeader
      id="discovery-page-title"
      class="dc-header"
      eyebrow="発見 / DISCOVERY"
      title={i18n.t("discovery.title")}
      description={i18n.t("discovery.subtitle")}
    >
      {#snippet actions()}
        <div class="dc-stats" aria-label={i18n.t("discovery.title")}>
          <span><strong>{recommendations.length}</strong> {i18n.t("discovery.stats_recommendations")}</span>
          <span><strong>{collections.length}</strong> {i18n.t("discovery.stats_collections")}</span>
        </div>
      {/snippet}
    </PageHeader>

    <div class="dc-tabs">
      <FilterBar label={i18n.t("discovery.tabs_aria")}>
        <SegmentControl options={tabOptions} value={active} onChange={(v) => (active = v as TabId)} size="sm" />
      </FilterBar>
    </div>

    <main class="dc-content">
      {#if active === "search"}
        {#if selectedResult}
          <DiscoveryDetail result={selectedResult} onClose={() => (selectedResult = null)} />
        {:else}
          <FilterBar label={i18n.t("discovery.filters_aria")}>
            <SearchInput
              bind:value={query}
              placeholder={platformStore.capabilities.steamIntegration ? i18n.t("discovery.search_placeholder_steam") : i18n.t("discovery.search_placeholder")}
              class="dc-search-field"
            />
            {#snippet actions()}
              <Button variant="primary" press={search} disabled={searching}>
                {searching ? i18n.t("discovery.searching") : i18n.t("discovery.search_button")}
              </Button>
            {/snippet}
          </FilterBar>

          <StateBoundary
            state={searchViewState}
            onRetry={search}
            retryLabel={i18n.t("button.retry")}
            title={searchViewState === "error" ? i18n.t("discovery.search_error_title") : i18n.t("discovery.search_idle_title")}
            description={searchViewState === "error" ? (searchError ?? undefined) : searchSourcesDesc}
            loadingRows={4}
          >
            <ContentGrid label={i18n.t("discovery.search_results")} minItemWidth="15rem">
              {#each results as r}
                <DiscoveryCard result={r} onSelect={handleCardSelect} />
              {/each}
            </ContentGrid>
          </StateBoundary>
        {/if}

      {:else if active === "recommend"}
        <StateBoundary
          state={recommendViewState}
          onRetry={loadRecommendations}
          retryLabel={i18n.t("button.retry")}
          title={recommendViewState === "error" ? i18n.t("discovery.load_error_title") : i18n.t("discovery.rec_empty_title")}
          description={recommendViewState === "error" ? (loadError ?? undefined) : i18n.t("discovery.rec_empty_desc")}
          loadingRows={4}
        >
          <ContentGrid label={i18n.t("discovery.tab_recommend")} minItemWidth="15rem">
            {#each recommendations as item}
              <Card class="rec-card" padding="md">
                <strong>{item.name}</strong>
                <div class="rec-row">
                  <Tag variant="accent" size="sm">{i18n.t("discovery.rec_score", { score: item.score })}</Tag>
                </div>
                <RecommendationExplanation
                  localSignals={item.reasons ?? []}
                  aiExplanation={aiExplanations[item.game_id] ?? null}
                  aiState={aiRecommendationState}
                />
              </Card>
            {/each}
          </ContentGrid>
        </StateBoundary>

      {:else if active === "ai"}
        {#if appliedFilterDsl}
          <div class="applied-filter-banner">
            <Icon name="check" size={15} />
            <span>{i18n.t("discovery.ai_filter_applied", { count: appliedFilterDsl.filters.length })}</span>
            <Button variant="quiet" size="sm" press={() => (appliedFilterDsl = null)}>{i18n.t("discovery.clear")}</Button>
          </div>
        {/if}
        <AiExperienceWorkbench client={aiClient} onApplyFilter={applyCompiledFilter} />

      {:else if active === "developers"}
        <StateBoundary
          state={devsViewState}
          onRetry={loadRecommendations}
          retryLabel={i18n.t("button.retry")}
          title={devsViewState === "error" ? i18n.t("discovery.load_error_title") : i18n.t("discovery.facet_empty_title")}
          description={devsViewState === "error" ? (loadError ?? undefined) : undefined}
          loadingRows={5}
        >
          <div class="facet-editorial">
            <header><span>STUDIO INDEX</span><h2>{i18n.t("discovery.devs_title")}</h2><p>{i18n.t("discovery.devs_desc")}</p></header>
            <div class="facet-lines">
              {#each topDevs.slice(0, 20) as d, index}
                <article><span>{String(index + 1).padStart(2, "0")}</span><strong>{d.name}</strong><i style={`--facet:${(d.count / maxDeveloperCount) * 100}%`}></i><small>{i18n.t("discovery.count_games", { count: d.count })}</small></article>
              {/each}
            </div>
          </div>
        </StateBoundary>

      {:else if active === "tags"}
        <StateBoundary
          state={tagsViewState}
          onRetry={loadRecommendations}
          retryLabel={i18n.t("button.retry")}
          title={tagsViewState === "error" ? i18n.t("discovery.load_error_title") : i18n.t("discovery.facet_empty_title")}
          description={tagsViewState === "error" ? (loadError ?? undefined) : undefined}
          loadingRows={5}
        >
          <div class="facet-editorial tag-editorial">
            <header><span>TAG LANDSCAPE</span><h2>{i18n.t("discovery.tags_title")}</h2><p>{i18n.t("discovery.tags_desc")}</p></header>
            <div class="tag-field">
              {#each topTags.slice(0, 30) as t}
                <span class="tag-chip" style={`--weight:${Math.max(0.45, t.count / maxTagCount)}`}><strong>{t.name}</strong><small>{t.count}</small></span>
              {/each}
            </div>
          </div>
        </StateBoundary>

      {:else if active === "years" || active === "ratings"}
        {@const isYears = active === "years"}
        {@const sectionState = isYears ? yearsViewState : ratingsViewState}
        {@const list = isYears ? yearCollections : ratingCollections}
        <StateBoundary
          state={sectionState}
          onRetry={loadRecommendations}
          retryLabel={i18n.t("button.retry")}
          title={sectionState === "error" ? i18n.t("discovery.load_error_title") : i18n.t("discovery.facet_empty_title")}
          description={sectionState === "error" ? (loadError ?? undefined) : i18n.t("discovery.collections_empty_desc")}
          loadingRows={4}
        >
          <div class="collection-editorial">
            <header><span>{isYears ? "TIME INDEX" : "SCORE INDEX"}</span><h2>{isYears ? i18n.t("discovery.years_title") : i18n.t("discovery.ratings_title")}</h2></header>
            <div>
              {#each list as c, index}
                <Card class="coll-card" padding="md"><span class="coll-no">{String(index + 1).padStart(2, "0")}</span><strong>{c.name}</strong><Tag variant="muted" size="sm">{i18n.t("discovery.count_games", { count: c.game_count })}</Tag><p>{c.description}</p></Card>
              {/each}
            </div>
          </div>
        </StateBoundary>
      {/if}
    </main>
  </div>
</PageShell>

<style>
  :global(.discovery-v2-shell) { height: 100%; }
  :global(.discovery-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .dc {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .dc-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.dc-header) {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto;
    padding: 26px 28px 14px;
    flex-shrink: 0;
  }
  .dc-stats { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .dc-stats span {
    display: inline-flex; align-items: baseline; gap: 5px;
    padding: 7px 10px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--bg-deep); color: var(--text-secondary); font-size: 0.76rem;
  }
  .dc-stats strong { color: var(--text-primary); font-size: 0.95rem; }

  .dc-tabs {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto 14px;
    padding: 0 28px;
  }
  .dc-content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto;
    padding: 0 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    scroll-behavior: smooth;
  }
  :global(.dc-search-field) { flex: 1; }

  /* ── AI filter applied banner ── */
  .applied-filter-banner {
    display: flex; align-items: center; gap: 9px;
    padding: 10px 12px; border: 1px solid color-mix(in srgb, var(--color-success, #4ade80) 35%, var(--border)); border-radius: 8px;
    background: var(--bg-card); color: var(--color-success, #4ade80); font-size: 11px;
  }
  .applied-filter-banner span { flex: 1; color: var(--text-secondary); }

  /* ── Recommendation / collection cards ── */
  :global(.rec-card), :global(.coll-card) { display: flex; flex-direction: column; gap: 8px; }
  .rec-row { display: flex; align-items: center; gap: 10px; }
  p { color: var(--text-secondary); font-size: 0.8rem; line-height: 1.4; }

  /* ── Facet editorials (kept from the legacy page) ── */
  .facet-editorial,.collection-editorial { display:grid; grid-template-columns:minmax(220px,.36fr) minmax(0,1fr); gap:clamp(24px,5vw,72px); padding:clamp(22px,4vw,56px); border:1px solid var(--border); background:linear-gradient(125deg,color-mix(in srgb,var(--bg-card) 92%,transparent),transparent); }
  .facet-editorial>header>span,.collection-editorial>header>span { color:var(--accent); font:700 8px/1 var(--font-mono); letter-spacing:.16em; }
  .facet-editorial h2,.collection-editorial h2 { margin:14px 0 8px; font:720 clamp(1.8rem,3vw,3.8rem)/.92 var(--font-display); letter-spacing:-.055em; }
  .facet-editorial header p { max-width:42ch; color:var(--text-muted); font-size:11px; line-height:1.65; }
  .facet-lines { border-top:1px solid var(--border); }
  .facet-lines article { min-height:46px; display:grid; grid-template-columns:34px minmax(120px,.55fr) minmax(100px,1fr) 52px; align-items:center; gap:12px; border-bottom:1px solid var(--border); }
  .facet-lines article>span,.facet-lines small,.coll-no { color:var(--text-dim); font:700 8px/1 var(--font-mono); }
  .facet-lines article>strong { overflow:hidden; font-size:12px; text-overflow:ellipsis; white-space:nowrap; }
  .facet-lines i { width:var(--facet); height:3px; background:var(--accent); }
  .facet-lines small { text-align:right; }
  .tag-field { align-content:start; display:flex; flex-wrap:wrap; gap:6px; }
  .tag-field .tag-chip { display:flex; align-items:baseline; gap:7px; padding:8px 10px; border:1px solid var(--border); border-radius:1px; background:transparent; color:var(--text-primary); opacity:calc(.45 + var(--weight)*.55); }
  .tag-field strong { font-size:calc(.72rem + var(--weight)*.36rem); }
  .tag-field small { color:var(--text-muted); font:700 8px/1 var(--font-mono); }
  .collection-editorial>div { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:8px; }
  :global(.collection-editorial .coll-card) { position:relative; min-height:150px; border-radius:2px; }
  .coll-no { position:absolute; right:12px; top:12px; }

  /* ── Responsive ── */
  @media (max-width: 760px) {
    .facet-editorial,.collection-editorial { grid-template-columns:1fr; }
    .collection-editorial>div { grid-template-columns:1fr; }
    .facet-lines article { grid-template-columns:28px minmax(0,1fr) 48px; }
    .facet-lines i { display:none; }
    .dc-content { padding: 0 16px 36px; }
    .dc-tabs { padding: 0 16px; }
    :global(.dc-header) { padding: 20px 16px 12px; }
  }
  @media (max-width: 620px) {
    .dc-content { padding-inline: 12px; }
    .dc-tabs { padding-inline: 12px; }
    :global(.dc-header) { padding-inline: 12px; }
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .dc-content { scroll-behavior: auto; }
  }
  :global([data-motion="reduce"]) .dc-content { scroll-behavior: auto; }
</style>
