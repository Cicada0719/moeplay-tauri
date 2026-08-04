<script lang="ts">
  // 漫画阅读器（FR-11 / spec §4 步骤 5）
  //
  // 自包含渲染组件：接收单页序图片 URL 列表，实现单页 / 双页（左右分屏）阅读、
  // LTR/RTL 翻页方向、跨页大图独占一屏、<800px 窄窗口降级提示、模式按漫画持久化、
  // ±2 屏预取 + 200MB LRU、位置 debounce 上报 + 退出 flush。
  //
  // **页索引始终以单页原子单位存储**（`currentPage` / `pageIndex` 都是单页序号，
  // 与渲染模式无关）；双页仅为视图层配对，由 `buildScreens` 计算。
  import { onDestroy, untrack } from 'svelte';
  import { buildHistoryId, upsertHistory } from '../../history/historyApi';
  import type { HistoryItem } from '../../history/types';
  import { probeImageSize } from '../../reader/imageMeta';
  import {
    buildScreens,
    intentFromInput,
    nextScreen,
    screenIndexOfPage,
    type PageMeta,
  } from '../../reader/dualPage';
  import {
    createReaderSettingsStore,
    type PageMode,
    type ReadingDirection,
  } from '../../stores/readerSettings';
  import Icon from '../Icon.svelte';

  let {
    contentId,
    sourceId,
    chapterId,
    chapterTitle,
    pages,
    initialPageIndex = 0,
    cover = null,
    onclose,
  }: {
    contentId: string;
    sourceId: string;
    chapterId: string;
    chapterTitle: string;
    pages: string[];
    initialPageIndex?: number;
    cover?: string | null;
    onclose?: () => void;
  } = $props();

  const settingsStore = untrack(() => createReaderSettingsStore(contentId));
  const pageMode = $derived($settingsStore.pageMode);
  const direction = $derived($settingsStore.direction);
  const forceNarrowDual = $derived($settingsStore.forceNarrowDual);

  let pageMetas = $state<PageMeta[]>([]);
  let currentPage = $state(untrack(() => Math.max(0, Math.trunc(initialPageIndex) || 0)));
  let toolbarVisible = $state(true);
  let narrowConfirm = $state(false);
  let reportTimer: ReturnType<typeof setTimeout> | null = null;

  // 屏幕序列：双页模式由 `buildScreens` 配对（跨页大图独占、其余两两配对）；
  // 单页模式退化为每页一屏（spec §4 步骤 5.1：可复用同一代码路径）。
  // 无论哪种模式，屏内 `pageIndexes` 存的都是**单页原子序号**，与渲染模式无关（R8）。
  const screens = $derived(
    pageMode === 'dual'
      ? buildScreens(pageMetas)
      : pageMetas.map((meta) => ({ pageIndexes: [meta.index], anchorIndex: meta.index })),
  );
  const screenIndex = $derived.by(() => {
    if (screens.length === 0) return 0;
    try {
      return screenIndexOfPage(screens, currentPage);
    } catch {
      return 0;
    }
  });
  const currentScreen = $derived(screens[Math.min(screenIndex, screens.length - 1)] ?? null);

  const pageIndicator = $derived.by(() => {
    if (pages.length === 0) return '0 / 0';
    const screen = currentScreen;
    if (!screen) return `${Math.min(currentPage + 1, pages.length)} / ${pages.length}`;
    const first = screen.pageIndexes[0] + 1;
    const last = screen.pageIndexes[screen.pageIndexes.length - 1] + 1;
    return first === last ? `${first} / ${pages.length}` : `${first}-${last} / ${pages.length}`;
  });

  const currentScreenIsSpread = $derived(
    pageMode === 'dual' && (currentScreen?.pageIndexes.length ?? 0) === 1,
  );

  // ---- 状态装配：pages 变化 → 重建 PageMeta，并异步探测尺寸后重建 screens ----
  $effect(() => {
    const urls = pages;
    pageMetas = urls.map((_, index) => ({ index }));
    let cancelled = false;
    void Promise.allSettled(urls.map((url) => probeImageSize(url))).then((results) => {
      if (cancelled) return;
      const next = urls.map((_, index) => {
        const result = results[index];
        if (result.status === 'fulfilled' && result.value.width > 0 && result.value.height > 0) {
          return { index, width: result.value.width, height: result.value.height };
        }
        return { index };
      });
      pageMetas = next;
    });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    if (pages.length === 0) currentPage = 0;
    else currentPage = Math.min(currentPage, pages.length - 1);
  });

  $effect(() => {
    if (pages.length === 0) return;
    prefetchNearbyScreens();
  });

  // ---- 翻页 ----
  function go(intent: 'forward' | 'backward') {
    if (screens.length === 0) return;
    const next = nextScreen(screens, screenIndex, intent);
    currentPage = screens[next].anchorIndex;
    scheduleReport();
  }

  function onTapLeft() {
    go(intentFromInput('left', direction));
  }

  function onTapRight() {
    go(intentFromInput('right', direction));
  }

  function toggleToolbar() {
    toolbarVisible = !toolbarVisible;
  }

  // ---- 工具栏：模式 / 方向 ----
  function setPageMode(mode: PageMode) {
    $settingsStore = { ...$settingsStore, pageMode: mode };
  }

  function togglePageMode() {
    const next: PageMode = pageMode === 'dual' ? 'single' : 'dual';
    if (next === 'dual' && typeof window !== 'undefined' && window.innerWidth < 800 && !forceNarrowDual) {
      narrowConfirm = true;
      return;
    }
    setPageMode(next);
  }

  function toggleDirection() {
    const next: ReadingDirection = direction === 'rtl' ? 'ltr' : 'rtl';
    $settingsStore = { ...$settingsStore, direction: next };
  }

  function forceNarrowDualMode() {
    $settingsStore = { ...$settingsStore, pageMode: 'dual', forceNarrowDual: true };
    narrowConfirm = false;
  }

  // ---- 键盘：←/→ 映射物理方向，空格 = forward ----
  function handleKey(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      go(intentFromInput('left', direction));
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      go(intentFromInput('right', direction));
    } else if (event.key === ' ') {
      event.preventDefault();
      go('forward');
    }
  }

  // ---- 位置上报（精确到单页；debounce 2s + destroy flush）----
  function reportAnchor() {
    if (pages.length === 0) return;
    const anchor = currentScreen?.anchorIndex ?? currentPage;
    const item: HistoryItem = {
      id: buildHistoryId(contentId, sourceId, chapterId),
      contentId,
      contentType: 'manga',
      title: chapterTitle || `第 ${chapterId ?? ''} 话`,
      cover,
      sourceId,
      chapterId,
      chapterTitle,
      pageIndex: anchor,
      positionSec: 0,
      scrollPct: 0,
      progress: anchor,
      updatedAt: Date.now(),
      deviceId: '',
      deleted: false,
    };
    // history_upsert 尚未在 Rust 侧注册：失败静默降级，不打断阅读。
    void upsertHistory(item).catch(() => {});
  }

  function scheduleReport() {
    if (reportTimer) clearTimeout(reportTimer);
    reportTimer = setTimeout(() => reportAnchor(), 2000);
  }

  onDestroy(() => {
    if (reportTimer) clearTimeout(reportTimer);
    reportAnchor();
  });

  // ---- 预取 + LRU（模块级 Map，估算 width*height*4 字节，>200MB 淘汰最久未用）----
  interface PreloadEntry {
    url: string;
    lastUsed: number;
    bytes: number;
  }
  const preloadCache = new Map<string, PreloadEntry>();
  let preloadBytes = 0;
  const MAX_PRELOAD_BYTES = 200 * 1024 * 1024;

  function touchPreload(url: string, bytes = 0) {
    const existing = preloadCache.get(url);
    if (existing) {
      existing.lastUsed = Date.now();
      return;
    }
    preloadCache.set(url, { url, lastUsed: Date.now(), bytes });
    preloadBytes += bytes;
    while (preloadBytes > MAX_PRELOAD_BYTES && preloadCache.size > 0) {
      let oldestKey: string | null = null;
      let oldest = Number.POSITIVE_INFINITY;
      for (const [key, entry] of preloadCache) {
        if (entry.lastUsed < oldest) {
          oldest = entry.lastUsed;
          oldestKey = key;
        }
      }
      if (oldestKey === null) break;
      const removed = preloadCache.get(oldestKey);
      if (removed) preloadBytes = Math.max(0, preloadBytes - removed.bytes);
      preloadCache.delete(oldestKey);
    }
  }

  function prefetchNearbyScreens() {
    const target = new Set<number>();
    for (
      let index = Math.max(0, screenIndex - 2);
      index <= Math.min(screens.length - 1, screenIndex + 2);
      index += 1
    ) {
      for (const pageIndex of screens[index].pageIndexes) target.add(pageIndex);
    }
    for (const pageIndex of target) {
      const url = pages[pageIndex];
      if (!url) continue;
      touchPreload(url);
      const img = new Image();
      img.src = url;
    }
  }
