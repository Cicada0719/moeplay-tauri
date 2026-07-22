<script lang="ts">
  import { onMount, tick } from "svelte";
  import type { ViewState } from "./ui-v2";
  import {
    comicStore,
    ORDINARY_SOURCE_OPTIONS,
    SORT_OPTIONS,
    type ComicChapter,
    type ComicSummary,
    type OrdinaryComicSource,
    type OrdinarySourceKey,
    type ReadRecord,
  } from "../stores/comic.svelte";
  import { focusComicRovingItem, nextComicRovingIndex } from "./comic/a11y";
  import ComicCard from "./comic/ComicCard.svelte";
  import ComicDetail from "./comic/ComicDetail.svelte";
  import ComicReader from "./comic/ComicReader.svelte";
  import ProviderV2Page from "./comic/provider-v2/ProviderV2Page.svelte";
  import Icon from "./Icon.svelte";
  import { Button, Input, Tag } from "./ui";
  import { AsyncSection, ContentGrid, Drawer, FilterBar, MediaRow, PageHeader, PageShell } from "./ui-v2";

  type PageMode = "normal" | "provider-v2" | "picacg";
  type PicacgTab = "explore" | "ranking" | "random" | "favorites" | "history";

  let pageMode = $state<PageMode>("normal");
  let searchInput = $state("");
  let adultOpen = $state(false);
  let adultTrigger = $state<HTMLElement>();
  let adultEmail = $state(comicStore.savedEmail);
  let adultPassword = $state("");
  let adultLoginError = $state("");
  let adultBusy = $state(false);
  let picacgSearchInput = $state("");
  let punchingIn = $state(false);
  let detailReturnFocus = $state<HTMLElement | null>(null);
  let chapterReturnFocusKey = $state<string>();
  let sourceTabRefs: Array<HTMLButtonElement | null> = [];
  let picacgTabRefs: Array<HTMLButtonElement | null> = [];

  const sourceOptions = ORDINARY_SOURCE_OPTIONS;
  const picacgTabs: Array<{ value: PicacgTab; label: string }> = [
    { value: "explore", label: "探索" },
    { value: "ranking", label: "排行榜" },
    { value: "random", label: "随机" },
    { value: "favorites", label: "收藏" },
    { value: "history", label: "历史" },
  ];

  const activeSourceLabel = $derived(
    sourceOptions.find((source) => source.value === comicStore.ordinarySource)?.label ?? "自动",
  );

  const ordinaryResultCount = $derived(
    comicStore.ordinarySourceSections.reduce((total, section) => total + section.docs.length, 0),
  );

  const ordinaryState = $derived.by<ViewState>(() => {
    if (comicStore.mangaDexLoading && ordinaryResultCount === 0) return "loading";
    if (comicStore.mangaDexLoading && ordinaryResultCount > 0) return "refreshing";
    if (comicStore.mangaDexError && ordinaryResultCount > 0) return "partial";
    if (comicStore.mangaDexError) return "error";
    if (comicStore.ordinaryKeyword && ordinaryResultCount === 0) return "no-results";
    return "ready";
  });

  const picacgItems = $derived.by<ComicSummary[]>(() => {
    if (comicStore.searchKeyword) return comicStore.searchResults;
    if (comicStore.activeTab === "ranking") return comicStore.ranking;
    if (comicStore.activeTab === "random") return comicStore.randomList;
    if (comicStore.activeTab === "favorites") return comicStore.favorites;
    return comicStore.comicList;
  });

  const picacgState = $derived.by<ViewState>(() => {
    if (comicStore.error && picacgItems.length > 0) return "partial";
    if (comicStore.error) return "error";
    if (comicStore.loading && picacgItems.length === 0) return "loading";
    if (comicStore.loading && picacgItems.length > 0) return "refreshing";
    if (comicStore.searchKeyword && picacgItems.length === 0) return "no-results";
    if (comicStore.activeTab === "history") return comicStore.readHistory.length > 0 ? "ready" : "empty";
    return picacgItems.length > 0 ? "ready" : "empty";
  });

  async function searchOrdinary(value = searchInput) {
    const keyword = value.trim();
    if (!keyword) return;
    searchInput = keyword;
    await comicStore.searchOrdinary(keyword);
  }

  async function selectOrdinarySource(source: OrdinaryComicSource, index?: number) {
    if (comicStore.ordinarySource !== source) {
      comicStore.setOrdinarySource(source);
      if (searchInput.trim()) await searchOrdinary(searchInput);
    }
    if (index != null) focusComicRovingItem(sourceTabRefs, index);
  }

  async function loginPicacg(event: Event) {
    event.preventDefault();
    if (!adultEmail || !adultPassword) {
      adultLoginError = "请填写账号和密码";
      return;
    }
    adultLoginError = "";
    adultBusy = true;
    try {
      await comicStore.login(adultEmail, adultPassword);
      await comicStore.loadProfile();
      await comicStore.loadCategories();
      adultOpen = false;
      pageMode = "picacg";
    } catch {
      adultLoginError = comicStore.error ?? "登录失败";
    } finally {
      adultBusy = false;
    }
  }

  async function handlePicacgSearch() {
    const keyword = picacgSearchInput.trim();
    if (!keyword) return;
    await comicStore.search(keyword);
  }

  function clearPicacgSearch() {
    picacgSearchInput = "";
    comicStore.searchKeyword = "";
    comicStore.setTab("explore");
  }

  async function enterPicacg() {
    adultOpen = false;
    pageMode = "picacg";
    if (comicStore.isLoggedIn) {
      await comicStore.loadProfile();
      await comicStore.loadCategories();
    }
  }

  function leavePicacg() {
    pageMode = "normal";
    adultOpen = false;
  }

  async function handlePunchIn() {
    punchingIn = true;
    await comicStore.punchIn();
    punchingIn = false;
  }

  function fmtDate(timestamp: number) {
    const date = new Date(timestamp);
    return new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }).format(date);
  }

  function rememberTrigger(event?: MouseEvent): HTMLElement | null {
    return event?.currentTarget instanceof HTMLElement
      ? event.currentTarget
      : document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
  }

  async function openOrdinaryComic(comic: ComicSummary, event: MouseEvent) {
    detailReturnFocus = rememberTrigger(event);
    await comicStore.openOrdinaryComic(comic.id);
  }

  async function openPicacgComic(comic: ComicSummary, event: MouseEvent) {
    detailReturnFocus = rememberTrigger(event);
    await comicStore.openComic(comic.id);
  }

  async function resumeHistory(record: ReadRecord, event: MouseEvent) {
    detailReturnFocus = rememberTrigger(event);
    await comicStore.resumeHistory(record);
  }

  async function openChapter(chapter: ComicChapter, event: MouseEvent) {
    const target = event.currentTarget as HTMLElement;
    chapterReturnFocusKey = target.dataset.chapterFocusKey ?? `${comicStore.currentComic?.id ?? "comic"}:${chapter.order}`;
    await comicStore.openChapter(chapter.order, chapter.title);
  }

  function closeDetail() {
    comicStore.closeComic();
    queueMicrotask(() => detailReturnFocus?.focus({ preventScroll: true }));
  }

  async function closeReader() {
    comicStore.closeReader();
    await tick();
    if (!chapterReturnFocusKey) return;
    document.querySelector<HTMLElement>(`[data-chapter-focus-key="${CSS.escape(chapterReturnFocusKey)}"]`)?.focus({ preventScroll: true });
  }

  function handleSourceTabKeydown(event: KeyboardEvent, index: number) {
    const next = nextComicRovingIndex(event.key, index, sourceOptions.length);
    if (next == null) return;
    event.preventDefault();
    void selectOrdinarySource(sourceOptions[next].value, next);
  }

  function activatePicacgTab(tab: PicacgTab, index?: number) {
    comicStore.setTab(tab);
    if (index != null) focusComicRovingItem(picacgTabRefs, index);
  }

  function handlePicacgTabKeydown(event: KeyboardEvent, index: number) {
    const next = nextComicRovingIndex(event.key, index, picacgTabs.length);
    if (next == null) return;
    event.preventDefault();
    activatePicacgTab(picacgTabs[next].value, next);
  }

  function retryPicacg() {
    comicStore.clearError();
    if (comicStore.searchKeyword) void comicStore.search(comicStore.searchKeyword);
    else comicStore.setTab(comicStore.activeTab);
  }

  onMount(() => {
    void comicStore.rehydrate().then(() => {
      if (comicStore.isLoggedIn) void comicStore.loadProfile();
    });
  });
