<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
  import { formatPlayTime } from "../../api";
  import { developerOf, gameCompletionStatus, gameLastPlayed, gameTotalSeconds, releaseYearOf, tagsOf as gameTagsOf } from "../../utils/game";
  import type { Game } from "../../stores/games.svelte";
  import { attachGamepad, type GamepadAttachment } from "../switch/useGamepad.svelte";

  let {
    game,
    weekHours,
    active = false,
    onLaunch,
    onFavorite,
    onDetail,
    onMoveToWheel,
    onMoveToTop,
    onTabPrevious,
    onTabNext,
    onToggleFilter,
  }: {
    game: Game;
    weekHours: string;
    active?: boolean;
    onLaunch: () => void;
    onFavorite: () => void;
    onDetail: () => void;
    onMoveToWheel: () => void;
    onMoveToTop: () => void;
    onTabPrevious: () => void;
    onTabNext: () => void;
    onToggleFilter: () => void;
  } = $props();

  let actionsEl = $state<HTMLDivElement>();
  let actionIdx = $state(0);
  let scope: GamepadAttachment | null = null;

  const STATUS: Record<string, string> = {
    not_started: "未开始", playing: "游玩中", completed: "已通关",
    on_hold: "搁置", dropped: "已弃坑", plan_to_play: "计划中", replaying: "重温中",
  };
  const completion = $derived(STATUS[gameCompletionStatus(game)] ?? "库中作品");
  const allTags = $derived(gameTagsOf(game));
  const description = $derived(game.description?.trim() || allTags.slice(0, 4).join(" / ") || "这款游戏暂时还没有补充说明。");

  function metaLine(current: Game): string {
    const parts: string[] = [];
    const developer = developerOf(current);
    if (developer) parts.push(developer);
    const year = releaseYearOf(current);
    if (year) parts.push(String(year));
    const seconds = gameTotalSeconds(current);
    if (seconds > 0) parts.push(formatPlayTime(seconds));
    return parts.join(" / ");
  }

  function timeAgo(value: string | null | undefined): string {
    if (!value) return "尚未游玩";
    const days = Math.floor((Date.now() - new Date(value).getTime()) / 86400000);
    if (days <= 0) return "今天";
    if (days === 1) return "昨天";
    if (days < 7) return `${days} 天前`;
    if (days < 30) return `${Math.floor(days / 7)} 周前`;
    return `${Math.floor(days / 30)} 个月前`;
  }

  function focusAction(index = actionIdx) {
    actionIdx = Math.max(0, Math.min(2, index));
    queueMicrotask(() => actionsEl?.querySelector<HTMLElement>(`[data-hero-index="${actionIdx}"]`)?.focus({ preventScroll: true }));
  }
  function moveAction(delta: number) { focusAction(Math.max(0, Math.min(2, actionIdx + delta))); }
  function activateAction() { if (actionIdx === 0) onLaunch(); else if (actionIdx === 1) onFavorite(); else onDetail(); }
  function onActionKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowLeft": event.preventDefault(); moveAction(-1); break;
      case "ArrowRight": event.preventDefault(); moveAction(1); break;
      case "ArrowUp": event.preventDefault(); onMoveToTop(); break;
      case "ArrowDown": event.preventDefault(); onMoveToWheel(); break;
      case "Escape": event.preventDefault(); onMoveToWheel(); break;
    }
  }

  $effect(() => { if (active) focusAction(); });

  onMount(() => {
    scope = attachGamepad({
      left: () => moveAction(-1), right: () => moveAction(1),
      up: () => onMoveToTop(), down: () => onMoveToWheel(),
      launch: () => activateAction(), activate: () => activateAction(),
      favorite: () => onFavorite(), filter: () => onToggleFilter(), back: () => onMoveToWheel(),
      pageLeft: () => onTabPrevious(), pageRight: () => onTabNext(),
    }, { id: "big-picture-hero", zone: "hero", priority: 20 });
    return () => { scope?.(); scope = null; };
  });
</script>

