<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import type { Game } from "../../stores/games.svelte";
  import TileCard from "./TileCard.svelte";

  let {
    items,
    selectedId,
    onselect,
    onactivate,
    onlaunch,
    onfavorite,
    onshowall,
    onfocussearch,
    onback,
    onbigpicture,
  }: {
    items: Game[];
    selectedId: string | null;
    onselect: (id: string) => void;
    onactivate: (id: string) => void;
    onlaunch: (id: string) => void;
    onfavorite?: () => void;
    onshowall: () => void;
    onfocussearch?: () => void;
    onback?: () => void;
    onbigpicture?: () => void;
  } = $props();

  let scroller = $state<HTMLDivElement>();
  let focusIndex = $state(0);
  let syncedSelectedId = $state<string | null>(null);
  const sentinelIndex = $derived(items.length);
  const railRadius = 6;
  const railRange = $derived({
    start: Math.max(0, focusIndex - railRadius),
    end: Math.min(items.length - 1, focusIndex + railRadius),
  });
  const visibleItems = $derived(
    items.slice(railRange.start, railRange.end + 1).map((game, visibleIndex) => ({
      game,
      originalIndex: visibleIndex + railRange.start,
    })),
  );

  function prefersReducedMotion(): boolean {
    return typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
  }

  function focusCard(index = focusIndex) {
    queueMicrotask(() => {
      const target = scroller?.querySelector<HTMLElement>(`[data-idx="${index}"] [data-focus-key]`);
      target?.focus({ preventScroll: true });
    });
  }

  function syncIndex(index: number, moveFocus = false) {
    focusIndex = Math.max(0, Math.min(sentinelIndex, index));
    if (moveFocus) focusCard(focusIndex);
  }

  $effect(() => {
    if (selectedId === syncedSelectedId) return;
    syncedSelectedId = selectedId;
    const index = items.findIndex((game) => game.id === selectedId);
    if (index >= 0 && index !== focusIndex) focusIndex = index;
  });

  $effect(() => {
    if (focusIndex > sentinelIndex) focusIndex = sentinelIndex;
  });

  $effect(() => {
    const index = focusIndex;
    if (index >= 0 && index < items.length) onselect(items[index].id);
    queueMicrotask(() => {
      const node = scroller?.querySelector<HTMLElement>(`[data-idx="${index}"]`);
      node?.scrollIntoView({
        inline: "center",
        block: "nearest",
        behavior: prefersReducedMotion() ? "auto" : "smooth",
      });
    });
  });

  function move(delta: number) {
    syncIndex(focusIndex + delta, true);
  }

  function handleWheel(event: WheelEvent) {
    if (Math.abs(event.deltaY) < 1 && Math.abs(event.deltaX) < 1) return;
    event.preventDefault();
    move(event.deltaY > 0 || event.deltaX > 0 ? 1 : -1);
  }

  function handleKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowRight":
      case "d":
      case "D":
        move(1); event.preventDefault(); break;
      case "ArrowLeft":
      case "a":
      case "A":
        move(-1); event.preventDefault(); break;
      case "PageDown":
        syncIndex(focusIndex + 6, true); event.preventDefault(); break;
      case "PageUp":
        syncIndex(focusIndex - 6, true); event.preventDefault(); break;
      case "Home":
        syncIndex(0, true); event.preventDefault(); break;
      case "End":
        syncIndex(sentinelIndex, true); event.preventDefault(); break;
      case "/":
        onfocussearch?.(); event.preventDefault(); break;
      case "Escape":
        onback?.(); event.preventDefault(); break;
      case "f":
      case "F":
        onfavorite?.(); break;
    }
    if (event.ctrlKey && event.key.toLowerCase() === "b") {
      onbigpicture?.();
      event.preventDefault();
    }
  }

  function railInteraction(node: HTMLElement) {
    const wheel = (event: WheelEvent) => handleWheel(event);
    const keydown = (event: KeyboardEvent) => handleKeydown(event);
    node.addEventListener("wheel", wheel, { passive: false });
    node.addEventListener("keydown", keydown);
    return {
      destroy() {
        node.removeEventListener("wheel", wheel);
        node.removeEventListener("keydown", keydown);
      },
    };
  }

  onMount(() => {
    const index = items.findIndex((game) => game.id === selectedId);
    if (index >= 0) focusIndex = index;

    let context: gsap.Context | null = null;
    if (scroller && !prefersReducedMotion()) {
      context = gsap.context(() => {
        gsap.from(".slot", {
          autoAlpha: 0,
          y: 14,
          duration: 0.34,
          ease: "power3.out",
          stagger: 0.03,
        });
      }, scroller);
    }

    return () => {
      context?.revert();
    };
  });
</script>

<div
  class="rail"
  bind:this={scroller}
  role="list"
  aria-label="最近游戏"
  use:railInteraction
>
  {#if railRange.start > 0}<div class="rail-spacer" aria-hidden="true" style={`width:${railRange.start * 240}px`}></div>{/if}

  {#each visibleItems as { game, originalIndex } (game.id)}
    <div class="slot" data-idx={originalIndex} role="listitem">
      <TileCard
        {game}
        selected={originalIndex === focusIndex}
        idle={originalIndex !== focusIndex}
        tabIndex={originalIndex === focusIndex ? 0 : -1}
        focusKey={`game-card-${game.id}`}
        onfocus={() => syncIndex(originalIndex)}
        onpick={() => { syncIndex(originalIndex); onactivate(game.id); }}
        onlaunch={() => onlaunch(game.id)}
      />
    </div>
  {/each}

  {#if railRange.end < items.length - 1}<div class="rail-spacer" aria-hidden="true" style={`width:${(items.length - 1 - railRange.end) * 240}px`}></div>{/if}

  <div class="slot" data-idx={sentinelIndex} role="listitem">
    <TileCard
      game={null}
      selected={sentinelIndex === focusIndex}
      idle={sentinelIndex !== focusIndex}
      tabIndex={sentinelIndex === focusIndex ? 0 : -1}
      focusKey="library-show-all"
      onfocus={() => syncIndex(sentinelIndex)}
      onpick={() => { syncIndex(sentinelIndex); onshowall(); }}
    />
  </div>
</div>

<style>
  .rail {
    display: flex;
    gap: 18px;
    align-items: center;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 48px 8vw 56px;
    scroll-padding-inline: 8vw;
    outline: none;
    scrollbar-width: none;
    min-height: calc(var(--sw-tile-selected-width) * 1.35);
  }
  .rail::-webkit-scrollbar { display: none; }
  .slot { flex: 0 0 auto; }
  .rail-spacer { flex: 0 0 auto; pointer-events: none; }

  @media (max-width: 760px) {
    .rail {
      gap: 12px;
      padding: 34px 18px 42px;
      scroll-padding-inline: 18px;
      min-height: calc(var(--sw-tile-selected-width) * 1.42);
    }
  }
</style>
