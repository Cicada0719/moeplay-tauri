<script lang="ts">
  import { animeStore } from '../../stores/anime.svelte';
  import Icon from '../Icon.svelte';
  import { Button, Card, EmptyState } from '../ui';
  import { AsyncState, Drawer, MediaCard } from '../ui-v2';
  import { focusRovingItem, nextRovingIndex } from './a11y';

  let searchInput = $state(animeStore.searchKeyword || '');
  let activeSource = $state<string>('all');
  let showImageSearch = $state(false);
  let imageUrlInput = $state('');
  let sourceTabRefs: Array<HTMLButtonElement | null> = [];
  const sourceTabs = $derived([
    { id: 'all', label: '全部' },
    { id: 'image', label: '图片搜番' },
    ...animeStore.rules.map((rule) => ({ id: rule.name, label: rule.name })),
  ]);


  function selectSource(id: string, index: number) {
    activeSource = id;
    showImageSearch = id === 'image';
    focusRovingItem(sourceTabRefs, index);
  }

  function handleSourceTabKeydown(event: KeyboardEvent, index: number) {
    const next = nextRovingIndex(event.key, index, sourceTabs.length, 'horizontal');
    if (next === null) return;
    event.preventDefault();
    selectSource(sourceTabs[next].id, next);
  }

  function closeDrawer() {
    animeStore.drawerOpen = false;
  }

  function handleHistoryClick(keyword: string) {
    searchInput = keyword;
    handleSearch();
  }

  // 判断是否显示搜索历史（输入框为空且非图片搜索模式）
  const showHistory = $derived(
    activeSource !== 'image' && searchInput.trim() === '' && animeStore.searchHistory.length > 0
  );

  function handleSearch() {
    if (!searchInput.trim()) return;
    animeStore.addSearchHistory(searchInput.trim());
    animeStore.search(searchInput.trim());
  }
  
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') handleSearch();
    if (e.key === 'Escape') animeStore.drawerOpen = false;
  }
  
  function handleImageSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') handleImageSearch();
  }

  function handleImageSearch() {
    if (!imageUrlInput.trim()) return;
    animeStore.imageSearch(imageUrlInput.trim());
  }

  function formatTime(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${String(s).padStart(2, '0')}`;
  }

  function formatSimilarity(val: number): string {
    return `${(val * 100).toFixed(1)}%`;
  }

  function openResult(ruleName: string, item: any) {
    animeStore.drawerOpen = false;
    animeStore.openDetail(ruleName, item);
  }
</script>

<Drawer
  open={animeStore.drawerOpen}
  title="搜索番剧"
  description="按经典规则来源检索，也可以使用截图识别。"
  side="bottom"
  size="lg"
  onClose={closeDrawer}
  initialFocus={activeSource === 'image' ? '[data-image-search]' : '[data-anime-drawer-search]'}
  returnFocus
  class="anime-search-drawer"
>
  <div class="drawer-content">
    <!-- Source Tabs -->
    <div class="source-tabs" role="tablist" aria-label="搜索来源">
      {#each sourceTabs as tab, index (tab.id)}
        <button
          bind:this={sourceTabRefs[index]}
          class="source-tab"
          class:active={activeSource === tab.id}
          type="button"
          role="tab"
          aria-selected={activeSource === tab.id}
          tabindex={activeSource === tab.id ? 0 : -1}
          onclick={() => selectSource(tab.id, index)}
          onkeydown={(event) => handleSourceTabKeydown(event, index)}
        >
          {#if tab.id === 'image'}<Icon name="image" size={12} />{/if}
          {tab.label}
        </button>
      {/each}
    </div>

    {#if activeSource === 'image'}
      <!-- Image Search Panel -->
      <div class="image-search-panel">
        <div class="image-search-header">
          <Icon name="image" size={16} />
          <span>通过截图搜索番剧</span>
          <small>Powered by trace.moe</small>
        </div>
        <div class="search-row">
          <input
            class="drawer-input"
            type="url"
            placeholder="粘贴图片 URL..."
            bind:value={imageUrlInput}
            onkeydown={handleImageSearchKeydown}
            data-image-search
          />
          <Button variant="primary" press={handleImageSearch} disabled={animeStore.imageSearchLoading}>
            {animeStore.imageSearchLoading ? '搜索中...' : '搜索'}
          </Button>
        </div>

        {#if animeStore.imageSearchError}
          <EmptyState icon="x" title="识别失败" description={animeStore.imageSearchError} class="image-search-error" />
        {/if}

        {#if animeStore.imageSearchLoading}
          <div class="drawer-loading"><div class="spinner"></div> 正在识别中...</div>
        {:else if animeStore.imageSearchResults.length > 0}
          <div class="image-results">
            {#each animeStore.imageSearchResults as result, i (i)}
              <Card padding="sm" class="image-result-item">
                {#if result.image}
                  <div class="result-thumbnail">
                    <img src={result.image} alt="预览" loading="lazy" />
                  </div>
                {/if}
                <div class="result-info">
                  <div class="result-title">
                    {#if result.title_chinese}
                      {result.title_chinese}
                    {:else if result.title_native}
                      {result.title_native}
                    {:else}
                      Anilist #{result.anilist_id}
                    {/if}
                  </div>
                  <div class="result-meta">
                    <span class="similarity" class:high={result.similarity >= 0.9} class:mid={result.similarity >= 0.7 && result.similarity < 0.9}>
                      {formatSimilarity(result.similarity)}
                    </span>
                    {#if result.episode}
                      <span class="ep-tag">EP {result.episode}</span>
                    {/if}
                    <span class="time-tag">{formatTime(result.from)} - {formatTime(result.to)}</span>
                  </div>
                  <div class="result-filename">{result.filename}</div>
                </div>
                {#if result.video}
                  <Button variant="ghost" size="sm" press={() => window.open(result.video, '_blank')} ariaLabel="预览片段" class="preview-btn">
                    <Icon name="play" size={14} />
                  </Button>
                {/if}
              </Card>
            {/each}
          </div>
        {:else if !animeStore.imageSearchError && !animeStore.imageSearchLoading}
          <EmptyState icon="search" title="通过截图搜索番剧" description="输入图片URL，trace.moe 会通过AI识别截图对应的动漫作品" class="image-search-hint" />
        {/if}
      </div>
    {:else}
      <!-- Normal Search -->
      <div class="search-row">
        <input
          class="drawer-input"
          type="search"
          placeholder="搜索番剧..."
          bind:value={searchInput}
          onkeydown={handleKeydown}
          data-anime-drawer-search
        />
        <Button variant="primary" press={handleSearch}>搜索</Button>
      </div>

      <!-- Search History -->
      {#if showHistory}
        <div class="search-history">
          <div class="history-header">
            <span class="history-title">搜索历史</span>
            <Button variant="ghost" size="sm" press={() => animeStore.clearSearchHistory()} class="history-clear-btn">
              <Icon name="trash" size={12} /> 清空
            </Button>
          </div>
          <div class="history-list">
            {#each animeStore.searchHistory as keyword (keyword)}
              <div class="history-item">
                <Button variant="quiet" fullWidth press={() => handleHistoryClick(keyword)} class="history-btn">
                  <Icon name="clock" size={12} />
                  <span class="history-keyword">{keyword}</span>
                </Button>
                <Button variant="quiet" size="sm" press={() => animeStore.removeSearchHistory(keyword)} ariaLabel="删除" class="history-delete">
                  <Icon name="x" size={12} />
                </Button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
      
      <!-- Results -->
      <div class="drawer-results">
        <AsyncState
          state={animeStore.loading ? 'loading' : animeStore.error ? (animeStore.error.includes('未找到') ? 'no-results' : 'error') : 'ready'}
          details={animeStore.error || undefined}
          compact
        >
          {#each animeStore.searchResults.filter(([source]) => activeSource === 'all' || activeSource === source) as [source, items] (source)}
            <section class="result-group" aria-labelledby={`drawer-source-${source}`}>
              <h4 class="result-source" id={`drawer-source-${source}`}>{source}</h4>
              <div class="drawer-result-list" role="list">
                {#each items as item (item.url)}
                  <MediaCard
                    title={item.name}
                    subtitle={source}
                    variant="landscape"
                    ariaLabel={`查看 ${item.name} 详情`}
                    focusKey={`anime-drawer-${source}-${item.url}`}
                    onActivate={() => openResult(source, item)}
                  />
                {/each}
              </div>
            </section>
          {/each}
        </AsyncState>
      </div>
      
      <!-- Bottom Actions -->
      <div class="drawer-actions">
        <Button variant="quiet" size="sm" class="action-link" press={() => {
          if (animeStore.detailSubject) {
            const altName = animeStore.detailSubject.name || animeStore.detailSubject.name_cn;
            searchInput = altName;
            handleSearch();
          }
        }}>别名检索</Button>
        <Button variant="quiet" size="sm" class="action-link" press={() => {
          searchInput = '';
          (document.querySelector('.drawer-input') as HTMLElement)?.focus();
        }}>手动检索</Button>
      </div>
    {/if}
  </div>
</Drawer>

<style>
  :global(.v2-drawer.anime-search-drawer) { width: min(56rem, calc(100vw - 1rem)); max-height: min(78vh, 52rem); }
  :global(.v2-drawer.anime-search-drawer .v2-drawer__body) { padding: 0; }
  .drawer-content { min-height: 0; display: flex; flex-direction: column; }
  .drawer-result-list { display: grid; gap: .65rem; }

  .source-tabs {
    display: flex; gap: 6px; padding: 8px 16px;
    overflow-x: auto; scrollbar-width: none;
  }
  .source-tabs::-webkit-scrollbar { display: none; }
  .source-tab {
    flex-shrink: 0; padding: 4px 12px; border-radius: 6px;
    background: transparent; border: 1px solid #2d3748;
    color: #9ca3af; font-size: 12px; cursor: pointer;
    transition: all 0.15s; display: inline-flex; align-items: center; gap: 4px;
  }
  .source-tab.active { border-color: #E8557F; color: #E8557F; background: rgba(232,85,127,0.1); }
  .search-row {
    display: flex; gap: 8px; padding: 0 16px 8px;
  }
  .drawer-input {
    flex: 1;
  }
  .drawer-results {
    flex: 1; overflow-y: auto; padding: 0 16px;
    min-height: 100px;
  }
  .drawer-loading {
    display: flex; align-items: center; gap: 8px;
    color: #9ca3af; padding: 20px 0; justify-content: center;
  }
  .result-group { margin-bottom: 12px; }
  .result-source { font-size: 11px; color: #6b7280; margin: 0 0 4px; font-weight: 600; }
  :global(.ui-button.result-item) {
    display: flex; align-items: center; gap: 8px; width: 100%;
    padding: 8px 10px; border: none; background: transparent;
    color: #d1d5db; font-size: 13px; cursor: pointer;
    border-radius: 6px; transition: background 0.1s;
    text-align: left;
  }
  :global(.ui-button.result-item:hover) { background: rgba(255,255,255,0.05); color: #fff; }
  .drawer-actions {
    display: flex; justify-content: center; gap: 16px;
    padding: 12px 16px; border-top: 1px solid #2d3748;
  }
  :global(.ui-button.action-link) {
    background: none; border: none; color: #6b7280;
    font-size: 12px; cursor: pointer; text-decoration: underline;
  }
  :global(.ui-button.action-link:hover) { color: #E8557F; }
  .spinner {
    width: 18px; height: 18px; border: 2px solid #374151;
    border-top-color: #E8557F; border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes slide-up { from { transform: translate(-50%, 100%); } to { transform: translate(-50%, 0); } }
  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }

  /* Image Search */
  .image-search-panel {
    flex: 1; overflow-y: auto; padding: 0 16px;
    min-height: 100px;
  }
  .image-search-header {
    display: flex; align-items: center; gap: 6px;
    color: #d1d5db; font-size: 13px; padding: 8px 0;
  }
  .image-search-header small {
    color: #6b7280; font-size: 11px; margin-left: auto;
  }
  :global(.ui-empty.image-search-error) {
    display: flex; align-items: center; gap: 6px;
    color: #ef4444; font-size: 12px; padding: 8px 0;
  }
  :global(.ui-empty.image-search-hint) {
    display: flex; flex-direction: column; align-items: center;
    gap: 8px; padding: 32px 0; color: #6b7280;
  }
  :global(.ui-empty.image-search-hint p) { font-size: 12px; text-align: center; max-width: 300px; }
  .image-results { padding: 8px 0; }
  :global(.ui-card.image-result-item) {
    display: flex; align-items: center; gap: 10px;
    padding: 10px 8px; border-radius: 8px;
    transition: background 0.1s;
  }
  :global(.ui-card.image-result-item:hover) { background: rgba(255,255,255,0.04); }
  .result-thumbnail {
    width: 64px; height: 40px; border-radius: 4px;
    overflow: hidden; flex-shrink: 0; background: #111;
  }
  .result-thumbnail img {
    width: 100%; height: 100%; object-fit: cover;
  }
  .result-info { flex: 1; min-width: 0; }
  .result-title {
    font-size: 13px; font-weight: 600; color: #e5e7eb;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .result-meta {
    display: flex; align-items: center; gap: 8px;
    margin-top: 3px;
  }
  .similarity {
    font-size: 11px; font-weight: 700; font-family: monospace;
    padding: 1px 5px; border-radius: 3px;
    background: rgba(107,114,128,0.2); color: #9ca3af;
  }
  .similarity.high { background: rgba(34,197,94,0.15); color: #22c55e; }
  .similarity.mid { background: rgba(234,179,8,0.15); color: #eab308; }
  .ep-tag {
    font-size: 10px; color: #6b7280; font-family: monospace;
    padding: 1px 4px; border-radius: 3px; background: rgba(107,114,128,0.15);
  }
  .time-tag {
    font-size: 10px; color: #6b7280; font-family: monospace;
  }
  .result-filename {
    font-size: 10px; color: #4b5563; margin-top: 2px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  :global(.ui-button.preview-btn) {
    flex-shrink: 0; width: 32px; height: 32px;
    border-radius: 50%; border: 1px solid rgba(255,255,255,0.1);
    background: rgba(255,255,255,0.05); color: #9ca3af;
    display: flex; align-items: center; justify-content: center;
    cursor: pointer; transition: all 0.15s;
  }
  :global(.ui-button.preview-btn:hover) {
    border-color: var(--accent, #E8557F); color: var(--accent, #E8557F);
    background: rgba(232,85,127,0.1);
  }

  /* Search History */
  .search-history {
    padding: 0 16px 8px;
  }
  .history-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 4px 0 6px;
  }
  .history-title {
    font-size: 12px; font-weight: 600; color: #6b7280;
  }
  :global(.ui-button.history-clear-btn) {
    display: inline-flex; align-items: center; gap: 4px;
    background: none; border: none; color: #6b7280;
    font-size: 11px; cursor: pointer; padding: 2px 6px; border-radius: 4px;
    transition: all 0.15s;
  }
  :global(.ui-button.history-clear-btn:hover) { color: #ef4444; background: rgba(239,68,68,0.1); }
  .history-list {
    display: flex; flex-direction: column; gap: 2px;
  }
  .history-item {
    display: flex; align-items: center; gap: 4px;
  }
  :global(.ui-button.history-btn) {
    flex: 1; display: flex; align-items: center; gap: 8px;
    padding: 7px 10px; border: none; background: transparent;
    color: #d1d5db; font-size: 13px; cursor: pointer;
    border-radius: 6px; transition: background 0.1s;
    text-align: left;
  }
  :global(.ui-button.history-btn:hover) { background: rgba(255,255,255,0.05); }
  .history-keyword {
    flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  :global(.ui-button.history-delete) {
    flex-shrink: 0; width: 24px; height: 24px;
    border-radius: 4px; border: none;
    background: transparent; color: #4b5563;
    display: flex; align-items: center; justify-content: center;
    cursor: pointer; transition: all 0.15s;
    opacity: 0;
  }
  .history-item:hover :global(.ui-button.history-delete) { opacity: 1; }
  :global(.ui-button.history-delete:hover) { color: #ef4444; background: rgba(239,68,68,0.1); }

  @media (prefers-reduced-motion: reduce) {
    .drawer-content, .drawer-content * { animation: none !important; transition: none !important; }
  }
  :global([data-motion="reduce"]) .drawer-content,
  :global([data-motion="reduce"]) .drawer-content * { animation: none !important; transition: none !important; }
</style>
