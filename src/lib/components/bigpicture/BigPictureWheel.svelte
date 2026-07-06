<script lang="ts">
  import { coverOf as gameCoverOf, isInstalled } from "../../utils/game";
  import type { Game } from "../../stores/games.svelte";
  import { fileSrc } from "../../utils";
  import Icon from "../Icon.svelte";

  let {
    games,
    focusIdx,
    filterAll,
    prefersReducedMotion,
    onSelect,
    onActivate,
    onToggleFilter,
    onOpenImport,
  }: {
    games: Game[];
    focusIdx: number;
    filterAll: boolean;
    prefersReducedMotion: boolean;
    onSelect: (idx: number) => void;
    onActivate: (idx: number) => void;
    onToggleFilter: () => void;
    onOpenImport: () => void;
  } = $props();

  let railEl = $state<HTMLDivElement>();

  const WHEEL_RADIUS = 8;
  const wheelRange = $derived({
    start: Math.max(0, focusIdx - WHEEL_RADIUS),
    end: Math.min(games.length - 1, focusIdx + WHEEL_RADIUS),
  });
  const visibleWheelGames = $derived(
    games.slice(wheelRange.start, wheelRange.end + 1).map((g, vi) => ({ g, origIdx: vi + wheelRange.start }))
  );

  $effect(() => {
    const idx = focusIdx;
    queueMicrotask(() => {
      railEl?.querySelector<HTMLElement>(`[data-idx="${idx}"]`)?.scrollIntoView({
        inline: "nearest",
        block: "center",
        behavior: prefersReducedMotion ? "auto" : "smooth",
      });
    });
  });

  const monogram = (g: Game) => (g.name?.trim()?.[0] ?? "?").toUpperCase();
</script>

