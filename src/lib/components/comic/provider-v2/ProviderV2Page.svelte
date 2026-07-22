<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { createComicProviderStore } from "../../../features/comic/store";
  import { providerErrorMessage } from "../../../features/comic/logic";
  import type { ComicChapter, ComicFeatureState, ComicProviderConfigureRequest, ComicProviderDescriptor, ComicResolvedTarget, ComicSeries } from "../../../features/comic/types";
  import Icon from "../../Icon.svelte";
  import { Button, EmptyState, Input, LoadingSkeleton, Tag } from "../../ui";
  import { AsyncSection, ContentGrid, DetailPanel, FilterBar, MediaCard, PageHeader, PageShell } from "../../ui-v2";
  import ProviderConfigPanel from "./ProviderConfigPanel.svelte";
  import ProviderReader from "./ProviderReader.svelte";

  let { onlegacy }: { onlegacy: () => void } = $props();

  const providerStore = createComicProviderStore();
  let featureState = $state<ComicFeatureState>(providerStore.state);
  let query = $state("");
  let showConfig = $state(false);
  let selectedSeriesId = $state<string>();
  let activeChapter = $state<ComicChapter>();
  let readerTarget = $state<ComicResolvedTarget>();
  let actionError = $state("");
  let removalConfirmId = $state<string>();
  let detailReturnFocus = $state<HTMLElement | null>(null);
  let chapterReturnFocusKey = $state<string>();

  const selectedProvider = $derived(featureState.providers.find((provider) => provider.id === featureState.providerId));
  const selectedDetail = $derived(selectedSeriesId ? featureState.detailsBySeries[selectedSeriesId] : undefined);
  const selectedChapters = $derived(selectedSeriesId ? featureState.chaptersBySeries[selectedSeriesId] ?? [] : []);
  const currentProbe = $derived(selectedProvider ? featureState.probesByProvider[selectedProvider.id] : undefined);

  function sync() { featureState = providerStore.state; }

  async function run<T>(operation: () => Promise<T>, fallback: string): Promise<T | undefined> {
    actionError = "";
    try {
      const pending = operation();
      sync();
      const result = await pending;
      sync();
      return result;
    } catch (error) {
      sync();
      actionError = providerErrorMessage(error, fallback);
      return undefined;
    }
  }

  async function refreshProviders() {
    await run(() => providerStore.refreshProviders(), "无法读取已配置的漫画源");
    if (featureState.providers.length === 0) showConfig = true;
  }

  function selectProvider(providerId: string) {
    providerStore.selectProvider(providerId);
    selectedSeriesId = undefined;
    activeChapter = undefined;
    readerTarget = undefined;
    query = "";
    actionError = "";
    removalConfirmId = undefined;
    sync();
  }

  async function configure(request: ComicProviderConfigureRequest) {
    const provider = await run(() => providerStore.configureProvider(request), "漫画源配置失败");
    if (provider) {
      showConfig = false;
      query = "";
      selectedSeriesId = undefined;
    }
  }

  async function removeProvider(providerId: string) {
    if (removalConfirmId !== providerId) {
      removalConfirmId = providerId;
      return;
    }
    const removed = await run(() => providerStore.removeProvider(providerId), "移除漫画源失败");
    if (removed) {
      removalConfirmId = undefined;
      selectedSeriesId = undefined;
      readerTarget = undefined;
      if (featureState.providers.length === 0) showConfig = true;
    }
  }

  async function probe(providerId: string) {
    await run(() => providerStore.probe(providerId), "漫画源连接测试失败");
  }

  async function search() {
    if (!selectedProvider) return;
    selectedSeriesId = undefined;
    readerTarget = undefined;
    await run(() => providerStore.search(selectedProvider.id, { query: query.trim(), page: 1, pageSize: 50 }), "搜索漫画失败");
  }

  async function openSeries(series: ComicSeries, event?: MouseEvent) {
    detailReturnFocus = event?.currentTarget instanceof HTMLElement ? event.currentTarget : document.activeElement instanceof HTMLElement ? document.activeElement : null;
    selectedSeriesId = series.id;
    activeChapter = undefined;
    readerTarget = undefined;
    await run(() => providerStore.loadSeries(series.providerId, series.id), "加载漫画详情失败");
  }

  function closeDetail() {
    providerStore.cancelPending();
    selectedSeriesId = undefined;
    activeChapter = undefined;
    sync();
    queueMicrotask(() => detailReturnFocus?.focus({ preventScroll: true }));
  }

  async function openChapter(chapter: ComicChapter, event?: MouseEvent) {
    chapterReturnFocusKey = event?.currentTarget instanceof HTMLElement ? event.currentTarget.dataset.chapterFocusKey : chapter.identity.stableKey;
    if (!selectedProvider || !selectedSeriesId) return;
    activeChapter = chapter;
    const target = await run(
      () => providerStore.resolve(selectedProvider.id, selectedSeriesId!, chapter.identity.chapterId),
      "解析章节失败",
    );
    if (target) readerTarget = target;
  }

  async function retryChapter() {
    if (!activeChapter) return;
    readerTarget = undefined;
    await openChapter(activeChapter);
  }

  async function closeReader() {
    if (activeChapter) providerStore.clearTarget(activeChapter.identity.chapterId);
    readerTarget = undefined;
    activeChapter = undefined;
    sync();
    await tick();
    if (chapterReturnFocusKey) document.querySelector<HTMLElement>(`[data-chapter-focus-key="${CSS.escape(chapterReturnFocusKey)}"]`)?.focus({ preventScroll: true });
  }

  async function retryCurrentAction() {
    if (activeChapter) return retryChapter();
    if (selectedSeriesId) {
      const series = featureState.series.find((item) => item.id === selectedSeriesId);
      if (series) return openSeries(series);
    }
    if (selectedProvider) return search();
    return refreshProviders();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    if (readerTarget) return;
    if (selectedSeriesId) closeDetail();
    else if (showConfig && featureState.providers.length > 0) showConfig = false;
  }

  function providerKindLabel(provider: ComicProviderDescriptor): string {
    if (provider.kind === "local") return "本地";
    return provider.kind === "komga" ? "Komga" : "Kavita";
  }

  function authLabel(provider: ComicProviderDescriptor): string {
    if (provider.authMode === "none") return "无认证";
    if (provider.authMode === "api_key") return "API Key";
    return provider.authMode === "basic" ? "Basic" : "Bearer";
  }

  onMount(refreshProviders);
  onDestroy(() => providerStore.cancelPending());
