<script lang="ts">
  import { onMount } from "svelte";
  import {
    createAnimeProviderFeatureStore,
    createTauriAnimeProviderApi,
    type AnimeEpisode,
    type AnimeProviderHealth,
    type ProviderError,
  } from "../../../features/anime";
  import Icon from "../../Icon.svelte";
  import ProviderConfigPanel from "./ProviderConfigPanel.svelte";
  import ProviderV2Player from "./ProviderV2Player.svelte";
  import { AsyncSection, FilterBar, MediaCard, PageHeader, PageShell } from "../../ui-v2";

  let { onExit }: { onExit: () => void } = $props();

  const api = createTauriAnimeProviderApi();
  const store = createAnimeProviderFeatureStore(api);
  let snapshot = $state(store.getSnapshot());
  let queryInput = $state("");
  let configOpen = $state(false);
  let activeEpisode = $state<AnimeEpisode | null>(null);
  let activeEpisodeTrigger = $state<HTMLElement | null>(null);

  const selectedProvider = $derived(snapshot.providers.find((provider) => provider.id === snapshot.selectedProviderId) ?? null);
  const selectedProviderLabel = $derived(selectedProvider?.name ?? "全部来源");
  const canSearch = $derived(snapshot.providers.length > 0 && !snapshot.isSearching);
  const detailTitle = $derived(snapshot.selectedDetail?.title ?? "选择一个搜索结果");

  function healthFor(providerId: string): AnimeProviderHealth | null {
    const rows = snapshot.providerHealth.filter((health) => health.providerId === providerId);
    if (rows.some((health) => health.state === "open_circuit")) return rows.find((health) => health.state === "open_circuit") ?? null;
    if (rows.some((health) => health.state === "degraded")) return rows.find((health) => health.state === "degraded") ?? null;
    return rows.find((health) => health.state === "healthy") ?? rows[0] ?? null;
  }

  function healthLabel(health: AnimeProviderHealth | null): string {
    if (!health) return "未检测";
    if (health.state === "healthy") return "正常";
    if (health.state === "degraded") return "波动";
    if (health.state === "open_circuit") return "暂停请求";
    if (health.state === "disabled") return "已停用";
    return "未知";
  }

  function friendlyError(error: ProviderError): string {
    switch (error.kind) {
      case "auth_required": return "来源凭据无效或已失效，请重新配置。";
      case "network":
      case "timeout": return "来源暂时无法连接，请稍后重试。";
      case "rate_limited": return "来源请求过于频繁，请稍后重试。";
      case "unsupported_drm": return "该剧集使用了内置播放器不支持的 DRM。";
      case "policy_blocked": return "该请求未通过安全策略，请检查来源地址或目录。";
      case "unsupported": return "该来源暂不支持此操作。";
      case "cancelled": return "请求已取消。";
      default: return error.message || "来源请求失败，请重试。";
    }
  }

  async function submitSearch(event: SubmitEvent) {
    event.preventDefault();
    if (!canSearch) return;
    await store.search(queryInput.trim());
  }

  async function openResult(providerId: string, itemId: string) {
    activeEpisode = null;
    await Promise.all([
      store.loadDetail(providerId, itemId),
      store.loadEpisodes(providerId, itemId),
    ]);
  }

  async function playEpisode(episode: AnimeEpisode, trigger?: HTMLElement) {
    activeEpisodeTrigger = trigger ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    activeEpisode = episode;
    const resolution = await store.resolve(episode.identity);
    if (!resolution) activeEpisode = null;
  }

  async function openFallback() {
    if (!activeEpisode) return;
    await store.openFallback(activeEpisode.identity);
  }

  function closePlayer() {
    activeEpisode = null;
    queueMicrotask(() => activeEpisodeTrigger?.focus({ preventScroll: true }));
  }

  function exitProviderV2() {
    store.cancelSearch();
    store.cancelContent();
    onExit();
  }

  onMount(() => {
    const unsubscribe = store.subscribe((next) => snapshot = next);
    void (async () => {
      await store.refreshProviders();
      if (store.getSnapshot().providers.length === 0) configOpen = true;
      await store.refreshHealth();
    })();
    return () => {
      unsubscribe();
      store.cancelSearch();
      store.cancelContent();
    };
  });
