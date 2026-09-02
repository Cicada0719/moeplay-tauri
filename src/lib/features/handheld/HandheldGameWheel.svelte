<script lang="ts">
  import { onMount } from "svelte";
  import type { Game } from "../../stores/games.svelte";
  import { coverOf, isInstalled } from "../../utils/game";
  import { fileSrc } from "../../utils";
  import Icon from "../../components/Icon.svelte";

  interface Props {
    games: Game[];
    focusIdx?: number;
    platformLabel: string;
    active?: boolean;
    launching?: string | null;
    onSelect: (index: number) => void;
    onActivate: (index: number) => void;
    onFavorite?: (index: number) => void;
    onOpenImport?: () => void;
    onOpenLibrary?: () => void;
  }

  let {
    games,
    focusIdx = 0,
    platformLabel,
    active = true,
    launching = null,
    onSelect,
    onActivate,
    onFavorite,
    onOpenImport,
    onOpenLibrary,
  }: Props = $props();

  let wheelEl = $state<HTMLElement>();
  let pointerStartX = $state<number | null>(null);
  let swiping = $state(false);

  const REEL_RADIUS = 3;
  const visibleGames = $derived(
    games
      .slice(Math.max(0, focusIdx - REEL_RADIUS), Math.min(games.length, focusIdx + REEL_RADIUS + 1))
      .map((game, localIndex) => ({ game, index: localIndex + Math.max(0, focusIdx - REEL_RADIUS) })),
  );

  const currentGame = $derived(games[focusIdx] ?? null);

  function move(delta: number) {
    if (games.length === 0) return;
    onSelect(Math.max(0, Math.min(games.length - 1, focusIdx + delta)));
  }

  function monogram(game: Game): string {
    return (game.name?.trim()?.[0] ?? "?").toUpperCase();
  }

  function focusCurrent() {
    if (!active) return;
    requestAnimationFrame(() => {
      wheelEl?.querySelector<HTMLElement>(`[data-game-index="${focusIdx}"]`)?.focus({ preventScroll: true });
    });
  }

  function onCardKeydown(event: KeyboardEvent, index: number) {
    switch (event.key) {
      case "ArrowLeft":
      case "ArrowUp":
        event.preventDefault();
        move(-1);
        break;
      case "ArrowRight":
      case "ArrowDown":
        event.preventDefault();
        move(1);
        break;
      case "Home":
        event.preventDefault();
        onSelect(0);
        break;
      case "End":
        event.preventDefault();
        onSelect(Math.max(0, games.length - 1));
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        onActivate(index);
        break;
      case "Escape":
        event.preventDefault();
        break;
    }
  }

  function onPointerDown(event: PointerEvent) {
    pointerStartX = event.clientX;
    swiping = false;
  }

  function onPointerUp(event: PointerEvent) {
    if (pointerStartX == null) return;
    const delta = event.clientX - pointerStartX;
    if (Math.abs(delta) >= 24) {
      swiping = true;
      move(delta < 0 ? 1 : -1);
    }
    pointerStartX = null;
  }

  function onCardClick(index: number) {
    if (swiping) {
      swiping = false;
      return;
    }
    if (index === focusIdx) onActivate(index);
    else onSelect(index);
  }

  $effect(() => {
    focusIdx;
    active;
    focusCurrent();
  });

  onMount(() => () => { pointerStartX = null; });
</script>

