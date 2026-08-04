<script lang="ts">
  // 小说阅读器（spec §4 步骤 6）
  //
  // 职责：章节文本渲染 + 滚动位置（scrollPct = scrollTop/(scrollHeight-clientHeight)，
  // 0~1）debounce 500ms 上报 `history_upsert`；`initialScrollPct` 进入后在内容渲染
  // 完成后（tick + 两帧 rAF）恢复滚动位置；`onDestroy` 立即 flush 一次。
  //
  // 与 ComicReader 相同：`history_upsert` 命令在子任务 4 尚未注册，上报失败静默降级
  // （`.catch(() => {})`），等命令补齐后直接生效。
  import { onDestroy, onMount, tick } from 'svelte';
  import { buildHistoryId, upsertHistory } from '../../history/historyApi';
  import type { HistoryItem } from '../../history/types';
  import Icon from '../Icon.svelte';

  let {
    contentId,
    sourceId,
    chapterId,
    chapterTitle,
    title = '',
    cover = null,
    content,
    initialScrollPct = 0,
    onclose,
  }: {
    contentId: string;
    sourceId: string;
    chapterId: string;
    chapterTitle: string;
    title?: string;
    cover?: string | null;
    /** 章节全文（纯文本，按段落渲染） */
    content: string;
    /** 从历史进入时传入的滚动百分比（0~1）；默认 0 = 顶部 */
    initialScrollPct?: number;
    onclose?: () => void;
  } = $props();

  let containerEl = $state<HTMLElement | null>(null);
  let reportTimer: ReturnType<typeof setTimeout> | null = null;
  let lastReported = -1;

  function computeScrollPct(): number {
    if (!containerEl) return 0;
    const { scrollTop, scrollHeight, clientHeight } = containerEl;
    const denominator = scrollHeight - clientHeight;
    if (denominator <= 0) return 0; // 除零保护：内容不足一屏时记 0
    return Math.min(1, Math.max(0, scrollTop / denominator));
  }

  function reportScroll(pct: number) {
    if (Math.abs(pct - lastReported) < 0.002) return; // 1 万条级防抖：微小位移不重复写
    lastReported = pct;
    const item: HistoryItem = {
      id: buildHistoryId(contentId, sourceId, chapterId),
      contentId,
      contentType: 'novel',
      title: title || chapterTitle || '小说',
      cover,
      sourceId,
      chapterId,
      chapterTitle,
      pageIndex: 0,
      positionSec: 0,
      scrollPct: pct,
      progress: pct,
      updatedAt: Date.now(),
      deviceId: '',
      deleted: false,
    };
    void upsertHistory(item).catch(() => {});
  }

  function onScroll() {
    if (reportTimer) clearTimeout(reportTimer);
    const pct = computeScrollPct();
    reportTimer = setTimeout(() => reportScroll(pct), 500);
  }

  async function restorePosition() {
    if (!containerEl || initialScrollPct <= 0) return;
    await tick();
    // 等两帧 rAF：让字体/图片加载后真实滚动高度稳定下来，再恢复位置。
    await new Promise<void>((resolve) => {
      if (typeof window === 'undefined' || typeof requestAnimationFrame !== 'function') {
        resolve();
        return;
      }
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          if (!containerEl) {
            resolve();
            return;
          }
          const denominator = containerEl.scrollHeight - containerEl.clientHeight;
          containerEl.scrollTo({ top: initialScrollPct * Math.max(0, denominator) });
          resolve();
        }),
      );
    });
  }

  onMount(() => {
    void restorePosition();
  });

  onDestroy(() => {
    if (reportTimer) clearTimeout(reportTimer);
    reportScroll(computeScrollPct());
  });
</script>

<div class="novel-reader" data-testid="novel-reader">
  <header class="novel-toolbar">
    {#if onclose}
      <button type="button" class="icon-btn" onclick={onclose} aria-label="关闭阅读器" data-testid="novel-close">
        <Icon name="x" size={16} />
      </button>
    {/if}
    <strong class="chapter-title" data-testid="novel-chapter-title">{chapterTitle}</strong>
  </header>

  <div
    bind:this={containerEl}
    class="novel-scroll"
    data-testid="novel-scroll"
    onscroll={onScroll}
    role="region"
    aria-label="小说正文"
  >
    {#if !content}
      <div class="novel-empty" data-testid="novel-empty">
        <Icon name="collection" size={28} />
        <span>暂无章节内容</span>
      </div>
    {:else}
      <article class="novel-article">
        {#each content.split(/\n+/) as paragraph, index (index)}
          <p>{paragraph}</p>
        {/each}
      </article>
    {/if}
  </div>
</div>

<style>
  .novel-reader {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    overflow: hidden;
    background: var(--v2-color-surface, #101318);
    color: var(--v2-color-text, var(--text-primary));
    z-index: 50;
  }

  .novel-toolbar {
    display: flex;
    align-items: center;
    gap: var(--v2-space-3, 0.75rem);
    min-height: 3.5rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.08));
    background: rgba(16, 19, 24, 0.94);
  }
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
  .chapter-title { overflow: hidden; font-size: var(--v2-text-sm, 0.875rem); text-overflow: ellipsis; white-space: nowrap; }

  .novel-scroll {
    min-height: 0;
    overflow: auto;
    padding: 1.5rem clamp(1rem, 6vw, 4rem) 4rem;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    outline: none;
  }
  .novel-article {
    max-width: 46rem;
    margin: 0 auto;
    font-size: var(--v2-text-base, 1rem);
    line-height: 1.9;
  }
  .novel-article p { margin: 0 0 1.25em; white-space: pre-wrap; }

  .novel-empty {
    display: grid;
    min-height: 60vh;
    place-content: center;
    justify-items: center;
    gap: 0.5rem;
    color: var(--v2-color-text-secondary, var(--text-muted));
  }
</style>
