<script lang="ts">
  import { onMount } from "svelte";
  import { animeStore, COLLECT_TYPES } from "../stores/anime.svelte";
  import type { SearchItem, BangumiSubject } from "../stores/anime.svelte";
  import AnimeDetail from "./anime/AnimeDetail.svelte";
  import AnimePlayer from "./anime/AnimePlayer.svelte";
  import SearchDrawer from "./anime/SearchDrawer.svelte";
  import SourceSheet from "./anime/SourceSheet.svelte";
  import ProviderV2Workspace from "./anime/provider-v2/ProviderV2Workspace.svelte";
  import AnimeMediaSection from "./anime/AnimeMediaSection.svelte";
  import { focusRovingItem, nextRovingIndex } from "./anime/a11y";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, SegmentControl, StatBlock, Tag } from "./ui";
  import { AsyncSection, FilterBar, MediaCard, PageHeader, PageShell } from "./ui-v2";
  import type { ViewState } from "./ui-v2";

  let searchInput = $state("");
  let isSearching = $state(false);
  let importText = $state("");
  let importMsg = $state("");
  let providerV2Active = $state(false);
  let detailReturnFocus = $state<HTMLElement | null>(null);
  let providerV2ReturnFocus = $state<HTMLElement | null>(null);
  let mainTabRefs: Array<HTMLButtonElement | null> = [];

  const MAIN_TABS = [
    { id: "recommend", label: "推荐", icon: "star" },
    { id: "calendar", label: "时间表", icon: "calendar" },
    { id: "my", label: "我的", icon: "user" },
    { id: "rules", label: "规则", icon: "settings" },
  ] as const;
  type MainTab = (typeof MAIN_TABS)[number]["id"];

  function mediaSectionState(loading: boolean, count: number): ViewState {
    if (loading && count === 0) return "loading";
    if (loading && count > 0) return "refreshing";
    return count > 0 ? "ready" : "empty";
  }

  function searchSectionState(): ViewState {
    const resultCount = animeStore.searchResults.reduce((total, [, items]) => total + items.length, 0);
    if (animeStore.loading && resultCount === 0) return "loading";
    if (animeStore.loading && resultCount > 0) return "refreshing";
    if (animeStore.error?.includes("未找到")) return "no-results";
    if (animeStore.error) return "error";
    return resultCount > 0 ? "ready" : "empty";
  }

  async function handleSearch(e: Event) {
    e.preventDefault();
    if (!searchInput.trim()) return;
    isSearching = true;
    await animeStore.search(searchInput.trim());
    isSearching = false;
  }

  function clearSearch() {
    searchInput = "";
    animeStore.goHome();
  }

  async function handleImport() {
    if (!importText.trim()) return;
    importMsg = "";
    try {
      const count = await animeStore.importRules(importText.trim());
      importMsg = `导入成功: ${count} 条规则`;
      importText = "";
    } catch (e) {
      importMsg = `导入失败: ${e}`;
    }
  }

  function openResult(ruleName: string, item: SearchItem, trigger?: HTMLElement) {
    detailReturnFocus = trigger ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    animeStore.openDetail(ruleName, item);
  }

  function searchBangumi(subject: BangumiSubject, trigger?: HTMLElement) {
    // 点封面 → 进 Bangumi 详情页（简介/吐槽/角色/制作人员），而非直接插件搜索。
    // 选播放源由详情页的「开始观看」触发。
    detailReturnFocus = trigger ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    animeStore.openInfo(subject);
  }


  function activateMainTab(tab: MainTab, index = MAIN_TABS.findIndex((item) => item.id === tab)) {
    animeStore.setTab(tab);
    if (index >= 0) focusRovingItem(mainTabRefs, index);
  }

  function handleMainTabKeydown(event: KeyboardEvent, index: number) {
    const next = nextRovingIndex(event.key, index, MAIN_TABS.length, "horizontal");
    if (next === null) return;
    event.preventDefault();
    activateMainTab(MAIN_TABS[next].id, next);
  }

  function openProviderV2(event: MouseEvent) {
    providerV2ReturnFocus = event.currentTarget as HTMLElement;
    providerV2Active = true;
  }

  function closeProviderV2() {
    providerV2Active = false;
    queueMicrotask(() => providerV2ReturnFocus?.focus({ preventScroll: true }));
  }

  function fmtDate(ts: number) {
    if (!ts) return "";
    const d = new Date(ts);
    return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`;
  }

  function fmtRelativeTime(iso: string): string {
    if (!iso) return "";
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "刚刚";
    if (mins < 60) return `${mins}分钟前`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}小时前`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}天前`;
    return fmtDate(new Date(iso).getTime());
  }

  function onKeydown(e: KeyboardEvent) {
    if (providerV2Active) return;
    if (e.key === "Escape") {
      if (animeStore.view === "player") {
        return; // 让播放器自己处理 Escape（先退出全屏，再关闭）
      } else if (animeStore.view === "detail") {
        e.stopImmediatePropagation();
        animeStore.closeDetail();
      } else if (animeStore.view === "search") {
        e.stopImmediatePropagation();
        animeStore.goHome();
      }
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKeydown, { capture: true });
    animeStore.init();
    if (animeStore.activeTab === "recommend") {
      animeStore.loadRecommendations();
    }
    return () => window.removeEventListener("keydown", onKeydown, { capture: true });
  });

  const WEEKDAY_NAMES = ["", "周一", "周二", "周三", "周四", "周五", "周六", "周日"];
</script>

<PageShell as="div" ariaLabel="番剧主内容" width="full" class="anime-page">
  <section class="anime-page-frame" data-testid="anime-page">
  <div class="anime-shell" class:hidden-by-overlay={providerV2Active || animeStore.view === "detail" || animeStore.view === "player"}>
    <!-- Header -->
    {#snippet pageActions()}
      <button class="provider-v2-entry" type="button" onclick={openProviderV2}>
        <Icon name="layers" size={15} />
        <span>Provider v2</span>
      </button>
    {/snippet}

    <PageHeader
      title="番剧"
      eyebrow="Anime"
      description="在 Bangumi 元数据、经典规则源与 Provider v2 之间安全浏览和播放。"
      actions={pageActions}
      id="anime-page-title"
    />

    {#snippet searchControls()}
      <form class="search-form" onsubmit={handleSearch}>
        <label class="search-wrap" aria-label="搜索番剧">
          <Icon name="search" size={15} />
          <input
            class="search-input"
            type="search"
            placeholder="搜索番剧..."
            bind:value={searchInput}
            aria-busy={animeStore.loading}
            data-search-scope="anime"
          />
          {#if searchInput}
            <button class="search-clear" type="button" aria-label="清空搜索" onclick={clearSearch}>
              <Icon name="x" size={13} />
            </button>
          {/if}
        </label>

        {#if animeStore.rules.length > 0}
          <label class="rule-filter">
            <span class="sr-only">搜索来源</span>
            <select class="rule-select" aria-label="搜索来源" onchange={(e) => animeStore.setSelectedRule((e.currentTarget as HTMLSelectElement).value || null)}>
              <option value="">全部源</option>
              {#each animeStore.rules as rule (rule.name)}
                <option value={rule.name}>{rule.name}</option>
              {/each}
            </select>
          </label>
        {/if}

        <Button type="submit" variant="primary" disabled={!searchInput.trim() || isSearching}>
          {isSearching ? "搜索中…" : "搜索"}
        </Button>
      </form>
    {/snippet}

    <FilterBar
      controls={searchControls}
      label="番剧搜索与来源筛选"
      activeCount={searchInput.trim() ? 1 : 0}
      onClear={clearSearch}
      busy={isSearching}
      class="anime-filter-bar"
    />

    <!-- Tab Bar -->
    <div class="tab-bar" role="tablist" aria-label="番剧内容分类">
      {#if animeStore.view === "search"}
        <span class="search-label" aria-live="polite">搜索：“{animeStore.searchKeyword}”</span>
        <Button variant="ghost" size="sm" press={clearSearch}>清除</Button>
      {:else}
        {#each MAIN_TABS as tab, index (tab.id)}
          <button
            bind:this={mainTabRefs[index]}
            class="tab-btn"
            class:active={animeStore.activeTab === tab.id}
            type="button"
            role="tab"
            id={`anime-tab-${tab.id}`}
            aria-selected={animeStore.activeTab === tab.id}
            aria-controls={`anime-panel-${tab.id}`}
            tabindex={animeStore.activeTab === tab.id ? 0 : -1}
            onclick={() => activateMainTab(tab.id, index)}
            onkeydown={(event) => handleMainTabKeydown(event, index)}
          >
            <Icon name={tab.icon} size={14} />
            {tab.label}
          </button>
        {/each}
      {/if}
    </div>

    <!-- Content -->
    <div class="anime-content">
      {#if animeStore.loading && animeStore.view !== "search"}
        <div class="content-loading">
          <div class="spinner"></div>
          <span>加载中...</span>
        </div>

      <!-- 搜索结果 -->
      {:else if animeStore.view === "search"}
        <AsyncSection
          title={`搜索：“${animeStore.searchKeyword}”`}
          description="按来源分组展示；部分来源失败时保留已经返回的结果。"
          state={searchSectionState()}
          details={animeStore.error || undefined}
          preserveContent={animeStore.searchResults.length > 0}
          primaryAction={animeStore.error ? { label: "重新搜索", onSelect: () => void animeStore.search(animeStore.searchKeyword) } : undefined}
          class="search-results-section"
        >
          <div class="search-result-groups">
            {#each animeStore.searchResults as [source, items] (source)}
              <AsyncSection title={source} description={`${items.length} 个结果`} headingLevel={3} compact>
                <div class="result-list" role="list" aria-label={`${source} 搜索结果`}>
                  {#each items as item (item.url)}
                    <MediaCard
                      title={item.name}
                      subtitle={source}
                      variant="landscape"
                      focusKey={`anime-search-${source}-${item.url}`}
                      ariaLabel={`查看 ${item.name} 详情`}
                      onActivate={(event) => openResult(source, item, event.currentTarget as HTMLElement)}
                    />
                  {/each}
                </div>
              </AsyncSection>
            {/each}
          </div>
        </AsyncSection>

      <!-- ═══════════════════════════════════════════════════════════
           推荐页 — Kazumi 风格 (热门 + 本季新番 + 高分)
           ═══════════════════════════════════════════════════════════ -->
      {:else if animeStore.activeTab === "recommend"}
        <div
          class="rec-page"
          id="anime-panel-recommend"
          role="tabpanel"
          aria-labelledby="anime-tab-recommend"
          tabindex="0"
        >
          <AnimeMediaSection
            title="本季新番"
            description="按当前季度整理的放送作品。"
            subjects={animeStore.recSeasonal}
            state={mediaSectionState(animeStore.recSeasonalLoading, animeStore.recSeasonal.length)}
            getImage={(url) => animeStore.getImg(url)}
            onOpen={searchBangumi}
            onMore={() => animeStore.loadMoreSeasonal()}
            moreAvailable={animeStore.recSeasonalTotal > animeStore.recSeasonal.length}
            moreBusy={animeStore.recSeasonalLoading}
            accent="seasonal"
          />
          <AnimeMediaSection
            title="热门推荐"
            description="近期讨论度较高的作品。"
            subjects={animeStore.recTrending}
            state={mediaSectionState(animeStore.recTrendingLoading, animeStore.recTrending.length)}
            getImage={(url) => animeStore.getImg(url)}
            onOpen={searchBangumi}
            onMore={() => animeStore.loadMoreTrending()}
            moreAvailable={animeStore.recTrendingTotal > animeStore.recTrending.length}
            moreBusy={animeStore.recTrendingLoading}
            accent="trending"
          />
          <AnimeMediaSection
            title="Bangumi 排行"
            description="基于 Bangumi 评分与排名整理。"
            subjects={animeStore.recTopRated}
            state={mediaSectionState(animeStore.recTopRatedLoading, animeStore.recTopRated.length)}
            getImage={(url) => animeStore.getImg(url)}
            onOpen={searchBangumi}
            onMore={() => animeStore.loadMoreTopRated()}
            moreAvailable={animeStore.recTopRatedTotal > animeStore.recTopRated.length}
            moreBusy={animeStore.recTopRatedLoading}
            accent="toprated"
          />
        </div>

      <!-- ═══════════════════════════════════════════════════════════
           时间表 — Bangumi Calendar
           ═══════════════════════════════════════════════════════════ -->
      {:else if animeStore.activeTab === "calendar"}
        {#if animeStore.calendarLoading}
          <div class="content-loading">
            <div class="spinner"></div>
            <span>加载时间表...</span>
          </div>
        {:else if animeStore.calendar.length > 0}
          <div class="calendar-section">
            <div class="weekday-tabs">
              {#each animeStore.calendar as day (day.weekday)}
                <button class="weekday-tab"
                  class:active={animeStore.calendarDay === day.weekday}
                  class:today={day.weekday === (new Date().getDay() || 7)}
                  onclick={() => { animeStore.calendarDay = day.weekday; }}>
                  {day.weekday_cn}
                  <span class="weekday-count">
                    {day.items.length}
                  </span>
                  {#if day.weekday === (new Date().getDay() || 7)}
                    <span class="today-dot"></span>
                  {/if}
                </button>
              {/each}
            </div>
            {#each animeStore.calendar.filter(d => d.weekday === animeStore.calendarDay) as currentDay (currentDay.weekday)}
              <div class="cover-grid">
                {#each currentDay.items as sub (sub.id)}
                  <Card padding="none" hoverable={false} class="cover-card" onclick={() => searchBangumi(sub)}>
                    <div class="cover-img-wrap">
                      {#if animeStore.getImg(sub.image)}
                        <img src={animeStore.getImg(sub.image)} alt={sub.name_cn || sub.name} class="cover-img" />
                      {:else if sub.image}
                        <div class="cover-img-loading"><div class="spinner sm"></div></div>
                      {:else}
                        <div class="cover-img-placeholder"><Icon name="film" size={28} /></div>
                      {/if}
                      {#if sub.rating > 0}
                        <span class="cover-rating">{sub.rating.toFixed(1)}</span>
                      {/if}
                    </div>
                    <div class="cover-meta">
                      <span class="cover-title">{sub.name_cn || sub.name}</span>
                      {#if sub.name_cn && sub.name !== sub.name_cn}
                        <span class="cover-sub">{sub.name}</span>
                      {/if}
                      {#if sub.eps_count > 0}
                        <span class="cover-eps">{sub.eps_count} 话</span>
                      {/if}
                    </div>
                  </Card>
                {/each}
              </div>
            {/each}
          </div>
        {:else if animeStore.error}
          <EmptyState
            icon="x"
            title="加载失败"
            description={animeStore.error}
            action={{ label: '重试', onclick: () => animeStore.loadCalendar() }}
            class="content-empty"
          />
        {:else}
          <EmptyState icon="film" title="暂无时间表数据" class="content-empty" />
        {/if}

      <!-- ═══════════════════════════════════════════════════════════
           我的 — 收藏 + 历史 + 统计 (Kazumi 数据页)
           ═══════════════════════════════════════════════════════════ -->
      {:else if animeStore.activeTab === "my"}
        <!-- 统计卡片 -->
        <div class="my-stats-bar">
          <StatBlock label="收藏" value={animeStore.stats.total} class="stat-card" />
          <StatBlock label="在看" value={animeStore.stats.watching} class="stat-card" />
          <StatBlock label="想看" value={animeStore.stats.planned} class="stat-card" />
          <StatBlock label="看过" value={animeStore.stats.watched} class="stat-card" />
          <StatBlock label="历史" value={animeStore.stats.historyCount} class="stat-card" />
        </div>

        <!-- 子 Tab -->
        <SegmentControl
          class="my-sub-tabs"
          options={[
            { value: "collection", label: "收藏" },
            { value: "history", label: "历史记录" },
            { value: "stats", label: "数据统计" },
          ]}
          value={animeStore.mySubTab}
          onChange={(v) => { animeStore.mySubTab = v as any; }}
        />

        <!-- 收藏子页 -->
        {#if animeStore.mySubTab === "collection"}
          <!-- 分类筛选 -->
          <div class="collect-filters">
            {#each [
              { type: 0, label: "全部", count: animeStore.stats.total },
              { type: 1, label: "在看", count: animeStore.stats.watching },
              { type: 2, label: "想看", count: animeStore.stats.planned },
              { type: 3, label: "搁置", count: animeStore.stats.onHold },
              { type: 4, label: "看过", count: animeStore.stats.watched },
              { type: 5, label: "抛弃", count: animeStore.stats.dropped },
            ] as f (f.type)}
              <Tag
                active={animeStore.collectFilter === f.type}
                onclick={() => { animeStore.collectFilter = f.type; }}
                class="filter-chip"
              >
                {f.label}
                {#if f.count > 0}
                  <span class="filter-count">{f.count}</span>
                {/if}
              </Tag>
            {/each}
          </div>

          {#if animeStore.filteredCollection.length === 0}
            <EmptyState
              icon="heart"
              title={animeStore.collectFilter === 0 ? "还没有收藏" : `没有${COLLECT_TYPES[animeStore.collectFilter]}的番剧`}
              description="在番剧详情页中点击收藏按钮添加"
              class="content-empty"
            />
          {:else}
            <div class="collect-grid">
              {#each animeStore.filteredCollection as item (item.key)}
                <Card padding="none" hoverable={false} class="collect-card" onclick={() => {
                  if (item.ruleSource && item.sourceUrl) {
                    animeStore.openDetail(item.ruleSource, { name: item.name, url: item.sourceUrl }, item.image);
                  }
                }}>
                  <div class="collect-card-img">
                    {#if item.image}
                      <img src={animeStore.getImg(item.image) || item.image} alt=""
                        onerror={(e) => { (e.currentTarget as HTMLImageElement).style.display = "none"; }} />
                    {:else}
                      <div class="collect-card-placeholder"><Icon name="film" size={24} /></div>
                    {/if}
                    <span class="collect-card-type" data-type={item.collectType}>{COLLECT_TYPES[item.collectType]}</span>
                  </div>
                  <div class="collect-card-meta">
                    <span class="collect-card-name">{item.name}</span>
                    {#if item.ruleSource}
                      <span class="collect-card-source">{item.ruleSource}</span>
                    {/if}
                  </div>
                </Card>
              {/each}
            </div>
          {/if}

        <!-- 历史子页 -->
        {:else if animeStore.mySubTab === "history"}
          {#if animeStore.history.length === 0}
            <EmptyState icon="eye" title="暂无观看记录" description="开始观看番剧后将自动记录" class="content-empty" />
          {:else}
            <div class="history-toolbar">
              <span class="history-count">{animeStore.history.length} 条记录</span>
              <Button variant="ghost" size="sm" press={() => animeStore.clearHistory()} class="clear-btn">
                <Icon name="trash" size={13} />
                清空
              </Button>
            </div>
            <div class="history-list">
              {#each animeStore.history as item (item.key)}
                <div class="history-row" role="button" tabindex="0" onclick={() => {
                  animeStore.openDetail(item.ruleName, { name: item.name, url: item.sourceUrl }, item.image);
                }} onkeydown={(e) => { if (e.key === "Enter") animeStore.openDetail(item.ruleName, { name: item.name, url: item.sourceUrl }, item.image); }}>
                  <div class="history-thumb">
                    {#if item.image}
                      <img src={animeStore.getImg(item.image) || item.image} alt=""
                        onerror={(e) => { (e.currentTarget as HTMLImageElement).style.display = "none"; }} />
                    {:else}
                      <div class="history-thumb-placeholder"><Icon name="film" size={18} /></div>
                    {/if}
                  </div>
                  <div class="history-info">
                    <span class="history-name">{item.name}</span>
                    <span class="history-meta">
                      看到 {item.lastEpisodeName || `第${item.lastEpisode + 1}集`}
                    </span>
                    <span class="history-sub">
                      {item.ruleName} · {fmtRelativeTime(item.updatedAt)}
                    </span>
                  </div>
                  <Button variant="quiet" size="sm" press={(e) => { e.stopPropagation(); animeStore.removeHistory(item.key); }} ariaLabel="删除" class="remove-btn">
                    <Icon name="x" size={12} />
                  </Button>
                </div>
              {/each}
            </div>
          {/if}

        <!-- 统计子页 -->
        {:else if animeStore.mySubTab === "stats"}
          <div class="stats-page">
            <div class="stats-grid">
              <Card padding="md" class="stats-block">
                <div class="stats-block-title">收藏概览</div>
                <div class="stats-items">
                  <div class="stats-row">
                    <span class="stats-label">总收藏数</span>
                    <span class="stats-val">{animeStore.stats.total}</span>
                  </div>
                  <div class="stats-row">
                    <span class="stats-label">在看</span>
                    <span class="stats-val accent">{animeStore.stats.watching}</span>
                  </div>
                  <div class="stats-row">
                    <span class="stats-label">想看</span>
                    <span class="stats-val">{animeStore.stats.planned}</span>
                  </div>
                  <div class="stats-row">
                    <span class="stats-label">搁置</span>
                    <span class="stats-val">{animeStore.stats.onHold}</span>
                  </div>
                  <div class="stats-row">
                    <span class="stats-label">看过</span>
                    <span class="stats-val green">{animeStore.stats.watched}</span>
                  </div>
                  <div class="stats-row">
                    <span class="stats-label">抛弃</span>
                    <span class="stats-val muted">{animeStore.stats.dropped}</span>
                  </div>
                </div>
                {#if animeStore.stats.total > 0}
                  <div class="stats-bar-wrap">
                    {#if animeStore.stats.watching > 0}
                      <div class="stats-bar watching" style="flex:{animeStore.stats.watching}" title="在看 {animeStore.stats.watching}"></div>
                    {/if}
                    {#if animeStore.stats.planned > 0}
                      <div class="stats-bar planned" style="flex:{animeStore.stats.planned}" title="想看 {animeStore.stats.planned}"></div>
                    {/if}
                    {#if animeStore.stats.onHold > 0}
                      <div class="stats-bar onhold" style="flex:{animeStore.stats.onHold}" title="搁置 {animeStore.stats.onHold}"></div>
                    {/if}
                    {#if animeStore.stats.watched > 0}
                      <div class="stats-bar watched" style="flex:{animeStore.stats.watched}" title="看过 {animeStore.stats.watched}"></div>
                    {/if}
                    {#if animeStore.stats.dropped > 0}
                      <div class="stats-bar dropped" style="flex:{animeStore.stats.dropped}" title="抛弃 {animeStore.stats.dropped}"></div>
                    {/if}
                  </div>
                {/if}
              </Card>

              <Card padding="md" class="stats-block">
                <div class="stats-block-title">观看历史</div>
                <div class="stats-items">
                  <div class="stats-row">
                    <span class="stats-label">历史记录</span>
                    <span class="stats-val">{animeStore.stats.historyCount}</span>
                  </div>
                  <div class="stats-row">
                    <span class="stats-label">已安装规则</span>
                    <span class="stats-val">{animeStore.stats.rulesCount}</span>
                  </div>
                </div>
              </Card>
            </div>
          </div>
        {/if}

      <!-- ═══════════════════════════════════════════════════════════
           规则源
           ═══════════════════════════════════════════════════════════ -->
      {:else if animeStore.activeTab === "rules"}
        <div class="rules-section">
          <!-- GitHub 规则仓库 -->
          <div class="catalog-area">
            <div class="catalog-header">
              <div>
                <h3 class="import-title">Kazumi 规则仓库</h3>
                <p class="import-desc">从 GitHub 一键安装 / 更新社区规则</p>
              </div>
              <div class="catalog-actions">
                <Button variant="secondary" size="sm" press={() => animeStore.loadCatalog()}
                  disabled={animeStore.catalogLoading}>
                  <Icon name="refresh" size={13} />
                  {animeStore.catalogLoading ? "加载中..." : "刷新"}
                </Button>
                {#if animeStore.catalog.length > 0}
                  <Button variant="primary" size="sm" press={() => animeStore.installAllRules()}
                    disabled={animeStore.catalogLoading}>
                    全部安装 ({animeStore.catalog.length})
                  </Button>
                {/if}
              </div>
            </div>

            {#if animeStore.catalogError}
              <div class="catalog-error">
                <Icon name="x" size={14} />
                <span>{animeStore.catalogError}</span>
              </div>
            {/if}

            {#if animeStore.catalog.length > 0}
              <div class="catalog-grid">
                {#each animeStore.catalog as item (item.name)}
                  {@const installed = animeStore.isRuleInstalled(item.name)}
                  {@const localVer = animeStore.getRuleVersion(item.name)}
                  {@const hasUpdate = installed && localVer !== item.version}
                  {@const installing = animeStore.isRuleInstalling(item.name)}
                  <div class="catalog-item" class:installed>
                    <div class="catalog-item-info">
                      <span class="catalog-name">{item.name}</span>
                      <span class="catalog-meta">
                        v{item.version}
                        {#if item.lastUpdate}
                          · {fmtDate(item.lastUpdate)}
                        {/if}
                        {#if item.antiCrawlerEnabled}
                          <span class="catalog-badge warn">验证码</span>
                        {/if}
                      </span>
                    </div>
                    {#if installing}
                      <span class="catalog-status installing">安装中...</span>
                    {:else if hasUpdate}
                      <Button variant="secondary" size="sm" press={() => animeStore.installRule(item.name)}>
                        更新
                      </Button>
                    {:else if installed}
                      <span class="catalog-status installed-badge">已安装</span>
                    {:else}
                      <Button variant="primary" size="sm" press={() => animeStore.installRule(item.name)}>
                        安装
                      </Button>
                    {/if}
                  </div>
                {/each}
              </div>
            {:else if !animeStore.catalogLoading}
              <EmptyState title="暂无规则数据" description="点击「刷新」从 GitHub 加载规则列表" class="catalog-empty" />
            {/if}
          </div>

          <!-- 手动导入 -->
          <details class="manual-import">
            <summary class="manual-summary">手动导入 JSON 规则</summary>
            <div class="import-area">
              <textarea
                class="import-textarea"
                bind:value={importText}
                placeholder={'[\n  {\n    "name": "规则名称",\n    "baseURL": "https://...",\n    ...\n  }\n]'}
                rows="5"
              ></textarea>
              <div class="import-actions">
                <Button variant="primary" press={handleImport} disabled={!importText.trim()}>
                  导入规则
                </Button>
                {#if importMsg}
                  <span class="import-msg" class:error={importMsg.includes("失败")}>{importMsg}</span>
                {/if}
              </div>
            </div>
          </details>

          <!-- 已安装规则列表 -->
          {#if animeStore.rules.length > 0}
            <div class="rules-list">
              <h3 class="rules-title">已安装规则 ({animeStore.rules.length})</h3>
              {#each animeStore.rules as rule (rule.name)}
                <div class="rule-row">
                  <div class="rule-info">
                    <span class="rule-name">{rule.name}</span>
                    <span class="rule-meta">
                      v{rule.version}
                      · {rule.baseUrl}
                      {#if rule.useWebview}<span class="rule-badge">WebView</span>{/if}
                      {#if rule.adBlocker}<span class="rule-badge">AdBlock</span>{/if}
                    </span>
                  </div>
                  <Button variant="quiet" size="sm" press={() => animeStore.removeRule(rule.name)} ariaLabel="删除规则" class="remove-rule">
                    <Icon name="trash" size={14} />
                  </Button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Overlays -->
  {#if !providerV2Active && (animeStore.view === "detail" || animeStore.view === "player")}
    <div class="overlays">
      {#if animeStore.view === "player"}
        <AnimePlayer />
      {:else}
        <AnimeDetail returnFocus={() => detailReturnFocus} />
      {/if}
    </div>
  {/if}

  {#if providerV2Active}
    <div class="provider-v2-overlay">
      <ProviderV2Workspace onExit={closeProviderV2} />
    </div>
  {:else}
    <!-- Search drawer (always available in classic mode) -->
    <SearchDrawer />

    <!-- Source sheet (opened from detail page FAB) -->
    <SourceSheet />
  {/if}
</section>
</PageShell>

<style>
  :global(.v2-page-shell.anime-page) { height: 100%; padding: 0; }
  :global(.v2-page-shell.anime-page > .v2-page-shell__inner) { height: 100%; max-width: none; padding: 0; }
  .anime-page-frame { height: 100%; min-height: 0; }
  :global(.anime-filter-bar) { margin: 0 20px 8px; }
  .rule-filter { display: contents; }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0; }
  .search-clear { width: 2rem; height: 2rem; display: grid; place-items: center; border: 0; border-radius: 999px; background: transparent; color: var(--text-muted); cursor: pointer; }
  .search-clear:hover { color: var(--text-primary); background: rgba(255,255,255,.06); }

  .anime-page-frame {
    --accent: #7c6cf0;
    --accent-hi: #6558d4;
    --accent-lo: rgba(124,108,240,0.12);
    --accent-ring: rgba(124,108,240,0.35);
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    color: var(--text-primary);
  }

  .anime-shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .anime-shell.hidden-by-overlay {
    visibility: hidden;
    pointer-events: none;
  }

  .provider-v2-entry {
    height: 34px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    flex-shrink: 0;
    padding: 0 10px;
    border: 1px solid rgba(103,183,163,0.28);
    border-radius: 8px;
    background: rgba(103,183,163,0.07);
    color: #7dc4b3;
    font: inherit;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
  }
  .provider-v2-entry:hover {
    border-color: rgba(103,183,163,0.5);
    background: rgba(103,183,163,0.12);
    color: #a1d9cc;
  }

  .search-form {
    flex: 1;
    display: flex;
    gap: 8px;
  }
  .search-wrap {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: rgba(255,255,255,0.03);
    color: var(--text-muted);
    transition: border-color 0.15s;
  }
  .search-wrap:focus-within {
    border-color: var(--accent);
    color: var(--text-primary);
  }
  :global(.ui-input.search-input) {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    outline: none;
    padding: 8px 0;
  }
  :global(.ui-input.search-input::placeholder) { color: var(--text-muted); }
  :global(.ui-button.search-clear) {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
  }
  .rule-select {
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: rgba(255,255,255,0.03);
    color: var(--text-muted);
    font-size: 12.5px;
    cursor: pointer;
    outline: none;
    max-width: 120px;
  }
  .rule-select:focus { border-color: var(--accent); }
  /* ── Tab bar (Kazumi 风格) ───────────────────────────────── */
  .tab-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 20px 8px;
    border-bottom: 1px solid var(--border);
  }
  .tab-btn {
    padding: 7px 16px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 550;
    cursor: pointer;
    transition: all 0.15s;
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .tab-btn.active {
    background: var(--accent-lo, rgba(232,85,127,0.1));
    border-color: var(--accent-ring, rgba(232,85,127,0.3));
    color: var(--accent);
    font-weight: 700;
  }
  .tab-btn:not(.active):hover {
    background: rgba(255,255,255,0.04);
    color: var(--text-primary);
  }
  .search-label {
    font-size: 13px;
    color: var(--text-muted);
    flex: 1;
  }
  .anime-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* ═══════════════════════════════════════════════════════════
     推荐页
     ═══════════════════════════════════════════════════════════ */
  .rec-page { display: flex; flex-direction: column; gap: 28px; }
  :global(.ui-empty.rec-empty) {
    padding: 24px 0; text-align: center; color: var(--text-muted); font-size: 13px;
  }

  /* ── Cover Card Grid ─────────────────────────────────────── */
  .cover-grid {
    display: grid;
    grid-template-columns: repeat(10, minmax(0, 1fr));
    gap: 14px;
  }
  :global(.ui-card.cover-card) {
    display: flex; flex-direction: column; gap: 8px;
    border: none; border-radius: 10px;
    background: transparent; padding: 0; cursor: pointer;
    text-align: left; color: var(--text-primary);
    transition: transform 0.18s, opacity 0.18s;
  }
  :global(.ui-card.cover-card:hover) { transform: translateY(-3px); }
  :global(.ui-card.cover-card:active) { transform: scale(0.97); }

  .cover-img-wrap {
    position: relative; width: 100%; aspect-ratio: 3/4;
    border-radius: 8px; overflow: hidden;
    background: rgba(255,255,255,0.04);
  }
  .cover-img {
    width: 100%; height: 100%; object-fit: cover;
    display: block;
  }
  .cover-img-placeholder, .cover-img-loading {
    width: 100%; height: 100%;
    display: grid; place-items: center;
    color: var(--text-muted);
    background: linear-gradient(135deg, rgba(255,255,255,0.03), rgba(255,255,255,0.06));
  }
  .cover-rating {
    position: absolute; top: 6px; right: 6px;
    padding: 2px 7px; border-radius: 4px;
    background: rgba(0,0,0,0.65); backdrop-filter: blur(4px);
    color: #fbbf24; font-size: 11px; font-weight: 700;
    font-family: var(--font-mono);
  }
  .cover-meta { padding: 0 2px 4px; display: flex; flex-direction: column; gap: 2px; }
  .cover-title {
    font-size: 13px; font-weight: 650; line-height: 1.35;
    overflow: hidden; text-overflow: ellipsis;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical;
  }
  .cover-sub {
    font-size: 10px; color: var(--text-muted); line-height: 1.3;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cover-eps { font-size: 10.5px; color: var(--text-muted); }

  /* ── Calendar ──────────────────────────────────────────────── */
  .calendar-section { display: flex; flex-direction: column; gap: 14px; }
  .weekday-tabs { display: flex; gap: 4px; flex-wrap: wrap; }
  .weekday-tab {
    padding: 7px 16px; border: 1px solid var(--border); border-radius: 6px;
    background: transparent; color: var(--text-muted);
    font-size: 13px; font-weight: 550; cursor: pointer;
    transition: all 0.15s; position: relative;
    display: inline-flex; align-items: center; gap: 6px;
  }
  .weekday-tab.active {
    background: var(--accent-lo, rgba(232,85,127,0.1));
    border-color: var(--accent-ring, rgba(232,85,127,0.3));
    color: var(--accent); font-weight: 700;
  }
  .weekday-tab:not(.active):hover { background: rgba(255,255,255,0.04); color: var(--text-primary); }
  .weekday-count {
    font-size: 10px; font-weight: 700; color: var(--text-muted);
    background: rgba(255,255,255,0.06); padding: 1px 5px; border-radius: 3px;
  }
  .weekday-tab.active .weekday-count {
    color: var(--accent); background: var(--accent-lo);
  }
  .today-dot {
    position: absolute; top: 4px; right: 4px;
    width: 5px; height: 5px; border-radius: 50%;
    background: var(--accent);
  }

  /* ═══════════════════════════════════════════════════════════
     我的 — 数据页
     ═══════════════════════════════════════════════════════════ */
  .my-stats-bar {
    display: flex; gap: 8px; flex-wrap: wrap;
  }
  :global(.ui-stat.stat-card) {
    flex: 1; min-width: 80px;
  }
  
  :global(.ui-segment.my-sub-tabs) {
    padding: 4px 0;
  }

  /* 收藏筛选 */
  .collect-filters {
    display: flex; gap: 6px; flex-wrap: wrap; padding: 4px 0;
  }
  .filter-count {
    font-size: 10px; font-weight: 700;
    padding: 0 4px; border-radius: 8px;
    background: rgba(255,255,255,0.06);
  }
  :global(.ui-tag.filter-chip.is-active) .filter-count {
    background: var(--accent-lo);
  }

  /* 收藏卡片网格 */
  .collect-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 14px;
  }
  :global(.ui-card.collect-card) {
    display: flex; flex-direction: column; gap: 8px;
    border: none; border-radius: 10px;
    background: transparent; padding: 0; cursor: pointer;
    text-align: left; color: var(--text-primary);
    transition: transform 0.18s;
  }
  :global(.ui-card.collect-card:hover) { transform: translateY(-3px); }
  :global(.ui-card.collect-card:active) { transform: scale(0.97); }

  .collect-card-img {
    position: relative; width: 100%; aspect-ratio: 3/4;
    border-radius: 8px; overflow: hidden;
    background: rgba(255,255,255,0.04);
  }
  .collect-card-img img {
    width: 100%; height: 100%; object-fit: cover; display: block;
  }
  .collect-card-placeholder {
    width: 100%; height: 100%;
    display: grid; place-items: center; color: var(--text-muted);
    background: linear-gradient(135deg, rgba(255,255,255,0.03), rgba(255,255,255,0.06));
  }
  .collect-card-type {
    position: absolute; bottom: 6px; left: 6px;
    padding: 2px 8px; border-radius: 4px;
    background: rgba(0,0,0,0.65); backdrop-filter: blur(4px);
    color: var(--accent); font-size: 10px; font-weight: 700;
  }
  .collect-card-type[data-type="4"] { color: #34d399; }
  .collect-card-type[data-type="5"] { color: var(--text-muted); }
  .collect-card-meta { padding: 0 2px 4px; display: flex; flex-direction: column; gap: 2px; }
  .collect-card-name {
    font-size: 13px; font-weight: 650; line-height: 1.35;
    overflow: hidden; text-overflow: ellipsis;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical;
  }
  .collect-card-source { font-size: 10px; color: var(--text-muted); }

  /* 历史 */
  .history-toolbar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 4px 0;
  }
  .history-count { font-size: 12px; color: var(--text-muted); font-weight: 550; }
  :global(.ui-button.clear-btn) {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 5px 12px; border: 1px solid rgba(248,113,113,0.3); border-radius: 6px;
    background: transparent; color: #f87171; font-size: 12px; cursor: pointer;
    transition: all 0.15s;
  }
  :global(.ui-button.clear-btn:hover) { background: rgba(248,113,113,0.1); }

  .history-list { display: flex; flex-direction: column; gap: 4px; }
  .history-row {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 14px; border: 1px solid transparent; border-radius: 10px;
    background: rgba(255,255,255,0.02); cursor: pointer;
    transition: all 0.15s; color: var(--text-primary);
  }
  .history-row:hover { border-color: var(--border); background: rgba(255,255,255,0.04); }
  .history-thumb {
    width: 48px; height: 64px; border-radius: 6px; overflow: hidden;
    flex-shrink: 0; background: rgba(255,255,255,0.04);
  }
  .history-thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .history-thumb-placeholder {
    width: 100%; height: 100%; display: grid; place-items: center; color: var(--text-muted);
  }
  .history-info { flex: 1; display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .history-name { font-size: 14px; font-weight: 650; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .history-meta { font-size: 12px; color: var(--accent); font-weight: 550; }
  .history-sub { font-size: 11px; color: var(--text-muted); }

  :global(.ui-button.remove-btn) {
    flex-shrink: 0; width: 28px; height: 28px;
    display: grid; place-items: center;
    border: 1px solid transparent; border-radius: 6px;
    background: transparent; color: var(--text-muted); cursor: pointer;
    transition: all 0.15s;
  }
  :global(.ui-button.remove-btn:hover) { border-color: rgba(248,113,113,0.3); color: #f87171; background: rgba(248,113,113,0.08); }

  /* 统计页 */
  .stats-page { display: flex; flex-direction: column; gap: 16px; padding: 4px 0; }
  .stats-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  :global(.ui-card.stats-block) {
    display: flex; flex-direction: column; gap: 10px;
  }
  .stats-block-title {
    font-size: 13px; font-weight: 700; color: var(--text-primary);
    padding-bottom: 6px; border-bottom: 1px solid var(--border);
  }
  .stats-items { display: flex; flex-direction: column; gap: 6px; }
  .stats-row { display: flex; align-items: center; justify-content: space-between; }
  .stats-label { font-size: 12.5px; color: var(--text-muted); }
  .stats-val { font-size: 14px; font-weight: 700; font-family: var(--font-mono); color: var(--text-primary); }
  .stats-val.accent { color: var(--accent); }
  .stats-val.green { color: #34d399; }
  .stats-val.muted { color: var(--text-muted); }

  .stats-bar-wrap {
    display: flex; height: 6px; border-radius: 3px; overflow: hidden;
    background: rgba(255,255,255,0.04);
  }
  .stats-bar { min-width: 2px; transition: flex 0.3s; }
  .stats-bar.watching { background: var(--accent); }
  .stats-bar.planned { background: #60a5fa; }
  .stats-bar.onhold { background: #f59e0b; }
  .stats-bar.watched { background: #34d399; }
  .stats-bar.dropped { background: rgba(255,255,255,0.15); }

  /* ── Search results ────────────────────────────────────────── */
  .result-list { display: flex; flex-direction: column; gap: 3px; }
  :global(.ui-button.result-row) {
    display: flex; align-items: center; gap: 10px;
    padding: 10px 12px; border: 1px solid transparent; border-radius: 8px;
    background: rgba(255,255,255,0.02); cursor: pointer;
    transition: all 0.15s; text-align: left; width: 100%; color: var(--text-primary);
  }
  :global(.ui-button.result-row:hover) { border-color: var(--border); background: rgba(255,255,255,0.04); }

  /* ── Rules / Catalog ──────────────────────────────────────── */
  .rules-section { display: flex; flex-direction: column; gap: 24px; }

  .catalog-area { display: flex; flex-direction: column; gap: 12px; }
  .catalog-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
  .catalog-actions { display: flex; gap: 8px; flex-shrink: 0; }

  .catalog-error {
    display: flex; align-items: center; gap: 6px;
    padding: 8px 12px; border-radius: 6px;
    background: rgba(248,113,113,0.08); border: 1px solid rgba(248,113,113,0.2);
    color: #f87171; font-size: 12px;
  }

  .catalog-grid { display: flex; flex-direction: column; gap: 3px; }
  .catalog-item {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 14px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.02); transition: border-color 0.15s;
  }
  .catalog-item.installed { border-color: rgba(52,211,153,0.2); }
  .catalog-item-info { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .catalog-name { font-size: 13.5px; font-weight: 650; color: var(--text-primary); }
  .catalog-meta {
    font-size: 11px; color: var(--text-muted);
    display: flex; align-items: center; gap: 4px; flex-wrap: wrap;
  }
  .catalog-badge {
    font-size: 9.5px; padding: 1px 5px; border-radius: 3px;
    background: rgba(255,255,255,0.06); color: var(--text-muted);
  }
  .catalog-badge.warn { background: rgba(251,191,36,0.12); color: #fbbf24; }

  .catalog-status {
    font-size: 11px; color: var(--text-muted); flex-shrink: 0; padding: 0 4px;
  }
  .catalog-status.installing { color: var(--accent); font-weight: 600; }
  .installed-badge { color: #34d399; font-weight: 600; }
  :global(.ui-empty.catalog-empty) { text-align: center; color: var(--text-muted); font-size: 13px; padding: 20px 0; }

  .manual-import {
    border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.01);
  }
  .manual-summary {
    padding: 10px 14px; cursor: pointer; font-size: 13px;
    color: var(--text-muted); font-weight: 550;
  }
  .manual-summary:hover { color: var(--text-primary); }
  .import-area { display: flex; flex-direction: column; gap: 8px; padding: 0 14px 14px; }
  .import-title { font-size: 15px; font-weight: 700; margin: 0; color: var(--text-primary); }
  .import-desc { font-size: 13px; color: var(--text-muted); margin: 0; }
  .import-textarea {
    width: 100%; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.03); color: var(--text-primary);
    font-family: var(--font-mono); font-size: 12px; resize: vertical;
    outline: none; line-height: 1.5;
  }
  .import-textarea:focus { border-color: var(--accent); }
  .import-actions { display: flex; align-items: center; gap: 12px; }
  .import-msg { font-size: 12.5px; color: var(--accent); }
  .import-msg.error { color: #f87171; }

  .rules-list { display: flex; flex-direction: column; gap: 8px; }
  .rules-title { font-size: 14px; font-weight: 700; margin: 0; color: var(--text-primary); }
  .rule-row {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 14px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.02);
  }
  .rule-info { flex: 1; display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .rule-name { font-size: 14px; font-weight: 650; color: var(--text-primary); }
  .rule-meta {
    font-size: 11.5px; color: var(--text-muted);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .rule-badge {
    display: inline-block; padding: 1px 6px; border-radius: 4px;
    background: rgba(255,255,255,0.06); font-size: 10px; margin-left: 4px;
  }
  :global(.ui-button.remove-rule) {
    flex-shrink: 0; width: 32px; height: 32px;
    display: grid; place-items: center;
    border: 1px solid transparent; border-radius: 6px;
    background: transparent; color: var(--text-muted); cursor: pointer;
    transition: all 0.15s;
  }
  :global(.ui-button.remove-rule:hover) { border-color: rgba(248,113,113,0.3); color: #f87171; background: rgba(248,113,113,0.08); }

  /* ── Empty & loading ──────────────────────────────────────── */
  :global(.ui-empty.content-empty) {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 10px; color: var(--text-muted); padding: 60px 0; text-align: center;
  }
  :global(.ui-empty.content-empty) h3 { margin: 0; font-size: 16px; color: var(--text-primary); }
  :global(.ui-empty.content-empty) p { margin: 0; font-size: 13px; max-width: 400px; }
  .content-loading {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 12px; color: var(--text-muted); padding: 60px 0;
  }
  .spinner {
    width: 32px; height: 32px;
    border: 3px solid rgba(255,255,255,0.08);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .overlays { position: absolute; inset: 0; z-index: 20; pointer-events: all; }
  .provider-v2-overlay { position: absolute; inset: 0; z-index: 40; }
</style>