<section class="hh-game-wheel" bind:this={wheelEl} data-testid="handheld-game-wheel" aria-label={`${platformLabel}游戏转盘`}>
  <header class="hh-game-wheel__head">
    <div>
      <span class="hh-game-wheel__kicker">BIG SCREEN / GAME SELECT</span>
      <h1>{currentGame?.name ?? "游戏库"}</h1>
    </div>
    <div class="hh-game-wheel__stats">
      <strong>{games.length ? String(focusIdx + 1).padStart(2, "0") : "--"}</strong>
      <span>/ {String(games.length).padStart(2, "0")}</span>
      <small>{platformLabel}</small>
    </div>
  </header>

  {#if games.length > 0}
    <div
      class="hh-game-wheel__reel"
      role="listbox"
      tabindex="-1"
      aria-label="游戏列表"
      onpointerdown={onPointerDown}
      onpointerup={onPointerUp}
    >
      <div class="hh-game-wheel__line" aria-hidden="true"></div>
      {#each visibleGames as item (item.game.id)}
        {@const offset = item.index - focusIdx}
        {@const absoluteOffset = Math.abs(offset)}
        <button
          type="button"
          class="hh-game-wheel__card"
          class:focused={item.index === focusIdx}
          style={`--offset:${offset};--abs:${absoluteOffset}`}
          data-game-index={item.index}
          role="option"
          aria-selected={item.index === focusIdx}
          tabindex={active && item.index === focusIdx ? 0 : -1}
          aria-label={item.game.name}
          onclick={() => onCardClick(item.index)}
          onfocus={() => onSelect(item.index)}
          onkeydown={(event) => onCardKeydown(event, item.index)}
        >
          <span class="hh-game-wheel__poster">
            {#if fileSrc(coverOf(item.game))}
              <img src={fileSrc(coverOf(item.game))!} alt="" draggable="false" loading={absoluteOffset < 2 ? "eager" : "lazy"} />
            {:else}
              <span class="hh-game-wheel__monogram">{monogram(item.game)}</span>
            {/if}
            <span class="hh-game-wheel__wash"></span>
            <span class="hh-game-wheel__number">{String(item.index + 1).padStart(2, "0")}</span>
            {#if isInstalled(item.game)}<span class="hh-game-wheel__ready"><i></i>READY</span>{/if}
            {#if item.game.favorite}<span class="hh-game-wheel__favorite"><Icon name="heartFill" size={12} /></span>{/if}
          </span>
          <strong>{item.game.name}</strong>
        </button>
      {/each}
    </div>
  {:else}
    <div class="hh-game-wheel__empty">
      <span><Icon name="gamepad" size={28} /></span>
      <div><strong>这个游戏平台还没有游戏</strong><small>导入 ROM，或打开完整游戏库补充你的掌机档案。</small></div>
      <div class="hh-game-wheel__empty-actions">
        {#if onOpenImport}<button type="button" onclick={onOpenImport}><Icon name="database" size={15} />模拟器导入</button>{/if}
        {#if onOpenLibrary}<button type="button" onclick={onOpenLibrary}>打开游戏库</button>{/if}
      </div>
    </div>
  {/if}

  <footer class="hh-game-wheel__foot">
    <span><b>LS / ← →</b>选择游戏</span>
    <span><b>A</b>{launching === currentGame?.id ? "启动中" : "启动"}</span>
    <span><b>Y</b>详情</span>
    {#if onFavorite}<span><b>X</b>收藏</span>{/if}
  </footer>
</section>

<style>
  .hh-game-wheel {
    --wheel-accent: var(--accent-hi, #ff769a);
    position: relative;
    display: flex;
    min-height: 0;
    height: 100%;
    flex-direction: column;
    overflow: hidden;
    border-top: 1px solid color-mix(in srgb, var(--text-primary, #fff) 16%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--text-primary, #fff) 16%, transparent);
    background: linear-gradient(110deg, rgb(5 7 12 / .78), color-mix(in srgb, var(--accent, #e8557f) 8%, transparent) 58%, rgb(6 8 13 / .48));
  }
  .hh-game-wheel__head { z-index: 5; display: flex; align-items: end; justify-content: space-between; gap: 12px; padding: 8px 12px 5px; }
  .hh-game-wheel__kicker { color: var(--wheel-accent); font: 800 8px/1 var(--font-mono, monospace); letter-spacing: .16em; }
  .hh-game-wheel__head h1 { max-width: min(48vw, 440px); margin: 5px 0 0; overflow: hidden; color: var(--text-primary, #fff); font: 850 clamp(1.05rem, 2.3vw, 1.8rem)/1 var(--font-display, system-ui); letter-spacing: -.045em; text-overflow: ellipsis; white-space: nowrap; }
  .hh-game-wheel__stats { display: grid; grid-template-columns: auto auto; align-items: baseline; gap: 0 4px; color: var(--text-muted, #89909d); font-family: var(--font-mono, monospace); text-align: right; }
  .hh-game-wheel__stats strong { color: var(--text-primary, #fff); font-size: 1.25rem; }
  .hh-game-wheel__stats span { font-size: .65rem; }
  .hh-game-wheel__stats small { grid-column: 1 / -1; color: var(--wheel-accent); font-size: .55rem; letter-spacing: .1em; }
  .hh-game-wheel__reel { position: relative; flex: 1; min-height: 0; overflow: hidden; perspective: 900px; touch-action: pan-y; }
  .hh-game-wheel__line { position: absolute; right: 4%; bottom: 18px; left: 4%; height: 1px; background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--wheel-accent) 80%, white) 50%, transparent); box-shadow: 0 0 22px color-mix(in srgb, var(--wheel-accent) 34%, transparent); }
  .hh-game-wheel__card { --offset: 0; --abs: 0; position: absolute; bottom: 24px; left: 50%; z-index: calc(20 - var(--abs)); width: clamp(102px, 13vw, 162px); aspect-ratio: 2 / 3; padding: 0; border: 0; background: transparent; color: var(--text-secondary, #b7bbc5); cursor: pointer; opacity: calc(1 - var(--abs) * .18); transform: translateX(calc(-50% + var(--offset) * clamp(76px, 9.6vw, 124px))) translateY(calc(var(--abs) * 10px)) rotateY(calc(var(--offset) * -7deg)) rotateZ(calc(var(--offset) * 1.1deg)) scale(calc(1 - var(--abs) * .12)); transform-origin: center bottom; filter: saturate(calc(1 - var(--abs) * .2)) brightness(calc(1 - var(--abs) * .12)); transition: transform 220ms ease, opacity 180ms ease, filter 180ms ease; }
  .hh-game-wheel__card.focused { z-index: 30; color: var(--text-primary, #fff); opacity: 1; filter: saturate(1.05) brightness(1); transform: translateX(-50%) translateY(-7px) rotateY(0) rotateZ(0) scale(1); }
  .hh-game-wheel__card:focus-visible { outline: none; }
  .hh-game-wheel__poster { position: absolute; inset: 0; display: block; overflow: hidden; border: 1px solid rgb(255 255 255 / .18); background: linear-gradient(145deg, color-mix(in srgb, var(--wheel-accent) 34%, #161820), #080910 72%); box-shadow: 0 20px 42px -22px rgb(0 0 0 / .95); }
  .hh-game-wheel__card.focused .hh-game-wheel__poster { border-color: color-mix(in srgb, var(--wheel-accent) 72%, white 20%); box-shadow: 0 0 0 2px rgb(4 6 10 / .82), 0 0 0 4px color-mix(in srgb, var(--wheel-accent) 80%, white 18%), 0 26px 58px -20px rgb(0 0 0 / .98), 0 0 44px -24px var(--wheel-accent); }
  .hh-game-wheel__poster img { display: block; width: 100%; height: 100%; object-fit: cover; }
  .hh-game-wheel__wash { position: absolute; inset: 0; background: linear-gradient(180deg, transparent 48%, rgb(4 5 9 / .83)); }
  .hh-game-wheel__number { position: absolute; top: 7px; left: 8px; color: #fff; font: 850 9px/1 var(--font-mono, monospace); text-shadow: 0 2px 9px rgb(0 0 0 / .7); }
  .hh-game-wheel__ready { position: absolute; bottom: 7px; left: 8px; display: flex; align-items: center; gap: 4px; color: #b4f4cf; font: 800 7px/1 var(--font-mono, monospace); letter-spacing: .12em; }
  .hh-game-wheel__ready i { width: 4px; height: 4px; border-radius: 50%; background: #74e5ad; box-shadow: 0 0 8px #74e5ad; }
  .hh-game-wheel__favorite { position: absolute; top: 6px; right: 6px; display: grid; width: 21px; height: 21px; place-items: center; border-radius: 50%; background: var(--accent, #e8557f); color: #fff; }
  .hh-game-wheel__monogram { position: absolute; inset: 0; display: grid; place-items: center; color: rgb(255 255 255 / .75); font: 850 4rem/1 var(--font-display, system-ui); }
  .hh-game-wheel__card > strong { position: absolute; right: -16%; bottom: -19px; left: -16%; overflow: hidden; color: inherit; font: 750 9px/1 var(--font-ui, system-ui); text-align: center; text-overflow: ellipsis; white-space: nowrap; }
  .hh-game-wheel__empty { display: flex; flex: 1; min-height: 0; align-items: center; justify-content: center; gap: 14px; padding: 20px; }
  .hh-game-wheel__empty > span { display: grid; width: 54px; height: 54px; place-items: center; border: 1px solid color-mix(in srgb, var(--wheel-accent) 55%, transparent); border-radius: 50%; color: var(--wheel-accent); }
  .hh-game-wheel__empty > div:not(.hh-game-wheel__empty-actions) { display: grid; gap: 5px; min-width: 0; }
  .hh-game-wheel__empty strong { color: var(--text-primary, #fff); font-size: .88rem; }
  .hh-game-wheel__empty small { color: var(--text-muted, #89909d); font-size: .66rem; }
  .hh-game-wheel__empty-actions { display: flex; flex-wrap: wrap; gap: 6px; }
  .hh-game-wheel__empty-actions button { display: inline-flex; align-items: center; gap: 5px; min-height: 36px; padding: 0 9px; border: 1px solid color-mix(in srgb, var(--wheel-accent) 42%, transparent); background: color-mix(in srgb, var(--wheel-accent) 15%, transparent); color: var(--text-primary, #fff); cursor: pointer; }
  .hh-game-wheel__foot { display: flex; justify-content: flex-end; gap: 12px; padding: 5px 12px 6px; color: var(--text-muted, #89909d); font: 650 8px/1 var(--font-mono, monospace); }
  .hh-game-wheel__foot b { margin-right: 4px; color: var(--wheel-accent); }
  @media (max-width: 900px), (max-height: 620px) {
    .hh-game-wheel__head { padding: 6px 9px 3px; }
    .hh-game-wheel__head h1 { font-size: 1rem; }
    .hh-game-wheel__card { width: clamp(96px, 14.5vw, 150px); transform: translateX(calc(-50% + var(--offset) * clamp(72px, 10vw, 116px))) translateY(calc(var(--abs) * 7px)) rotateY(calc(var(--offset) * -6deg)) scale(calc(1 - var(--abs) * .11)); }
    .hh-game-wheel__card.focused { transform: translateX(-50%) translateY(-5px) scale(1); }
    .hh-game-wheel__foot { gap: 7px; padding-inline: 9px; font-size: 7px; }
  }
  @media (prefers-reduced-motion: reduce) { .hh-game-wheel__card { transition: none; } }
  :global(:root[data-motion="reduce"]) .hh-game-wheel__card { transition: none; }
</style>
