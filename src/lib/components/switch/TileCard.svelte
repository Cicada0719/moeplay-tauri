<script lang="ts">
  import { onDestroy } from "svelte";
  import { gsap } from "gsap";
  import type { Game } from "../../stores/games.svelte";
  import { coverOf } from "../../utils/game";
  import CachedImage from "../CachedImage.svelte";
  import Icon from "../Icon.svelte";

  // game === null 表示末尾“全部游戏”哨兵磁贴
  let { game, selected = false, idle = false, onpick, onlaunch }: {
    game: Game | null;
    selected?: boolean;
    idle?: boolean;
    onpick?: () => void;
    onlaunch?: () => void;
  } = $props();

  const isSentinel = $derived(game === null);
  const cover = $derived(coverOf(game));
  const monogram = $derived((game?.name?.trim()?.[0] ?? "?").toUpperCase());
  let tileEl = $state<HTMLButtonElement>();
  let clickTimer: number | undefined;
  // 封面加载失败（路径存在但图裂）时退回首字母占位，避免整块黑卡
  let coverFailed = $state(false);
  // game 变了要重置失败态（TileCard 按 id keyed，正常每番一份实例，这里防御性兜底）
  $effect(() => { void game?.id; coverFailed = false; });

  function handleClick() {
    if (clickTimer) window.clearTimeout(clickTimer);
    clickTimer = window.setTimeout(() => {
      clickTimer = undefined;
      onpick?.();
    }, 180);
  }

  function handleDoubleClick() {
    if (clickTimer) {
      window.clearTimeout(clickTimer);
      clickTimer = undefined;
    }
    onlaunch?.();
  }

  onDestroy(() => {
    if (clickTimer) window.clearTimeout(clickTimer);
  });

  $effect(() => {
    const node = tileEl;
    if (!node) return;
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    if (reduce) return;
    const ctx = gsap.context(() => {
      gsap.to(node, {
        scale: selected ? 1.04 : 1,
        y: selected ? -4 : 0,
        duration: 0.22,
        ease: "power2.out",
        overwrite: "auto",
      });
    }, node);
    return () => ctx.revert();
  });
</script>

<button
  bind:this={tileEl}
  class="tile"
  class:selected
  class:idle
  class:sentinel={isSentinel}
  title={game?.name ?? "全部游戏"}
  onclick={handleClick}
  ondblclick={handleDoubleClick}
>
  <span class="art">
    {#if isSentinel}
      <span class="all">
        <Icon name="collection" size={26} />
        <small>全部游戏</small>
      </span>
    {:else if cover && !coverFailed}
      <CachedImage
        source={cover}
        cacheKey={`sw-tile-${game!.id}`}
        alt={game!.name}
        loading="lazy"
        onfail={() => (coverFailed = true)}
      />
    {:else}
      <span class="mono">{monogram}</span>
    {/if}

    {#if game?.favorite}
      <span class="fav"><Icon name="heartFill" size={13} /></span>
    {/if}
  </span>
</button>

<style>
  .tile {
    flex: 0 0 auto;
    width: var(--sw-tile-width);
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: var(--sw-tile-radius);
    transition: filter 0.24s ease, width 0.22s ease;
    will-change: transform;
  }
  .art {
    position: relative;
    display: block;
    width: 100%;
    aspect-ratio: 3 / 4;
    border-radius: var(--sw-tile-radius);
    overflow: hidden;
    background: var(--bg-elev);
    box-shadow: var(--shadow-tile);
  }
  .art :global(.cached-image) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .tile.idle { filter: brightness(var(--sw-tile-idle-bright)); }
  .tile.selected {
    width: var(--sw-tile-selected-width);
    filter: none;
    z-index: 3;
  }
  .tile.selected .art { box-shadow: var(--ring-switch), var(--shadow-lift); }
  .tile:focus-visible { outline: none; }
  .tile:focus-visible .art { box-shadow: var(--ring-switch), var(--shadow-lift); }

  .mono {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    font-family: var(--font-display);
    font-size: 38px;
    font-weight: 700;
    color: var(--text-muted);
    background: linear-gradient(135deg, rgba(232, 85, 127, 0.18), rgba(110, 120, 160, 0.14));
  }
  .all {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    align-content: center;
    gap: 8px;
    color: var(--text-secondary);
    background: var(--bg-card);
    border: 1px dashed var(--border-hover);
    border-radius: var(--sw-tile-radius);
  }
  .all small { font-size: 12px; }

  .fav {
    position: absolute;
    top: 8px;
    right: 8px;
    color: var(--accent);
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.6));
  }
</style>
