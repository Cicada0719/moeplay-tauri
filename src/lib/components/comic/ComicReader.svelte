<script lang="ts">
  import { comicStore } from "../../stores/comic.svelte";
  import Icon from "../Icon.svelte";
  import { Button, EmptyState } from "../ui";

  const images = $derived(comicStore.readerImages);
  const webUrl = $derived(comicStore.readerWebUrl);
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
    <Button variant="ghost" size="sm" press={() => comicStore.closeReader()}>
      <Icon name="x" size={16} />
      关闭
    </Button>
    <div class="chapter-info">
      <span class="chapter-title">{title || `第 ${order} 话`}</span>
      <span class="chapter-pos">{chapterIdx + 1} / {chapters.length}</span>
    </div>
    <div class="chapter-nav">
      <Button variant="ghost" size="sm" press={() => comicStore.prevChapter()} disabled={!hasPrev}>
        <Icon name="chevronLeft" size={15} />
        上一话
      </Button>
      <Button variant="ghost" size="sm" press={() => comicStore.nextChapter()} disabled={!hasNext}>
        下一话
        <Icon name="chevronRight" size={15} />
      </Button>
    </div>
  </div>

  <!-- Content -->
  <div class="reader-scroll">
    {#if loading}
      <div class="reader-loading">
        <div class="spinner"></div>
        <span>加载图片中...</span>
      </div>
    {:else if webUrl}
      <iframe
        class="web-reader"
        src={webUrl}
        title={title || `第 ${order} 话`}
        allow="fullscreen; autoplay; encrypted-media; picture-in-picture"
        sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-top-navigation-by-user-activation"
      ></iframe>
    {:else if images.length === 0}
      <EmptyState class="reader-empty" icon="x" title="未能加载图片" />
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
        <Button variant="ghost" size="md" press={() => comicStore.prevChapter()} disabled={!hasPrev}>
          <Icon name="chevronLeft" size={16} />
          上一话
        </Button>
        <Button variant="secondary" size="md" press={() => comicStore.closeReader()}>
          返回详情
        </Button>
        <Button variant="ghost" size="md" press={() => comicStore.nextChapter()} disabled={!hasNext}>
          下一话
          <Icon name="chevronRight" size={16} />
        </Button>
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

  /* ── Scroll area ── */
  .reader-scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    background: #111318;
  }
  .web-reader {
    width: 100%;
    height: 100%;
    min-height: 100%;
    border: 0;
    display: block;
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
  :global(.reader-empty) {
    flex: 1;
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
