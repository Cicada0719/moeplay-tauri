<script lang="ts">
  import { tick } from "svelte";
  import { focusTrap } from "../../actions/a11y/focusTrap";
  import {
    getReaderKeyboardCommand,
    moveReaderPage,
    nextReadingDirection,
    normalizeReaderZoom,
    readerDirectionLabel,
    readerSwipePageDelta,
    READER_ZOOM_STEP,
    type ComicReadingDirection,
  } from "../../features/comic/reader";
  import { comicStore } from "../../stores/comic.svelte";
  import Icon from "../Icon.svelte";
  import { Button } from "../ui";
  import { AsyncState } from "../ui-v2";

  let {
    onclose,
    returnFocusKey,
  }: {
    onclose?: () => void | Promise<void>;
    returnFocusKey?: string;
  } = $props();

  const images = $derived(comicStore.readerImages);
  const webUrl = $derived(comicStore.readerWebUrl);
  const chapters = $derived(comicStore.chapters);
  const order = $derived(comicStore.readerChapterOrder);
  const title = $derived(comicStore.readerChapterTitle);
  const loading = $derived(comicStore.readerLoading);
  const chapterIdx = $derived(chapters.findIndex((chapter) => chapter.order === order));
  const hasPrev = $derived(chapterIdx > 0);
  const hasNext = $derived(chapterIdx >= 0 && chapterIdx < chapters.length - 1);

  let readerRoot = $state<HTMLElement>();
  let scrollRoot = $state<HTMLElement>();
  let direction = $state<ComicReadingDirection>("vertical");
  let zoom = $state(100);
  let toolbarVisible = $state(true);
  let currentPage = $state(0);
  let loadedPages = $state<Set<string>>(new Set());
  let failedPages = $state<Record<string, string>>({});
  let retryVersions = $state<Record<string, number>>({});
  let swipePointerId = $state<number | null>(null);
  let swipeStartX = $state(0);
  let swipeStartY = $state(0);

  const pageCount = $derived(images.length);
  const directionLabel = $derived(readerDirectionLabel(direction));
  const pageStatus = $derived(pageCount > 0 ? `${Math.min(currentPage + 1, pageCount)} / ${pageCount}` : "0 / 0");

  $effect(() => {
    images;
    currentPage = 0;
    loadedPages = new Set();
    failedPages = {};
    retryVersions = {};
  });

  function isTypingTarget(target: EventTarget | null): boolean {
    return target instanceof HTMLElement && (
      target.matches("input, textarea, select, [contenteditable='true']")
      || Boolean(target.closest("input, textarea, select, [contenteditable='true']"))
    );
  }

  async function closeReader() {
    if (onclose) await onclose();
    else comicStore.closeReader();

    if (!returnFocusKey) return;
    await tick();
    document.querySelector<HTMLElement>(`[data-chapter-focus-key="${CSS.escape(returnFocusKey)}"]`)?.focus({ preventScroll: true });
  }

  function pageElement(index: number): HTMLElement | null {
    return readerRoot?.querySelector<HTMLElement>(`[data-reader-page-index="${index}"]`) ?? null;
  }

  function movePage(delta: number) {
    if (pageCount === 0) return;
    const next = moveReaderPage(currentPage, delta, pageCount);
    if (next === currentPage && direction !== "vertical") return;
    currentPage = next;
    if (direction === "vertical") {
      pageElement(next)?.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }

  function jumpPage(index: number) {
    if (pageCount === 0) return;
    currentPage = moveReaderPage(0, index, pageCount);
    if (direction === "vertical") pageElement(currentPage)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  async function changeChapter(delta: -1 | 1) {
    if (delta < 0 && !hasPrev) return;
    if (delta > 0 && !hasNext) return;
    if (delta < 0) await comicStore.prevChapter();
    else await comicStore.nextChapter();
    await tick();
    scrollRoot?.scrollTo({ top: 0, behavior: "auto" });
  }

  function cycleDirection() {
    direction = nextReadingDirection(direction);
    currentPage = Math.min(currentPage, Math.max(0, pageCount - 1));
    queueMicrotask(() => pageElement(currentPage)?.focus({ preventScroll: true }));
  }

  function changeZoom(delta: number) {
    zoom = normalizeReaderZoom(zoom + delta);
  }

  function recordPageLoaded(id: string) {
    loadedPages = new Set([...loadedPages, id]);
    if (failedPages[id]) {
      const next = { ...failedPages };
      delete next[id];
      failedPages = next;
    }
  }

  function recordPageFailure(id: string) {
    failedPages = { ...failedPages, [id]: "图片加载失败，请重试当前页" };
  }

  function retryPage(id: string) {
    const next = { ...failedPages };
    delete next[id];
    failedPages = next;
    retryVersions = { ...retryVersions, [id]: (retryVersions[id] ?? 0) + 1 };
  }

  function updateCurrentPageFromScroll() {
    if (direction !== "vertical" || !scrollRoot || pageCount === 0) return;
    const rootTop = scrollRoot.getBoundingClientRect().top;
    let nearest = currentPage;
    let nearestDistance = Number.POSITIVE_INFINITY;
    for (let index = 0; index < pageCount; index += 1) {
      const element = pageElement(index);
      if (!element) continue;
      const distance = Math.abs(element.getBoundingClientRect().top - rootTop - 8);
      if (distance < nearestDistance) {
        nearest = index;
        nearestDistance = distance;
      }
    }
    currentPage = nearest;
  }

  function handlePointerDown(event: PointerEvent) {
    if (direction === "vertical" || (event.pointerType !== "touch" && event.pointerType !== "pen")) return;
    if ((event.target as HTMLElement).closest("button, input, select, textarea, a")) return;
    swipePointerId = event.pointerId;
    swipeStartX = event.clientX;
    swipeStartY = event.clientY;
    try { scrollRoot?.setPointerCapture?.(event.pointerId); } catch { /* synthetic events and older WebViews */ }
  }

  function handlePointerUp(event: PointerEvent) {
    if (swipePointerId !== event.pointerId) return;
    const delta = readerSwipePageDelta(event.clientX - swipeStartX, event.clientY - swipeStartY, direction);
    swipePointerId = null;
    if (delta !== 0) movePage(delta);
  }

  function cancelPointerGesture() {
    swipePointerId = null;
  }

  async function handleKeydown(event: KeyboardEvent) {
    if (isTypingTarget(event.target)) return;
    const command = getReaderKeyboardCommand(event, direction);
    if (!command) return;
    event.preventDefault();
    event.stopPropagation();

    if (command === "close") return closeReader();
    if (command === "previous_page") return movePage(-1);
    if (command === "next_page") return movePage(1);
    if (command === "previous_chapter") return changeChapter(-1);
    if (command === "next_chapter") return changeChapter(1);
    if (command === "first_page") return jumpPage(0);
    if (command === "last_page") return jumpPage(pageCount - 1);
    if (command === "cycle_direction") return cycleDirection();
    if (command === "zoom_in") return changeZoom(READER_ZOOM_STEP);
    if (command === "zoom_out") return changeZoom(-READER_ZOOM_STEP);
    if (command === "reset_zoom") { zoom = 100; return; }
    if (command === "toggle_toolbar") toolbarVisible = !toolbarVisible;
  }
</script>

<div
  bind:this={readerRoot}
  class="reader-overlay"
  class:toolbar-hidden={!toolbarVisible}
  role="dialog"
  aria-modal="true"
  aria-labelledby="comic-reader-title"
  aria-describedby="comic-reader-help"
  tabindex="-1"
  data-reading-direction={direction}
  style={`--reader-zoom:${zoom / 100}`}
  use:focusTrap={{
    initialFocus: ".reader-close",
    returnFocus: false,
    closeOnEscape: true,
    onEscape: () => void closeReader(),
  }}
  onkeydown={handleKeydown}
>
  {#if toolbarVisible}
    <header class="reader-toolbar" aria-label="漫画阅读控制栏">
      <Button class="reader-close" variant="ghost" size="sm" press={closeReader} ariaLabel="关闭阅读器并返回章节">
        <Icon name="x" size={16} />
        关闭
      </Button>

      <div class="chapter-info">
        <strong id="comic-reader-title" class="chapter-title">{title || `第 ${order} 话`}</strong>
        <span class="chapter-pos">章节 {chapterIdx + 1} / {chapters.length} · 页面 {pageStatus}</span>
      </div>

      <div class="reader-tools" aria-label="阅读显示设置">
        <Button variant="quiet" size="sm" press={cycleDirection} title="按 D 切换阅读方向" ariaLabel={`阅读方向：${directionLabel}`}>
          <Icon name="layers" size={14} />{directionLabel}
        </Button>
        <Button variant="quiet" size="sm" press={() => changeZoom(-READER_ZOOM_STEP)} disabled={zoom <= 60} ariaLabel="缩小漫画">−</Button>
        <output class="zoom-output" aria-label="当前缩放">{zoom}%</output>
        <Button variant="quiet" size="sm" press={() => changeZoom(READER_ZOOM_STEP)} disabled={zoom >= 200} ariaLabel="放大漫画">＋</Button>
        <Button variant="quiet" size="sm" press={() => (toolbarVisible = false)} ariaLabel="隐藏阅读工具栏" title="按 T 恢复工具栏">
          <Icon name="chevronDown" size={14} />
        </Button>
      </div>
    </header>
  {:else}
    <button class="toolbar-reveal" type="button" onclick={() => (toolbarVisible = true)} aria-label="显示阅读工具栏" title="显示工具栏 (T)">
      <Icon name="chevronDown" size={16} />
    </button>
  {/if}

  <p id="comic-reader-help" class="sr-only">
    Escape 退出，方向键或 PageUp/PageDown 翻页，方括号切换章节，D 切换阅读方向，加减号缩放，T 显示或隐藏工具栏。
  </p>

  <div
    bind:this={scrollRoot}
    class="reader-scroll"
    class:single-page-mode={direction !== "vertical"}
    role="region"
    aria-label="漫画页面"
    onscroll={updateCurrentPageFromScroll}
    onpointerdown={handlePointerDown}
    onpointerup={handlePointerUp}
    onpointercancel={cancelPointerGesture}
  >
    {#if loading}
      <AsyncState state="loading" title="正在准备漫画页面" description="正在解析当前章节，请稍候。" loadingDelayMs={0} />
    {:else if webUrl}
      <div class="web-reader-shell">
        <iframe
          class="web-reader"
          src={webUrl}
          title={`${title || `第 ${order} 话`} 网页阅读器`}
          allow="fullscreen; autoplay; encrypted-media; picture-in-picture"
          sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-top-navigation-by-user-activation"
        ></iframe>
      </div>
    {:else if images.length === 0}
      <AsyncState
        state="error"
        title="未能加载漫画页面"
        description={comicStore.error || "当前章节没有返回可阅读图片。"}
        primaryAction={{ label: "重试章节", onSelect: () => void comicStore.openChapter(order, title) }}
        secondaryAction={{ label: "返回章节", onSelect: () => void closeReader() }}
      />
    {:else}
      <div class="images-list" class:single-page={direction !== "vertical"} aria-live="polite">
        {#each images as image, index (image.id)}
          {#if direction === "vertical" || index === currentPage}
            <article
              class="img-wrap"
              class:page-failed={Boolean(failedPages[image.id])}
              data-reader-page-index={index}
              tabindex="-1"
              aria-label={`${title || `第 ${order} 话`} 第 ${index + 1} 页`}
            >
              {#if failedPages[image.id]}
                <div class="page-error" role="alert">
                  <Icon name="refresh" size={22} />
                  <strong>第 {index + 1} 页加载失败</strong>
                  <span>{failedPages[image.id]}</span>
                  <Button variant="secondary" size="sm" press={() => retryPage(image.id)}>重试当前页</Button>
                </div>
              {:else}
                {#key `${image.id}:${retryVersions[image.id] ?? 0}`}
                  <img
                    src={image.url}
                    alt={`${title || `第 ${order} 话`} 第 ${index + 1} 页`}
                    loading={index <= currentPage + 2 ? "eager" : "lazy"}
                    class="comic-img"
                    class:is-loaded={loadedPages.has(image.id)}
                    onload={() => recordPageLoaded(image.id)}
                    onerror={() => recordPageFailure(image.id)}
                  />
                {/key}
              {/if}
            </article>
          {/if}
        {/each}
        {#if direction !== "vertical" && pageCount > 1}
          <button
            class="page-edge page-edge-previous"
            class:rtl={direction === "right-to-left"}
            type="button"
            onclick={() => movePage(-1)}
            disabled={currentPage === 0}
            aria-label="上一页"
          ><Icon name="chevronLeft" size={22} /></button>
          <button
            class="page-edge page-edge-next"
            class:rtl={direction === "right-to-left"}
            type="button"
            onclick={() => movePage(1)}
            disabled={currentPage >= pageCount - 1}
            aria-label="下一页"
          ><Icon name="chevronRight" size={22} /></button>
        {/if}
      </div>

      <nav class="reader-bottom-nav" aria-label="漫画阅读导航">
        <Button variant="ghost" size="md" press={() => movePage(-1)} disabled={currentPage === 0}>
          <Icon name="chevronLeft" size={16} />上一页
        </Button>
        <Button variant="ghost" size="md" press={() => changeChapter(-1)} disabled={!hasPrev}>上一话</Button>
        <Button variant="secondary" size="md" press={closeReader}>返回详情</Button>
        <Button variant="ghost" size="md" press={() => changeChapter(1)} disabled={!hasNext}>下一话</Button>
        <Button variant="ghost" size="md" press={() => movePage(1)} disabled={currentPage >= pageCount - 1}>
          下一页<Icon name="chevronRight" size={16} />
        </Button>
      </nav>
    {/if}
  </div>
</div>

<style>
  .reader-overlay {
    position: absolute;
    inset: 0;
    z-index: 50;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    overflow: hidden;
    background: #090b10;
    color: var(--v2-color-text, var(--text-primary));
    outline: none;
  }

  .reader-overlay.toolbar-hidden { grid-template-rows: minmax(0, 1fr); }

  .reader-toolbar {
    z-index: 2;
    display: grid;
    grid-template-columns: minmax(7rem, auto) minmax(0, 1fr) minmax(16rem, auto);
    align-items: center;
    gap: var(--v2-space-3, 0.75rem);
    min-height: 3.75rem;
    padding: max(0.55rem, env(safe-area-inset-top)) max(1rem, env(safe-area-inset-right)) 0.55rem max(1rem, env(safe-area-inset-left));
    border-bottom: 1px solid var(--v2-color-border, rgba(255,255,255,0.08));
    background: rgba(10, 12, 18, 0.94);
    backdrop-filter: blur(0.8rem);
  }

  .chapter-info { min-width: 0; text-align: center; }
  .chapter-title,
  .chapter-pos { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chapter-title { font-size: var(--v2-text-sm, 0.875rem); }
  .chapter-pos { margin-top: 0.12rem; color: var(--v2-color-text-secondary, var(--text-muted)); font-size: var(--v2-text-xs, 0.75rem); }

  .reader-tools { display: flex; align-items: center; justify-content: flex-end; gap: 0.3rem; }
  .zoom-output { min-width: 3.2rem; color: var(--v2-color-text-secondary, var(--text-muted)); text-align: center; font-size: var(--v2-text-xs, 0.75rem); }

  .toolbar-reveal {
    position: absolute;
    z-index: 4;
    top: max(0.5rem, env(safe-area-inset-top));
    right: max(0.5rem, env(safe-area-inset-right));
    display: grid;
    width: 3rem;
    min-height: 3rem;
    place-items: center;
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 999px;
    background: rgba(10,12,18,0.82);
    color: #fff;
    cursor: pointer;
  }

  .toolbar-reveal:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }

  .reader-scroll {
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    background: #111318;
    scroll-behavior: smooth;
    scroll-padding-top: 0.75rem;
  }

  .reader-scroll.single-page-mode { touch-action: pan-y; }

  .web-reader-shell { width: 100%; height: 100%; min-height: 32rem; }
  .web-reader { display: block; width: 100%; height: 100%; min-height: 32rem; border: 0; background: #111318; }

  .images-list {
    display: flex;
    width: min(100%, calc(58rem * var(--reader-zoom)));
    margin-inline: auto;
    padding: 0.75rem 0 1.5rem;
    flex-direction: column;
    align-items: center;
    gap: 0.15rem;
  }

  .images-list.single-page {
    position: relative;
    width: 100%;
    min-height: calc(100% - 6rem);
    padding: 1rem;
    justify-content: center;
  }

  .page-edge {
    position: absolute;
    z-index: 3;
    top: 1rem;
    bottom: 1rem;
    display: grid;
    width: min(18%, 7rem);
    place-items: center;
    border: 0;
    background: transparent;
    color: transparent;
    cursor: pointer;
  }
  .page-edge:not(:disabled):focus-visible,
  .page-edge:not(:disabled):active { outline: none; background: linear-gradient(90deg, rgba(0,0,0,.5), transparent); color: #fff; }
  .page-edge:disabled { pointer-events: none; }
  .page-edge-previous { left: 0; }
  .page-edge-next { right: 0; }
  .page-edge-next:not(:disabled):focus-visible,
  .page-edge-next:not(:disabled):active { background: linear-gradient(-90deg, rgba(0,0,0,.5), transparent); }
  .page-edge-previous.rtl { right: 0; left: auto; }
  .page-edge-next.rtl { right: auto; left: 0; }

  .img-wrap {
    position: relative;
    width: 100%;
    min-height: 8rem;
    line-height: 0;
    outline: none;
  }

  .single-page .img-wrap {
    display: flex;
    min-height: calc(100dvh - 10rem);
    align-items: center;
    justify-content: center;
  }

  .comic-img {
    display: block;
    width: 100%;
    height: auto;
    min-height: 12rem;
    background: #1a1d27;
    opacity: 0.65;
    transition: opacity var(--v2-motion-fast, 120ms) ease;
  }

  .single-page .comic-img {
    width: auto;
    max-width: calc(100% * var(--reader-zoom));
    max-height: calc((100dvh - 10rem) * var(--reader-zoom));
    object-fit: contain;
  }

  .comic-img.is-loaded { min-height: 0; opacity: 1; }

  .page-error {
    display: flex;
    min-height: 18rem;
    padding: 2rem;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.7rem;
    border: 1px dashed rgba(248,113,113,0.38);
    background: rgba(248,113,113,0.08);
    color: #fca5a5;
    line-height: 1.4;
    text-align: center;
  }

  .page-error span { color: var(--v2-color-text-secondary, #bbb); font-size: 0.82rem; }

  .reader-bottom-nav {
    display: flex;
    padding: 2rem 1rem max(2.5rem, env(safe-area-inset-bottom));
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.65rem;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (max-width: 52rem) {
    .reader-toolbar {
      grid-template-columns: auto minmax(0, 1fr);
      align-items: start;
      padding-inline: 0.65rem;
    }

    .reader-tools { grid-column: 1 / -1; justify-content: center; overflow-x: auto; padding-bottom: 0.2rem; }
    .chapter-info { text-align: left; }
    .images-list { width: min(100%, calc(46rem * var(--reader-zoom))); }
    .reader-bottom-nav { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); padding-inline: 0.65rem; }
    .reader-bottom-nav :global(button:nth-child(3)) { grid-column: 1 / -1; grid-row: 1; }
  }

  @media (max-width: 52rem) and (orientation: portrait) {
    .reader-overlay { grid-template-rows: auto minmax(0, 1fr) auto; }
    .reader-overlay.toolbar-hidden { grid-template-rows: minmax(0, 1fr) auto; }
    .reader-scroll {
      scroll-padding-bottom: calc(5rem + env(safe-area-inset-bottom));
      overscroll-behavior-y: contain;
    }
    .reader-bottom-nav {
      position: sticky;
      bottom: 0;
      z-index: 3;
      padding-block: 0.65rem max(0.85rem, env(safe-area-inset-bottom));
      border-top: 1px solid rgba(255,255,255,0.08);
      background: rgba(9, 11, 16, 0.94);
      backdrop-filter: blur(0.85rem);
    }
    .reader-bottom-nav :global(button) { min-height: 44px; }
    .single-page .img-wrap { min-height: calc(100dvh - 9rem); }
    .single-page .comic-img { max-height: calc((100dvh - 9rem) * var(--reader-zoom)); }
  }

  @media (max-width: 34rem) {
    .reader-toolbar { min-height: 3.25rem; }
    .reader-tools :global(button:first-child) { flex: 1; }
    .zoom-output { min-width: 2.6rem; }
    .images-list.single-page { padding-inline: 0.25rem; }
  }

  @media (max-height: 520px) and (orientation: landscape) {
    .reader-toolbar { grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: .4rem; min-height: 3rem; padding-block: max(.25rem, env(safe-area-inset-top)) .25rem; }
    .reader-tools { grid-column: auto; justify-content: flex-end; padding: 0; }
    .chapter-info { text-align: center; }
    .chapter-pos { display: none; }
    .reader-tools :global(button) { min-height: 38px; padding-inline: .55rem; }
    .single-page .img-wrap { min-height: calc(100dvh - 5.25rem); }
    .single-page .comic-img { max-height: calc((100dvh - 5.25rem) * var(--reader-zoom)); }
    .images-list.single-page { min-height: calc(100% - 3rem); padding: .25rem; }
    .reader-scroll.single-page-mode { touch-action: pan-y pinch-zoom; }
    .page-edge { top: .25rem; bottom: .25rem; width: min(22%, 8rem); }
    .reader-bottom-nav { display: none; }
  }

  @media (min-width: 100rem) {
    .reader-toolbar { min-height: 4.5rem; padding-inline: 2rem; }
    .chapter-title { font-size: 1.1rem; }
    .chapter-pos { font-size: 0.9rem; }
    .reader-tools :global(button),
    .reader-toolbar :global(button),
    .reader-bottom-nav :global(button) { min-height: 3.5rem; }
    .images-list { width: min(100%, calc(74rem * var(--reader-zoom))); }
  }

  @media (prefers-reduced-motion: reduce) {
    .reader-scroll { scroll-behavior: auto; }
    .comic-img { transition: none; }
  }
</style>
