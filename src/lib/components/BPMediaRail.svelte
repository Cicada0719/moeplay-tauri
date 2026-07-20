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
  .bpmr-section { margin-bottom: clamp(18px, 2.4vh, 30px); }
  .bpmr-header { display: flex; align-items: center; justify-content: space-between; padding: 0 4px; margin-bottom: 10px; }
  .bpmr-title { margin: 0; color: #fff; font: 820 clamp(15px, 1vw, 19px) var(--font-display); }
  .bpmr-nav { display: flex; gap: 6px; }
  .bpmr-arrow { display: grid; place-items: center; width: 30px; height: 30px; border: 1px solid rgba(255,255,255,.11); border-radius: 9px; color: var(--text-muted); background: rgba(255,255,255,.045); cursor: pointer; }
  .bpmr-arrow:hover { color: white; border-color: color-mix(in srgb, var(--accent) 55%, white 15%); }

  .bpmr-track { display: flex; gap: clamp(11px, .9vw, 17px); overflow-x: auto; padding: 8px 7px 12px; margin-inline: -7px; scroll-padding-inline: 18%; scrollbar-width: none; }
  .bpmr-track::-webkit-scrollbar { display: none; }

  .bpmr-card { flex: 0 0 clamp(112px, 8.2vw, 172px); display: flex; flex-direction: column; gap: 7px; padding: 0; border: 0; color: white; background: none; text-align: left; cursor: pointer; outline: none; transition: transform 180ms ease, opacity 180ms ease; }
  .bpmr-card:hover { transform: translateY(-3px); }
  .bpmr-cover { position: relative; width: 100%; aspect-ratio: 3 / 4; overflow: hidden; border: 1px solid rgba(255,255,255,.1); border-radius: clamp(12px, .9vw, 18px); background: rgba(255,255,255,.055); box-shadow: 0 18px 34px -24px #000; transition: border-color 180ms ease, box-shadow 180ms ease; }
  .bpmr-cover :global(.cached-image) { width: 100%; height: 100%; object-fit: cover; }
  .bpmr-placeholder { display: grid; place-items: center; width: 100%; height: 100%; color: rgba(255,255,255,.56); font: 850 clamp(27px, 2.2vw, 44px) var(--font-display); background: radial-gradient(circle at 25% 20%, color-mix(in srgb, var(--accent) 32%, transparent), transparent 60%), #141925; }
  .bpmr-progress-bar { position: absolute; left: 8px; right: 8px; bottom: 8px; height: 4px; overflow: hidden; border-radius: 999px; background: rgba(0,0,0,.54); }
  .bpmr-progress-fill { height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--accent), var(--accent-hi)); }
  .bpmr-type-badge { position: absolute; top: 7px; right: 7px; display: grid; place-items: center; width: 25px; height: 25px; border-radius: 9px; background: rgba(0,0,0,.52); font-size: 11px; backdrop-filter: blur(8px); }
  .bpmr-info { min-width: 0; padding: 0 3px; }
  .bpmr-name { display: block; overflow: hidden; color: #fff; font-size: clamp(10px, .72vw, 13px); font-weight: 780; text-overflow: ellipsis; white-space: nowrap; }
  .bpmr-progress-label { display: block; margin-top: 3px; color: var(--text-muted); font-size: 9px; }

  .bpmr-card:focus-visible .bpmr-cover,
  .bpmr-card.zone-focus .bpmr-cover { border-color: color-mix(in srgb, var(--accent) 70%, white 30%); box-shadow: 0 0 0 3px rgba(7,9,15,.92), 0 0 0 6px color-mix(in srgb, var(--accent) 78%, white 22%), 0 24px 50px -20px color-mix(in srgb, var(--accent) 64%, transparent); }
  .bpmr-card.zone-focus { transform: translateY(-5px) scale(1.02); }

  @media (max-height: 780px) {
    .bpmr-card { flex-basis: clamp(82px, 7.2vw, 104px); }
    .bpmr-section { margin-bottom: 14px; }
    .bpmr-track { padding-block: 5px 8px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .bpmr-card, .bpmr-cover { transition: none; }
  }
  :global([data-motion="reduce"]) .bpmr-card,
  :global([data-motion="reduce"]) .bpmr-cover { transition: none; }
</style>
