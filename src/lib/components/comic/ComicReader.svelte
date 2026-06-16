<script lang="ts">
  import { comicStore } from "../../stores/comic.svelte";
  import Icon from "../Icon.svelte";

  const images = $derived(comicStore.readerImages);
  const chapters = $derived(comicStore.chapters);
  const order = $derived(comicStore.readerChapterOrder);
  const title = $derived(comicStore.readerChapterTitle);
  const loading = $derived(comicStore.readerLoading);

  const chapterIdx = $derived(chapters.findIndex((c) => c.order === order));
  const hasPrev = $derived(chapterIdx > 0);
  const hasNext = $derived(chapterIdx >= 0 && chapterIdx < chapters.length - 1);

  let loadedCount = $state(0);
  $effect(() => {
    // reset counter when chapter changes
    images;
    loadedCount = 0;
  });
</script>

<div class="reader-overlay" role="dialog">
  <!-- Toolbar -->
  <div class="reader-toolbar">
    <button class="tool-btn" onclick={() => comicStore.closeReader()}>
      <Icon name="x" size={16} />
      关闭
    </button>
    <div class="chapter-info">
      <span class="chapter-title">{title || `第 ${order} 话`}</span>
      <span class="chapter-pos">{chapterIdx + 1} / {chapters.length}</span>
    </div>
    <div class="chapter-nav">
      <button class="nav-btn" onclick={() => comicStore.prevChapter()} disabled={!hasPrev}>
        <Icon name="chevronLeft" size={15} />
        上一话
      </button>
      <button class="nav-btn" onclick={() => comicStore.nextChapter()} disabled={!hasNext}>
        下一话
        <Icon name="chevronRight" size={15} />
      </button>
    </div>
  </div>

  <!-- Content -->
  <div class="reader-scroll">
    {#if loading}
      <div class="reader-loading">
        <div class="spinner"></div>
        <span>加载图片中...</span>
      </div>
    {:else if images.length === 0}
      <div class="reader-loading">
        <Icon name="x" size={28} />
        <span>未能加载图片</span>
      </div>
    {:else}
      <div class="images-list">
        {#each images as img (img.id)}
          <div class="img-wrap">
            <img
              src={img.url}
              alt="漫画页"
              loading="lazy"
              class="comic-img"
              onload={() => { loadedCount += 1; }}
              onerror={(e) => {
                const el = e.currentTarget as HTMLImageElement;
                el.style.minHeight = "60px";
                el.alt = "图片加载失败";
              }}
            />
          </div>
        {/each}
      </div>

      <!-- Bottom nav -->
      <div class="reader-bottom-nav">
        <button class="bottom-nav-btn" onclick={() => comicStore.prevChapter()} disabled={!hasPrev}>
          <Icon name="chevronLeft" size={16} />
          上一话
        </button>
        <button class="bottom-nav-btn close" onclick={() => comicStore.closeReader()}>
          返回详情
        </button>
        <button class="bottom-nav-btn" onclick={() => comicStore.nextChapter()} disabled={!hasNext}>
          下一话
          <Icon name="chevronRight" size={16} />
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .reader-overlay {
    position: absolute;
    inset: 0;
    background: #0a0c12;
    z-index: 30;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* ── Toolbar ── */
  .reader-toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    background: rgba(10, 12, 18, 0.9);
    border-bottom: 1px solid rgba(255,255,255,0.06);
    backdrop-filter: blur(8px);
  }
  .tool-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 6px 12px;
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 6px;
    background: rgba(255,255,255,0.05);
    color: var(--text-muted);
    font-size: 12.5px;
    cursor: pointer;
    transition: all 0.15s;
    flex-shrink: 0;
  }
  .tool-btn:hover {
    background: rgba(255,255,255,0.1);
    color: var(--text-primary);
  }
  .chapter-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
  }
  .chapter-title {
    font-size: 13px;
    font-weight: 650;
    color: var(--text-primary);
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chapter-pos {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .chapter-nav {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .nav-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .nav-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .nav-btn:not(:disabled):hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-lo, rgba(232,85,127,0.08));
  }

  /* ── Scroll area ── */
  .reader-scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    background: #111318;
  }
  .images-list {
    display: flex;
    flex-direction: column;
    align-items: center;
    max-width: 840px;
    margin: 0 auto;
    padding: 12px 0 24px;
    gap: 2px;
  }
  .img-wrap {
    width: 100%;
    line-height: 0;
  }
  .comic-img {
    display: block;
    width: 100%;
    height: auto;
    min-height: 200px;
    background: #1a1d27;
  }

  /* ── Bottom nav ── */
  .reader-bottom-nav {
    display: flex;
    justify-content: center;
    gap: 12px;
    padding: 28px 16px 40px;
  }
  .bottom-nav-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 10px 20px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: rgba(255,255,255,0.04);
    color: var(--text-muted);
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .bottom-nav-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .bottom-nav-btn.close {
    border-color: var(--accent-ring, rgba(232,85,127,0.3));
    color: var(--accent);
  }
  .bottom-nav-btn:not(:disabled):hover {
    border-color: var(--accent);
    background: var(--accent-lo, rgba(232,85,127,0.1));
    color: var(--accent);
  }

  .reader-loading {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    color: var(--text-muted);
    min-height: 400px;
  }
  .spinner {
    width: 36px;
    height: 36px;
    border: 3px solid rgba(255,255,255,0.08);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