</script>

<PageShell as="div" ariaLabel="Anime Provider v2 来源工作台" width="full" class="provider-v2-shell">
  <section class="provider-workspace" data-testid="anime-provider-v2">
  {#snippet workspaceActions()}
    <button class="back-button" type="button" onclick={exitProviderV2}>
      <Icon name="arrowLeft" size={15} />经典模式
    </button>
    <button class="config-button" type="button" onclick={() => configOpen = true}>
      <Icon name="settings" size={16} />来源配置
    </button>
  {/snippet}

  <PageHeader
    title="来源工作台"
    eyebrow="Anime Provider v2"
    description="统一浏览本地媒体与 Jellyfin；经典规则源仍可随时作为回退。"
    actions={workspaceActions}
    id="anime-provider-v2-title"
  />

  {#snippet providerSearchControls()}
    <form class="provider-search" onsubmit={submitSearch}>
      <Icon name="search" size={16} />
      <input
        type="search"
        bind:value={queryInput}
        placeholder={snapshot.providers.length > 0 ? `在${selectedProviderLabel}中搜索，留空可浏览` : "请先配置来源"}
        disabled={snapshot.providers.length === 0}
        aria-label="搜索 Provider v2 番剧"
        data-autofocus
      />
      {#if snapshot.isSearching}
        <button class="cancel-search" type="button" onclick={() => store.cancelSearch()}>取消</button>
      {:else}
        <button class="search-submit" type="submit" disabled={!canSearch}>搜索</button>
      {/if}
    </form>
  {/snippet}

  <FilterBar
    controls={providerSearchControls}
    label="Provider v2 搜索"
    activeCount={queryInput.trim() ? 1 : 0}
    onClear={() => queryInput = ""}
    busy={snapshot.isSearching}
    class="provider-v2-filter"
  />

  <div class="workspace-body">
    <aside class="provider-rail" aria-label="Provider v2 来源列表">
      <div class="rail-heading">
        <span>来源</span>
        <button type="button" aria-label="刷新来源状态" onclick={() => { void store.refreshProviders(); void store.refreshHealth(); }}>
          <Icon name="refresh" size={14} />
        </button>
      </div>

      <button class="provider-chip" class:active={snapshot.selectedProviderId === null} type="button" onclick={() => store.selectProvider(null)} disabled={snapshot.providers.length === 0}>
        <span class="provider-icon"><Icon name="layers" size={16} /></span>
        <span><strong>全部来源</strong><small>聚合搜索</small></span>
      </button>

      {#each snapshot.providers as provider (provider.id)}
        {@const health = healthFor(provider.id)}
        <button class="provider-chip" class:active={snapshot.selectedProviderId === provider.id} type="button" onclick={() => store.selectProvider(provider.id)}>
          <span class="provider-icon"><Icon name={provider.kind === "local_media" ? "folder" : "database"} size={16} /></span>
          <span class="provider-meta">
            <strong>{provider.name}</strong>
            <small>{provider.kind === "local_media" ? `${provider.localFileCount ?? 0} 个文件` : "自托管媒体库"}</small>
          </span>
          <span class:healthy={health?.state === "healthy"} class:warning={health?.state === "degraded" || health?.state === "open_circuit"} class="health-dot" title={healthLabel(health)}></span>
        </button>
      {/each}

      {#if snapshot.providers.length === 0 && !snapshot.isLoadingProviders}
        <div class="rail-empty">
          <Icon name="database" size={22} />
          <p>尚未配置来源</p>
          <button type="button" onclick={() => configOpen = true}>添加来源</button>
        </div>
      {/if}

      <div class="legacy-note">
        <Icon name="shield" size={15} />
        <p>Provider v2 不会替换规则源。经典模式会一直保留作为回退。</p>
      </div>
    </aside>

    <div class="results-pane" role="region" aria-labelledby="provider-results-title">
      <div class="pane-heading">
        <div>
          <span class="eyebrow">Catalog</span>
          <h2 id="provider-results-title">{snapshot.query ? `“${snapshot.query}”的结果` : "来源内容"}</h2>
        </div>
        {#if snapshot.searchItems.length > 0}<span>{snapshot.searchItems.length} 项</span>{/if}
      </div>

      {#if snapshot.isSearching}
        <div class="result-skeletons" aria-label="正在搜索">
          {#each Array(6) as _}
            <div class="result-skeleton"><span></span><div><i></i><i></i><i></i></div></div>
          {/each}
        </div>
      {:else if snapshot.searchItems.length > 0}
        <div class="result-list">
          {#each snapshot.searchItems as item (item.providerId + item.itemId)}
            <MediaCard
              title={item.title}
              subtitle={item.originalTitle && item.originalTitle !== item.title ? item.originalTitle : (snapshot.providers.find((provider) => provider.id === item.providerId)?.name ?? item.providerId)}
              description={item.synopsis || "暂无简介"}
              imageSrc={item.artworkUrl ?? undefined}
              imageAlt={item.title}
              variant="landscape"
              selected={snapshot.selectedDetail?.providerId === item.providerId && snapshot.selectedDetail?.itemId === item.itemId}
              focusKey={`anime-provider-result-${item.providerId}-${item.itemId}`}
              ariaLabel={`查看 ${item.title} 详情`}
              onActivate={() => openResult(item.providerId, item.itemId)}
              class="provider-result-card"
            />
          {/each}
        </div>
      {:else if snapshot.providers.length === 0}
        <div class="large-empty">
          <span><Icon name="database" size={30} /></span>
          <h3>先连接一个番剧来源</h3>
          <p>选择本地媒体目录，或连接 Jellyfin。经典规则源不会受到影响。</p>
          <button type="button" onclick={() => configOpen = true}><Icon name="plus" size={15} />配置来源</button>
        </div>
      {:else}
        <div class="large-empty quiet">
          <span><Icon name="search" size={28} /></span>
          <h3>{snapshot.query ? "没有匹配结果" : "开始浏览来源"}</h3>
          <p>{snapshot.query ? "尝试更短的关键词，或切换到其他来源。" : "搜索框可以留空，以浏览当前来源中的内容。"}</p>
        </div>
      {/if}

      {#if snapshot.searchFailures.length > 0}
        <div class="partial-warning">
          <Icon name="info" size={15} />
          <span>{snapshot.searchFailures.length} 个来源未能返回结果，其他可用结果仍已保留。</span>
        </div>
      {/if}
    </div>

    <aside class="detail-pane">
      <div class="detail-scroll">
        {#if snapshot.isLoadingDetail}
          <div class="detail-loading"><span class="large-spinner"></span><p>正在读取详情</p></div>
        {:else if snapshot.selectedDetail}
          <div class="detail-hero">
            <div class="detail-art">
              {#if snapshot.selectedDetail.artworkUrl}<img src={snapshot.selectedDetail.artworkUrl} alt="" />{:else}<Icon name="film" size={32} />{/if}
            </div>
            <div class="detail-title">
              <span class="eyebrow">{selectedProviderLabel}</span>
              <h2>{snapshot.selectedDetail.title}</h2>
              {#if snapshot.selectedDetail.originalTitle}<p>{snapshot.selectedDetail.originalTitle}</p>{/if}
            </div>
          </div>
          {#if snapshot.selectedDetail.genres.length > 0}
            <div class="genre-row">{#each snapshot.selectedDetail.genres as genre}<span>{genre}</span>{/each}</div>
          {/if}
          <p class="synopsis">{snapshot.selectedDetail.synopsis || "该来源暂未提供简介。"}</p>

          <div class="episode-heading">
            <h3>剧集</h3>
            {#if snapshot.episodes.length > 0}<span>{snapshot.episodes.length} 集</span>{/if}
          </div>
          {#if snapshot.isLoadingEpisodes}
            <div class="episode-loading"><span class="spinner"></span>正在读取剧集</div>
          {:else if snapshot.episodes.length > 0}
            <div class="episode-list">
              {#each snapshot.episodes as episode (episode.identity.episodeId)}
                <button
                  type="button"
                  data-focus-key={`anime-provider-episode-${episode.identity.episodeId}`}
                  onclick={(event) => playEpisode(episode, event.currentTarget as HTMLElement)}
                  disabled={snapshot.isResolving}
                >
                  <span class="episode-number">{episode.number ?? "—"}</span>
                  <span class="episode-title">{episode.title}</span>
                  <span class="play-mark"><Icon name="play" size={15} /></span>
                </button>
              {/each}
            </div>
          {:else}
            <div class="episode-empty">该来源没有返回可播放剧集。</div>
          {/if}
        {:else}
          <div class="detail-placeholder">
            <span><Icon name="film" size={28} /></span>
            <h3>{detailTitle}</h3>
            <p>详情、剧集和播放解析会在此处完成，并通过请求代次隔离过期响应。</p>
          </div>
        {/if}
      </div>
    </aside>
  </div>

  {#if snapshot.error && snapshot.error.kind !== "cancelled"}
    <div class="error-toast" role="alert">
      <Icon name="info" size={17} />
      <div><strong>操作未完成</strong><p>{friendlyError(snapshot.error)}</p></div>
      <button type="button" aria-label="关闭错误提示" onclick={() => store.clearError()}><Icon name="x" size={14} /></button>
    </div>
  {/if}

  {#if configOpen}
    <ProviderConfigPanel {api} {store} {snapshot} onClose={() => configOpen = false} />
  {/if}

  {#if snapshot.resolution && activeEpisode}
    <ProviderV2Player
      resolution={snapshot.resolution}
      episode={activeEpisode}
      seriesTitle={snapshot.selectedDetail?.title ?? activeEpisode.title}
      openingFallback={snapshot.isOpeningFallback}
      onClose={closePlayer}
      onFallback={openFallback}
    />
  {/if}
</section>
</PageShell>

<style>
  :global(.v2-page-shell.provider-v2-shell) { height: 100%; padding: 0; }
  :global(.v2-page-shell.provider-v2-shell > .v2-page-shell__inner) { height: 100%; max-width: none; padding: 0; }
  :global(.provider-v2-filter) { margin: 0 18px 12px; }
  :global(.provider-result-card) { --v2-media-landscape-media-width: 5rem; }

  .provider-workspace {
    --v2-accent: #67b7a3;
    --v2-panel: #11161d;
    --v2-panel-2: #151b23;
    position: relative;
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background:
      radial-gradient(circle at 74% -20%, rgba(103,183,163,0.11), transparent 35%),
      #0c1016;
    color: #edf1f5;
  }
  .back-button, .config-button, .search-submit, .cancel-search {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border-radius: 8px;
    font: inherit;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
  }
  .back-button { height: 32px; padding: 0 9px; border: 1px solid rgba(255,255,255,0.09); background: rgba(255,255,255,0.03); color: #aab2bf; }
  .eyebrow { display: block; color: var(--v2-accent); font-family: var(--font-mono); font-size: 9px; font-weight: 750; letter-spacing: .12em; text-transform: uppercase; }
  h2, h3, p { margin: 0; }
  .provider-search { height: 40px; display: flex; align-items: center; gap: 9px; padding-left: 12px; border: 1px solid rgba(255,255,255,0.1); border-radius: 10px; background: rgba(255,255,255,0.035); color: #7f8997; }
  .provider-search:focus-within { border-color: rgba(103,183,163,.52); box-shadow: 0 0 0 3px rgba(103,183,163,.07); color: #d9dee5; }
  .provider-search input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: #edf1f5; font: inherit; font-size: 12px; }
  .provider-search input::placeholder { color: #687280; }
  .search-submit, .cancel-search { align-self: stretch; min-width: 65px; margin: 3px; border: 0; }
  .search-submit { background: var(--v2-accent); color: #07120f; }
  .cancel-search { background: rgba(255,255,255,.07); color: #d0d5dc; }
  .config-button { height: 38px; padding: 0 13px; border: 1px solid rgba(103,183,163,.28); background: rgba(103,183,163,.075); color: #88caba; }
  button:disabled { opacity: .48; cursor: wait; }
  .workspace-body { min-height: 0; flex: 1; display: grid; grid-template-columns: 205px minmax(360px, 1.05fr) minmax(330px, .95fr); }
  .provider-rail, .results-pane, .detail-pane { min-height: 0; }
  .provider-rail { display: flex; flex-direction: column; gap: 7px; padding: 16px 12px; border-right: 1px solid rgba(255,255,255,.07); background: rgba(14,18,25,.7); overflow-y: auto; }
  .rail-heading { display: flex; align-items: center; justify-content: space-between; padding: 0 6px 6px; color: #707b89; font-family: var(--font-mono); font-size: 9px; font-weight: 750; letter-spacing: .1em; text-transform: uppercase; }
  .rail-heading button { width: 28px; height: 28px; display: grid; place-items: center; border: 0; border-radius: 7px; background: transparent; color: #76818f; cursor: pointer; }
  .rail-heading button:hover { background: rgba(255,255,255,.05); color: #cbd1d9; }
  .provider-chip { width: 100%; min-width: 0; display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 9px; padding: 9px; border: 1px solid transparent; border-radius: 10px; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .provider-chip:hover { background: rgba(255,255,255,.035); }
  .provider-chip.active { border-color: rgba(103,183,163,.24); background: rgba(103,183,163,.07); }
  .provider-icon { width: 31px; height: 31px; display: grid; place-items: center; border-radius: 8px; background: rgba(255,255,255,.05); color: #8e98a5; }
  .provider-chip.active .provider-icon { background: rgba(103,183,163,.13); color: var(--v2-accent); }
  .provider-chip > span:nth-child(2), .provider-meta { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .provider-chip strong { overflow: hidden; color: #dfe4ea; font-size: 11.5px; text-overflow: ellipsis; white-space: nowrap; }
  .provider-chip small { overflow: hidden; color: #697482; font-size: 9.5px; text-overflow: ellipsis; white-space: nowrap; }
  .health-dot { width: 6px; height: 6px; border-radius: 50%; background: #596270; }
  .health-dot.healthy { background: #55bd8e; box-shadow: 0 0 0 3px rgba(85,189,142,.09); }
  .health-dot.warning { background: #d9a65d; box-shadow: 0 0 0 3px rgba(217,166,93,.09); }
  .rail-empty { display: flex; flex-direction: column; align-items: center; gap: 8px; margin: 18px 5px; padding: 20px 10px; border: 1px dashed rgba(255,255,255,.09); border-radius: 11px; color: #687380; text-align: center; }
  .rail-empty p { font-size: 11px; }
  .rail-empty button { border: 0; background: transparent; color: var(--v2-accent); font-size: 11px; cursor: pointer; }
  .legacy-note { margin-top: auto; display: flex; align-items: flex-start; gap: 8px; padding: 11px; border-top: 1px solid rgba(255,255,255,.07); color: #697482; }
  .legacy-note p { font-size: 9.5px; line-height: 1.55; }
  .results-pane { display: flex; flex-direction: column; padding: 18px; border-right: 1px solid rgba(255,255,255,.07); overflow: hidden; }
  .pane-heading { flex: 0 0 auto; display: flex; align-items: flex-end; justify-content: space-between; padding: 0 2px 13px; }
  .pane-heading h2 { margin-top: 4px; font-size: 16px; letter-spacing: -.02em; }
  .pane-heading > span { color: #727d8b; font-size: 10px; }
  .result-list, .result-skeletons { min-height: 0; flex: 1; display: flex; flex-direction: column; gap: 7px; overflow-y: auto; padding-right: 3px; }
  .result-skeleton { height: 76px; display: grid; grid-template-columns: 46px 1fr; gap: 11px; padding: 8px; border: 1px solid rgba(255,255,255,.05); border-radius: 10px; }
  .result-skeleton > span, .result-skeleton i { display: block; border-radius: 6px; background: linear-gradient(90deg, rgba(255,255,255,.035), rgba(255,255,255,.075), rgba(255,255,255,.035)); background-size: 200% 100%; animation: shimmer 1.3s infinite; }
  .result-skeleton > span { height: 58px; }
  .result-skeleton div { display: flex; flex-direction: column; gap: 7px; padding-top: 3px; }
  .result-skeleton i:nth-child(1) { width: 54%; height: 10px; }
  .result-skeleton i:nth-child(2) { width: 35%; height: 8px; }
  .result-skeleton i:nth-child(3) { width: 82%; height: 18px; }
  .large-empty { min-height: 0; flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; color: #697482; }
  .large-empty > span, .detail-placeholder > span { width: 62px; height: 62px; display: grid; place-items: center; margin-bottom: 13px; border: 1px solid rgba(103,183,163,.18); border-radius: 17px; background: rgba(103,183,163,.055); color: var(--v2-accent); }
  .large-empty h3, .detail-placeholder h3 { color: #dce1e7; font-size: 15px; }
  .large-empty p, .detail-placeholder p { max-width: 330px; margin-top: 7px; font-size: 11px; line-height: 1.65; }
  .large-empty button { min-height: 36px; display: inline-flex; align-items: center; gap: 7px; margin-top: 16px; padding: 0 12px; border: 0; border-radius: 8px; background: var(--v2-accent); color: #07120f; font: inherit; font-size: 11px; font-weight: 750; cursor: pointer; }
  .large-empty.quiet > span { border-color: rgba(255,255,255,.08); background: rgba(255,255,255,.025); color: #687381; }
  .partial-warning { display: flex; align-items: center; gap: 8px; margin-top: 10px; padding: 9px 10px; border: 1px solid rgba(218,165,91,.18); border-radius: 8px; background: rgba(218,165,91,.055); color: #cba777; font-size: 9.5px; }
  .detail-pane { background: rgba(10,14,20,.42); overflow: hidden; }
  .detail-scroll { height: 100%; overflow-y: auto; padding: 20px; }
  .detail-hero { display: grid; grid-template-columns: 112px minmax(0,1fr); align-items: end; gap: 16px; }
  .detail-art { width: 112px; aspect-ratio: 3/4; display: grid; place-items: center; overflow: hidden; border: 1px solid rgba(255,255,255,.08); border-radius: 11px; background: #181f28; color: #65707e; box-shadow: 0 16px 36px rgba(0,0,0,.28); }
  .detail-art img { width: 100%; height: 100%; object-fit: cover; }
  .detail-title h2 { margin-top: 7px; color: #f0f3f6; font-size: 21px; line-height: 1.18; letter-spacing: -.035em; }
  .detail-title p { margin-top: 7px; color: #778291; font-size: 10.5px; line-height: 1.45; }
  .genre-row { display: flex; flex-wrap: wrap; gap: 5px; margin-top: 15px; }
  .genre-row span { padding: 3px 7px; border: 1px solid rgba(255,255,255,.07); border-radius: 5px; background: rgba(255,255,255,.025); color: #828d9b; font-size: 9px; }
  .synopsis { margin-top: 13px; color: #919aa7; font-size: 11px; line-height: 1.75; }
  .episode-heading { display: flex; align-items: center; justify-content: space-between; margin-top: 24px; padding-bottom: 9px; border-bottom: 1px solid rgba(255,255,255,.075); }
  .episode-heading h3 { color: #e6eaef; font-size: 13px; }
  .episode-heading span { color: #6f7a88; font-size: 9.5px; }
  .episode-list { display: flex; flex-direction: column; gap: 6px; margin-top: 10px; }
  .episode-list button { min-width: 0; display: grid; grid-template-columns: 33px minmax(0,1fr) auto; align-items: center; gap: 9px; min-height: 43px; padding: 6px 8px; border: 1px solid rgba(255,255,255,.065); border-radius: 8px; background: rgba(255,255,255,.018); color: inherit; text-align: left; cursor: pointer; }
  .episode-list button:hover { border-color: rgba(103,183,163,.28); background: rgba(103,183,163,.045); }
  .episode-number { width: 31px; height: 28px; display: grid; place-items: center; border-radius: 6px; background: rgba(255,255,255,.045); color: #8b95a2; font-family: var(--font-mono); font-size: 9px; }
  .episode-title { overflow: hidden; color: #cdd3db; font-size: 10.5px; text-overflow: ellipsis; white-space: nowrap; }
  .play-mark { color: var(--v2-accent); }
  .episode-loading, .episode-empty { display: flex; align-items: center; gap: 8px; margin-top: 12px; padding: 13px; border: 1px dashed rgba(255,255,255,.08); border-radius: 8px; color: #737e8c; font-size: 10.5px; }
  .detail-loading, .detail-placeholder { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; color: #6f7987; text-align: center; }
  .detail-loading p { margin-top: 10px; font-size: 11px; }
  .spinner, .large-spinner { display: inline-block; border: 2px solid rgba(255,255,255,.1); border-top-color: var(--v2-accent); border-radius: 50%; animation: spin .75s linear infinite; }
  .spinner { width: 14px; height: 14px; }
  .large-spinner { width: 28px; height: 28px; }
  .error-toast { position: absolute; right: 20px; bottom: 18px; z-index: 42; width: min(390px, calc(100% - 40px)); display: grid; grid-template-columns: auto 1fr auto; align-items: start; gap: 10px; padding: 12px; border: 1px solid rgba(239,121,131,.25); border-radius: 10px; background: rgba(35,17,21,.96); color: #ef969e; box-shadow: 0 16px 38px rgba(0,0,0,.35); }
  .error-toast strong { color: #f3c6ca; font-size: 11px; }
  .error-toast p { margin-top: 2px; color: #c9959a; font-size: 10px; line-height: 1.5; }
  .error-toast button { width: 27px; height: 27px; display: grid; place-items: center; border: 0; border-radius: 7px; background: rgba(255,255,255,.04); color: #c9959a; cursor: pointer; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes shimmer { to { background-position: -200% 0; } }
  @media (max-width: 1050px) {
    .workspace-body { grid-template-columns: 180px minmax(330px,1fr); }
    .detail-pane { position: absolute; top: 72px; right: 0; bottom: 0; width: min(430px, 48%); border-left: 1px solid rgba(255,255,255,.08); background: #0e131a; box-shadow: -20px 0 50px rgba(0,0,0,.28); }
    .detail-pane:has(.detail-placeholder) { display: none; }
  }
  @media (max-width: 760px) {
    .provider-search { grid-column: 1 / -1; grid-row: 2; }
    .workspace-body { grid-template-columns: 1fr; }
    .provider-rail { display: none; }
    .detail-pane { top: 122px; width: 100%; }
    .detail-pane:has(.detail-placeholder) { display: none; }
    .config-button { font-size: 0; width: 38px; padding: 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .provider-workspace, .provider-workspace * { animation: none !important; transition: none !important; }
  }
  :global([data-motion="reduce"]) .provider-workspace,
  :global([data-motion="reduce"]) .provider-workspace * { animation: none !important; transition: none !important; }
</style>
