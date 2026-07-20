<script lang="ts">
  import { onMount } from "svelte";
  import { coverOf as gameCoverOf, isInstalled } from "../../utils/game";
  import type { Game } from "../../stores/games.svelte";
  import { fileSrc } from "../../utils";
  import Icon from "../Icon.svelte";
  import { attachGamepad, type GamepadAttachment } from "../switch/useGamepad.svelte";

  let {
    games,
    focusIdx,
    filterAll,
    prefersReducedMotion,
    active = false,
    onSelect,
    onActivate,
    onLaunch,
    onFavorite,
    onMoveToHero,
    onMoveToTop,
    onBack,
    onTabPrevious,
    onTabNext,
    onToggleFilter,
    onOpenImport,
  }: {
    games: Game[];
    focusIdx: number;
    filterAll: boolean;
    prefersReducedMotion: boolean;
    active?: boolean;
    onSelect: (idx: number) => void;
    onActivate: (idx: number) => void;
    onLaunch: (idx: number) => void;
    onFavorite: (idx: number) => void;
    onMoveToHero: () => void;
    onMoveToTop: () => void;
    onBack: () => void;
    onTabPrevious: () => void;
    onTabNext: () => void;
    onToggleFilter: () => void;
    onOpenImport: () => void;
  } = $props();

  let reelEl = $state<HTMLDivElement>();
  let scope: GamepadAttachment | null = null;

  const REEL_RADIUS = 3;
  const reelRange = $derived({
    start: Math.max(0, focusIdx - REEL_RADIUS),
    end: Math.min(games.length - 1, focusIdx + REEL_RADIUS),
  });
  const visibleGames = $derived(
    games.slice(reelRange.start, reelRange.end + 1).map((game, localIndex) => ({
      game,
      originalIndex: localIndex + reelRange.start,
    })),
  );

  function move(delta: number) {
    if (games.length === 0) return;
    onSelect(Math.max(0, Math.min(games.length - 1, focusIdx + delta)));
  }

  function focusSelected() {
    if (!active) return;
    queueMicrotask(() => reelEl?.querySelector<HTMLElement>(`[data-idx="${focusIdx}"]`)?.focus({ preventScroll: true }));
  }

  function onCardKeydown(event: KeyboardEvent, index: number) {
    switch (event.key) {
      case "ArrowLeft":
      case "ArrowDown": event.preventDefault(); move(-1); break;
      case "ArrowRight": event.preventDefault(); move(1); break;
      case "ArrowUp": event.preventDefault(); onMoveToHero(); break;
      case "PageUp": event.preventDefault(); move(-5); break;
      case "PageDown": event.preventDefault(); move(5); break;
      case "Home": event.preventDefault(); onSelect(0); break;
      case "End": event.preventDefault(); onSelect(games.length - 1); break;
      case "Enter": event.preventDefault(); onActivate(index); break;
      case " ": event.preventDefault(); onLaunch(index); break;
      case "Escape": event.preventDefault(); onBack(); break;
    }
  }

  $effect(() => { focusIdx; active; focusSelected(); });

  onMount(() => {
    scope = attachGamepad({
      left: () => move(-1),
      right: () => move(1),
      down: () => move(1),
      up: () => onMoveToHero(),
      launch: () => onLaunch(focusIdx),
      activate: () => onActivate(focusIdx),
      favorite: () => onFavorite(focusIdx),
      filter: () => onToggleFilter(),
      back: () => onBack(),
      pageLeft: () => onTabPrevious(),
      pageRight: () => onTabNext(),
    }, { id: "big-picture-wheel", zone: "wheel", priority: 20 });
    return () => { scope?.(); scope = null; };
  });

  const monogram = (game: Game) => (game.name?.trim()?.[0] ?? "?").toUpperCase();
  const numberLabel = (index: number) => String(index + 1).padStart(2, "0");
</script>

