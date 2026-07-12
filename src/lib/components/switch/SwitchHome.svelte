<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore } from "../../stores/games.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { navigateTo } from "../../stores/router.svelte";
  import { gameLastPlayed, tagsOf } from "../../utils/game";
  import type { ViewState } from "../ui-v2";
  import { AsyncState, FilterBar, PageHeader, PageShell } from "../ui-v2";
  import Icon from "../Icon.svelte";
  import GameGrid from "../GameGrid.svelte";
  import { MediaModeSwitcher } from "../../features/media-workspace/components";
  import { AdaptiveChromaStage } from "../../features/media-workspace/chroma";
  import { GameSceneV2, GameVisualV2 } from "../../features/media-workspace/v2";
  import { MediaWorkspaceShell } from "../../features/media-workspace/shell";
  import { adaptGamesToPresentation, type ContentMode, type MediaPresentationAction, type MediaPresentationItem } from "../../features/media-workspace/model";
  import { mediaWorkspaceState } from "../../features/media-workspace/state";
  import { composeGameVisual } from "../../features/media-workspace/composition";
  import LibraryHealthPanel from "../library/LibraryHealthPanel.svelte";
  import { readLibraryV2Flag } from "../library/feature-flag";

  const quickFilters = [
    { id: "", label: "全部" },
    { id: "recent", label: "最近" },
    { id: "installed", label: "已安装" },
    { id: "favorite", label: "收藏" },
    { id: "playing", label: "游玩中" },
    { id: "completed", label: "已通关" },
    { id: "unplayed", label: "未开玩" },
    { id: "missing_metadata", label: "待补全" },
  ];

  const sortOptions = [
    { id: "recent", label: "最近游玩" },
    { id: "added", label: "最近添加" },
    { id: "name", label: "名称" },
    { id: "rating", label: "评分" },
    { id: "playtime", label: "游玩时长" },
  ];
  const viewModes = [
    { id: "grid", label: "标准", icon: "collection" },
    { id: "compact", label: "封面墙", icon: "diamond" },
    { id: "list", label: "列表", icon: "paperclip" },
  ] as const;

  let now = $state(new Date());
  let searchInput = $state<HTMLInputElement>();
  let healthOpen = $state(false);
  let libraryV2Enabled = $state(false);
  const pendingMediaActions = new Set<string>();

  const allGames = $derived(gameStore.allGames);
  const searching = $derived(!!gameStore.searchQuery.trim());
  const workspaceMode = $derived(mediaWorkspaceState.memoryFor("games").mode);
  const effectiveMode = $derived<ContentMode>(searching ? "index" : workspaceMode);
  const showGrid = $derived(effectiveMode === "index");
  const allGamesTitle = $derived(
    searching ? `搜索：${gameStore.searchQuery}` : uiStore.viewMode === "list" ? "列表视图" : uiStore.viewMode === "compact" ? "封面墙" : "全部游戏",
  );
  const activeFilterLabel = $derived(quickFilters.find((item) => item.id === (gameStore.quickFilter ?? ""))?.label ?? "全部");
  const activeFilterCount = $derived([
    searching,
    Boolean(gameStore.quickFilter),
    Boolean(gameStore.filterTag),
    gameStore.sortBy !== "recent",
  ].filter(Boolean).length);
  const libraryState = $derived.by<ViewState>(() => {
    if (gameStore.loadError && allGames.length > 0) return "partial";
    if (gameStore.loadError) return "error";
    if (gameStore.loading && allGames.length > 0) return "refreshing";
    if (gameStore.loading) return "loading";
    if (allGames.length === 0) return "empty";
    return "ready";
  });
  const tagOptions = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const game of allGames) {
      for (const tag of tagsOf(game)) {
        const clean = tag.trim();
        if (!clean) continue;
        counts.set(clean, (counts.get(clean) ?? 0) + 1);
      }
    }
    return [...counts.entries()]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0], "zh-CN"))
      .slice(0, 32)
      .map(([name, count]) => ({ name, count }));
  });

  function dateOf(value: string | null | undefined): number {
    return value ? new Date(value).getTime() || 0 : 0;
  }

  const recent = $derived.by(() => {
    const played = allGames
      .filter((game) => gameLastPlayed(game))
      .sort((a, b) => dateOf(gameLastPlayed(b)) - dateOf(gameLastPlayed(a)));
    const fresh = allGames
      .filter((game) => !gameLastPlayed(game))
      .sort((a, b) => dateOf(b.created_at || b.add_date) - dateOf(a.created_at || a.add_date));
    return [...played, ...fresh].slice(0, 16);
  });

  const selected = $derived(gameStore.selectedGame ?? recent[0] ?? null);
  const clock = $derived(now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }));
  const presentationItems = $derived.by(() => adaptGamesToPresentation(allGames, {
    open: (game) => onactivate(game.id),
    select: onselect,
    launch: onlaunch,
    toggleFavorite: (id) => gameStore.toggleFavorite(id),
  }));
  const visualComposition = $derived.by(() => composeGameVisual(presentationItems, selected?.id ?? null));
  const adaptiveChromaSource = $derived(visualComposition.chromaAsset?.src ?? null);

  $effect(() => {
    if (!gameStore.selectedGame && recent[0]) gameStore.selectGame(recent[0].id);
  });

  onMount(() => {
    libraryV2Enabled = readLibraryV2Flag();
    const storedMode = window.localStorage.getItem("moeplay:game-workspace-mode");
    if (storedMode === "visual" || storedMode === "index" || storedMode === "scene") {
      setWorkspaceMode(storedMode, false);
    } else if (uiStore.libraryMode === "all") {
      setWorkspaceMode("index", false);
    }
    const timer = setInterval(() => (now = new Date()), 30_000);
    return () => clearInterval(timer);
  });

  function setWorkspaceMode(mode: ContentMode, persist = true) {
    mediaWorkspaceState.setMode(mode, "games");
    uiStore.libraryMode = mode === "index" ? "all" : "home";
    if (persist && typeof window !== "undefined") window.localStorage.setItem("moeplay:game-workspace-mode", mode);
  }

  function onselect(id: string) {
    gameStore.selectGame(id);
    mediaWorkspaceState.selectItem(id, "games");
  }
  function onactivate(id: string) {
    onselect(id);
    uiStore.libraryMode = workspaceMode === "index" ? "all" : "home";
    navigateTo("game-detail", { entity: { kind: "game", id }, focus: "start" });
  }
  function onlaunch(id: string) {
    return gameStore.launch(id);
  }
  async function onMediaAction(item: MediaPresentationItem, action: MediaPresentationAction) {
    const key = `${item.id}:${action.id}`;
    if (pendingMediaActions.has(key)) return;
    pendingMediaActions.add(key);
    if (action.id === "launch") uiStore.notify(`正在启动 ${item.title}…`);
    try {
      await action.run();
    } catch (error) {
      const verb = action.id === "launch" ? "启动" : action.id === "toggle-favorite" ? "收藏操作" : "操作";
      uiStore.notify(`${verb}失败：${error instanceof Error ? error.message : String(error)}`, "error");
    } finally {
      pendingMediaActions.delete(key);
    }
  }
  function focusSearch() { searchInput?.focus(); }
  function closeSearchOrGrid() {
    if (gameStore.searchQuery) {
      gameStore.searchQuery = "";
      return;
    }
    if (effectiveMode === "index") setWorkspaceMode("visual");
  }
  function clearGridFilters() {
    gameStore.searchQuery = "";
    gameStore.quickFilter = null;
    gameStore.filterTag = null;
    gameStore.sortBy = "recent";
  }
  function openPlatformImport() { navigateTo("steam-import", { focus: "start" }); }
