<script lang="ts">
  import Icon from "./Icon.svelte";
  import CachedImage from "./CachedImage.svelte";

  interface MediaItem {
    id: string;
    title: string;
    cover: string | null;
    progress?: number;
    progressLabel?: string;
    type: "anime" | "comic";
  }

  let {
    title,
    items,
    startIndex = 0,
    activeIndex = -1,
    zoneActive = false,
    onfocusitem,
    onselect,
  }: {
    title: string;
    items: MediaItem[];
    startIndex?: number;
    activeIndex?: number;
    zoneActive?: boolean;
    onfocusitem?: (index: number) => void;
    onselect?: (item: MediaItem) => void;
  } = $props();

  let scroller = $state<HTMLDivElement>();

  function scroll(dir: number) {
    if (!scroller) return;
    scroller.scrollBy({ left: dir * 300, behavior: "smooth" });
  }

  $effect(() => {
    const index = activeIndex;
    if (!zoneActive || index < startIndex || index >= startIndex + items.length) return;
    queueMicrotask(() => {
      const target = scroller?.querySelector<HTMLElement>(`[data-media-index="${index}"]`);
      target?.scrollIntoView({ inline: "nearest", block: "nearest", behavior: "smooth" });
      target?.focus({ preventScroll: true });
    });
  });
</script>

{#if items.length > 0}
  <section class="bpmr-section" aria-label={title}>
    <div class="bpmr-header">
      <h4 class="bpmr-title">{title}</h4>
      <div class="bpmr-nav" aria-hidden="true">
        <button class="bpmr-arrow" tabindex="-1" onclick={() => scroll(-1)} aria-label="左滚">
          <Icon name="chevronLeft" size={14} />
        </button>
        <button class="bpmr-arrow" tabindex="-1" onclick={() => scroll(1)} aria-label="右滚">
          <Icon name="chevronRight" size={14} />
        </button>
      </div>
    </div>
    <div class="bpmr-track" bind:this={scroller} role="group" aria-label={title}>
      {#each items as item, localIndex (item.id)}
        {@const globalIndex = startIndex + localIndex}
        <button
          class="bpmr-card"
          class:zone-focus={zoneActive && activeIndex === globalIndex}
          data-media-index={globalIndex}
          tabindex={zoneActive && activeIndex === globalIndex ? 0 : -1}
          aria-current={activeIndex === globalIndex ? "true" : undefined}
          onclick={() => onselect?.(item)}
          onfocus={() => onfocusitem?.(globalIndex)}
        >
          <div class="bpmr-cover">
            {#if item.cover}
              <CachedImage source={item.cover} cacheKey={`bpmr-${item.id}`} alt={item.title} />
            {:else}
              <div class="bpmr-placeholder">{item.title[0]}</div>
            {/if}
            {#if item.progress !== undefined}
              <div class="bpmr-progress-bar"><div class="bpmr-progress-fill" style="width:{item.progress}%"></div></div>
            {/if}
            <span class="bpmr-type-badge">{item.type === "anime" ? "📺" : "📖"}</span>
          </div>
          <div class="bpmr-info">
            <span class="bpmr-name">{item.title}</span>
            {#if item.progressLabel}<span class="bpmr-progress-label">{item.progressLabel}</span>{/if}
          </div>
        </button>
      {/each}
    </div>
  </section>
{/if}

<style>
  .bpmr-section { margin-bottom: 20px; }
  .bpmr-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0 4px; margin-bottom: 8px;
  }
  .bpmr-title { margin: 0; font-size: 14px; font-weight: 700; color: var(--text-primary); }
  .bpmr-nav { display: flex; gap: 4px; }
  .bpmr-arrow {
    display: flex; align-items: center; justify-content: center;
    width: 24px; height: 24px; border: 1px solid rgba(255,255,255,0.1); border-radius: 6px;
    background: rgba(255,255,255,0.04); color: var(--text-muted); cursor: pointer;
  }
  .bpmr-arrow:hover { border-color: var(--accent); color: var(--accent); }

  .bpmr-track {
    display: flex; gap: 10px; overflow-x: auto; padding: 4px 0;
    scrollbar-width: none;
  }
  .bpmr-track::-webkit-scrollbar { display: none; }

  .bpmr-card {
    flex: 0 0 auto; width: 110px; cursor: pointer;
    background: none; border: 2px solid transparent; border-radius: 8px;
    padding: 2px; transition: all 0.15s;
  }
  .bpmr-card:hover { border-color: rgba(255,255,255,0.15); }
  .bpmr-card:focus, .bpmr-card.zone-focus { border-color: var(--accent); outline: none; box-shadow: var(--ring-switch, 0 0 0 3px rgba(232,85,127,.35)); }

  .bpmr-cover {
    width: 100%; aspect-ratio: 3/4; border-radius: 6px; overflow: hidden;
    background: rgba(255,255,255,0.06); position: relative;
  }
  .bpmr-cover :global(.cached-image) { width: 100%; height: 100%; object-fit: cover; }
  .bpmr-placeholder {
    width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;
    font-size: 28px; font-weight: 700; color: var(--text-muted);
  }
  .bpmr-progress-bar {
    position: absolute; bottom: 0; left: 0; right: 0; height: 3px;
    background: rgba(0,0,0,0.5);
  }
  .bpmr-progress-fill { height: 100%; background: var(--accent, #e8557f); }
  .bpmr-type-badge {
    position: absolute; top: 4px; right: 4px; font-size: 12px;
  }

  .bpmr-info { padding: 4px 2px 0; text-align: left; }
  .bpmr-name {
    display: block; font-size: 11px; font-weight: 600; color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .bpmr-progress-label {
    display: block; font-size: 10px; color: var(--text-muted);
  }

  @media (prefers-reduced-motion: reduce) {
    .bpmr-card { transition: none; }
  }
  :global([data-motion="reduce"]) .bpmr-card { transition: none; }
</style>
