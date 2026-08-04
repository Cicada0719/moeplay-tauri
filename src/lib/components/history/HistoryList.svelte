<script lang="ts">
  // 历史列表页组件（FR-10 / spec §4 步骤 4）
  //
  // 职责：Tab 类型筛选 / 关键词搜索（150ms 防抖 + <mark> 高亮，200ms 内完成）/
  //       最近更新排序（SQL 层已排好）/ 自实现轻量虚拟滚动（固定行高 72px）/
  //       单条删除 / 批量删除 / 按类型清空（全部走墓碑软删除，同步（任务 5）依据墓碑传播删除）。
  import { onDestroy, onMount, untrack } from 'svelte';
  import { clearHistoryByType, deleteHistory, queryHistory } from '../../history/historyApi';
  import type { ContentType, HistoryItem } from '../../history/types';
  import { uiStore } from '../../stores/ui.svelte';
  import Icon from '../Icon.svelte';
  import { Button } from '../ui';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import HistoryRow from './HistoryRow.svelte';

  const ROW_H = 72;
  const OVERSCAN = 5;
  const SEARCH_DEBOUNCE_MS = 150;

  type TabValue = ContentType | 'all';
  type ConfirmState =
    | { kind: 'single'; item: HistoryItem }
    | { kind: 'batch'; ids: string[] }
    | { kind: 'clear'; contentType: ContentType };

  let {
    initialTab = 'all' as TabValue,
    onresume,
    ondeleted,
  }: {
    initialTab?: TabValue;
    /** 点击条目，请求续读/续播（spec §3.3：on:resume 的 Svelte 5 回调等价） */
    onresume?: (event: { item: HistoryItem }) => void;
    /** 删除完成（供外层 Toast/统计） */
    ondeleted?: (event: { ids: string[] }) => void;
  } = $props();

  const TABS: Array<{ value: TabValue; label: string; icon: string }> = [
    { value: 'all', label: '全部', icon: 'list' },
    { value: 'anime', label: '番剧', icon: 'film' },
    { value: 'manga', label: '漫画', icon: 'book' },
    { value: 'novel', label: '小说', icon: 'collection' },
  ];

  let activeTab = $state<TabValue>(untrack(() => initialTab));
  let keyword = $state('');
  let debouncedKeyword = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  let items = $state<HistoryItem[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let error = $state<string | null>(null);

  let batchMode = $state(false);
  let selectedIds = $state<Set<string>>(new Set());
  let confirm = $state<ConfirmState | null>(null);

  let listEl = $state<HTMLElement | null>(null);
  let scrollTop = $state(0);
  let viewportH = $state(600);

  const trimmedKeyword = $derived(debouncedKeyword.trim().toLowerCase());
  const filtered = $derived(
    trimmedKeyword
      ? items.filter((item) => item.title.toLowerCase().includes(trimmedKeyword))
      : items,
  );

  const startIndex = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  const endIndex = $derived(
    Math.min(filtered.length, Math.ceil((scrollTop + viewportH) / ROW_H) + OVERSCAN),
  );
  const visibleRows = $derived(
    filtered.slice(startIndex, endIndex).map((item, index) => ({ item, index: startIndex + index })),
  );

  const hasNoRecords = $derived(!loading && !error && items.length === 0);
  const searchEmpty = $derived(!loading && !error && items.length > 0 && filtered.length === 0);

  async function load() {
    loading = true;
    error = null;
    try {
      const page = await queryHistory({ contentType: activeTab === 'all' ? undefined : activeTab });
      items = page.items;
      total = page.total;
    } catch (loadError) {
      error = loadError instanceof Error ? loadError.message : '历史记录读取失败';
      items = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void load();
  });

  $effect(() => {
    measure();
  });

  function measure() {
    if (listEl) viewportH = Math.max(240, listEl.clientHeight || 600);
  }

  function onScroll() {
    if (!listEl) return;
    scrollTop = listEl.scrollTop;
    measure();
  }

  function onSearchInput() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      debouncedKeyword = keyword;
      scrollTop = 0;
    }, SEARCH_DEBOUNCE_MS);
  }

  function selectTab(tab: TabValue) {
    if (tab === activeTab) return;
    activeTab = tab;
    selectedIds = new Set();
    debouncedKeyword = '';
    keyword = '';
    scrollTop = 0;
  }

  function enterBatchMode() {
    batchMode = true;
    selectedIds = new Set();
  }

  function exitBatchMode() {
    batchMode = false;
    selectedIds = new Set();
  }

  function toggleSelect(id: string) {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedIds = next;
  }

  function toggleSelectAll() {
    if (filtered.length > 0 && selectedIds.size === filtered.length) selectedIds = new Set();
    else selectedIds = new Set(filtered.map((item) => item.id));
  }

  function handleActivate(item: HistoryItem) {
    if (batchMode) {
      toggleSelect(item.id);
      return;
    }
    onresume?.({ item });
  }

  function requestSingleDelete(item: HistoryItem) {
    confirm = { kind: 'single', item };
  }

  function requestBatchDelete() {
    if (selectedIds.size === 0) return;
    confirm = { kind: 'batch', ids: [...selectedIds] };
  }

  function requestClearType() {
    if (activeTab === 'all') return;
    confirm = { kind: 'clear', contentType: activeTab };
  }

  async function runDelete(ids: string[]) {
    try {
      await deleteHistory(ids);
      const removed = new Set(ids);
      items = items.filter((item) => !removed.has(item.id));
      total = Math.max(0, total - ids.length);
      selectedIds = new Set();
      uiStore.toast(ids.length === 1 ? '已删除' : `已删除 ${ids.length} 条`, 'success');
      ondeleted?.({ ids });
    } catch (deleteError) {
      uiStore.toast(deleteError instanceof Error ? deleteError.message : '删除失败', 'error');
    }
  }

  async function runClearType(contentType: ContentType) {
    const removed = items.filter((item) => item.contentType === contentType);
    try {
      await clearHistoryByType(contentType);
      items = items.filter((item) => item.contentType !== contentType);
      total = Math.max(0, total - removed.length);
      selectedIds = new Set();
      uiStore.toast('已清空该类型', 'success');
      ondeleted?.({ ids: removed.map((item) => item.id) });
    } catch (clearError) {
      uiStore.toast(clearError instanceof Error ? clearError.message : '清空失败', 'error');
    }
  }

  async function handleConfirm() {
    const current = confirm;
    confirm = null;
    if (!current) return;
    if (current.kind === 'single') await runDelete([current.item.id]);
    else if (current.kind === 'batch') await runDelete(current.ids);
    else await runClearType(current.contentType);
  }

  function confirmMessage(): { title: string; message: string; label: string } {
    if (confirm?.kind === 'single') {
      return {
        title: '删除这条记录',
        message: `「${confirm.item.title}」将被软删除。同步后其他设备也会删除（任务 5 依据墓碑传播）。`,
        label: '删除',
      };
    }
    if (confirm?.kind === 'batch') {
      return {
        title: '批量删除',
        message: `将删除 ${confirm.ids.length} 条记录，同步后其他设备也会删除。`,
        label: '删除所选',
      };
    }
    if (confirm?.kind === 'clear') {
      const label = confirm.contentType === 'anime' ? '番剧' : confirm.contentType === 'manga' ? '漫画' : '小说';
      return {
        title: '清空该类型',
        message: `将清空所有「${label}」历史记录，同步后其他设备也会删除。`,
        label: '清空',
      };
    }
    return { title: '', message: '', label: '确定' };
  }

  function onRowActivate(item: HistoryItem) {
    handleActivate(item);
  }

  onMount(() => {
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(() => measure());
    if (listEl) observer.observe(listEl);
    return () => observer.disconnect();
  });

  onDestroy(() => {
    if (searchTimer) clearTimeout(searchTimer);
  });