</script>

<svelte:window onkeydown={handleKeydown} />

<PageShell as="div" width="full" ariaLabel="Comic Provider v2" class="provider-page" labelledBy="comic-provider-title">
  <div class="provider-frame" data-testid="comic-provider-v2">
    {#snippet headerActions()}
      <Button variant="ghost" size="sm" press={onlegacy}><Icon name="chevronLeft" size={14} />返回普通 / PicACG</Button>
      <Button variant="secondary" size="sm" press={() => (showConfig = !showConfig)}><Icon name={showConfig ? "x" : "plus"} size={14} />{showConfig ? "收起配置" : "添加漫画源"}</Button>
    {/snippet}

    <PageHeader
      id="comic-provider-title"
      eyebrow="Comic Provider v2"
      title="漫画源"
      description="本地漫画、Komga 与 Kavita 的统一安全阅读入口。"
      actions={headerActions}
    />

    {#if showConfig}
      <div class="config-wrap">
        <ProviderConfigPanel busy={featureState.loading} onconfigure={configure} oncancel={featureState.providers.length > 0 ? () => showConfig = false : undefined} />
      </div>
    {/if}

    {#if actionError}
      <div class="global-error" role="alert">
        <Icon name="x" size={15} />
        <span>{actionError}</span>
        <Button variant="quiet" size="sm" press={retryCurrentAction}>重试</Button>
        <Button variant="quiet" size="sm" press={() => (actionError = "")}>关闭</Button>
      </div>
    {/if}

    {#if featureState.providers.length > 0}
      <div class="provider-rail" role="list" aria-label="已配置漫画源">
        {#each featureState.providers as provider (provider.id)}
          {@const probeResult = featureState.probesByProvider[provider.id]}
          <article class="provider-chip" class:active={featureState.providerId === provider.id} role="listitem">
            <button class="provider-select" type="button" onclick={() => selectProvider(provider.id)} aria-pressed={featureState.providerId === provider.id}>
              <span class="provider-mark"><Icon name={provider.kind === "local" ? "folder" : "globe"} size={16} /></span>
              <span class="provider-copy"><strong>{provider.name}</strong><small>{providerKindLabel(provider)} · {authLabel(provider)}</small></span>
              <span class="status-dot" class:ok={probeResult?.reachable && probeResult?.authenticated} title={probeResult ? (probeResult.reachable && probeResult.authenticated ? "连接正常" : "连接异常") : "尚未测试"}></span>
            </button>
            <div class="provider-tools">
              <Button variant="quiet" size="sm" press={() => probe(provider.id)} disabled={featureState.loading} title="测试连接" ariaLabel={`测试 ${provider.name} 连接`}><Icon name="zap" size={13} /></Button>
              <Button variant="quiet" size="sm" press={() => removeProvider(provider.id)} disabled={featureState.loading} title={removalConfirmId === provider.id ? "再次点击确认移除" : "移除漫画源"} ariaLabel={removalConfirmId === provider.id ? `确认移除 ${provider.name}` : `移除 ${provider.name}`}><Icon name={removalConfirmId === provider.id ? "check" : "trash"} size={13} /></Button>
            </div>
          </article>
        {/each}
      </div>
    {/if}

    {#if selectedProvider}
      {#snippet providerSearchControls()}
        <form class="provider-search" onsubmit={(event) => { event.preventDefault(); void search(); }}>
          <label class="search-field">
            <Icon name="search" size={16} />
            <Input bind:value={query} placeholder={selectedProvider.kind === "local" ? "输入标题；留空可列出全部本地漫画" : "搜索此服务器中的漫画"} ariaLabel="搜索 Provider v2 漫画" disabled={featureState.loading} />
          </label>
          <Button type="submit" variant="secondary" loading={featureState.loading}>搜索</Button>
        </form>
      {/snippet}

      <FilterBar controls={providerSearchControls} label="Provider v2 漫画搜索" activeCount={query.trim() ? 1 : 0} onClear={() => (query = "")} busy={featureState.loading} />

      <div class="workspace">
        <aside class="source-panel">
          <div class="source-meta"><div class="source-icon"><Icon name={selectedProvider.kind === "local" ? "folder" : "globe"} size={20} /></div><div><span>当前漫画源</span><h2>{selectedProvider.name}</h2></div></div>
          <dl>
            <div><dt>类型</dt><dd>{providerKindLabel(selectedProvider)}</dd></div>
            <div><dt>认证</dt><dd>{authLabel(selectedProvider)}{selectedProvider.secretConfigured ? " · 已安全配置" : ""}</dd></div>
            <div><dt>位置</dt><dd title={selectedProvider.localRoot ?? selectedProvider.baseUrl}>{selectedProvider.localRoot ?? selectedProvider.baseUrl ?? "-"}</dd></div>
          </dl>
          {#if currentProbe}
            <div class="probe-card" class:bad={!currentProbe.reachable || !currentProbe.authenticated}>
              <strong>{currentProbe.reachable && currentProbe.authenticated ? "连接正常" : "需要检查配置"}</strong>
              <span>{currentProbe.latencyMs != null ? `${currentProbe.latencyMs} ms` : "延迟未知"} · {currentProbe.libraries.length} 个资料库</span>
            </div>
          {/if}
          <Button fullWidth variant="secondary" press={() => probe(selectedProvider.id)} loading={featureState.loading}>测试连接</Button>
        </aside>

        <AsyncSection
          title="漫画目录"
          description={`${selectedProvider.name} · ${featureState.series.length} 部结果`}
          state={featureState.loading && featureState.series.length === 0 ? "loading" : featureState.loading ? "refreshing" : featureState.series.length > 0 ? "ready" : "empty"}
          preserveContent={featureState.series.length > 0}
          loadingDelayMs={0}
          class="catalog-panel"
        >
          <ContentGrid minItemWidth="15rem" gap="md" label={`${selectedProvider.name} 漫画结果`} busy={featureState.loading}>
            {#each featureState.series as series (series.id)}
              <MediaCard
                title={series.title}
                subtitle={[series.year, series.language].filter(Boolean).join(" · ") || providerKindLabel(selectedProvider)}
                description={series.summary}
                variant="landscape"
                focusKey={`provider-series:${series.providerId}:${series.id}`}
                ariaLabel={`查看 ${series.title} 详情`}
                onActivate={(event) => void openSeries(series, event)}
              >
                {#snippet badges()}<span class="series-kind">{series.language ?? "COMIC"}</span>{/snippet}
              </MediaCard>
            {/each}
          </ContentGrid>
        </AsyncSection>
      </div>
    {:else if !showConfig && !featureState.loading}
      <AsyncSection title="漫画源" state="empty" description="添加本地目录、Komga 或 Kavita 后即可搜索和阅读。" primaryAction={{ label: "添加漫画源", onSelect: () => (showConfig = true) }} />
    {/if}

    <DetailPanel
      open={Boolean(selectedSeriesId)}
      title={selectedDetail?.series.title ?? featureState.series.find((item) => item.id === selectedSeriesId)?.title ?? "漫画详情"}
      description={selectedDetail?.series.summary}
      onClose={closeDetail}
      size="lg"
      returnFocus={detailReturnFocus ?? true}
      class="provider-detail-panel"
    >
      {#if featureState.loading && !selectedDetail}
        <AsyncSection title="漫画详情" state="loading" loadingDelayMs={0} />
      {:else if selectedDetail}
        <div class="detail-summary">
          <div class="detail-tags">
            {#if selectedDetail.status}<Tag>{selectedDetail.status}</Tag>{/if}
            {#if selectedDetail.series.language}<Tag>{selectedDetail.series.language}</Tag>{/if}
            {#each selectedDetail.genres as genre}<Tag variant="muted">{genre}</Tag>{/each}
          </div>
        </div>
        <AsyncSection title="章节" description={`${selectedChapters.length} 条`} state={selectedChapters.length > 0 ? "ready" : "empty"} compact>
          <div class="chapter-list" aria-label={`${selectedDetail.series.title} 章节列表`}>
            {#each selectedChapters as chapter (chapter.identity.stableKey)}
              <button type="button" data-chapter-focus-key={chapter.identity.stableKey} onclick={(event) => void openChapter(chapter, event)} disabled={featureState.loading}>
                <span class="chapter-order">{chapter.sort.chapterNumber ?? chapter.sort.ordinal ?? "·"}</span>
                <span class="chapter-copy"><strong>{chapter.title}</strong><small>{[chapter.language, chapter.pageCount ? `${chapter.pageCount} 页` : undefined, chapter.fileName].filter(Boolean).join(" · ")}</small></span>
                <Icon name="chevronRight" size={15} />
              </button>
            {/each}
          </div>
        </AsyncSection>
      {:else}
        <AsyncSection title="详情加载失败" state="error" description={actionError || "请稍后重试"} primaryAction={{ label: "重试", onSelect: () => { const series = featureState.series.find((item) => item.id === selectedSeriesId); if (series) void openSeries(series); } }} />
      {/if}
    </DetailPanel>

    {#if readerTarget && activeChapter && selectedProvider && selectedDetail}
      <ProviderReader
        provider={selectedProvider}
        target={readerTarget}
        title={selectedDetail.series.title}
        chapterTitle={activeChapter.title}
        onclose={closeReader}
        onretry={retryChapter}
        returnFocusKey={chapterReturnFocusKey}
      />
    {/if}
  </div>
</PageShell>
<style>
  :global(.provider-page) {
    height: 100%;
    min-height: 0;
    background:
      radial-gradient(circle at 90% 10%, color-mix(in srgb, var(--v2-color-accent) 12%, transparent), transparent 30rem),
      repeating-linear-gradient(120deg, transparent 0 42px, color-mix(in srgb, var(--v2-color-text) 2.5%, transparent) 42px 43px);
  }
  :global(.provider-page .v2-page-shell__inner) { display: flex; min-height: 100%; flex-direction: column; }
  .provider-frame { position: relative; display: flex; width: min(100%, 108rem); min-height: 100%; margin-inline: auto; padding: clamp(var(--v2-space-3), 1.4vw, var(--v2-space-5)); flex-direction: column; gap: var(--v2-space-5); border: 1px solid color-mix(in srgb, var(--v2-color-border) 78%, transparent); border-radius: clamp(var(--v2-radius-lg), 1.5vw, 1.75rem); background: color-mix(in srgb, var(--v2-color-surface) 84%, transparent); box-shadow: 0 1.5rem 4rem color-mix(in srgb, #020617 16%, transparent); backdrop-filter: blur(1rem) saturate(112%); }
  .config-wrap { width: 100%; }

  .global-error { display: flex; align-items: center; gap: var(--v2-space-2); padding: var(--v2-space-3); border: 1px solid rgba(248,113,113,.32); border-radius: var(--v2-radius-md); background: rgba(248,113,113,.08); color: #fca5a5; }
  .global-error > span { min-width: 0; flex: 1; }

  .provider-rail { display: flex; padding-bottom: var(--v2-space-2); gap: var(--v2-space-2); overflow-x: auto; }
  .provider-chip { display: flex; min-width: 15rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-lg); background: linear-gradient(145deg, var(--v2-color-surface), color-mix(in srgb, var(--v2-color-surface-subtle) 92%, var(--v2-color-accent))); box-shadow: 0 .45rem 1.2rem color-mix(in srgb, #020617 8%, transparent); }
  .provider-chip.active { border-color: var(--v2-color-accent); background: color-mix(in srgb, var(--v2-color-accent) 10%, var(--v2-color-surface)); }
  .provider-select { display: grid; min-width: 0; min-height: 3.5rem; padding: .55rem .7rem; flex: 1; grid-template-columns: auto minmax(0,1fr) auto; align-items: center; gap: .55rem; border: 0; background: transparent; color: var(--v2-color-text); text-align: left; cursor: pointer; }
  .provider-select:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .provider-mark, .source-icon { display: grid; width: 2.25rem; height: 2.25rem; place-items: center; border-radius: var(--v2-radius-md); background: var(--v2-color-surface-subtle); color: var(--v2-color-accent); }
  .provider-copy { min-width: 0; }
  .provider-copy strong, .provider-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .provider-copy small { margin-top: .15rem; color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  .status-dot { width: .5rem; height: .5rem; border-radius: 50%; background: var(--v2-color-text-secondary); }
  .status-dot.ok { background: #34d399; box-shadow: 0 0 .6rem rgba(52,211,153,.55); }
  .provider-tools { display: flex; align-items: center; padding-right: .35rem; }

  .provider-search { display: flex; min-width: min(100%, 30rem); flex: 1; gap: var(--v2-space-2); }
  .search-field { min-width: 0; flex: 1; }

  .workspace { display: grid; min-height: 0; grid-template-columns: minmax(14rem, 18rem) minmax(0, 1fr); align-items: start; gap: var(--v2-space-5); }
  .source-panel { position: sticky; top: 0; display: flex; padding: var(--v2-space-4); flex-direction: column; gap: var(--v2-space-4); overflow: hidden; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-lg); background: radial-gradient(circle at 100% 0, color-mix(in srgb, var(--v2-color-accent) 13%, transparent), transparent 13rem), var(--v2-color-surface); box-shadow: 0 .75rem 2rem color-mix(in srgb, #020617 10%, transparent); }
  .source-meta { display: flex; align-items: center; gap: var(--v2-space-3); }
  .source-meta span { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  .source-meta h2 { margin: .15rem 0 0; font-size: var(--v2-text-lg); }
  .source-panel dl { display: grid; gap: var(--v2-space-2); margin: 0; }
  .source-panel dl div { display: grid; grid-template-columns: 3.3rem minmax(0,1fr); gap: var(--v2-space-2); }
  .source-panel dt { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  .source-panel dd { min-width: 0; margin: 0; overflow: hidden; font-size: var(--v2-text-xs); text-overflow: ellipsis; white-space: nowrap; }
  .probe-card { display: flex; padding: var(--v2-space-3); flex-direction: column; gap: .2rem; border: 1px solid rgba(52,211,153,.28); border-radius: var(--v2-radius-md); background: rgba(52,211,153,.07); }
  .probe-card.bad { border-color: rgba(248,113,113,.3); background: rgba(248,113,113,.07); }
  .probe-card span { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  :global(.catalog-panel) { min-width: 0; padding: clamp(var(--v2-space-3), 1.2vw, var(--v2-space-4)); border: 1px solid color-mix(in srgb, var(--v2-color-border) 82%, transparent); border-radius: var(--v2-radius-lg); background: color-mix(in srgb, var(--v2-color-surface) 88%, transparent); }
  .series-kind { display: inline-flex; min-height: 1.4rem; align-items: center; padding: 0 .45rem; border-radius: var(--v2-radius-sm); background: var(--v2-color-accent); color: #fff; font-size: var(--v2-text-xs); font-weight: 700; }

  .detail-summary { margin-bottom: var(--v2-space-4); }
  .detail-tags { display: flex; flex-wrap: wrap; gap: var(--v2-space-2); }
  .chapter-list { display: grid; gap: var(--v2-space-2); }
  .chapter-list button { display: grid; min-height: 3.25rem; padding: .65rem .8rem; grid-template-columns: 2.4rem minmax(0,1fr) auto; align-items: center; gap: var(--v2-space-2); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: var(--v2-color-surface); color: var(--v2-color-text); text-align: left; cursor: pointer; }
  .chapter-list button:hover { border-color: var(--v2-color-accent); }
  .chapter-list button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .chapter-order { display: grid; width: 2rem; height: 2rem; place-items: center; border-radius: var(--v2-radius-sm); background: var(--v2-color-surface-subtle); color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  .chapter-copy { min-width: 0; }
  .chapter-copy strong, .chapter-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chapter-copy small { margin-top: .15rem; color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }

  @media (max-width: 52rem) {
    .workspace { grid-template-columns: 1fr; }
    .source-panel { position: static; }
  }

  @media (max-width: 36rem) {
    .provider-search { flex-direction: column; }
    .provider-chip { min-width: 13rem; }
    .provider-frame { padding: 0; border: 0; background: transparent; box-shadow: none; backdrop-filter: none; }
    :global(.catalog-panel) { padding: var(--v2-space-3); }
  }

  @media (prefers-contrast: more) {
    :global(.provider-page),
    .provider-frame,
    .provider-chip,
    .source-panel,
    :global(.catalog-panel) { background: var(--v2-color-surface); box-shadow: none; }
  }
</style>
