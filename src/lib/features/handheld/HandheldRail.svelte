<script lang="ts">
  import type { HandheldMediaKind } from "./mediaTypes";

  export interface HandheldRailItem {
    id: string;
    title: string;
    cover?: string | null;
    subtitle?: string;
    progress?: number;
    kind?: HandheldMediaKind | "game";
  }

  interface Props {
    title: string;
    kicker?: string;
    items: HandheldRailItem[];
    activeIndex?: number;
    onSelect?: (item: HandheldRailItem, index: number) => void;
    onOpen?: (item: HandheldRailItem, index: number) => void;
  }

  let { title, kicker = "CONTINUE", items, activeIndex = 0, onSelect, onOpen }: Props = $props();
</script>

{#if items.length > 0}
  <section class="hh-rail" aria-label={title} data-testid="handheld-rail">
    <header class="hh-rail__header"><span>{kicker}</span><h2>{title}</h2><small>{items.length} ITEMS · ← → 浏览</small></header>
    <div class="hh-rail__track" role="listbox" aria-label={title} tabindex="0">
      {#each items as item, index (item.id)}
        <button
          type="button"
          class:active={index === activeIndex}
          class="hh-rail__card"
          data-rail-index={index}
          role="option"
          aria-selected={index === activeIndex}
          onclick={() => onSelect?.(item, index)}
          ondblclick={() => onOpen?.(item, index)}
        >
          <span class="hh-rail__cover">
            {#if item.cover}<img src={item.cover} alt="" loading="lazy" />{:else}<span>{item.title.slice(0, 1)}</span>{/if}
            {#if item.progress != null}<span class="hh-rail__progress"><i style={`width:${Math.max(0, Math.min(1, item.progress)) * 100}%`}></i></span>{/if}
          </span>
          <strong>{item.title}</strong>
          {#if item.subtitle}<small>{item.subtitle}</small>{/if}
        </button>
      {/each}
    </div>
  </section>
{/if}

<style>
  .hh-rail { min-width: 0; }
  .hh-rail__header { display: flex; align-items: baseline; gap: 12px; padding: 0 2px 10px; border-bottom: 1px solid color-mix(in srgb, var(--border, #fff) 18%, transparent); }
  .hh-rail__header span { color: var(--accent, #e8557f); font: 700 9px/1 var(--font-mono, monospace); letter-spacing: .14em; }
  .hh-rail__header h2 { margin: 0; font: 800 1rem/1 var(--font-display, system-ui); }
  .hh-rail__header small { margin-left: auto; color: var(--text-muted, #89909d); font: 700 9px/1 var(--font-mono, monospace); letter-spacing: .08em; }
  .hh-rail__track { display: flex; gap: 12px; overflow-x: auto; padding: 12px 2px 8px; scrollbar-width: none; scroll-snap-type: x proximity; }
  .hh-rail__track::-webkit-scrollbar { display: none; }
  .hh-rail__card { flex: 0 0 clamp(104px, 13vw, 152px); display: grid; gap: 6px; min-width: 44px; padding: 0; border: 0; background: transparent; color: var(--text-secondary, #b7bbc5); text-align: left; cursor: pointer; scroll-snap-align: start; }
  .hh-rail__card.active { color: var(--text-primary, #fff); }
  .hh-rail__cover { position: relative; display: grid; place-items: center; aspect-ratio: 3 / 4; overflow: hidden; border: 2px solid transparent; background: linear-gradient(135deg, var(--bg-card, #1c202a), var(--bg-elev, #282e3a)); font: 800 2rem/1 var(--font-display, system-ui); color: var(--text-dim, #667080); }
  .hh-rail__card.active .hh-rail__cover { border-color: var(--accent, #e8557f); box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent, #e8557f) 18%, transparent); }
  .hh-rail__cover img { width: 100%; height: 100%; object-fit: cover; }
  .hh-rail__progress { position: absolute; right: 5px; bottom: 5px; left: 5px; height: 3px; background: rgb(0 0 0 / .55); }
  .hh-rail__progress i { display: block; height: 100%; background: var(--accent, #e8557f); }
  .hh-rail__card strong, .hh-rail__card small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hh-rail__card strong { font-size: .74rem; }
  .hh-rail__card small { color: var(--text-muted, #89909d); font-size: .64rem; }
  .hh-rail__card:focus-visible { outline: 2px solid var(--accent-hi, #ff769a); outline-offset: 3px; }
</style>
