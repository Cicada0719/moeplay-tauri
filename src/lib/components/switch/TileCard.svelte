<script lang="ts">
  import { onDestroy } from "svelte";
  import type { Game } from "../../stores/games.svelte";
  import { fileSrc } from "../../utils";
  import { coverOf } from "../../utils/game";
  import { MediaCard } from "../ui-v2";
  import Icon from "../Icon.svelte";

  let {
    game,
    selected = false,
    idle = false,
    disabled = false,
    loading = false,
    focusKey,
    tabIndex = -1,
    onpick,
    onlaunch,
    onfocus,
  }: {
    game: Game | null;
    selected?: boolean;
    idle?: boolean;
    disabled?: boolean;
    loading?: boolean;
    focusKey?: string;
    tabIndex?: number;
    onpick?: () => void;
    onlaunch?: () => void;
    onfocus?: () => void;
  } = $props();

  const isSentinel = $derived(game === null);
  const cover = $derived(coverOf(game));
  const imageSource = $derived(cover ? (fileSrc(cover) ?? cover) : undefined);
  const monogram = $derived((game?.name?.trim()?.[0] ?? "?").toUpperCase());
  const resolvedFocusKey = $derived(focusKey ?? (game ? `game-card-${game.id}` : "library-show-all"));
  let interactiveEl = $state<HTMLElement>();
  let clickTimer: number | undefined;

  $effect(() => {
    const node = interactiveEl;
    if (!node) return;
    node.dataset.focusKey = resolvedFocusKey;
    node.tabIndex = disabled || loading ? -1 : tabIndex;
    node.addEventListener("keydown", handleKeydown);
    node.addEventListener("dblclick", handleDoubleClick);
    const handleFocus = () => onfocus?.();
    node.addEventListener("focus", handleFocus);
    return () => {
      node.removeEventListener("keydown", handleKeydown);
      node.removeEventListener("dblclick", handleDoubleClick);
      node.removeEventListener("focus", handleFocus);
    };
  });

  function handleClick() {
    if (clickTimer) window.clearTimeout(clickTimer);
    clickTimer = window.setTimeout(() => {
      clickTimer = undefined;
      onpick?.();
    }, 180);
  }

  function handleDoubleClick(event: MouseEvent) {
    event.preventDefault();
    if (clickTimer) {
      window.clearTimeout(clickTimer);
      clickTimer = undefined;
    }
    if (!isSentinel) onlaunch?.();
    else onpick?.();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === " " && !isSentinel) onlaunch?.();
    else onpick?.();
  }

  onDestroy(() => {
    if (clickTimer) window.clearTimeout(clickTimer);
  });
</script>

<div
  class="tile-host"
  class:selected
  class:idle
  class:sentinel={isSentinel}
>
  {#snippet badges()}
    {#if isSentinel}
      <span class="all-badge"><Icon name="collection" size={28} /><small>全部游戏</small></span>
    {:else if !imageSource}
      <span class="tile-monogram" aria-hidden="true">{monogram}</span>
    {/if}
    {#if game?.favorite}<span class="favorite-badge"><Icon name="heartFill" size={13} /></span>{/if}
  {/snippet}

  <MediaCard
    title={game?.name ?? "全部游戏"}
    imageSrc={imageSource}
    imageAlt={imageSource ? (game?.name ?? "") : ""}
    onActivate={handleClick}
    badges={badges}
    variant="poster"
    {selected}
    {disabled}
    {loading}
    ariaLabel={game ? `打开 ${game.name}` : "查看全部游戏"}
    itemRole="none"
    class="tile-media-card"
    bind:interactiveRef={interactiveEl}
  />
</div>

<style>
  .tile-host {
    flex: 0 0 auto;
    width: var(--sw-tile-width);
    border-radius: var(--sw-tile-radius);
    transition: filter .24s ease, width .22s ease, transform .22s ease;
    will-change: transform;
  }
  .tile-host.idle { filter: brightness(var(--sw-tile-idle-bright)); }
  .tile-host.selected { width: var(--sw-tile-selected-width); transform: translateY(-4px) scale(1.02); z-index: 3; filter: none; }

  :global(.tile-media-card.v2-media-card) {
    border: 0;
    border-radius: var(--sw-tile-radius);
    background: var(--bg-elev);
    box-shadow: var(--shadow-tile);
  }
  :global(.tile-media-card.v2-media-card.is-selected) { box-shadow: var(--ring-switch), var(--shadow-lift); }
  :global(.tile-media-card .v2-media-card__media) { aspect-ratio: 3 / 4; }
  :global(.tile-media-card .v2-media-card__media img) { object-fit: cover; }
  :global(.tile-media-card .v2-media-card__copy) {
    position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0,0,0,0); padding: 0; margin: -1px;
  }
  :global(.tile-media-card .v2-media-card__primary:focus-visible) { box-shadow: inset var(--ring-switch); }

  .all-badge,
  .tile-monogram { position: absolute; inset: 0; display: grid; place-items: center; }
  .all-badge { align-content: center; gap: .55rem; color: var(--text-secondary); background: linear-gradient(145deg, rgba(0,255,153,.13), rgba(0,90,70,.2)); }
  .all-badge small { font: 700 .75rem/1 var(--font-ui); }
  .tile-monogram { font: 700 2.4rem/1 var(--font-display); color: var(--text-muted); background: radial-gradient(circle at 50% 20%, rgba(0,255,153,.15), transparent 65%); }
  .favorite-badge { position: absolute; top: .55rem; right: .55rem; display: grid; place-items: center; width: 1.65rem; height: 1.65rem; border-radius: 999px; color: var(--accent-pink); background: rgba(8,11,18,.7); }

  @media (max-width: 760px) {
    .tile-host { width: calc(var(--sw-tile-width) * .86); }
    .tile-host.selected { width: calc(var(--sw-tile-selected-width) * .86); }
  }

  @media (prefers-reduced-motion: reduce) {
    .tile-host { transition: none; }
  }
</style>