</script>

<section class="comic-page" data-testid="comic-page">
  {#if pageMode === "provider-v2"}
    <ProviderV2Page onlegacy={() => pageMode = "normal"} />
  {:else if pageMode === "normal"}
    <PageShell as="div" width="full" ariaLabel="漫画" class="comic-v2-shell" labelledBy="comic-page-title">
      <div class="comic-page-frame">
      {#snippet normalActions()}
        <Button variant="secondary" size="sm" press={() => (pageMode = "provider-v2")}><Icon name="layers" size={15} />Provider v2</Button>
        <button bind:this={adultTrigger} class="adult-button" type="button" aria-expanded={adultOpen} aria-controls="comic-adult-drawer" onclick={() => (adultOpen = true)}>
          <Icon name="shield" size={15} />18+
        </button>
      {/snippet}

      <PageHeader
        id="comic-page-title"
        eyebrow="Manga"
        title="漫画"
        description="聚合公开源，并支持 PicACG 与本地漫画。"
        actions={normalActions}
      />

      {#snippet ordinaryControls()}
        <form class="comic-search" aria-label="搜索普通漫画" onsubmit={(event) => { event.preventDefault(); void searchOrdinary(); }}>
          <label class="search-field">
            <Icon name="search" size={16} />
            <input type="search" bind:value={searchInput} placeholder="搜索普通漫画..." data-search-scope="comic" data-gamepad-nav-right="#comic-source-tab-auto" aria-label="搜索普通漫画" />
            {#if searchInput}<button type="button" class="search-clear" aria-label="清空普通漫画关键词" onclick={() => (searchInput = "")}><Icon name="x" size={13} /></button>{/if}
          </label>
          <Button type="submit" variant="primary" disabled={!searchInput.trim()} loading={comicStore.mangaDexLoading}>搜索</Button>
        </form>

        <div class="source-tabs" role="tablist" aria-label="普通漫画源">
          {#each sourceOptions as source, index (source.value)}
            <button
              bind:this={sourceTabRefs[index]}
              type="button"
              role="tab"
              id={`comic-source-tab-${source.value}`}
              aria-selected={comicStore.ordinarySource === source.value}
              aria-controls="comic-ordinary-results"
              tabindex={comicStore.ordinarySource === source.value ? 0 : -1}
              class:active={comicStore.ordinarySource === source.value}
              disabled={comicStore.mangaDexLoading}
              onclick={() => void selectOrdinarySource(source.value, index)}
              onkeydown={(event) => handleSourceTabKeydown(event, index)}
            >
              <strong>{source.label}</strong><span>{source.hint}</span>
            </button>
          {/each}
        </div>
      {/snippet}

      <FilterBar
        controls={ordinaryControls}
        label="普通漫画搜索和来源"
        activeCount={(searchInput.trim() ? 1 : 0) + (comicStore.ordinarySource === "auto" ? 0 : 1)}
        onClear={() => { searchInput = ""; comicStore.setOrdinarySource("auto"); }}
        busy={comicStore.mangaDexLoading}
        class="comic-filter-bar"
      />

      <div id="comic-ordinary-results" role="tabpanel" aria-labelledby={`comic-source-tab-${comicStore.ordinarySource}`} class:hidden-under-overlay={comicStore.view !== "home"}>
        <AsyncSection
          title={comicStore.ordinaryKeyword ? `搜索：“${comicStore.ordinaryKeyword}”` : "发现漫画"}
          description={`当前来源：${activeSourceLabel}。自动模式会并行搜索，单个来源失败不会隐藏其他结果。`}
          state={ordinaryState}
          preserveContent={ordinaryResultCount > 0}
          details={comicStore.mangaDexError || undefined}
          primaryAction={comicStore.mangaDexError && searchInput.trim() ? { label: "重新搜索", onSelect: () => void searchOrdinary() } : undefined}
          loadingDelayMs={0}
          class="ordinary-results-section"
        >
          {#if comicStore.ordinarySourceSections.length > 0}
            <div class="ordinary-source-results">
              {#each comicStore.ordinarySourceSections as section (section.source)}
                <AsyncSection
                  title={section.label}
                  description={`${section.docs.length} 条结果`}
                  state={section.loading && section.docs.length === 0 ? "loading" : section.loading ? "refreshing" : section.error && section.docs.length > 0 ? "partial" : section.error ? "error" : section.docs.length > 0 ? "ready" : "no-results"}
                  details={section.error || undefined}
                  preserveContent={section.docs.length > 0}
                  primaryAction={section.error ? { label: "重试此来源", onSelect: () => void comicStore.retryOrdinarySource(section.source as OrdinarySourceKey) } : undefined}
                  loadingDelayMs={0}
                  headingLevel={3}
                  compact
                  class="ordinary-source-section"
                >
                  <ContentGrid minItemWidth="9.5rem" gap="md" label={`${section.label} 搜索结果`} busy={section.loading}>
                    {#each section.docs as comic (comic.id)}
                      <ComicCard comic={comic} focusKey={`ordinary:${section.source}:${comic.id}`} onclick={(event) => void openOrdinaryComic(comic, event)} />
                    {/each}
                  </ContentGrid>
                </AsyncSection>
              {/each}
            </div>
          {:else if ordinaryState === "ready"}
            <div class="ordinary-home">
              <section class="ordinary-lead">
                <span>01 / SEARCH ARCHIVE</span>
                <Icon name="search" size={30} />
                <div><h2>搜索漫画，直接阅读</h2><p>支持 MangaDex、包子漫画、DM5 与 1kkk，多源结果分段显示。</p></div>
                <div class="quick-searches" aria-label="热门搜索">
                  {#each ["海贼王", "葬送的芙莉莲", "迷宫饭", "电锯人"] as keyword}
                    <Button variant="ghost" size="sm" press={() => void searchOrdinary(keyword)}>{keyword}</Button>
                  {/each}
                </div>
              </section>
              <section class="ordinary-side" aria-label="漫画阅读摘要">
                <div class="source-register"><span>02 / SOURCES</span><strong>4 个公开来源</strong><small>自动并行检索，保留可用结果</small></div>
                {#if comicStore.readHistory.length}
                  <div class="recent-reading">
                    <span>03 / CONTINUE</span>
                    {#each comicStore.readHistory.slice(0, 3) as record}
                      <button type="button" onclick={(event) => void resumeHistory(record, event)}>
                        {#if record.thumb_url}<img src={record.thumb_url} alt="" />{/if}
                        <span><strong>{record.title}</strong><small>{record.last_title || `第 ${record.last_order} 话`} · {fmtDate(record.ts)}</small></span>
                      </button>
                    {/each}
                  </div>
                {:else}
                  <div class="reading-ready"><span>03 / READING LOG</span><strong>阅读记录尚未建立</strong><small>打开章节后自动记录。</small></div>
                {/if}
              </section>
            </div>
          {/if}
        </AsyncSection>
      </div>
      </div>
    </PageShell>

    <Drawer
      id="comic-adult-drawer"
      open={adultOpen}
      title="PicACG 18+"
      description="成人内容入口与普通漫画搜索隔离。"
      side="right"
      size="sm"
      onClose={() => (adultOpen = false)}
      returnFocus={() => adultTrigger}
      initialFocus={comicStore.isLoggedIn ? ".enter-picacg" : "input"}
    >
      {#if !comicStore.isLoggedIn}
        <form class="adult-form" onsubmit={loginPicacg}>
          <label><span>邮箱 / 用户名</span><Input type="text" bind:value={adultEmail} placeholder="邮箱或用户名" autocomplete="username" disabled={adultBusy} /></label>
          <label><span>密码</span><Input type="password" bind:value={adultPassword} placeholder="••••••••" autocomplete="current-password" disabled={adultBusy} /></label>
          {#if adultLoginError}<p class="form-error" role="alert">{adultLoginError}</p>{/if}
          <Button type="submit" fullWidth loading={adultBusy}>登录 PicACG</Button>
        </form>
      {:else}
        <div class="adult-profile">
          <Tag variant="muted" size="md">Lv.{comicStore.profile?.level ?? "-"} · {comicStore.profile?.name ?? "PicACG"}</Tag>
          <p>进入后会切换到独立 PicACG 漫画视图，不影响普通漫画搜索。</p>
          <Button class="enter-picacg" variant="primary" fullWidth press={enterPicacg}>进入 PicACG</Button>
          <Button variant="quiet" fullWidth press={() => comicStore.logout()}>退出登录</Button>
        </div>
      {/if}
    </Drawer>
  {:else}
    <PageShell as="div" width="full" ariaLabel="PicACG 漫画" class="comic-v2-shell" labelledBy="picacg-page-title">
      <div class="comic-page-frame">
      {#snippet picacgActions()}
        {#if comicStore.profile}
          <Tag variant="muted" size="md">Lv.{comicStore.profile.level} · {comicStore.profile.name}</Tag>
          {#if !comicStore.profile.is_punched}
            <Button variant="ghost" size="sm" press={handlePunchIn} disabled={punchingIn} loading={punchingIn} ariaLabel="PicACG 每日打卡"><Icon name="zap" size={13} />打卡</Button>
          {:else}<Tag variant="accent" size="sm"><Icon name="check" size={13} />已打卡</Tag>{/if}
        {/if}
        <Button variant="ghost" size="sm" press={leavePicacg}>普通漫画</Button>
        <Button variant="quiet" size="sm" press={() => { comicStore.logout(); pageMode = "normal"; }} ariaLabel="退出 PicACG 登录"><Icon name="x" size={14} /></Button>
      {/snippet}

      <PageHeader
        id="picacg-page-title"
        eyebrow="PicACG"
        title="哔咔漫画"
        description="探索、排行、随机、收藏与本地阅读历史使用统一漫画浏览模式。"
        actions={picacgActions}
      />

      {#if !comicStore.isLoggedIn}
        <AsyncSection title="登录 PicACG" description="登录凭据通过安全存储管理。" state="ready" class="picacg-login-section">
          <form class="login-form" onsubmit={loginPicacg}>
            <label><span>邮箱 / 用户名</span><Input type="text" bind:value={adultEmail} placeholder="邮箱或用户名" autocomplete="username" disabled={adultBusy} /></label>
            <label><span>密码</span><Input type="password" bind:value={adultPassword} placeholder="••••••••" autocomplete="current-password" disabled={adultBusy} /></label>
            {#if adultLoginError}<p class="form-error" role="alert">{adultLoginError}</p>{/if}
            <div class="login-actions"><Button type="submit" loading={adultBusy}>登录</Button><Button variant="ghost" press={leavePicacg}>返回普通漫画</Button></div>
          </form>
        </AsyncSection>
      {:else}
        {#snippet picacgControls()}
          <form class="comic-search" aria-label="搜索 PicACG 漫画" onsubmit={(event) => { event.preventDefault(); void handlePicacgSearch(); }}>
            <label class="search-field">
              <Icon name="search" size={16} />
              <input type="search" bind:value={picacgSearchInput} placeholder="搜索 PicACG..." data-search-scope="comic" aria-label="搜索 PicACG 漫画" />
              {#if picacgSearchInput}<button type="button" class="search-clear" aria-label="清空 PicACG 关键词" onclick={clearPicacgSearch}><Icon name="x" size={13} /></button>{/if}
            </label>
            <Button type="submit" variant="primary" disabled={!picacgSearchInput.trim()} loading={comicStore.loading}>搜索</Button>
          </form>

          {#if !comicStore.searchKeyword}
            <div class="picacg-tabs" role="tablist" aria-label="PicACG 内容分类">
              {#each picacgTabs as tab, index (tab.value)}
                <button
                  bind:this={picacgTabRefs[index]}
                  type="button"
                  role="tab"
                  id={`picacg-tab-${tab.value}`}
                  aria-selected={comicStore.activeTab === tab.value}
                  aria-controls="picacg-results"
                  tabindex={comicStore.activeTab === tab.value ? 0 : -1}
                  class:active={comicStore.activeTab === tab.value}
                  onclick={() => activatePicacgTab(tab.value, index)}
                  onkeydown={(event) => handlePicacgTabKeydown(event, index)}
                >{tab.label}</button>
              {/each}
            </div>
          {:else}
            <span class="search-summary" aria-live="polite">搜索：“{comicStore.searchKeyword}”</span>
          {/if}

          {#if comicStore.activeTab === "explore" || comicStore.activeTab === "favorites" || comicStore.searchKeyword}
            <label class="sort-control"><span>排序</span><select value={comicStore.sort} onchange={(event) => { comicStore.setSort((event.currentTarget as HTMLSelectElement).value as never); if (comicStore.searchKeyword) void comicStore.search(comicStore.searchKeyword); }}>{#each SORT_OPTIONS as option (option.value)}<option value={option.value}>{option.label}</option>{/each}</select></label>
          {/if}
        {/snippet}

        <FilterBar
          controls={picacgControls}
          label="PicACG 搜索和分类"
          activeCount={(comicStore.searchKeyword ? 1 : 0) + (comicStore.selectedCategory ? 1 : 0)}
          onClear={clearPicacgSearch}
          busy={comicStore.loading}
          class="comic-filter-bar"
        />

        {#if comicStore.activeTab === "explore" && !comicStore.searchKeyword && comicStore.categories.length > 0}
          <div class="category-chips" aria-label="PicACG 分类筛选">
            <Tag active={comicStore.selectedCategory === null} onclick={() => comicStore.selectCategory(null)}>全部</Tag>
            {#each comicStore.categories as category (category.id || category.title)}
              <Tag active={comicStore.selectedCategory === category.title} onclick={() => comicStore.selectCategory(category.title)}>{category.title}</Tag>
            {/each}
          </div>
        {/if}

        <div id="picacg-results" role="tabpanel" aria-labelledby={comicStore.searchKeyword ? undefined : `picacg-tab-${comicStore.activeTab}`} class:hidden-under-overlay={comicStore.view !== "home"}>
          <AsyncSection
            title={comicStore.searchKeyword ? `搜索：“${comicStore.searchKeyword}”` : picacgTabs.find((tab) => tab.value === comicStore.activeTab)?.label ?? "漫画"}
            description={comicStore.activeTab === "history" ? `${comicStore.readHistory.length} 条本地阅读记录` : `${picacgItems.length} 部漫画`}
            state={picacgState}
            preserveContent={picacgItems.length > 0 || comicStore.readHistory.length > 0}
            details={comicStore.error || undefined}
            primaryAction={comicStore.error ? { label: "重试", onSelect: retryPicacg } : undefined}
            loadingDelayMs={0}
            class="picacg-results-section"
          >
            {#if comicStore.activeTab === "history" && !comicStore.searchKeyword}
              <div class="history-list" role="list" aria-label="漫画阅读历史">
                {#each comicStore.readHistory as record (record.id)}
                  <MediaRow
                    title={record.title}
                    subtitle={`${record.author || "未知作者"} · 读到 ${record.last_title || `第 ${record.last_order} 话`}`}
                    imageSrc={record.thumb_url}
                    imageAlt={`${record.title} 封面`}
                    focusKey={`history:${record.id}`}
                    ariaLabel={`继续阅读 ${record.title}`}
                    onActivate={(event) => void resumeHistory(record, event)}
                  >
                    {#snippet meta()}<time datetime={new Date(record.ts).toISOString()}>{fmtDate(record.ts)}</time>{/snippet}
                    {#snippet actions()}
                      <Button variant="quiet" size="sm" press={(event) => { event.stopPropagation(); comicStore.removeHistory(record.id); }} ariaLabel={`删除 ${record.title} 阅读记录`}><Icon name="x" size={12} /></Button>
                    {/snippet}
                  </MediaRow>
                {/each}
              </div>
              {#if comicStore.readHistory.length > 0}<Button variant="ghost" size="sm" press={() => comicStore.clearHistory()}>清空阅读历史</Button>{/if}
            {:else}
              {#if comicStore.activeTab === "ranking" && !comicStore.searchKeyword}
                <div class="ranking-periods" role="group" aria-label="排行榜周期">
                  {#each [{ value: "H24", label: "日榜" }, { value: "D7", label: "周榜" }, { value: "D30", label: "月榜" }] as period (period.value)}
                    <button type="button" class:active={comicStore.rankingType === period.value} onclick={() => void comicStore.loadRanking(period.value as never)}>{period.label}</button>
                  {/each}
                </div>
              {/if}
              {#if comicStore.activeTab === "random" && !comicStore.searchKeyword}
                <Button variant="ghost" size="sm" press={() => comicStore.loadRandom()} disabled={comicStore.loading} loading={comicStore.loading}><Icon name="refresh" size={15} />换一批</Button>
              {/if}
              <ContentGrid minItemWidth="9.5rem" gap="md" label="PicACG 漫画列表" busy={comicStore.loading}>
                {#each picacgItems as comic, index (comic.id)}
                  <ComicCard comic={comic} focusKey={`picacg:${comic.id}`} onclick={(event) => void openPicacgComic(comic, event)} selected={comicStore.activeTab === "ranking" && index === 0} />
                {/each}
              </ContentGrid>
            {/if}

            {#if comicStore.searchKeyword && comicStore.searchPage < comicStore.searchPages}
              <Button variant="ghost" press={() => comicStore.searchNextPage()} disabled={comicStore.loading} loading={comicStore.loading}>加载更多</Button>
            {:else if comicStore.activeTab === "explore" && comicStore.comicPage < comicStore.comicPages}
              <Button variant="ghost" press={() => comicStore.loadMoreComics()} disabled={comicStore.loading} loading={comicStore.loading}>加载更多</Button>
            {:else if comicStore.activeTab === "favorites" && comicStore.favPage < comicStore.favPages}
              <Button variant="ghost" press={() => comicStore.loadFavorites(comicStore.favPage + 1)} disabled={comicStore.loading} loading={comicStore.loading}>加载更多</Button>
            {/if}
          </AsyncSection>
        </div>
      {/if}
      </div>
    </PageShell>
  {/if}

  {#if pageMode !== "provider-v2" && (comicStore.view === "detail" || comicStore.view === "reader")}
    <div class="comic-overlays">
      {#if comicStore.view === "reader"}
        <ComicReader onclose={closeReader} returnFocusKey={chapterReturnFocusKey} />
      {:else}
        <ComicDetail onclose={closeDetail} onopenchapter={openChapter} returnFocus={detailReturnFocus ?? true} />
      {/if}
    </div>
  {/if}
</section>

<style>
  .comic-page {
    position: relative;
    isolation: isolate;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    color: var(--v2-color-text, var(--text-primary));
    background:
      radial-gradient(circle at 8% 12%, color-mix(in srgb, var(--v2-color-accent) 17%, transparent) 0, transparent 28rem),
      radial-gradient(circle at 92% 24%, color-mix(in srgb, #8b5cf6 12%, transparent) 0, transparent 32rem),
      linear-gradient(145deg, color-mix(in srgb, var(--v2-color-surface-subtle) 86%, transparent), color-mix(in srgb, var(--v2-color-surface) 96%, transparent));
  }
  .comic-page::before {
    position: absolute;
    inset: 0;
    z-index: -1;
    background:
      repeating-linear-gradient(115deg, transparent 0 34px, color-mix(in srgb, var(--v2-color-text) 3%, transparent) 34px 35px),
      repeating-linear-gradient(25deg, transparent 0 52px, color-mix(in srgb, var(--v2-color-accent) 3%, transparent) 52px 53px);
    content: "";
    pointer-events: none;
  }
  .comic-page::after {
    position: absolute;
    inset: clamp(1rem, 5vw, 5rem) auto auto max(1rem, 4vw);
    z-index: -1;
    width: clamp(5rem, 9vw, 9rem);
    aspect-ratio: 1;
    border: 1px solid color-mix(in srgb, var(--v2-color-accent) 24%, transparent);
    border-radius: 36% 64% 58% 42%;
    box-shadow: 0 0 0 1rem color-mix(in srgb, var(--v2-color-accent) 3%, transparent), 0 0 0 2.5rem color-mix(in srgb, var(--v2-color-accent) 2%, transparent);
    content: "";
    pointer-events: none;
    transform: rotate(18deg);
  }
  :global(.comic-v2-shell) { position: relative; z-index: 1; height: 100%; }
  :global(.comic-v2-shell .v2-page-shell__inner) { display: flex; min-height: 100%; flex-direction: column; gap: var(--v2-space-5); }
  .comic-page-frame {
    display: flex;
    width: min(100%, 108rem);
    min-height: 100%;
    margin-inline: auto;
    padding: clamp(var(--v2-space-3), 1.4vw, var(--v2-space-5));
    flex-direction: column;
    gap: var(--v2-space-5);
    border: 1px solid color-mix(in srgb, var(--v2-color-border) 78%, transparent);
    border-radius: clamp(var(--v2-radius-lg), 1.5vw, 1.75rem);
    background: color-mix(in srgb, var(--v2-color-surface) 82%, transparent);
    box-shadow: 0 1.5rem 4rem color-mix(in srgb, #020617 18%, transparent);
    backdrop-filter: blur(1rem) saturate(112%);
  }
  :global(.comic-filter-bar .v2-filter-bar__controls) { flex: 1 1 36rem; }
  .hidden-under-overlay { visibility: hidden; }

  .comic-search { display: flex; flex: 1 1 24rem; align-items: center; gap: var(--v2-space-2); }
  .search-field { display: flex; min-width: 12rem; min-height: 2.75rem; flex: 1; align-items: center; gap: var(--v2-space-2); padding: 0 var(--v2-space-3); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: var(--v2-color-surface); color: var(--v2-color-text-secondary); }
  .search-field:focus-within { border-color: var(--v2-color-accent); box-shadow: var(--v2-focus-ring); }
  .search-field input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--v2-color-text); font: inherit; }
  .search-clear { display: grid; width: 2rem; min-height: 2rem; place-items: center; border: 0; border-radius: 50%; background: transparent; color: var(--v2-color-text-secondary); cursor: pointer; }
  .search-clear:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }

  .adult-button { display: inline-flex; min-height: 2.5rem; padding: 0 0.85rem; align-items: center; gap: 0.4rem; border: 1px solid rgba(239,68,68,0.32); border-radius: var(--v2-radius-md); background: rgba(127,29,29,0.12); color: #fca5a5; font: inherit; font-weight: 700; cursor: pointer; }
  .adult-button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }

  .source-tabs, .picacg-tabs { display: flex; flex: 1 1 auto; flex-wrap: wrap; gap: var(--v2-space-1); }
  .source-tabs button, .picacg-tabs button, .ranking-periods button { min-height: 2.65rem; padding: 0.48rem 0.75rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: linear-gradient(145deg, color-mix(in srgb, var(--v2-color-surface) 96%, transparent), color-mix(in srgb, var(--v2-color-surface-subtle) 84%, transparent)); color: var(--v2-color-text-secondary); font: inherit; cursor: pointer; transition: border-color var(--v2-motion-fast) var(--v2-ease-standard), background var(--v2-motion-fast) var(--v2-ease-standard); }
  .source-tabs button { display: flex; flex-direction: column; justify-content: center; text-align: left; }
  .source-tabs button span { margin-top: 0.1rem; color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  .source-tabs button.active, .picacg-tabs button.active, .ranking-periods button.active { border-color: var(--v2-color-accent); background: color-mix(in srgb, var(--v2-color-accent) 12%, var(--v2-color-surface)); color: var(--v2-color-text); }
  .source-tabs button:hover, .picacg-tabs button:hover, .ranking-periods button:hover { border-color: color-mix(in srgb, var(--v2-color-accent) 58%, var(--v2-color-border)); }
  .source-tabs button:focus-visible, .picacg-tabs button:focus-visible, .ranking-periods button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }

  .ordinary-source-results { display: grid; gap: var(--v2-space-6); }
  :global(.ordinary-source-section) {
    padding: clamp(var(--v2-space-3), 1.5vw, var(--v2-space-5));
    border: 1px solid color-mix(in srgb, var(--v2-color-border) 82%, transparent);
    border-radius: var(--v2-radius-lg);
    background: linear-gradient(135deg, color-mix(in srgb, var(--v2-color-surface) 94%, transparent), color-mix(in srgb, var(--v2-color-accent) 4%, var(--v2-color-surface-subtle)));
    box-shadow: 0 0.75rem 2rem color-mix(in srgb, #020617 10%, transparent);
  }
  .quick-searches { grid-column: 1 / -1; display: flex; flex-wrap: wrap; gap: var(--v2-space-2); }

  .adult-form, .login-form { display: flex; flex-direction: column; gap: var(--v2-space-4); }
  .adult-form label, .login-form label { display: flex; flex-direction: column; gap: var(--v2-space-2); color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); font-weight: 650; }
  .adult-profile { display: flex; flex-direction: column; gap: var(--v2-space-4); }
  .adult-profile p { margin: 0; color: var(--v2-color-text-secondary); line-height: 1.6; }
  .form-error { margin: 0; padding: var(--v2-space-3); border: 1px solid rgba(248,113,113,0.32); border-radius: var(--v2-radius-md); background: rgba(248,113,113,0.08); color: #fca5a5; }
  :global(.picacg-login-section) { max-width: 34rem; margin: auto; }
  .login-actions { display: flex; gap: var(--v2-space-2); }

  .sort-control { display: flex; align-items: center; gap: var(--v2-space-2); color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); }
  .sort-control select { min-height: 2.65rem; padding: 0 2rem 0 0.75rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: var(--v2-color-surface); color: var(--v2-color-text); }
  .search-summary { align-self: center; color: var(--v2-color-text-secondary); }
  .category-chips { display: flex; flex-wrap: wrap; gap: var(--v2-space-2); }
  .ranking-periods { display: flex; margin-bottom: var(--v2-space-4); gap: var(--v2-space-2); }
  .history-list { display: grid; gap: var(--v2-space-2); }
  .comic-overlays { position: absolute; inset: 0; z-index: 40; }

  @media (max-width: 56rem) {
    .comic-search { flex-basis: 100%; }
    .source-tabs, .picacg-tabs { flex-basis: 100%; overflow-x: auto; flex-wrap: nowrap; }
    .source-tabs button, .picacg-tabs button { flex: 0 0 auto; }
  }

  @media (max-width: 36rem) {
    .comic-search { align-items: stretch; flex-direction: column; }
    .search-field { width: 100%; }
    .quick-searches { grid-column: 1; }
    .login-actions { flex-direction: column; }
  }
  .ordinary-home { position:relative; min-height:clamp(330px,48vh,560px); display:grid; grid-template-columns:minmax(0,1.2fr) minmax(260px,.8fr); overflow:hidden; border:1px solid color-mix(in srgb,var(--v2-color-border) 84%,transparent); border-radius:clamp(var(--v2-radius-lg),1.5vw,1.75rem); background:radial-gradient(circle at 12% 18%,color-mix(in srgb,var(--v2-color-accent) 16%,transparent),transparent 28rem),linear-gradient(135deg,color-mix(in srgb,var(--v2-color-surface) 95%,transparent),color-mix(in srgb,var(--v2-color-accent) 7%,var(--v2-color-surface-subtle))); box-shadow:0 1rem 3rem color-mix(in srgb,#020617 14%,transparent); }
  .ordinary-home::before { position:absolute; inset:0; background:repeating-linear-gradient(125deg,transparent 0 42px,color-mix(in srgb,var(--v2-color-text) 3%,transparent) 42px 43px); content:""; pointer-events:none; }
  .ordinary-lead { position:relative; z-index:1; display:grid; align-content:center; justify-items:start; gap:16px; padding:clamp(28px,5vw,72px); border-right:1px solid var(--v2-color-border); }
  .ordinary-lead>span,.source-register>span,.recent-reading>span,.reading-ready>span { color:var(--v2-color-accent); font:700 8px/1 var(--font-mono); letter-spacing:.15em; }
  .ordinary-lead h2 { margin:0; font:720 clamp(2rem,4vw,4.8rem)/.9 var(--font-display); letter-spacing:-.065em; }
  .ordinary-lead p { max-width:58ch; margin:8px 0 0; color:var(--v2-color-text-secondary); font-size:12px; line-height:1.7; }
  .ordinary-side { position:relative; z-index:1; display:grid; grid-template-rows:auto 1fr; min-width:0; background:color-mix(in srgb,var(--v2-color-surface) 62%,transparent); backdrop-filter:blur(.75rem); }
  .source-register,.reading-ready { display:grid; align-content:start; gap:8px; padding:22px; border-bottom:1px solid var(--v2-color-border); }
  .source-register strong,.reading-ready strong { font-size:14px; }
  .source-register small,.reading-ready small { color:var(--v2-color-text-secondary); font-size:10px; line-height:1.5; }
  .recent-reading { display:grid; align-content:start; padding:22px; }
  .recent-reading>span { margin-bottom:12px; }
  .recent-reading button { min-width:0; display:grid; grid-template-columns:46px minmax(0,1fr); align-items:center; gap:11px; padding:9px 0; border:0; border-top:1px solid var(--v2-color-border); background:transparent; color:var(--v2-color-text); text-align:left; cursor:pointer; transition:padding-left .25s var(--ui-ease-out),color .2s ease; }
  .recent-reading button:last-child { border-bottom:1px solid var(--v2-color-border); }
  .recent-reading button:hover { padding-left:6px; color:var(--v2-color-accent); }
  .recent-reading img { width:46px; aspect-ratio:3/4; object-fit:cover; }
  .recent-reading button>span { min-width:0; }
  .recent-reading strong,.recent-reading small { display:block; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .recent-reading strong { font-size:11px; } .recent-reading small{margin-top:5px;color:var(--v2-color-text-secondary);font-size:9px}
  @media(max-width:760px){.ordinary-home{grid-template-columns:1fr}.ordinary-lead{border-right:0;border-bottom:1px solid var(--v2-color-border)} }
  @media(max-height:520px) and (orientation:landscape){
    .ordinary-home{min-height:100%;grid-template-columns:minmax(0,1.2fr) minmax(240px,.8fr)}
    .ordinary-lead{padding:20px;border-right:1px solid var(--v2-color-border);border-bottom:0}
    .ordinary-lead h2{font-size:clamp(1.8rem,5vw,3.2rem)}
    .source-register,.reading-ready,.recent-reading{padding:14px}
  }

  @media (prefers-reduced-motion: reduce) {
    .recent-reading button { transition: none; }
  }
  :global([data-motion="reduce"]) .recent-reading button { transition: none; }

  @media (max-width: 48rem) {
    .comic-page::after { display: none; }
    .comic-page-frame { padding: var(--v2-space-3); border-radius: var(--v2-radius-lg); backdrop-filter: none; }
  }

  @media (max-width: 32rem) {
    .comic-page { background: var(--v2-color-surface); }
    .comic-page::before { opacity: 0.35; }
    .comic-page-frame { padding: 0; border: 0; background: transparent; box-shadow: none; }
    :global(.ordinary-source-section) { padding: var(--v2-space-3); box-shadow: none; }
  }

  @media (prefers-contrast: more) {
    .comic-page-frame,
    .ordinary-home,
    :global(.ordinary-source-section) { background: var(--v2-color-surface); box-shadow: none; }
    .comic-page::before,
    .comic-page::after,
    .ordinary-home::before { display: none; }
  }
</style>