</script>

<PageShell as="div" width="full" scrollable={false} ariaLabel="游戏库" class="library-page-shell">
  <MediaWorkspaceShell
    mode={workspaceMode}
    {searching}
    count={gameStore.games.length}
    searchValue={gameStore.searchQuery}
    bind:searchInput
    healthLegacy={!libraryV2Enabled}
    {healthOpen}
    onModeChange={setWorkspaceMode}
    onSearchInput={(value) => (gameStore.searchQuery = value)}
    onClearSearch={() => { gameStore.searchQuery = ""; searchInput?.focus(); }}
    onOpenHealth={() => (healthOpen = true)}
    onImport={() => gameStore.importGame()}
  >
    {#snippet content()}
      <AsyncState
        state={libraryState}
        loadingDelayMs={0}
        preserveContent={libraryState === "refreshing" || libraryState === "partial"}
        title={libraryState === "empty" ? "还没有游戏" : libraryState === "error" ? "游戏库加载失败" : undefined}
        description={libraryState === "empty"
          ? "同步 Steam / Epic、添加本地游戏，开始建立你的游戏库。"
          : libraryState === "error"
            ? (gameStore.loadError ?? undefined)
            : libraryState === "partial"
              ? `部分游戏库数据未能刷新：${gameStore.loadError}`
              : undefined}
        primaryAction={libraryState === "empty"
          ? { label: "添加本地游戏", onSelect: () => gameStore.importGame() }
          : libraryState === "error" || libraryState === "partial"
            ? { label: "重试", onSelect: () => gameStore.load() }
            : undefined}
        secondaryAction={libraryState === "empty" ? { label: "Steam / Epic 导入", onSelect: openPlatformImport } : undefined}
        class="library-async-state"
      >
        {#snippet children()}
          {#if showGrid}
            <div class="index-workbench" data-module-style="cinematic" data-testid="all-games-panel">
              <section class="index-toolbar" aria-labelledby="library-page-title">
                {#snippet gridHeaderActions()}
                  <button class="header-action add-game" type="button" onclick={() => gameStore.importGame()}><Icon name="plus" size={15} /> 添加游戏</button>
                {/snippet}
                <PageHeader
                  id="library-page-title"
                  title={allGamesTitle}
                  description={`${gameStore.games.length} / ${allGames.length} 款 · ${activeFilterLabel}`}
                  actions={gridHeaderActions}
                  class="library-grid-header"
                />
                {#snippet filterControls()}
                  <div class="chips" role="group" aria-label="快速筛选">
                    {#each quickFilters as filter}
                      <button type="button" class:active={(gameStore.quickFilter ?? "") === filter.id} aria-pressed={(gameStore.quickFilter ?? "") === filter.id} onclick={() => (gameStore.quickFilter = filter.id || null)}>{filter.label}</button>
                    {/each}
                  </div>
                {/snippet}
                {#snippet filterActions()}
                  <div class="selects">
                    <div class="view-switch" role="group" aria-label="视图模式">
                      {#each viewModes as view}
                        <button type="button" class:active={uiStore.viewMode === view.id} aria-pressed={uiStore.viewMode === view.id} onclick={() => (uiStore.viewMode = view.id)}><Icon name={view.icon} size={14}/>{view.label}</button>
                      {/each}
                    </div>
                    <label><span>标签</span><select value={gameStore.filterTag ?? ""} onchange={(event) => (gameStore.filterTag = event.currentTarget.value || null)}><option value="">全部标签</option>{#each tagOptions as tag}<option value={tag.name}>{tag.name} · {tag.count}</option>{/each}</select></label>
                    <label><span>排序</span><select value={gameStore.sortBy} onchange={(event) => (gameStore.sortBy = event.currentTarget.value)}>{#each sortOptions as option}<option value={option.id}>{option.label}</option>{/each}</select></label>
                  </div>
                {/snippet}
                <FilterBar controls={filterControls} actions={filterActions} activeCount={activeFilterCount} onClear={clearGridFilters} busy={gameStore.loading} class="library-filter-bar" />
              </section>
              <div class="index-grid" data-testid="all-games-grid"><GameGrid /></div>
            </div>
          {:else if effectiveMode === "scene"}
            <section class="workspace-view" data-module-style="film-sequence" data-testid="switch-home-scene"><AdaptiveChromaStage src={adaptiveChromaSource} strength="immersive" style="height: 100%;"><GameSceneV2 items={presentationItems} selectedId={selected?.id ?? null} onAction={onMediaAction} onImport={() => gameStore.importGame()} /></AdaptiveChromaStage></section>
          {:else}
            <section class="workspace-view" data-module-style="cube-editorial" data-testid="switch-home-stage"><AdaptiveChromaStage src={adaptiveChromaSource} strength="balanced" style="height: 100%;"><GameVisualV2 items={presentationItems} selectedId={selected?.id ?? null} onAction={onMediaAction} onImport={() => gameStore.importGame()} /></AdaptiveChromaStage></section>
          {/if}
        {/snippet}
      </AsyncState>
    {/snippet}
  </MediaWorkspaceShell>
  <LibraryHealthPanel open={healthOpen} games={allGames} onClose={() => (healthOpen = false)} />
</PageShell>

<style>
  :global(.library-page-shell.v2-page-shell) { height: 100%; overflow: hidden; }
  :global(.library-page-shell > .v2-page-shell__inner) { height: 100%; max-width: none; padding: 0; display: flex; min-height: 0; }
  :global(.library-page-shell .library-async-state) { width: 100%; height: 100%; min-height: 0; }
  :global(.library-page-shell .library-async-state[data-state]:not([data-state="ready"])) { margin: 1.5rem; height: auto; }
  .workspace-view { width: 100%; height: 100%; min-height: 0; overflow: hidden; }
  .index-workbench { height: 100%; min-height: 0; display: grid; grid-template-rows: auto minmax(0, 1fr); background: rgb(5 7 10 / .92); }
  .index-toolbar { display: grid; gap: 10px; padding: 18px 26px 12px; border-bottom: 1px solid rgb(255 255 255 / .12); background: linear-gradient(90deg, rgb(var(--media-primary-rgb, 36 45 54) / .12), transparent 58%); }
  .index-grid { min-height: 0; display: flex; overflow: hidden; }
  :global(.library-grid-header .v2-page-header__title) { font-family: var(--font-display); font-size: clamp(1.5rem, 3vw, 2.4rem); letter-spacing: -.045em; }
  :global(.library-filter-bar.v2-filter-bar) { padding: .65rem; border-radius: 0; border-color: rgb(255 255 255 / .12); background: rgb(255 255 255 / .025); }
  .header-action { min-height: 2.3rem; display: inline-flex; align-items: center; gap: .4rem; padding: 0 .9rem; border: 1px solid rgb(255 255 255 / .14); border-radius: 0; background: transparent; color: var(--text-secondary); font: 650 .78rem/1 var(--font-ui); cursor: pointer; }
  .header-action.add-game { border-color: rgb(var(--media-accent-rgb, 232 85 127) / .72); background: rgb(var(--media-accent-rgb, 232 85 127)); color: rgb(var(--media-on-accent-rgb, 12 13 16)); }
  .chips { min-width: 0; display: flex; align-items: center; gap: 6px; overflow-x: auto; scrollbar-width: none; }
  .chips button { flex: 0 0 auto; min-height: 2rem; padding: 0 12px; border: 1px solid rgb(255 255 255 / .12); border-radius: 0; background: transparent; color: var(--text-secondary); font: 650 11px/1 var(--font-ui); cursor: pointer; }
  .chips button.active { color: rgb(var(--media-accent-rgb, 232 85 127)); border-color: rgb(var(--media-accent-rgb, 232 85 127) / .55); background: rgb(var(--media-accent-rgb, 232 85 127) / .08); }
  .selects { display: flex; align-items: end; gap: 8px; }
  .selects label { min-width: 8.5rem; display: grid; gap: 4px; }
  .selects label > span { color: var(--text-muted); font-size: 9px; font-family: var(--font-mono); letter-spacing: .1em; }
  .selects select { min-height: 32px; padding: 0 9px; border: 1px solid rgb(255 255 255 / .13); border-radius: 0; background: rgb(7 9 12 / .9); color: var(--text-primary); font: 11px/1 var(--font-ui); }
  .view-switch { display: inline-flex; border: 1px solid rgb(255 255 255 / .13); }
  .view-switch button { min-height: 32px; display: inline-flex; align-items: center; gap: 5px; padding: 0 9px; border: 0; border-right: 1px solid rgb(255 255 255 / .1); background: transparent; color: var(--text-muted); font: 650 11px/1 var(--font-ui); cursor: pointer; }
  .view-switch button.active { color: rgb(var(--media-accent-rgb, 232 85 127)); background: rgb(var(--media-accent-rgb, 232 85 127) / .09); }
  @media (max-width: 760px) { .index-toolbar { padding: 14px; } .selects { width: 100%; flex-wrap: wrap; } .selects label { flex: 1; min-width: 8rem; } .view-switch { width: 100%; } .view-switch button { flex: 1; justify-content: center; } }
</style>
