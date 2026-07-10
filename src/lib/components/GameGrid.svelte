<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { routerStore } from "../stores/router.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { AsyncState, ContentGrid, MediaCard } from "./ui-v2";
  import GameCard from "./GameCard.svelte";

  const initialRouteOffset = routerStore.current.view === "home" ? routerStore.current.scrollOffset : 0;
  const initialWidth = typeof window === "undefined" ? 1200 : Math.max(320, window.innerWidth - 96);
  const initialHeight = typeof window === "undefined" ? 720 : Math.max(320, window.innerHeight - 120);

  let gridEl = $state<HTMLDivElement>();
  let errorMessage = $state("");
  let scrollTop = $state(initialRouteOffset);
  let viewportHeight = $state(initialHeight);
  let containerWidth = $state(initialWidth);
  let filterSignature = $state(`${gameStore.quickFilter ?? ""}|${gameStore.filterTag ?? ""}|${gameStore.sortBy}|${gameStore.searchQuery}`);

  const isListMode = $derived(uiStore.viewMode === "list");
  const isCompactMode = $derived(uiStore.viewMode === "compact");
  const gap = $derived(isCompactMode ? 12 : 16);
  const minColumnWidth = $derived(isCompactMode ? 132 : 150);
  const paddingX = $derived(containerWidth <= 760 ? 16 : 28);
  const paddingTop = 24;
  const paddingBottom = 32;
  const cardChrome = 98;
  const columnCount = $derived(
    isListMode ? 1 : Math.max(1, Math.floor((containerWidth - paddingX * 2 + gap) / (minColumnWidth + gap))),
  );
  const columnWidth = $derived(
    isListMode
      ? Math.max(0, containerWidth - paddingX * 2)
      : Math.max(minColumnWidth, (Math.max(0, containerWidth - paddingX * 2) - gap * (columnCount - 1)) / columnCount),
  );
  const rowHeight = $derived(isListMode ? 96 : Math.round(columnWidth * 4 / 3 + cardChrome));
  const rowCount = $derived(Math.ceil(gameStore.games.length / columnCount));
  const firstRow = $derived(Math.max(0, Math.floor(Math.max(0, scrollTop - paddingTop) / (rowHeight + gap)) - 2));
  const visibleRows = $derived(Math.ceil(viewportHeight / (rowHeight + gap)) + 5);
  const lastRow = $derived(Math.min(rowCount, firstRow + visibleRows));
  const visibleGames = $derived(
    gameStore.games.slice(firstRow * columnCount, Math.min(gameStore.games.length, lastRow * columnCount)),
  );
  const topSpacer = $derived(paddingTop + firstRow * (rowHeight + gap));
  const totalHeight = $derived(
    paddingTop + paddingBottom + Math.max(0, rowCount * rowHeight + Math.max(0, rowCount - 1) * gap),
  );
  const gridState = $derived(
    errorMessage
      ? "error"
      : gameStore.loading && gameStore.games.length === 0
        ? "loading"
        : gameStore.loading
          ? "refreshing"
          : gameStore.games.length === 0
            ? (gameStore.allGames.length === 0 ? "empty" : "no-results")
            : "ready",
  );

  $effect(() => {
    const nextSignature = `${gameStore.quickFilter ?? ""}|${gameStore.filterTag ?? ""}|${gameStore.sortBy}|${gameStore.searchQuery}`;
    if (nextSignature === filterSignature) return;
    filterSignature = nextSignature;
    if (gridEl) gridEl.scrollTop = 0;
    scrollTop = 0;
  });

  onMount(() => {
    if (!gridEl) return;
    const rect = gridEl.getBoundingClientRect();
    if (rect.height > 0) viewportHeight = rect.height;
    if (rect.width > 0) containerWidth = rect.width;
    if (initialRouteOffset > 0) {
      gridEl.scrollTop = initialRouteOffset;
      scrollTop = initialRouteOffset;
    }

    const observer = new ResizeObserver(([entry]) => {
      viewportHeight = entry.contentRect.height;
      containerWidth = entry.contentRect.width;
    });
    observer.observe(gridEl);

    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    let context: gsap.Context | null = null;
    if (!reduce) {
      context = gsap.context(() => {
        gsap.from(gridEl!.querySelectorAll("[data-testid^='game-card-']"), {
          opacity: 0,
          y: 14,
          duration: 0.4,
          ease: "power3.out",
          stagger: 0.025,
        });
      }, gridEl);
    }

    return () => {
      observer.disconnect();
      context?.revert();
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

  function clearFilters() {
    errorMessage = "";
    gameStore.searchQuery = "";
    gameStore.quickFilter = null;
    gameStore.filterTag = null;
    gameStore.sortBy = "recent";
  }
</script>

{#snippet loadingCards()}
  <ContentGrid
    label="正在加载游戏"
    role={isListMode ? "list" : "grid"}
    columns={isListMode ? 1 : undefined}
    minItemWidth={isCompactMode ? "8.25rem" : "9.375rem"}
    gap={isCompactMode ? "sm" : "md"}
    class="game-grid-loading"
  >
    {#each Array(isListMode ? 6 : 10) as _, index (index)}
      <MediaCard
        title={`游戏 ${index + 1}`}
        variant={isListMode ? "landscape" : "poster"}
        loading
        itemRole={isListMode ? "listitem" : "gridcell"}
      />
    {/each}
  </ContentGrid>
{/snippet}

<div
  class="game-grid"
  class:compact={isCompactMode}
  class:list={isListMode}
  bind:this={gridEl}
  data-route-scroll
  data-testid="game-library-scroll"
  onscroll={() => (scrollTop = gridEl?.scrollTop ?? 0)}
>
  <AsyncState
    state={gridState}
    loading={loadingCards}
    loadingDelayMs={0}
    preserveContent={gridState === "refreshing"}
    title={gridState === "no-results" ? "没有匹配的游戏" : gridState === "empty" ? "还没有游戏" : gridState === "error" ? "导入失败" : undefined}
    description={gridState === "no-results"
      ? "请清除筛选或尝试其他搜索条件。"
      : gridState === "empty"
        ? "导入本地 galgame 后，封面、标签与游玩状态会出现在这里。"
        : gridState === "error"
          ? errorMessage
          : undefined}
    primaryAction={gridState === "no-results"
      ? { label: "清除筛选", onSelect: clearFilters }
      : gridState === "empty" || gridState === "error"
        ? { label: "添加游戏", onSelect: handleImport }
        : undefined}
    class="game-grid-state"
  >
    {#snippet children()}
      <div
        class="virtual-canvas"
        style={`height:${totalHeight}px; --grid-cols:${columnCount}; --grid-gap:${gap}px; --grid-padding-x:${paddingX}px; --grid-col-width:${columnWidth}px; --grid-top-spacer:${topSpacer}px;`}
      >
        <ContentGrid
          label="游戏列表"
          role={isListMode ? "list" : "grid"}
          columns={columnCount}
          gap={isCompactMode ? "sm" : "md"}
          class="virtual-grid"
        >
          {#each visibleGames as game (game.id)}
            <GameCard
              {game}
              selected={gameStore.isSelected(game.id)}
              disabled={false}
              loading={false}
              itemRole={isListMode ? "listitem" : "gridcell"}
            />
          {/each}
        </ContentGrid>
      </div>
    {/snippet}
  </AsyncState>
</div>

<style>
  .game-grid { flex: 1; min-height: 0; overflow: auto; position: relative; overscroll-behavior: contain; }
  .virtual-canvas { position: relative; min-height: 100%; }

  :global(.virtual-grid.v2-content-grid) {
    position: absolute;
    left: var(--grid-padding-x);
    right: var(--grid-padding-x);
    top: var(--grid-top-spacer);
    gap: var(--grid-gap);
    align-content: start;
  }
  :global(.game-grid-loading.v2-content-grid) { padding: 1.5rem 1.75rem 2rem; }
  :global(.game-grid-state[data-state]:not([data-state="ready"])) { min-height: 58vh; margin: 1.5rem; }

  @media (max-width: 760px) {
    :global(.game-grid-loading.v2-content-grid) { padding-inline: 1rem; }
  }
</style>
