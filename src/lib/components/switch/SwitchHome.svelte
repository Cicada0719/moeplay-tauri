<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore } from "../../stores/games.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { navigateTo } from "../../stores/router.svelte";
  import { formatPlayTime } from "../../api";
  import {
    developerOf as gameDeveloperOf,
    gameCompletionStatus,
    gameLastPlayed,
    gameTotalSeconds,
    releaseYearOf,
    tagsOf,
  } from "../../utils/game";
  import type { Game } from "../../stores/games.svelte";
  import type { ViewState } from "../ui-v2";
  import { AsyncState, FilterBar, PageHeader, PageShell } from "../ui-v2";
  import Icon from "../Icon.svelte";
  import { fileSrc } from "../../utils";
  import { heroImageOf as gameHeroImageOf, hasHeroBackground } from "../../utils/game";
  import defaultLibraryBackdrop from "../../assets/default-library-backdrop.png";
  import { getThemePack, normalizeAppearance } from "../../theme-packs";
  import GameGrid from "../GameGrid.svelte";
  import TileRail from "./TileRail.svelte";
  import { settingsStore } from "../../stores/settings.svelte";
  import LibraryHealthPanel from "../library/LibraryHealthPanel.svelte";
  import { readLibraryV2Flag } from "../library/feature-flag";

  const STATUS: Record<string, string> = {
    not_started: "未开始", playing: "游玩中", completed: "已通关",
    on_hold: "搁置", dropped: "已弃坑", plan_to_play: "计划中", replaying: "重温中",
  };

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

  const allGames = $derived(gameStore.allGames);
  const searching = $derived(!!gameStore.searchQuery.trim());
  const showGrid = $derived(uiStore.libraryMode === "all" || searching);
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
  const bgArt = $derived(selected ? (fileSrc(gameHeroImageOf(selected)) ?? defaultLibraryBackdrop) : defaultLibraryBackdrop);
  const bgIsCover = $derived(!hasHeroBackground(selected));
  const appearance = $derived(normalizeAppearance(settingsStore.settings.appearance));
  const mascotEnabled = $derived(appearance.mascot_enabled);
  const mascotSrc = $derived(
    appearance.custom_mascot_path
      ? (fileSrc(appearance.custom_mascot_path) ?? getThemePack(appearance.theme_pack).mascot)
      : getThemePack(appearance.theme_pack).mascot,
  );

  function metaLine(game: Game | null): string {
    if (!game) return "";
    const parts: string[] = [];
    const developer = gameDeveloperOf(game);
    if (developer) parts.push(developer);
    const year = releaseYearOf(game);
    if (year) parts.push(String(year));
    const status = STATUS[gameCompletionStatus(game)];
    if (status) parts.push(status);
    const seconds = gameTotalSeconds(game);
    if (seconds > 0) parts.push(formatPlayTime(seconds));
    return parts.join("  ·  ");
  }

  $effect(() => {
    if (!gameStore.selectedGame && recent[0]) gameStore.selectGame(recent[0].id);
  });

  onMount(() => {
    libraryV2Enabled = readLibraryV2Flag();
    const timer = setInterval(() => (now = new Date()), 30_000);
    return () => clearInterval(timer);
  });

  function onselect(id: string) { gameStore.selectGame(id); }
  function onactivate(id: string) {
    gameStore.selectGame(id);
    uiStore.libraryMode = showGrid ? "all" : "home";
    navigateTo("game-detail", { entity: { kind: "game", id }, focus: "start" });
  }
  function onlaunch(id: string) {
    void gameStore.launch(id);
    const game = allGames.find((item) => item.id === id);
    uiStore.notify(`正在启动 ${game?.name ?? "游戏"}…`);
  }
  function onfavorite() { if (selected) void gameStore.toggleFavorite(selected.id); }
  function focusSearch() { searchInput?.focus(); }
  function closeSearchOrGrid() {
    if (gameStore.searchQuery) {
      gameStore.searchQuery = "";
      return;
    }
    if (uiStore.libraryMode === "all") uiStore.libraryMode = "home";
  }
  function clearGridFilters() {
    gameStore.searchQuery = "";
    gameStore.quickFilter = null;
    gameStore.filterTag = null;
    gameStore.sortBy = "recent";
  }
  function openPlatformImport() { navigateTo("steam-import", { focus: "start" }); }
  function openSettings() { navigateTo("settings", { focus: "start" }); }
