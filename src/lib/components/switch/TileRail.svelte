<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import type { Game } from "../../stores/games.svelte";
  import TileCard from "./TileCard.svelte";
  import { attachGamepad } from "./useGamepad.svelte";

  let { items, selectedId, onselect, onactivate, onlaunch, onfavorite, onshowall, onfocussearch, onback, onbigpicture }: {
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
  // rail 内部 focusIndex 为导航真相源；selectedId 仅用于 onMount 初始定位。
  let focusIndex = $state(0);
  let syncedSelectedId = $state<string | null>(null);
  const sentinelIndex = $derived(items.length);

  function prefersReducedMotion(): boolean {
    return typeof window !== "undefined"
      && window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
  }

  $effect(() => {
    if (selectedId === syncedSelectedId) return;
    syncedSelectedId = selectedId;
    const i = items.findIndex((g) => g.id === selectedId);
    if (i >= 0 && i !== focusIndex) focusIndex = i;
  });

  $effect(() => {
    if (focusIndex > sentinelIndex) focusIndex = sentinelIndex;
  });

  // focusIndex 变化 → 同步选中（信息区）+ 居中滚动
  $effect(() => {
    const idx = focusIndex;
    if (idx >= 0 && idx < items.length) onselect(items[idx].id);
    queueMicrotask(() => {
      const node = scroller?.querySelector<HTMLElement>(`[data-idx="${idx}"]`);
      node?.scrollIntoView({
        inline: "center",
        block: "nearest",
        behavior: prefersReducedMotion() ? "auto" : "smooth",
      });
    });
  });

  function move(d: number) {
    focusIndex = Math.max(0, Math.min(sentinelIndex, focusIndex + d));
  }
  function activateCurrent() {
    if (focusIndex === sentinelIndex) onshowall();
    else onactivate(items[focusIndex].id);
  }
  function launchCurrent() {
    if (focusIndex < items.length) onlaunch(items[focusIndex].id);
  }

  function onKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case "ArrowRight": move(1); e.preventDefault(); break;
      case "d": case "D": move(1); e.preventDefault(); break;
      case "ArrowLeft": move(-1); e.preventDefault(); break;
      case "a": case "A": move(-1); e.preventDefault(); break;
      case "PageDown": move(6); e.preventDefault(); break;
      case "PageUp": move(-6); e.preventDefault(); break;
      case "Home": focusIndex = 0; e.preventDefault(); break;
      case "End": focusIndex = sentinelIndex; e.preventDefault(); break;
      case "Enter": activateCurrent(); e.preventDefault(); break;
      case " ": launchCurrent(); e.preventDefault(); break;
      case "/": onfocussearch?.(); e.preventDefault(); break;
      case "Escape": onback?.(); e.preventDefault(); break;
      case "f": case "F": onfavorite?.(); break;
    }
    if (e.ctrlKey && e.key.toLowerCase() === "b") {
      onbigpicture?.();
      e.preventDefault();
    }
  }

  onMount(() => {
    const i = items.findIndex((g) => g.id === selectedId);
    if (i >= 0) focusIndex = i;
    scroller?.focus({ preventScroll: true });
    let ctx: gsap.Context | null = null;
    if (scroller && !prefersReducedMotion()) {
      ctx = gsap.context(() => {
        gsap.from(".slot", {
          autoAlpha: 0,
          y: 14,
          duration: 0.34,
          ease: "power3.out",
          stagger: 0.03,
        });
      }, scroller);
    }
    const detachGamepad = attachGamepad({
      left: () => move(-1),
      right: () => move(1),
      pageLeft: () => move(-6),
      pageRight: () => move(6),
      activate: () => activateCurrent(),
      launch: () => launchCurrent(),
      favorite: () => onfavorite?.(),
      back: () => onback?.(),
    });
    return () => {
      detachGamepad();
      ctx?.revert();
    };
  });
</script>

<div
  class="rail"
  bind:this={scroller}
  tabindex="0"
  role="listbox"
  aria-label="游戏库"
  onkeydown={onKeydown}
>
  {#each items as game, i (game.id)}
    <div class="slot" data-idx={i} role="option" aria-selected={i === focusIndex}>
      <TileCard
        {game}
        selected={i === focusIndex}
        idle={i !== focusIndex}
        onpick={() => { focusIndex = i; onactivate(game.id); }}
        onlaunch={() => onlaunch(game.id)}
      />
    </div>
  {/each}

  <div class="slot" data-idx={sentinelIndex} role="option" aria-selected={sentinelIndex === focusIndex}>
    <TileCard
      game={null}
      selected={sentinelIndex === focusIndex}
      idle={sentinelIndex !== focusIndex}
      onpick={() => { focusIndex = sentinelIndex; onshowall(); }}
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
  .rail:focus-visible {
    box-shadow: inset var(--focus-ring);
  }

  @media (max-width: 760px) {
    .rail {
      gap: 12px;
      padding: 34px 18px 42px;
      scroll-padding-inline: 18px;
      min-height: calc(var(--sw-tile-selected-width) * 1.42);
    }
  }
</style>