</script>

<div class="comic-reader" data-testid="comic-reader" data-page-mode={pageMode} data-direction={direction}>
  <header class="reader-toolbar" class:hidden={!toolbarVisible} data-testid="reader-toolbar">
    <div class="toolbar-left">
      {#if onclose}
        <button type="button" class="icon-btn" onclick={onclose} aria-label="关闭阅读器" data-testid="reader-close">
          <Icon name="x" size={16} />
        </button>
      {/if}
    </div>

    <div class="toolbar-title">
      <strong class="chapter-title" data-testid="chapter-title">{chapterTitle}</strong>
      <span class="page-indicator" data-testid="page-indicator" aria-live="polite">{pageIndicator}</span>
    </div>

    <div class="toolbar-actions">
      <button
        type="button"
        class="tool-btn"
        onclick={togglePageMode}
        data-testid="mode-toggle"
        aria-label={pageMode === 'dual' ? '切换到单页' : '切换到双页'}
      >
        <Icon name={pageMode === 'dual' ? 'image' : 'layers'} size={14} />
        {pageMode === 'dual' ? '单页' : '双页'}
      </button>
      <button
        type="button"
        class="tool-btn"
        onclick={toggleDirection}
        data-testid="direction-toggle"
        aria-label={direction === 'rtl' ? '切换到从左到右' : '切换到从右到左'}
      >
        <Icon name="arrowLeft" size={14} />
        {direction === 'rtl' ? 'RTL' : 'LTR'}
      </button>
      <button type="button" class="tool-btn" onclick={toggleToolbar} aria-label="隐藏工具栏" data-testid="toolbar-hide">
        <Icon name="chevronDown" size={14} />
      </button>
    </div>
  </header>

  {#if !toolbarVisible}
    <button type="button" class="toolbar-reveal" onclick={toggleToolbar} aria-label="显示工具栏" data-testid="toolbar-reveal">
      <Icon name="chevronDown" size={16} />
    </button>
  {/if}

  <div class="reader-stage" data-testid="reader-stage">
    {#if pages.length === 0}
      <div class="stage-empty" data-testid="stage-empty">
        <Icon name="image" size={30} />
        <span>暂无页面</span>
      </div>
    {:else}
      <div
        class="page-canvas"
        class:dual={pageMode === 'dual'}
        class:rtl={direction === 'rtl' && pageMode === 'dual'}
      >
        {#each currentScreen?.pageIndexes ?? [] as pageIndex (pageIndex)}
          <div class="page-cell" class:spread={currentScreenIsSpread} data-testid="page-cell">
            <img
              src={pages[pageIndex]}
              alt={`${chapterTitle} 第 ${pageIndex + 1} 页`}
              draggable="false"
              data-page-index={pageIndex}
            />
          </div>
        {/each}
      </div>

      <button type="button" class="tap-zone tap-left" onclick={onTapLeft} aria-label="上一屏" data-testid="zone-left"></button>
      <button type="button" class="tap-zone tap-center" onclick={toggleToolbar} aria-label="显示或隐藏工具栏" data-testid="zone-center"></button>
      <button type="button" class="tap-zone tap-right" onclick={onTapRight} aria-label="下一屏" data-testid="zone-right"></button>
    {/if}
  </div>

  {#if narrowConfirm}
    <div class="narrow-bar" role="alert" data-testid="narrow-bar">
      <span>窗口过窄，建议单页阅读</span>
      <div class="narrow-actions">
        <button type="button" class="narrow-btn" onclick={forceNarrowDualMode} data-testid="narrow-force">仍要双页</button>
        <button type="button" class="narrow-btn narrow-cancel" onclick={() => (narrowConfirm = false)} data-testid="narrow-cancel">取消</button>
      </div>
    </div>
  {/if}
</div>

<svelte:window onkeydown={handleKey} />

<style>
  .comic-reader {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    overflow: hidden;
    background: #0a0c10;
    color: var(--v2-color-text, var(--text-primary));
    z-index: 50;
  }

  .reader-toolbar {
    display: grid;
    grid-template-columns: minmax(5rem, auto) minmax(0, 1fr) minmax(14rem, auto);
    align-items: center;
    gap: var(--v2-space-3, 0.75rem);
    min-height: 3.5rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.08));
    background: rgba(10, 12, 16, 0.94);
    backdrop-filter: blur(0.8rem);
    transition: transform 180ms ease;
  }
  .reader-toolbar.hidden { transform: translateY(-100%); }

  .toolbar-left { display: flex; }
  .icon-btn {
    display: grid;
    width: 2.5rem;
    height: 2.5rem;
    place-items: center;
    border: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.12));
    border-radius: var(--v2-radius-md, 0.5rem);
    background: transparent;
    color: var(--v2-color-text-secondary, var(--text-muted));
    cursor: pointer;
  }
  .icon-btn:hover { color: var(--v2-color-text, var(--text-primary)); }

  .toolbar-title { min-width: 0; text-align: center; }
  .chapter-title { display: block; overflow: hidden; font-size: var(--v2-text-sm, 0.875rem); text-overflow: ellipsis; white-space: nowrap; }
  .page-indicator { display: block; margin-top: 0.1rem; color: var(--v2-color-text-secondary, var(--text-muted)); font-size: var(--v2-text-xs, 0.75rem); }

  .toolbar-actions { display: flex; justify-content: flex-end; gap: 0.35rem; }
  .tool-btn {
    display: inline-flex;
    min-height: 2.5rem;
    align-items: center;
    gap: 0.35rem;
    padding: 0 0.7rem;
    border: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.12));
    border-radius: var(--v2-radius-md, 0.5rem);
    background: transparent;
    color: var(--v2-color-text-secondary, var(--text-muted));
    font: inherit;
    font-size: var(--v2-text-sm, 0.875rem);
    cursor: pointer;
  }
  .tool-btn:hover { border-color: var(--v2-color-accent, var(--accent)); color: var(--v2-color-text, var(--text-primary)); }
  .tool-btn:focus-visible, .icon-btn:focus-visible, .tap-zone:focus-visible { outline: none; box-shadow: var(--v2-focus-ring, 0 0 0 2px var(--accent)); }

  .toolbar-reveal {
    position: absolute;
    z-index: 5;
    top: 0.5rem;
    right: 0.5rem;
    display: grid;
    width: 2.75rem;
    height: 2.75rem;
    place-items: center;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 999px;
    background: rgba(10, 12, 16, 0.85);
    color: #fff;
    cursor: pointer;
  }

  .reader-stage {
    position: relative;
    min-height: 0;
    overflow: hidden;
    background: #111318;
  }

  .page-canvas {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
  }
  .page-canvas.rtl { flex-direction: row-reverse; }

  .page-cell {
    display: grid;
    min-width: 0;
    height: 100%;
    flex: 1 1 50%;
    place-items: center;
    transition: opacity 200ms ease;
  }
  .page-cell:only-child { flex: 1 1 100%; }
  .page-cell.spread { flex: 1 1 100%; }

  .page-cell img {
    display: block;
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    user-select: none;
  }

  .tap-zone {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 2;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  .tap-left { left: 0; width: 35%; }
  .tap-center { left: 35%; width: 30%; cursor: default; }
  .tap-right { right: 0; width: 35%; }

  .stage-empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 0.5rem;
    color: var(--v2-color-text-secondary, var(--text-muted));
  }

  .narrow-bar {
    position: absolute;
    z-index: 6;
    left: 50%;
    bottom: 1.5rem;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.16));
    border-radius: var(--v2-radius-lg, 0.75rem);
    background: rgba(10, 12, 16, 0.96);
    box-shadow: 0 0.5rem 1.5rem rgba(0, 0, 0, 0.4);
    transform: translateX(-50%);
    color: var(--v2-color-text, var(--text-primary));
    font-size: var(--v2-text-sm, 0.875rem);
  }
  .narrow-actions { display: flex; gap: 0.4rem; }
  .narrow-btn {
    min-height: 2.25rem;
    padding: 0 0.7rem;
    border: 1px solid var(--v2-color-accent, var(--accent));
    border-radius: var(--v2-radius-md, 0.5rem);
    background: color-mix(in srgb, var(--v2-color-accent, var(--accent)) 14%, transparent);
    color: var(--v2-color-text, var(--text-primary));
    font: inherit;
    font-size: var(--v2-text-xs, 0.75rem);
    cursor: pointer;
  }
  .narrow-btn.narrow-cancel { border-color: var(--v2-color-border, rgba(255, 255, 255, 0.16)); background: transparent; }

  @media (prefers-reduced-motion: reduce) {
    .page-cell { transition: none; }
    .reader-toolbar { transition: none; }
  }
</style>
