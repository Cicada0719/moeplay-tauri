<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import GameCard from "./GameCard.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Skeleton from "./Skeleton.svelte";

  let gridEl = $state<HTMLDivElement>();
  let errorMessage = $state("");
  let scrollTop = $state(0);
  let viewportHeight = $state(0);
  let containerWidth = $state(0);

  const isListMode = $derived(uiStore.viewMode === "list");
  const gap = $derived(uiStore.viewMode === "compact" ? 12 : 16);
  const minColumnWidth = $derived(uiStore.viewMode === "compact" ? 132 : 150);
  const paddingX = $derived(containerWidth <= 760 ? 16 : 28);
  const paddingTop = 24;
  const paddingBottom = 32;
  const cardChrome = 98;
  const columnCount = $derived(
    isListMode ? 1 : Math.max(1, Math.floor((containerWidth - paddingX * 2 + gap) / (minColumnWidth + gap)))
  );
  const columnWidth = $derived(
    isListMode
      ? Math.max(0, containerWidth - paddingX * 2)
      : Math.max(minColumnWidth, (Math.max(0, containerWidth - paddingX * 2) - gap * (columnCount - 1)) / columnCount)
  );
  const rowHeight = $derived(isListMode ? 96 : Math.round(columnWidth * 4 / 3 + cardChrome));
  const rowCount = $derived(Math.ceil(gameStore.games.length / columnCount));
  const firstRow = $derived(Math.max(0, Math.floor(Math.max(0, scrollTop - paddingTop) / (rowHeight + gap)) - 2));
  const visibleRows = $derived(Math.ceil(viewportHeight / (rowHeight + gap)) + 5);
  const lastRow = $derived(Math.min(rowCount, firstRow + visibleRows));
  const visibleGames = $derived(
    gameStore.games.slice(firstRow * columnCount, Math.min(gameStore.games.length, lastRow * columnCount))
  );
  const topSpacer = $derived(paddingTop + firstRow * (rowHeight + gap));
  const totalHeight = $derived(paddingTop + paddingBottom + Math.max(0, rowCount * rowHeight + Math.max(0, rowCount - 1) * gap));

  onMount(() => {
    if (!gridEl) return;
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    const observer = new ResizeObserver(([entry]) => {
      const rect = entry.contentRect;
      viewportHeight = rect.height;
      containerWidth = rect.width;
    });
    observer.observe(gridEl);

    let ctx: gsap.Context | null = null;
    if (!reduce) {
      ctx = gsap.context(() => {
        gsap.from(gridEl!.querySelectorAll(".grid-item"), {
          opacity: 0,
          y: 14,
          duration: 0.5,
          ease: "power3.out",
          stagger: 0.03,
        });
      }, gridEl);
    }

    return () => {
      observer.disconnect();
      ctx?.revert();
    };
  });

  async function handleImport() {
    errorMessage = "";
    try {
      await gameStore.importGame();
    } catch (error) {
      errorMessage = `导入失败：${error}`;
    }
  }
</script>

<div
  class="game-grid"
  class:compact={uiStore.viewMode === "compact"}
  class:list={isListMode}
  bind:this={gridEl}
  onscroll={() => (scrollTop = gridEl?.scrollTop ?? 0)}
>
  {#if errorMessage}
    <div class="inline-error" role="alert">{errorMessage}</div>
  {/if}

  {#if gameStore.loading}
    <div class="static-grid">
      <Skeleton variant="card" count={10} className="grid-item" />
    </div>
  {:else if gameStore.games.length === 0}
    <div class="empty-wrap">
      <EmptyState
        title="还没有游戏"
        description="导入本地 galgame 后，封面、标签与游玩状态会出现在这里。"
        actionLabel="添加游戏"
        onAction={handleImport}
      />
    </div>
  {:else}
    <div
      class="virtual-canvas"
      style={`height:${totalHeight}px; --grid-cols:${columnCount}; --grid-gap:${gap}px; --grid-padding-x:${paddingX}px; --grid-col-width:${columnWidth}px;`}
    >
      <div class="virtual-grid" style={`transform:translateY(${topSpacer}px);`}>
        {#each visibleGames as game (game.id)}
          <div class="grid-item">
            <GameCard {game} />
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .game-grid {
    flex: 1;
    min-height: 0;
    overflow: auto;
    position: relative;
  }

  .static-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    align-content: start;
    gap: 16px;
    padding: 24px 28px 32px;
  }

  .game-grid.compact .static-grid {
    grid-template-columns: repeat(auto-fill, minmax(132px, 1fr));
    gap: 12px;
  }

  .game-grid.list .static-grid {
    grid-template-columns: 1fr;
  }

  .grid-item { min-width: 0; }

  .game-grid.list .grid-item {
    height: 96px;
  }

  .virtual-canvas {
    position: relative;
    min-height: 100%;
  }

  .virtual-grid {
    position: absolute;
    left: var(--grid-padding-x);
    right: var(--grid-padding-x);
    top: 0;
    display: grid;
    grid-template-columns: repeat(var(--grid-cols), minmax(0, var(--grid-col-width)));
    gap: var(--grid-gap);
    align-content: start;
    will-change: transform;
  }

  .empty-wrap,
  .inline-error {
    margin: 24px 28px 0;
  }

  .empty-wrap {
    display: grid;
    min-height: 58vh;
    align-items: center;
  }

  @media (max-width: 760px) {
    .static-grid {
      padding-inline: 16px;
    }

    .empty-wrap,
    .inline-error {
      margin-inline: 16px;
    }
  }

  .inline-error {
    border: 1px solid rgba(240, 85, 107, 0.35);
    background: rgba(240, 85, 107, 0.10);
    color: var(--color-error);
    border-radius: var(--radius-md);
    padding: 10px 12px;
    font-size: 13px;
  }
</style>