<div class="bp-hero" data-focus-zone="hero" data-active={active ? "true" : "false"}>
  <div class="bp-editorial-mark" aria-hidden="true"><span>選</span><i></i><small>SELECTED WORK</small></div>

  <div class="bp-copy">
    <div class="bp-state"><i></i><span>{completion}</span>{#if game.favorite}<em>收藏</em>{/if}</div>
    {#if game.metadata?.original_name}<p class="bp-original">{game.metadata.original_name}</p>{/if}
    <h1 class="bp-title">{game.name}</h1>
    <p class="bp-meta">{metaLine(game)}</p>
    <p class="bp-description">{description}</p>

    <div class="bp-actions" bind:this={actionsEl} role="toolbar" aria-label="游戏操作">
      <button class="bp-play" class:zone-focus={active && actionIdx === 0} data-hero-index="0" tabindex={active && actionIdx === 0 ? 0 : -1} onclick={onLaunch} onfocus={() => (actionIdx = 0)} onkeydown={onActionKeydown}>
        <span><Icon name="play" size={20} /></span><b>开始游戏</b><kbd>A</kbd>
      </button>
      <button class="bp-icon-action" class:active={game.favorite} class:zone-focus={active && actionIdx === 1} data-hero-index="1" tabindex={active && actionIdx === 1 ? 0 : -1} onclick={onFavorite} onfocus={() => (actionIdx = 1)} onkeydown={onActionKeydown} aria-label={game.favorite ? "取消收藏" : "收藏"}>
        <Icon name={game.favorite ? "heartFill" : "heart"} size={19} /><small>收藏</small>
      </button>
      <button class="bp-icon-action" class:zone-focus={active && actionIdx === 2} data-hero-index="2" tabindex={active && actionIdx === 2 ? 0 : -1} onclick={onDetail} onfocus={() => (actionIdx = 2)} onkeydown={onActionKeydown} aria-label="详情">
        <Icon name="database" size={19} /><small>档案</small>
      </button>
    </div>

    <div class="bp-facts">
      <span><small>PLAY TIME</small><b>{formatPlayTime(gameTotalSeconds(game))}</b></span>
      <span><small>LAST SESSION</small><b>{timeAgo(gameLastPlayed(game))}</b></span>
      <span><small>THIS WEEK</small><b>{weekHours}h</b></span>
    </div>

    {#if allTags.length}
      <div class="bp-tags">{#each allTags.slice(0, 4) as tag}<span>{tag}</span>{/each}</div>
    {/if}
  </div>

</div>

<style>
  .bp-hero {
    position: absolute;
    inset: 0 62% 0 0;
    z-index: 12;
    min-width: 0;
    color: var(--scene-paper, #f7f3ea);
    pointer-events: none;
  }
  .bp-copy {
    position: absolute;
    left: var(--bp-safe-x);
    bottom: clamp(62px, 7.8vh, 94px);
    width: min(32vw, 560px);
    pointer-events: auto;
    text-shadow: 0 3px 32px rgba(0,0,0,.62);
  }
  .bp-editorial-mark {
    position: absolute;
    left: var(--bp-safe-x);
    top: clamp(78px, 10vh, 124px);
    display: flex;
    align-items: center;
    gap: 10px;
    color: rgba(255,255,255,.48);
  }
  .bp-editorial-mark span { display:grid; place-items:center; width:27px; height:27px; color:#0a0b0e; background:var(--scene-accent); font:900 15px serif; }
  .bp-editorial-mark i { width:clamp(28px,3vw,58px); height:1px; background:rgba(255,255,255,.26); }
  .bp-editorial-mark small { font:800 7px var(--font-mono); letter-spacing:.2em; }

  .bp-state { display:flex; align-items:center; gap:8px; margin-bottom:clamp(8px,1vh,13px); color:rgba(255,255,255,.68); font:800 8px var(--font-mono); letter-spacing:.14em; }
  .bp-state i { width:6px; height:6px; border-radius:50%; background:var(--scene-accent); box-shadow:0 0 14px var(--scene-accent); }
  .bp-state em { padding-left:8px; border-left:1px solid rgba(255,255,255,.22); color:var(--scene-accent); font-style:normal; }
  .bp-original { max-width:44ch; margin:0 0 4px; overflow:hidden; color:rgba(255,255,255,.48); font-size:clamp(9px,.62vw,11px); text-overflow:ellipsis; white-space:nowrap; }
  .bp-title { max-width:11ch; margin:0; color:var(--scene-paper); font:900 clamp(34px,4.15vw,76px)/.91 var(--font-display); letter-spacing:-.065em; text-wrap:balance; filter:drop-shadow(0 14px 34px rgba(0,0,0,.58)); }
  .bp-title::first-letter { color:var(--scene-accent); }
  .bp-meta { margin:clamp(9px,1.2vh,14px) 0 0; color:rgba(255,255,255,.64); font:650 clamp(9px,.66vw,12px) var(--font-ui); letter-spacing:.04em; }
  .bp-description {
    display:-webkit-box;
    max-width:52ch;
    margin:8px 0 0;
    overflow:hidden;
    color:rgba(255,255,255,.62);
    font:500 clamp(10px,.7vw,13px)/1.58 var(--font-ui);
    text-shadow:0 2px 18px rgba(0,0,0,.82);
    -webkit-box-orient:vertical;
    -webkit-line-clamp:3;
    line-clamp:3;
  }

  .bp-actions { display:flex; align-items:stretch; gap:7px; margin-top:clamp(13px,1.8vh,21px); }
  .bp-actions button { border:1px solid rgba(255,255,255,.2); color:white; background:rgba(7,8,12,.46); backdrop-filter:blur(16px); cursor:pointer; transition:transform 180ms ease,border-color 180ms ease,background 180ms ease; }
  .bp-actions button:focus { outline:none; }
  .bp-actions button.zone-focus { box-shadow:0 0 0 2px rgba(5,6,9,.92),0 0 0 5px var(--scene-accent); }
  .bp-play { display:flex; align-items:center; gap:9px; min-width:clamp(150px,11.5vw,218px); min-height:clamp(42px,4.8vh,54px); padding:0 11px 0 8px; background:var(--scene-accent) !important; border-color:transparent !important; color:#08090c !important; }
  .bp-play > span { display:grid; place-items:center; width:31px; height:31px; border:1px solid rgba(0,0,0,.18); border-radius:50%; }
  .bp-play b { font-size:clamp(11px,.76vw,14px); }
  .bp-play kbd { margin-left:auto; padding:2px 6px; border:1px solid rgba(0,0,0,.22); font:800 8px var(--font-mono); }
  .bp-icon-action { display:grid; grid-template-rows:1fr auto; place-items:center; width:clamp(49px,3.5vw,62px); padding:7px; }
  .bp-icon-action small { color:rgba(255,255,255,.62); font-size:8px; }
  .bp-icon-action.active { color:var(--scene-accent); }

  .bp-facts { display:flex; align-items:stretch; margin-top:clamp(11px,1.5vh,17px); border-top:1px solid rgba(255,255,255,.16); border-bottom:1px solid rgba(255,255,255,.1); }
  .bp-facts span { display:grid; gap:4px; min-width:clamp(82px,6.5vw,116px); padding:9px 12px 9px 0; }
  .bp-facts span + span { padding-left:12px; border-left:1px solid rgba(255,255,255,.11); }
  .bp-facts small { color:rgba(255,255,255,.4); font:750 6px var(--font-mono); letter-spacing:.14em; }
  .bp-facts b { font-size:clamp(10px,.7vw,13px); }
  .bp-tags { display:flex; flex-wrap:wrap; gap:5px; margin-top:9px; }
  .bp-tags span { padding:3px 7px; border:1px solid rgba(255,255,255,.13); color:rgba(255,255,255,.52); font-size:8px; }

  @media (max-width:1180px) {
    .bp-hero { right:60%; }
    .bp-copy { width:min(35vw,430px); }
    .bp-title { font-size:clamp(31px,4.5vw,56px); }
    .bp-facts span:nth-child(3),.bp-tags { display:none; }
  }
  @media (max-width:850px) {
    .bp-hero { right:55%; }
    .bp-copy { width:39vw; }
    .bp-description { -webkit-line-clamp:2; line-clamp:2; }
    .bp-facts { display:none; }
  }
  @media (max-height:760px) {
    .bp-copy { bottom:52px; width:min(34vw,460px); }
    .bp-editorial-mark { top:68px; }
    .bp-title { font-size:clamp(30px,4vw,52px); }
    .bp-description { -webkit-line-clamp:2; line-clamp:2; }
    .bp-actions { margin-top:11px; }
    .bp-facts { margin-top:9px; }
    .bp-tags { display:none; }
  }
  @media (prefers-reduced-motion:reduce) { .bp-actions button { transition:none; } }
  :global([data-motion="reduce"]) .bp-actions button { transition:none; }
</style>