<aside class="bp-sidebar">
  <header class="bp-sidebar-head">
    <div class="bp-sidebar-titles">
      <span class="bp-sidebar-kicker">游戏库</span>
      <span class="bp-sidebar-count"><b>{games.length}</b><i>款</i></span>
    </div>
    <button
      class="bp-filter"
      data-on={filterAll ? "all" : "installed"}
      onclick={onToggleFilter}
      aria-label={filterAll ? "当前：全部，点击仅看已安装" : "当前：已安装，点击查看全部"}
    >
      <span class="bp-filter-opt">全部</span>
      <span class="bp-filter-opt">已装</span>
    </button>
  </header>

  <div class="bp-wheel" bind:this={railEl} role="listbox" aria-label="大屏游戏列表">
    {#if wheelRange.start > 0}<div class="bp-wheel-spacer" style="height:{wheelRange.start * 80}px"></div>{/if}
    {#each visibleWheelGames as item (item.g.id)}
      {@const i = item.origIdx}
      {@const g = item.g}
      {@const off = i - focusIdx}
      {@const coff = Math.max(-4, Math.min(4, off))}
      <button
        class="bp-card"
        class:focus={i === focusIdx}
        style="--off:{off}; --aoff:{Math.min(Math.abs(off), 4)}; --coff:{coff}"
        data-idx={i}
        role="option"
        aria-selected={i === focusIdx}
        onclick={() => onSelect(i)}
        ondblclick={() => { onSelect(i); onActivate(i); }}
        onfocus={() => onSelect(i)}
        aria-label={g.name}
        aria-current={i === focusIdx ? "true" : undefined}
        tabindex={i === focusIdx ? 0 : -1}
      >
        <span class="bp-card-art">
          {#if fileSrc(gameCoverOf(g))}
            <img src={fileSrc(gameCoverOf(g))!} alt={g.name} draggable="false" loading="lazy" />
          {:else}
            <span class="bp-mono">{monogram(g)}</span>
          {/if}
          {#if isInstalled(g)}
            <span class="bp-card-flag" title="已安装"></span>
          {/if}
          <span class="bp-card-name">{g.name}</span>
        </span>
      </button>
    {/each}
    {#if wheelRange.end < games.length - 1}<div class="bp-wheel-spacer" style="height:{(games.length - 1 - wheelRange.end) * 80}px"></div>{/if}
    {#if games.length === 0}
      <div class="bp-empty">
        <p>暂无游戏</p>
        <button class="bp-empty-action" onclick={onOpenImport}><Icon name="download" size={16} /> Steam / Epic 导入</button>
      </div>
    {/if}
  </div>

  {#if games.length > 1}
    <div class="bp-progress" aria-hidden="true">
      <span class="bp-progress-thumb" style="--p:{focusIdx / (games.length - 1)}"></span>
    </div>
  {/if}
</aside>

<style>
  .bp-sidebar {
    position: relative;
    width: 194px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: linear-gradient(180deg, rgba(11, 14, 22, 0.68) 0%, rgba(7, 9, 15, 0.62) 100%);
    backdrop-filter: blur(22px) saturate(1.15);
    border-right: 1px solid rgba(255, 255, 255, 0.07);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.06),
      inset 1px 0 0 rgba(255, 255, 255, 0.04),
      18px 0 46px -26px rgba(0, 0, 0, 0.7);
  }

  .bp-sidebar-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px;
    padding: 18px 16px 12px;
    flex-shrink: 0;
  }
  .bp-sidebar-titles { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .bp-sidebar-kicker {
    font-size: 10px; font-weight: 800; letter-spacing: 0.2em;
    text-transform: uppercase; color: var(--text-muted);
  }
  .bp-sidebar-count { display: flex; align-items: baseline; gap: 4px; }
  .bp-sidebar-count b {
    font-family: var(--font-display); font-size: 23px; font-weight: 800;
    line-height: 1; color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .bp-sidebar-count i { font-style: normal; font-size: 11px; color: var(--text-muted); }

  .bp-filter {
    position: relative; display: inline-flex; align-items: center;
    padding: 3px; border-radius: var(--radius-full);
    border: 1px solid var(--border); background: rgba(7, 9, 15, 0.5);
    cursor: pointer; overflow: hidden; flex-shrink: 0;
  }
  .bp-filter::before {
    content: ""; position: absolute; top: 3px; bottom: 3px; left: 3px;
    width: calc(50% - 3px); border-radius: var(--radius-full);
    background: var(--accent);
    box-shadow: 0 2px 8px -2px rgba(232, 85, 127, 0.6);
    transition: transform 0.28s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bp-filter[data-on="installed"]::before { transform: translateX(100%); }
  .bp-filter-opt {
    position: relative; z-index: 1;
    min-width: 34px; text-align: center; padding: 4px 4px;
    font-size: 10.5px; font-weight: 800; color: var(--text-muted);
    transition: color 0.2s ease;
  }
  .bp-filter[data-on="all"] .bp-filter-opt:first-child,
  .bp-filter[data-on="installed"] .bp-filter-opt:last-child { color: #fff; }

  .bp-wheel {
    flex: 1; min-height: 0;
    display: flex; flex-direction: column;
    gap: 14px;
    overflow-y: auto; overflow-x: hidden;
    padding: 14px 22px 16vh;
    scroll-padding-block: 50%;
    perspective: 1000px;
    scrollbar-width: none;
  }
  .bp-wheel::-webkit-scrollbar { display: none; }
  .bp-wheel-spacer { flex: 0 0 auto; pointer-events: none; }

  .bp-card {
    position: relative;
    flex: 0 0 auto;
    width: 100%;
    border: none; padding: 0; margin: 0; cursor: pointer;
    background: none; outline: 0;
    transform-style: preserve-3d;
    opacity: calc(1 - var(--aoff, 0) * 0.16);
    transform:
      rotateX(calc(var(--coff, 0) * -2deg))
      scale(calc(1 - var(--aoff, 0) * 0.07));
    transition:
      transform 0.34s cubic-bezier(0.22, 1, 0.36, 1),
      opacity 0.34s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bp-card-art {
    position: relative; display: block;
    width: 100%; aspect-ratio: 3 / 4;
    border-radius: var(--radius-md); overflow: hidden;
    background: var(--bg-elev);
    box-shadow: var(--shadow-tile);
  }
  .bp-card-art img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .bp-mono {
    width: 100%; height: 100%; display: grid; place-items: center;
    font-family: var(--font-display); font-size: 28px; font-weight: 800;
    color: var(--text-muted);
    background: linear-gradient(135deg, rgba(232, 85, 127, 0.2), rgba(110, 120, 160, 0.14));
  }
  .bp-card-flag {
    position: absolute; top: 7px; right: 7px;
    width: 7px; height: 7px; border-radius: 50%;
    background: #5fd39a; box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.45);
  }
  .bp-card-name {
    position: absolute; left: 0; right: 0; bottom: 0;
    padding: 18px 9px 7px;
    font-size: 11px; font-weight: 800; line-height: 1.2; text-align: left;
    color: #fff;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.86));
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    opacity: 0; transform: translateY(6px);
    transition: opacity 0.24s ease, transform 0.24s ease;
  }

  .bp-card.focus {
    opacity: 1;
    transform: scale(1.08);
    z-index: 3;
  }
  .bp-card.focus .bp-card-name { opacity: 1; transform: none; }
  .bp-card.focus::after {
    content: ""; position: absolute; inset: 0;
    border-radius: var(--radius-md); pointer-events: none;
    animation: bpFocusBreath 2.8s ease-in-out infinite;
  }
  @keyframes bpFocusBreath {
    0%, 100% { box-shadow: 0 0 0 2px var(--accent), 0 14px 32px -14px rgba(232, 85, 127, 0.5); }
    50% { box-shadow: 0 0 0 2px var(--accent), 0 18px 44px -12px rgba(232, 85, 127, 0.78); }
  }
  .bp-card:hover { opacity: 1; }
  .bp-card:focus-visible { outline: none; }
  .bp-card:focus-visible .bp-card-art { box-shadow: var(--ring-switch); }

  .bp-progress {
    position: absolute; right: 4px; top: 70px; bottom: 16px;
    width: 3px; border-radius: 2px;
    background: rgba(255, 255, 255, 0.06);
    pointer-events: none;
  }
  .bp-progress-thumb {
    position: absolute; left: 0; right: 0; height: 36px;
    border-radius: 2px;
    background: linear-gradient(180deg, var(--accent-hi), var(--accent));
    top: calc(var(--p, 0) * (100% - 36px));
    transition: top 0.3s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .bp-empty {
    display: flex; flex-direction: column; align-items: center; gap: 12px;
    padding: 28px 8px; color: var(--text-muted); text-align: center;
  }
  .bp-empty-action {
    display: inline-flex; align-items: center; gap: 6px;
    border: none; cursor: pointer; background: var(--accent); color: #fff;
    padding: 9px 14px; border-radius: var(--radius-full); font-weight: 700; font-size: 12px;
  }
</style>