<aside class="bp-reel" data-focus-zone="wheel" data-active={active ? "true" : "false"} aria-label="滚动游戏舞台" style={`--progress:${games.length ? (focusIdx + 1) / games.length : 0}`}>
  <div class="bp-reel-caption" aria-hidden="true">
    <span>LEFT STICK / D-PAD</span>
    <i></i>
    <b>左右手动切换 · {filterAll ? "全作品" : "本机安装"}</b>
  </div>

  <div class="bp-reel-window" bind:this={reelEl} role="listbox" aria-label="大屏游戏列表">
    {#each visibleGames as item (item.game.id)}
      {@const index = item.originalIndex}
      {@const game = item.game}
      {@const offset = index - focusIdx}
      <button
        class="bp-reel-card"
        class:focus={index === focusIdx}
        style={`--offset:${offset};--abs:${Math.abs(offset)}`}
        data-idx={index}
        role="option"
        aria-selected={index === focusIdx}
        aria-current={index === focusIdx ? "true" : undefined}
        aria-label={game.name}
        tabindex={active && index === focusIdx ? 0 : -1}
        onclick={() => onSelect(index)}
        ondblclick={() => { onSelect(index); onActivate(index); }}
        onfocus={() => onSelect(index)}
        onkeydown={(event) => onCardKeydown(event, index)}
      >
        <span class="bp-poster">
          {#if fileSrc(gameCoverOf(game))}
            <img src={fileSrc(gameCoverOf(game))!} alt="" draggable="false" />
          {:else}
            <span class="bp-monogram">{monogram(game)}</span>
          {/if}
          <span class="bp-poster-wash"></span>
          <span class="bp-poster-no">{numberLabel(index)}</span>
          {#if isInstalled(game)}<span class="bp-installed"><i></i>READY</span>{/if}
          {#if game.favorite}<span class="bp-favorite" aria-label="已收藏"><Icon name="heartFill" size={14} /></span>{/if}
        </span>
        <span class="bp-edge-title">{game.name}</span>
      </button>
    {/each}

    {#if games.length === 0}
      <div class="bp-empty">
        <span><Icon name="gamepad" size={32} /></span>
        <strong>你的游戏舞台还是空的</strong>
        <p>导入游戏后，这里会成为专属于大屏的滚动展廊。</p>
        <button onclick={onOpenImport}><Icon name="download" size={17} />导入游戏</button>
      </div>
    {/if}
  </div>

  {#if games.length > 0}
    <div class="bp-reel-index" aria-hidden="true">
      <strong>{numberLabel(focusIdx)}</strong>
      <span></span>
      <em>{numberLabel(games.length - 1)}</em>
    </div>
    <button class="bp-filter-stamp" onclick={onToggleFilter} tabindex="-1" aria-label={filterAll ? "当前：全部，点击仅看已安装" : "当前：已安装，点击查看全部"}>
      <span>{filterAll ? "ALL" : "LOCAL"}</span>
      <small>{filterAll ? "全部作品" : "本机安装"}</small>
    </button>
  {/if}
</aside>

<style>
  .bp-reel {
    position:absolute;
    left:34%;
    right:0;
    bottom:44px;
    height:min(51vh,560px);
    min-width:0;
    overflow:hidden;
    pointer-events:none;
    perspective:1600px;
  }

  .bp-reel-caption {
    position:absolute;
    left:clamp(24px,3.5vw,68px);
    top:0;
    z-index:40;
    display:flex;
    align-items:center;
    gap:11px;
    color:rgba(255,255,255,.5);
    font:750 clamp(7px,.5vw,9px) var(--font-mono);
    letter-spacing:.17em;
    text-shadow:0 2px 14px rgba(0,0,0,.7);
  }
  .bp-reel-caption i { width:clamp(36px,4vw,70px); height:1px; background:var(--scene-accent); box-shadow:0 0 18px color-mix(in srgb,var(--scene-accent) 65%,transparent); }
  .bp-reel-caption b { color:var(--scene-paper,#f4f0e8); font-weight:800; letter-spacing:.06em; }

  .bp-reel-window { position:absolute; inset:24px 0 0; pointer-events:auto; }
  .bp-reel-window::before {
    content:"";
    position:absolute;
    left:5%; right:4%; bottom:5px;
    height:1px;
    background:linear-gradient(90deg,transparent,rgba(255,255,255,.18) 16%,var(--scene-accent) 50%,rgba(255,255,255,.18) 84%,transparent);
    box-shadow:0 0 24px color-mix(in srgb,var(--scene-accent) 24%,transparent);
  }

  .bp-reel-card {
    --offset:0;
    --abs:0;
    position:absolute;
    left:52%;
    bottom:12px;
    z-index:calc(24 - var(--abs));
    width:clamp(148px,15.2vw,292px);
    aspect-ratio:2 / 3;
    padding:0;
    border:0;
    color:#fff;
    background:transparent;
    cursor:pointer;
    opacity:calc(1 - var(--abs) * .2);
    transform:
      translateX(calc(-50% + var(--offset) * clamp(118px,11.8vw,226px)))
      translateY(calc(var(--abs) * 22px))
      rotateY(calc(var(--offset) * -7deg))
      rotateZ(calc(var(--offset) * 1.4deg))
      scale(calc(1 - var(--abs) * .145));
    transform-origin:center bottom;
    transition:transform 560ms cubic-bezier(.2,.82,.2,1),opacity 380ms ease,filter 380ms ease;
    filter:saturate(calc(1 - var(--abs) * .24)) brightness(calc(1 - var(--abs) * .14));
  }
  .bp-reel-card.focus { z-index:32; opacity:1; filter:saturate(1.04) brightness(1); transform:translateX(-50%) translateY(-10px) rotateY(0) rotateZ(0) scale(1); }
  .bp-reel-card:focus,.bp-reel-card:focus-visible { outline:none !important; box-shadow:none !important; }

  .bp-poster {
    position:absolute;
    inset:0;
    display:block;
    overflow:hidden;
    border:1px solid rgba(255,255,255,.17);
    background:linear-gradient(145deg,color-mix(in srgb,var(--scene-accent) 38%,#161820),#080910 72%);
    box-shadow:0 30px 70px -26px rgba(0,0,0,.92);
  }
  .bp-reel-card.focus .bp-poster {
    border-color:color-mix(in srgb,var(--scene-accent) 72%,white 22%);
    box-shadow:0 0 0 3px rgba(5,7,11,.76),0 0 0 5px color-mix(in srgb,var(--scene-accent) 82%,white 18%),0 38px 90px -26px rgba(0,0,0,.96),0 0 72px -34px var(--scene-accent);
  }
  .bp-poster img { width:100%; height:100%; object-fit:cover; display:block; }
  .bp-poster-wash { position:absolute; inset:0; background:linear-gradient(180deg,transparent 52%,rgba(4,5,9,.78)); }
  .bp-poster-no { position:absolute; top:12px; left:13px; color:white; font:850 clamp(10px,.82vw,15px) var(--font-mono); letter-spacing:.08em; text-shadow:0 2px 12px rgba(0,0,0,.5); }
  .bp-installed { position:absolute; left:13px; bottom:12px; display:flex; align-items:center; gap:6px; font:800 8px var(--font-mono); letter-spacing:.13em; }
  .bp-installed i { width:5px; height:5px; border-radius:50%; background:#74e5ad; box-shadow:0 0 11px #74e5ad; }
  .bp-favorite { position:absolute; right:11px; top:10px; display:grid; place-items:center; width:27px; height:27px; border-radius:50%; color:#fff; background:var(--scene-accent); }
  .bp-monogram { position:absolute; inset:0; display:grid; place-items:center; color:rgba(255,255,255,.78); font:820 clamp(68px,8vw,142px)/1 var(--font-display); }

  .bp-edge-title {
    position:absolute;
    left:50%;
    bottom:-25px;
    width:125%;
    overflow:hidden;
    color:rgba(255,255,255,.42);
    font:750 9px/1.2 var(--font-ui);
    letter-spacing:.08em;
    text-align:center;
    text-overflow:ellipsis;
    white-space:nowrap;
    transform:translateX(-50%);
  }
  .bp-reel-card.focus .bp-edge-title { color:var(--scene-paper); }

  .bp-reel-index {
    position:absolute;
    right:var(--bp-safe-x,48px);
    top:1px;
    z-index:40;
    display:grid;
    grid-template-columns:auto minmax(82px,10vw) auto;
    align-items:center;
    gap:9px;
    color:rgba(255,255,255,.44);
    font-family:var(--font-mono);
  }
  .bp-reel-index strong { color:var(--scene-paper); font-size:clamp(15px,1.1vw,22px); }
  .bp-reel-index span { width:100%; height:1px; background:linear-gradient(90deg,var(--scene-accent) 0 calc((var(--progress,.5)) * 100%),rgba(255,255,255,.16) 0); }
  .bp-reel-index em { font-size:9px; font-style:normal; }

  .bp-filter-stamp {
    position:absolute;
    right:var(--bp-safe-x,48px);
    bottom:6px;
    z-index:42;
    display:grid;
    justify-items:end;
    gap:1px;
    padding:0 0 4px;
    border:0;
    border-bottom:1px solid rgba(255,255,255,.28);
    color:white;
    background:rgba(7,8,12,.18);
    cursor:pointer;
  }
  .bp-filter-stamp span { color:var(--scene-accent); font:900 11px var(--font-mono); letter-spacing:.18em; }
  .bp-filter-stamp small { color:rgba(255,255,255,.54); font-size:8px; letter-spacing:.08em; }

  .bp-empty { position:absolute; left:52%; top:50%; width:min(440px,70%); transform:translate(-50%,-50%); text-align:center; }
  .bp-empty > span { display:grid; place-items:center; width:68px; height:68px; margin:0 auto 16px; border:1px solid rgba(255,255,255,.18); border-radius:50%; color:var(--scene-accent); }
  .bp-empty strong { display:block; color:white; font:800 clamp(20px,1.8vw,31px) var(--font-display); }
  .bp-empty p { color:rgba(255,255,255,.58); }
  .bp-empty button { display:inline-flex; align-items:center; gap:8px; padding:11px 19px; border:0; color:white; background:var(--scene-accent); font-weight:800; cursor:pointer; }

  @media (max-width:1180px) {
    .bp-reel { left:38%; height:min(47vh,430px); }
    .bp-reel-card { left:51%; width:clamp(135px,16.5vw,222px); transform:translateX(calc(-50% + var(--offset) * clamp(102px,12.5vw,168px))) translateY(calc(var(--abs) * 18px)) rotateY(calc(var(--offset) * -6deg)) rotateZ(calc(var(--offset) * 1.2deg)) scale(calc(1 - var(--abs) * .14)); }
    .bp-reel-card.focus { transform:translateX(-50%) translateY(-8px) scale(1); }
    .bp-reel-caption b { display:none; }
  }
  @media (max-width:850px) {
    .bp-reel { left:42%; }
    .bp-reel-caption { display:none; }
    .bp-edge-title { display:none; }
  }
  @media (max-height:760px) {
    .bp-reel { bottom:37px; height:min(45vh,350px); }
    .bp-reel-card { width:clamp(128px,14.5vw,202px); }
    .bp-edge-title { bottom:-20px; }
  }
  @media (min-width:2800px) {
    .bp-reel { left:35%; height:min(52vh,820px); }
    .bp-reel-card { width:clamp(280px,14vw,470px); }
  }
  @media (prefers-reduced-motion:reduce) { .bp-reel-card { transition:none; } }
  :global([data-motion="reduce"]) .bp-reel-card { transition:none; }
</style>