</script>

<PageShell as="div" width="full" scrollable={false} ariaLabel="游戏库" class="library-page-shell">
  <div class="switch-home" class:grid-mode={showGrid} data-testid="switch-home">
    <div class="sw-bg" class:hidden={showGrid} aria-hidden="true">
      <div class="sw-bg-layer" class:cover={bgIsCover} style={`background-image:url("${bgArt}")`}></div>
      <div class="sw-bg-scrim"></div>
    </div>

    <header class="topbar">
      <div class="brand"><span class="avatar">萌</span><b>萌游</b></div>
      <div class="spacer"></div>
      <label class="search" for="library-search">
        <Icon name="search" size={15} />
        <input
          id="library-search"
          bind:this={searchInput}
          type="search"
          placeholder="搜索游戏 / 厂商 / 标签"
          aria-label="搜索游戏库"
          data-search-scope="home"
          data-route-search
          bind:value={gameStore.searchQuery}
        />
        {#if searching}
          <button class="clear" type="button" onclick={() => { gameStore.searchQuery = ""; searchInput?.focus(); }} aria-label="清空搜索">
            <Icon name="x" size={13} />
          </button>
        {/if}
      </label>
      <span class="clock">{clock}</span>
      <button
        class="icon-btn health-btn"
        class:legacy={!libraryV2Enabled}
        title={libraryV2Enabled ? "检查库健康" : "库健康（只读；导入仍使用旧流程）"}
        aria-haspopup="dialog"
        aria-expanded={healthOpen}
        onclick={() => (healthOpen = true)}
      >
        <Icon name="database" size={16} /><span>库健康</span>
      </button>
      <button class="icon-btn" type="button" title="设置" aria-label="打开设置" onclick={openSettings}>
        <Icon name="gear" size={17} />
      </button>
    </header>

    {#snippet libraryContent()}
      {#if showGrid}
        <section class="all-panel" aria-label="全部游戏" data-testid="all-games-panel">
          {#snippet gridHeaderActions()}
            {#if uiStore.libraryMode === "all" && !searching}
              <button class="header-action back" type="button" onclick={() => (uiStore.libraryMode = "home")}>
                <Icon name="chevronLeft" size={16} /> 返回主屏
              </button>
            {/if}
            <button class="header-action add-game" type="button" onclick={() => gameStore.importGame()}>
              <Icon name="plus" size={15} /> 添加游戏
            </button>
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
                <button
                  type="button"
                  class:active={(gameStore.quickFilter ?? "") === filter.id}
                  aria-pressed={(gameStore.quickFilter ?? "") === filter.id}
                  onclick={() => (gameStore.quickFilter = filter.id || null)}
                >{filter.label}</button>
              {/each}
            </div>
          {/snippet}

          {#snippet filterActions()}
            <div class="selects">
              <div class="view-switch" role="group" aria-label="视图模式">
                {#each viewModes as mode}
                  <button
                    type="button"
                    class:active={uiStore.viewMode === mode.id}
                    aria-pressed={uiStore.viewMode === mode.id}
                    title={mode.label}
                    onclick={() => (uiStore.viewMode = mode.id)}
                  >
                    <Icon name={mode.icon} size={14} /><span>{mode.label}</span>
                  </button>
                {/each}
              </div>
              <label><span>标签</span>
                <select
                  aria-label="按标签筛选"
                  value={gameStore.filterTag ?? ""}
                  onchange={(event) => (gameStore.filterTag = (event.target as HTMLSelectElement).value || null)}
                >
                  <option value="">全部标签</option>
                  {#each tagOptions as tag}<option value={tag.name}>{tag.name} ({tag.count})</option>{/each}
                </select>
              </label>
              <label><span>排序</span>
                <select
                  aria-label="游戏排序"
                  value={gameStore.sortBy}
                  onchange={(event) => (gameStore.sortBy = (event.target as HTMLSelectElement).value)}
                >
                  {#each sortOptions as option}<option value={option.id}>{option.label}</option>{/each}
                </select>
              </label>
            </div>
          {/snippet}

          <FilterBar
            controls={filterControls}
            actions={filterActions}
            activeCount={activeFilterCount}
            onClear={clearGridFilters}
            busy={gameStore.loading}
            class="library-filter-bar"
          />
        </section>
        <div class="all-grid" data-testid="all-games-grid"><GameGrid /></div>
      {:else}
        <section class="stage" data-testid="switch-home-stage">
          <TileRail
            items={recent}
            selectedId={selected?.id ?? null}
            {onselect}
            {onactivate}
            {onlaunch}
            {onfavorite}
            onshowall={() => (uiStore.libraryMode = "all")}
            onfocussearch={focusSearch}
            onback={closeSearchOrGrid}
            onbigpicture={() => uiStore.setBigPicture(true)}
          />

          {#snippet stageActions()}
            <button class="play" type="button" onclick={() => selected && onlaunch(selected.id)}>
              <Icon name="play" size={18} /><span>开始游戏</span>
            </button>
            <button type="button" class:active={selected?.favorite} onclick={onfavorite}>
              <Icon name={selected?.favorite ? "heartFill" : "heart"} size={16} />
              <span>{selected?.favorite ? "已收藏" : "收藏"}</span>
            </button>
            <button type="button" onclick={() => selected && onactivate(selected.id)}>
              <Icon name="database" size={16} /><span>详情</span>
            </button>
          {/snippet}

          <div class="info">
            <div class="info-card">
              <PageHeader
                title={selected?.name ?? "游戏库"}
                description={metaLine(selected)}
                actions={stageActions}
                class="library-stage-header"
              />
            </div>
          </div>
        </section>

        {#if mascotEnabled}<img class="home-mascot" src={mascotSrc} alt="" aria-hidden="true" />{/if}
      {/if}
    {/snippet}

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
      {#snippet children()}{@render libraryContent()}{/snippet}
    </AsyncState>

    <LibraryHealthPanel open={healthOpen} games={allGames} onClose={() => (healthOpen = false)} />
  </div>
</PageShell>

<style>
  :global(.library-page-shell.v2-page-shell) { height: 100%; overflow: hidden; }
  :global(.library-page-shell > .v2-page-shell__inner) { height: 100%; max-width: none; padding: 0; display: flex; min-height: 0; }
  :global(.library-page-shell .library-async-state[data-state]:not([data-state="ready"])) { flex: 1; min-height: 0; margin: 1.5rem; }
  :global(.library-grid-header .v2-page-header__title) { font-family: var(--font-display); font-size: 1.25rem; }
  :global(.library-stage-header .v2-page-header__title) { font-family: var(--font-display); font-size: clamp(1.65rem, 3.4vw, 2.4rem); text-shadow: 0 2px 16px rgba(0,0,0,.45); }
  :global(.library-stage-header .v2-page-header__description) { color: var(--text-muted); text-shadow: 0 1px 10px rgba(0,0,0,.45); }
  :global(.library-stage-header .v2-page-header__actions) { justify-content: flex-start; }
  :global(.library-filter-bar.v2-filter-bar) { padding: .65rem; background: rgba(10,20,15,.72); }

  .switch-home {
    position: relative; height: 100%; min-width: 0; min-height: 0; display: flex; flex-direction: column;
    overflow: hidden; color: var(--text-primary);
    background: radial-gradient(ellipse at center, rgba(255,255,255,.035), transparent 58%), var(--sw-bg);
  }
  .switch-home.grid-mode { background: linear-gradient(180deg, #07140d 0%, #041008 48%, #020503 100%); }

  .sw-bg { position: absolute; inset: 0; z-index: 0; pointer-events: none; transition: opacity .2s ease; }
  .sw-bg.hidden { opacity: 0; visibility: hidden; }
  .sw-bg-layer { position: absolute; inset: 0; background-size: cover; background-position: center 28%; transition: background-image .4s ease; }
  .sw-bg-layer.cover { background-size: 140%; background-position: center 20%; filter: blur(20px) brightness(.7); }
  .sw-bg-scrim {
    position: absolute; inset: 0;
    background: linear-gradient(180deg, rgba(7,9,15,.35) 0%, rgba(7,9,15,.06) 28%, rgba(7,9,15,.06) 55%, rgba(7,9,15,.62) 82%, rgba(7,9,15,.92) 100%);
  }

  .topbar, .stage, .all-panel, .all-grid { position: relative; z-index: 1; }
  .topbar { display: flex; align-items: center; gap: 14px; padding: 16px 28px; flex-shrink: 0; }
  .brand { display: flex; align-items: center; gap: 10px; }
  .brand .avatar {
    width: 30px; height: 30px; display: grid; place-items: center; border: 1px solid var(--border-hover);
    border-radius: 50%; background: var(--bg-elev); color: var(--accent); font-size: 13px; font-weight: 700;
  }
  .brand b { font-family: var(--font-display); font-size: 16px; }
  .spacer { flex: 1; }

  .search {
    width: min(22rem, 34vw); display: flex; align-items: center; gap: 8px; padding: 7px 14px;
    border: 1px solid var(--border); border-radius: var(--radius-full); background: var(--bg-elev); color: var(--text-muted);
  }
  .search:focus-within { border-color: var(--accent-ring); box-shadow: var(--focus-ring); }
  .search input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--text-primary); font: 13px/1.4 var(--font-ui); }
  .search input::-webkit-search-cancel-button { display: none; }
  .clear { display: grid; place-items: center; padding: 0; border: 0; background: transparent; color: var(--text-muted); cursor: pointer; }
  .clock { color: var(--text-secondary); font: 14px/1 var(--font-mono); font-variant-numeric: tabular-nums; }

  .icon-btn {
    width: 34px; height: 34px; display: grid; place-items: center; padding: 0; border: 0; border-radius: 50%;
    background: var(--bg-elev); color: var(--text-secondary); cursor: pointer;
  }
  .icon-btn:hover { color: var(--text-primary); }
  .icon-btn:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  .health-btn { width: auto; min-width: 34px; padding: 0 11px; display: inline-flex; gap: 6px; border: 1px solid var(--border); border-radius: var(--radius-full); font: 650 11px/1 var(--font-ui); }
  .health-btn:not(.legacy) { color: var(--accent); border-color: var(--accent-ring); background: var(--accent-lo); }
  .health-btn.legacy { color: var(--text-muted); }

  .all-panel { display: grid; gap: 12px; padding: 6px 28px 12px; border-bottom: 1px solid var(--border); background: linear-gradient(180deg, #07140d, #041008); flex-shrink: 0; }
  .all-grid { flex: 1; min-height: 0; display: flex; background: linear-gradient(180deg, #041008, #020503); }
  .header-action {
    min-height: 2.25rem; display: inline-flex; align-items: center; gap: .35rem; padding: 0 .8rem;
    border: 1px solid var(--border); border-radius: var(--radius-full); background: var(--bg-elev);
    color: var(--text-secondary); font: 650 .8rem/1 var(--font-ui); cursor: pointer;
  }
  .header-action:hover { color: var(--text-primary); border-color: var(--border-hover); }
  .header-action.add-game { border-color: var(--accent-ring); background: var(--accent); color: white; }

  .chips { min-width: 0; display: flex; align-items: center; gap: 8px; overflow-x: auto; scrollbar-width: none; }
  .chips::-webkit-scrollbar { display: none; }
  .chips button {
    flex: 0 0 auto; min-height: 2rem; padding: 0 12px; border: 1px solid var(--border); border-radius: var(--radius-full);
    background: var(--bg-elev); color: var(--text-secondary); font: 650 12px/1 var(--font-ui); cursor: pointer;
  }
  .chips button:hover { color: var(--text-primary); border-color: var(--border-hover); }
  .chips button.active { color: var(--accent); border-color: var(--accent-ring); background: var(--accent-lo); }
  .chips button:focus-visible { outline: none; box-shadow: var(--focus-ring); }

  .selects { flex: 0 0 auto; display: flex; align-items: center; gap: 10px; }
  .view-switch { display: inline-flex; align-items: center; gap: 2px; padding: 3px; border: 1px solid var(--border); border-radius: 10px; background: var(--bg-elev); }
  .view-switch button {
    min-height: 28px; display: inline-flex; align-items: center; gap: 5px; padding: 0 9px;
    border: 0; border-radius: 7px; background: transparent; color: var(--text-muted); font: 650 12px/1 var(--font-ui); cursor: pointer;
  }
  .view-switch button:hover { color: var(--text-primary); }
  .view-switch button.active { color: var(--accent); background: var(--accent-lo); }
  .view-switch button:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  .selects label { min-width: 9.25rem; display: grid; gap: 4px; }
  .selects label > span { color: var(--text-muted); font-size: 11px; font-weight: 650; }
  .selects select { min-height: 34px; padding: 0 10px; border: 1px solid var(--border); border-radius: 8px; outline: 0; background: var(--bg-elev); color: var(--text-primary); font: 12px/1 var(--font-ui); }
  .selects select:focus-visible { border-color: var(--accent-ring); box-shadow: var(--focus-ring); }

  .stage { flex: 1; min-height: 0; display: flex; flex-direction: column; justify-content: center; }
  .info { min-height: 124px; padding: 0 8vw 16px; position: relative; z-index: 2; }
  .info-card { display: inline-block; max-width: min(54rem, 82vw); padding: 18px 22px; border: 1px solid rgba(255,255,255,.08); border-radius: var(--radius-lg); background: rgba(10,13,20,.42); backdrop-filter: blur(12px); box-shadow: 0 14px 40px rgba(0,0,0,.25); }
  .info-card :global(.v2-page-header) { align-items: center; }
  .info-card :global(.v2-page-header__actions button) {
    min-height: 2.5rem; display: inline-flex; align-items: center; gap: .5rem; padding: 0 1rem;
    border: 1px solid var(--border); border-radius: var(--radius-full); background: var(--bg-elev);
    color: var(--text-secondary); font: 600 13px/1 var(--font-ui); cursor: pointer;
  }
  .info-card :global(.v2-page-header__actions button:hover) { color: var(--text-primary); border-color: var(--border-hover); }
  .info-card :global(.v2-page-header__actions button.play) { border-color: transparent; background: var(--accent); color: white; }
  .info-card :global(.v2-page-header__actions button.active) { color: var(--accent); }

  .home-mascot { position: absolute; right: -2vw; bottom: 0; z-index: 0; height: min(72vh, 520px); max-width: 45vw; object-fit: contain; object-position: right bottom; pointer-events: none; mask-image: linear-gradient(to top, transparent, black 18%); filter: drop-shadow(0 0 28px rgba(0,0,0,.25)); }

  @media (max-width: 760px) {
    .topbar { padding: 12px 14px; gap: 10px; flex-wrap: wrap; }
    .brand b, .health-btn span { display: none; }
    .spacer { display: none; }
    .search { order: 3; width: 100%; }
    .clock { margin-left: auto; }
    .health-btn { padding-inline: 0; justify-content: center; }
    .all-panel { padding-inline: 14px; }
    :global(.library-filter-bar.v2-filter-bar) { align-items: stretch; }
    .selects { width: 100%; flex-direction: column; align-items: stretch; }
    .view-switch { width: 100%; }
    .view-switch button { flex: 1; justify-content: center; padding-inline: 6px; }
    .selects label { min-width: 0; }
    .info { min-height: 150px; padding: 0 18px 12px; }
    .info-card { display: block; max-width: none; padding: 14px 16px; }
    .info-card :global(.v2-page-header) { align-items: flex-start; }
    .home-mascot { right: -4vw; height: min(48vh, 280px); max-width: 58vw; }
  }

  @media (prefers-reduced-motion: reduce) {
    .sw-bg, .sw-bg-layer, .home-mascot { transition: none; }
  }
</style>
