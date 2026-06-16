<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { animeStore } from '../../stores/anime.svelte';
  import type { SearchItem, Road } from '../../stores/anime.svelte';
  import Icon from '../Icon.svelte';

  const rules = $derived(animeStore.rules);
  const detailName = $derived(animeStore.detailName);
  // isOpen 用 nonce 门控：只有本次「打开会话」的瞬态已被 effect 同步重置过
  // (readyNonce 追上当前 nonce) 才渲染面板 —— 否则会闪现上一部番的旧步骤/旧分集，
  // 用户开 B 番却看到 A 番的分集、点下去就串台，这正是「播放源和开始观看冲突」。
  // 再加 view!=='player' 兜底：面板绝不盖在播放器上。
  let readyNonce = $state(-1);
  const isOpen = $derived(
    animeStore.sourceSheetOpen &&
    animeStore.view !== 'player' &&
    animeStore.sourceSheetNonce === readyNonce
  );

  type SourceStatus = 'pending' | 'success' | 'error' | 'noResult';

  // ── 第一步：搜索状态 (组件内部管理, 不污染 store) ──
  let searchResults = $state<Map<string, { items: SearchItem[]; status: SourceStatus }>>(new Map());
  let activeSourceIdx = $state(0);
  let searchToken = 0;

  // ── 第二步：选集状态 ──
  // 点了某个搜索结果后，拉取线路 → 展示「线路 + 分集」让用户挑，而不是盲目播第 1 集。
  let step = $state<'search' | 'episodes'>('search');
  let episodeRoads = $state<Road[]>([]);
  let episodeRuleName = $state('');
  let episodeSourceUrl = $state('');
  let episodeItemName = $state('');
  let activeRoadIdx = $state(0);
  let loadingRoads = $state(false);
  let loadError = $state<string | null>(null);

  // 每次「打开」面板用单调递增 nonce 触发一次搜索。nonce 每次必变 →
  // 永不会像旧的 prevOpen 布尔那样在反复进出后错位、卡死「开始观看没反应」。
  $effect(() => {
    const nonce = animeStore.sourceSheetNonce;
    const open = animeStore.sourceSheetOpen;
    if (open && nonce !== readyNonce && animeStore.detailName) {
      // 同步重置本次会话的所有瞬态，再开搜索。readyNonce 追上 nonce 后 isOpen 才放行渲染，
      // 因此面板出现时一定是干净的搜索步，绝不闪现/串到上一部番的旧分集。
      step = 'search';
      episodeRoads = [];
      episodeItemName = '';
      activeRoadIdx = 0;
      loadError = null;
      readyNonce = nonce;
      startSearch();
    }
  });

  function startSearch() {
    if (!detailName || rules.length === 0) return;
    const token = ++searchToken;
    loadError = null;
    activeSourceIdx = 0;
    // 初始全部 pending —— 从 rules 构建新 Map，**绝不读旧 searchResults**。
    // 否则 $effect 会把 searchResults 纳入依赖、而 startSearch 又写它 → effect 自触发死循环，
    // token 不停自增使所有 .then 回调过期 → 结果永远应用不上（一直「搜索中」、点别的源也没反应）。
    const init = new Map<string, { items: SearchItem[]; status: SourceStatus }>();
    for (const rule of rules) init.set(rule.name, { items: [], status: 'pending' });
    searchResults = init;

    // 所有源并发搜索；HTML 解析在 Rust 阻塞线程池里跑，各源独立返回、互不拖累
    for (const rule of rules) {
      const ruleName = rule.name;
      invoke<SearchItem[]>('anime_search', { ruleName, keyword: detailName })
        .then(items => {
          if (token !== searchToken) return;
          const ok = items.length > 0;
          const next = new Map(searchResults);
          next.set(ruleName, { items, status: ok ? 'success' : 'noResult' });
          searchResults = next;
          // 自动跳到第一个有结果的源
          if (ok) {
            const cur = rules[activeSourceIdx]?.name;
            const curOk = cur && next.get(cur)?.status === 'success';
            if (!curOk) activeSourceIdx = rules.findIndex(r => r.name === ruleName);
          }
        })
        .catch(() => {
          if (token !== searchToken) return;
          const next = new Map(searchResults);
          next.set(ruleName, { items: [], status: 'error' });
          searchResults = next;
        });
    }
  }

  // 单源重试：只重新搜索指定的源，不动其它源
  function retrySource(ruleName: string) {
    if (!detailName) return;
    const token = searchToken;
    const next = new Map(searchResults);
    next.set(ruleName, { items: [], status: 'pending' });
    searchResults = next;
    invoke<SearchItem[]>('anime_search', { ruleName, keyword: detailName })
      .then(items => {
        if (token !== searchToken) return;
        const ok = items.length > 0;
        const n = new Map(searchResults);
        n.set(ruleName, { items, status: ok ? 'success' : 'noResult' });
        searchResults = n;
      })
      .catch(() => {
        if (token !== searchToken) return;
        const n = new Map(searchResults);
        n.set(ruleName, { items: [], status: 'error' });
        searchResults = n;
      });
  }

  // 点击搜索结果 → 拉取线路 → 进入「选集」步（不再盲目自动播第 1 集）
  async function onSelectResult(ruleName: string, item: SearchItem) {
    loadingRoads = true;
    loadError = null;
    episodeRuleName = ruleName;
    episodeSourceUrl = item.url;
    episodeItemName = item.name;
    try {
      const roads = await invoke<Road[]>('anime_fetch_roads', {
        ruleName,
        pageUrl: item.url,
      });
      if (roads.length > 0) {
        episodeRoads = roads;
        activeRoadIdx = 0;
        step = 'episodes';
      } else {
        loadError = '未能解析到播放线路';
      }
    } catch (e) {
      loadError = String(e);
    } finally {
      loadingRoads = false;
    }
  }

  // 在选集步点某一集 → 设置线路数据、关面板、播放该集
  function playEpisodeFromSheet(epIdx: number) {
    animeStore.setRoadsForPlayback(episodeRoads, episodeRuleName, episodeSourceUrl);
    animeStore.sourceSheetOpen = false;
    animeStore.playEpisode(activeRoadIdx, epIdx);
  }

  function backToSearch() {
    step = 'search';
    loadError = null;
  }

  function closeSheet() {
    animeStore.sourceSheetOpen = false;
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'success': return '#4ade80';
      case 'noResult': return '#fb923c';
      case 'error': return '#f87171';
      default: return '#6b7280';
    }
  }

  // 当前源的搜索结果
  const currentSource = $derived(rules[activeSourceIdx]);
  const currentResult = $derived(currentSource ? searchResults.get(currentSource.name) : undefined);
  // 当前线路的分集
  const currentEpisodes = $derived(episodeRoads[activeRoadIdx]?.episodes ?? []);
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="source-backdrop" onclick={closeSheet}></div>
  <div class="source-sheet" role="dialog">
    <!-- Drag handle -->
    <div class="source-handle"></div>

    <!-- Header -->
    <div class="source-header">
      {#if step === 'episodes'}
        <button class="source-back" onclick={backToSearch} title="返回选源">
          <Icon name="chevronLeft" size={18} />
        </button>
        <span class="source-title source-title-ellipsis">{episodeItemName || '选择剧集'}</span>
      {:else}
        <span class="source-title">选择播放源</span>
      {/if}
      <button class="source-close" onclick={closeSheet}>
        <Icon name="x" size={16} />
      </button>
    </div>

    {#if step === 'episodes'}
      <!-- ── 第二步：线路 + 分集选择 ── -->
      <div class="episode-view">
        {#if episodeRoads.length > 1}
          <div class="road-tabs">
            {#each episodeRoads as road, i}
              <button
                class="road-tab"
                class:active={activeRoadIdx === i}
                onclick={() => { activeRoadIdx = i; }}
              >
                {road.name || `线路${i + 1}`}
              </button>
            {/each}
          </div>
        {/if}

        <div class="episode-scroll">
          {#if currentEpisodes.length === 0}
            <div class="source-empty">
              <Icon name="film" size={24} />
              <p>该线路暂无剧集</p>
            </div>
          {:else}
            <div class="episode-grid">
              {#each currentEpisodes as ep, i (ep.url + i)}
                <button class="episode-btn" onclick={() => playEpisodeFromSheet(i)} title={ep.name}>
                  {ep.name || `第${i + 1}集`}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <!-- ── 第一步：选源 ── -->
      <div class="source-body">
        <div class="source-tabs">
          {#each rules as rule, i}
            {@const result = searchResults.get(rule.name)}
            {@const status = result?.status ?? 'pending'}
            <button
              class="source-tab"
              class:active={activeSourceIdx === i}
              onclick={() => { activeSourceIdx = i; }}
            >
              <span class="source-dot" style="background: {getStatusColor(status)}"></span>
              <span class="source-name">{rule.name}</span>
              {#if result?.items.length}
                <span class="source-count">{result.items.length}</span>
              {/if}
            </button>
          {/each}
        </div>

        <!-- Result list -->
        <div class="source-results">
          {#if loadingRoads}
            <div class="source-loading">
              <div class="spinner"></div>
              <span>获取线路中...</span>
            </div>
          {:else if !currentResult || currentResult.status === 'pending'}
            <div class="source-loading">
              <div class="spinner"></div>
              <span>搜索中...</span>
            </div>
          {:else if currentResult.status === 'error'}
            <div class="source-empty">
              <Icon name="x" size={24} />
              <p>检索失败</p>
              <button class="retry-btn" onclick={() => currentSource && retrySource(currentSource.name)}>重试</button>
            </div>
          {:else if currentResult.status === 'noResult' || currentResult.items.length === 0}
            <div class="source-empty">
              <Icon name="search" size={24} />
              <p>该源未找到「{detailName}」</p>
              <div class="source-actions">
                <button class="action-link" onclick={() => currentSource && retrySource(currentSource.name)}>重试该源</button>
              </div>
            </div>
          {:else}
            <div class="result-list">
              {#each currentResult.items as item (item.url)}
                <button class="result-item" onclick={() => onSelectResult(currentSource.name, item)}>
                  <Icon name="film" size={16} />
                  <span class="result-name">{item.name}</span>
                  <Icon name="chevronRight" size={14} />
                </button>
              {/each}
            </div>
          {/if}

          {#if loadError}
            <div class="load-error">{loadError}</div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .source-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 40;
    animation: fade-in 0.2s ease;
  }

  .source-sheet {
    position: fixed;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    width: min(640px, 90vw);
    height: min(600px, 80vh);
    background: #161b22;
    border-radius: 16px 16px 0 0;
    z-index: 41;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: slide-up 0.25s ease;
    box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.5);
  }

  .source-handle {
    width: 36px;
    height: 4px;
    background: rgba(255, 255, 255, 0.15);
    border-radius: 2px;
    margin: 8px auto 0;
    flex-shrink: 0;
  }

  .source-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    flex-shrink: 0;
  }
  .source-title {
    font-size: 15px;
    font-weight: 650;
    color: #fff;
    flex: 1;
  }
  .source-title-ellipsis {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .source-back {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.7);
    cursor: pointer;
    transition: all 0.15s;
    flex-shrink: 0;
  }
  .source-back:hover { background: rgba(232, 85, 127, 0.15); color: #e8557f; }
  .source-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    transition: all 0.15s;
    flex-shrink: 0;
  }
  .source-close:hover { background: rgba(255, 255, 255, 0.1); color: #fff; }

  .source-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  /* ── Source tabs (vertical) ── */
  .source-tabs {
    width: 160px;
    flex-shrink: 0;
    overflow-y: auto;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    padding: 8px 0;
  }
  .source-tab {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 10px 16px;
    border: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.6);
    font-size: 13px;
    cursor: pointer;
    transition: all 0.12s;
    text-align: left;
  }
  .source-tab:hover { background: rgba(255, 255, 255, 0.04); color: #fff; }
  .source-tab.active {
    background: rgba(232, 85, 127, 0.08);
    color: #e8557f;
    border-right: 2px solid #e8557f;
  }
  .source-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .source-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .source-count {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.3);
    font-family: var(--font-mono);
  }

  /* ── Results ── */
  .source-results {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
    min-width: 0;
  }
  .source-loading, .source-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 40px 20px;
    color: rgba(255, 255, 255, 0.35);
    font-size: 13px;
  }
  .source-empty p { margin: 0; }
  .source-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .action-link {
    padding: 6px 14px;
    border: 1px solid rgba(232, 85, 127, 0.3);
    border-radius: 6px;
    background: transparent;
    color: #e8557f;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .action-link:hover { background: rgba(232, 85, 127, 0.1); }

  .result-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px;
  }
  .result-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: rgba(255, 255, 255, 0.8);
    font-size: 13.5px;
    cursor: pointer;
    transition: all 0.12s;
    text-align: left;
    width: 100%;
  }
  .result-item:hover {
    background: rgba(232, 85, 127, 0.08);
    color: #fff;
  }
  .result-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .load-error {
    text-align: center;
    padding: 12px;
    color: #f87171;
    font-size: 12px;
  }
  .retry-btn {
    padding: 6px 16px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    background: transparent;
    color: #fff;
    font-size: 12px;
    cursor: pointer;
  }
  .retry-btn:hover { background: rgba(255, 255, 255, 0.06); }

  /* ── 第二步：选集 ── */
  .episode-view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .road-tabs {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    padding: 12px 16px 4px;
    flex-shrink: 0;
  }
  .road-tab {
    padding: 5px 14px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    background: transparent;
    color: rgba(255, 255, 255, 0.6);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }
  .road-tab:hover { border-color: rgba(232, 85, 127, 0.4); color: #fff; }
  .road-tab.active {
    background: rgba(232, 85, 127, 0.15);
    border-color: #e8557f;
    color: #e8557f;
  }
  .episode-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px 16px 16px;
  }
  .episode-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(92px, 1fr));
    gap: 8px;
  }
  .episode-btn {
    padding: 11px 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.75);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .episode-btn:hover {
    border-color: rgba(232, 85, 127, 0.5);
    background: rgba(232, 85, 127, 0.1);
    color: #fff;
    transform: translateY(-1px);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid rgba(255, 255, 255, 0.1);
    border-top-color: #e8557f;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes slide-up { from { transform: translate(-50%, 100%); } to { transform: translate(-50%, 0); } }
  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
</style>