</script>

<section class="history-list" data-testid="history-list" aria-label="历史记录">
  <header class="history-header">
    <div class="tabs" role="tablist" aria-label="历史类型筛选">
      {#each TABS as tab (tab.value)}
        <button
          type="button"
          role="tab"
          class="tab"
          class:active={activeTab === tab.value}
          aria-selected={activeTab === tab.value}
          onclick={() => selectTab(tab.value)}
          data-testid={`tab-${tab.value}`}
        >
          <Icon name={tab.icon} size={13} />
          {tab.label}
        </button>
      {/each}
    </div>

    <label class="search-box" data-testid="search-box">
      <Icon name="search" size={15} />
      <input
        type="search"
        value={keyword}
        oninput={(event) => {
          keyword = (event.currentTarget as HTMLInputElement).value;
          onSearchInput();
        }}
        placeholder="搜索标题…"
        aria-label="搜索历史标题"
        data-testid="search-input"
      />
      {#if keyword}
        <button
          type="button"
          class="search-clear"
          aria-label="清空搜索"
          onclick={() => {
            keyword = '';
            debouncedKeyword = '';
            if (searchTimer) clearTimeout(searchTimer);
          }}
        >
          <Icon name="x" size={13} />
        </button>
      {/if}
    </label>

    {#if batchMode}
      <Button variant="ghost" size="sm" press={exitBatchMode} ariaLabel="退出批量管理模式">
        退出批量
      </Button>
    {:else}
      <Button variant="ghost" size="sm" press={enterBatchMode} ariaLabel="进入批量管理模式">
        <Icon name="check" size={14} />批量管理
      </Button>
    {/if}
  </header>

  {#if loading && items.length === 0}
    <div class="empty-state" data-testid="history-loading">
      <span class="empty-icon"><Icon name="clock" size={26} /></span>
      <strong>正在加载历史记录…</strong>
    </div>
  {:else if error && items.length === 0}
    <div class="empty-state" role="alert" data-testid="history-error">
      <span class="empty-icon"><Icon name="info" size={26} /></span>
      <strong>加载失败</strong>
      <p>{error}</p>
      <Button variant="secondary" size="sm" press={() => void load()}>重试</Button>
    </div>
  {:else if hasNoRecords}
    <div class="empty-state" data-testid="history-empty">
      <span class="empty-icon"><Icon name="list" size={26} /></span>
      <strong>暂无记录</strong>
      <p>阅读漫画或小说后，进度会自动出现在这里。</p>
    </div>
  {:else if searchEmpty}
    <div class="empty-state" data-testid="history-search-empty">
      <span class="empty-icon"><Icon name="search" size={26} /></span>
      <strong>未找到匹配记录</strong>
      <p>换个关键词试试。</p>
    </div>
  {:else}
    <div
      bind:this={listEl}
      class="virtual-list"
      role="list"
      aria-label="历史记录列表"
      onscroll={onScroll}
      data-testid="virtual-list"
    >
      <div
        class="virtual-spacer"
        style={`padding-top:${startIndex * ROW_H}px;padding-bottom:${Math.max(0, filtered.length - endIndex) * ROW_H}px;`}
      >
        {#each visibleRows as row (row.item.id)}
          <HistoryRow
            item={row.item}
            keyword={debouncedKeyword}
            batchMode={batchMode}
            selected={selectedIds.has(row.item.id)}
            onActivate={onRowActivate}
            onDelete={requestSingleDelete}
            onToggleSelect={toggleSelect}
          />
        {/each}
      </div>
    </div>
  {/if}

  <footer class="history-footer">
    <span class="count" data-testid="history-count" aria-live="polite">
      {trimmedKeyword ? `匹配 ${filtered.length}` : `共 ${total}`} 条{activeTab !== 'all' ? ` · ${activeTab}` : ''}
    </span>

    {#if batchMode}
      <div class="batch-actions">
        <Button variant="ghost" size="sm" press={toggleSelectAll} disabled={filtered.length === 0}>
          全选
        </Button>
        <Button
          variant="secondary"
          size="sm"
          press={requestBatchDelete}
          disabled={selectedIds.size === 0}
        >
          删除所选{selectedIds.size > 0 ? ` (${selectedIds.size})` : ''}
        </Button>
      </div>
    {/if}

    <Button
      variant="ghost"
      size="sm"
      press={requestClearType}
      disabled={activeTab === 'all'}
      title={activeTab === 'all' ? '全部 Tab 下不可按类型清空' : '清空当前类型'}
    >
      清空当前类型
    </Button>
  </footer>

  <ConfirmDialog
    open={confirm !== null}
    title={confirmMessage().title}
    message={confirmMessage().message}
    confirmLabel={confirmMessage().label}
    onconfirm={() => void handleConfirm()}
    oncancel={() => (confirm = null)}
  />
</section>

<style>
  .history-list {
    display: grid;
    min-width: 0;
    min-height: 0;
    height: 100%;
    grid-template-rows: auto minmax(0, 1fr) auto;
    color: var(--v2-color-text, var(--text-primary));
  }

  .history-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--v2-space-2, 0.5rem);
    padding: var(--v2-space-3, 0.75rem);
    border-bottom: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.08));
  }

  .tabs { display: flex; flex-wrap: wrap; gap: 0.25rem; }
  .tab {
    display: inline-flex;
    min-height: 2.25rem;
    align-items: center;
    gap: 0.35rem;
    padding: 0 0.7rem;
    border: 1px solid transparent;
    border-radius: var(--v2-radius-md, 0.5rem);
    background: transparent;
    color: var(--v2-color-text-secondary, var(--text-muted));
    font: inherit;
    font-size: var(--v2-text-sm, 0.875rem);
    cursor: pointer;
  }
  .tab.active { border-color: var(--v2-color-accent, var(--accent)); background: color-mix(in srgb, var(--v2-color-accent, var(--accent)) 12%, transparent); color: var(--v2-color-text, var(--text-primary)); }
  .tab:focus-visible { outline: none; box-shadow: var(--v2-focus-ring, 0 0 0 2px var(--accent)); }

  .search-box {
    display: flex;
    min-width: 10rem;
    min-height: 2.4rem;
    flex: 1 1 14rem;
    align-items: center;
    gap: 0.4rem;
    padding: 0 0.6rem;
    border: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.12));
    border-radius: var(--v2-radius-md, 0.5rem);
    background: var(--v2-color-surface, rgba(255, 255, 255, 0.03));
    color: var(--v2-color-text-secondary, var(--text-muted));
  }
  .search-box:focus-within { border-color: var(--v2-color-accent, var(--accent)); box-shadow: var(--v2-focus-ring, 0 0 0 2px var(--accent)); }
  .search-box input { min-width: 0; flex: 1 1 auto; border: 0; outline: 0; background: transparent; color: var(--v2-color-text, var(--text-primary)); font: inherit; }
  .search-clear { display: grid; width: 1.8rem; height: 1.8rem; place-items: center; border: 0; border-radius: 50%; background: transparent; color: var(--v2-color-text-secondary, var(--text-muted)); cursor: pointer; }
  .search-clear:focus-visible { outline: none; box-shadow: var(--v2-focus-ring, 0 0 0 2px var(--accent)); }

  .virtual-list {
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }
  .virtual-spacer { min-width: 0; }

  .history-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--v2-space-2, 0.5rem);
    padding: var(--v2-space-3, 0.75rem);
    border-top: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.08));
  }
  .count { color: var(--v2-color-text-secondary, var(--text-muted)); font-size: var(--v2-text-xs, 0.75rem); }
  .batch-actions { display: flex; gap: 0.4rem; }

  .empty-state {
    display: grid;
    min-height: 16rem;
    place-content: center;
    justify-items: center;
    gap: 0.5rem;
    padding: 2rem;
    color: var(--v2-color-text-secondary, var(--text-muted));
    text-align: center;
  }
  .empty-state strong { color: var(--v2-color-text, var(--text-primary)); }
  .empty-state p { margin: 0; font-size: var(--v2-text-sm, 0.875rem); line-height: 1.6; }
  .empty-icon { display: grid; width: 3.5rem; height: 3.5rem; place-items: center; border-radius: 1rem; background: color-mix(in srgb, var(--v2-color-accent, var(--accent)) 12%, transparent); color: var(--v2-color-accent, var(--accent)); }
</style>
