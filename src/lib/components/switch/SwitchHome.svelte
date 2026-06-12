<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore } from "../../stores/games.svelte";
  import { uiStore } from "../../stores/ui.svelte";
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
  import Icon from "../Icon.svelte";
  import EmptyState from "../EmptyState.svelte";
  import GameGrid from "../GameGrid.svelte";
  import TileRail from "./TileRail.svelte";
  import SystemDock from "./SystemDock.svelte";

  const STATUS: Record<string, string> = {
    not_started: "未开始", playing: "游玩中", completed: "已通关",
    on_hold: "搁置", dropped: "已弃坑", plan_to_play: "计划中", replaying: "重温中",
  };

  const dock = [
    { id: "discovery", label: "发现",   icon: "compass",  view: "discovery" },
    { id: "scraper",   label: "刮削",   icon: "star",     view: "scraper" },
    { id: "downloads", label: "下载",   icon: "download", view: "downloads" },
    { id: "backup",    label: "存档",   icon: "save",     view: "backup" },
    { id: "stats",     label: "统计",   icon: "chart",    view: "stats" },
    { id: "import",    label: "导入",   icon: "database", view: "steam-import" },
    { id: "emulator",  label: "模拟器", icon: "gamepad",  view: "emulator" },
    { id: "diag",      label: "诊断",   icon: "toolbox",  view: "diagnostics" },
    { id: "settings",  label: "设置",   icon: "gear",     view: "settings" },
    { id: "bigpic",    label: "大屏",   icon: "tv",       view: "__bigpicture" },
  ];

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

  const allGames = $derived(gameStore.allGames);
  const searching = $derived(!!gameStore.searchQuery.trim());
  const showGrid = $derived(uiStore.libraryMode === "all" || searching);
  const allGamesTitle = $derived(
    searching ? `搜索：${gameStore.searchQuery}` : uiStore.viewMode === "list" ? "列表视图" : uiStore.viewMode === "compact" ? "封面墙" : "全部游戏"
  );
  const activeFilterLabel = $derived(quickFilters.find((item) => item.id === (gameStore.quickFilter ?? ""))?.label ?? "全部");
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

  function dateOf(v: string | null | undefined): number {
    return v ? new Date(v).getTime() || 0 : 0;
  }

  const recent = $derived.by(() => {
    const played = allGames
      .filter((g) => gameLastPlayed(g))
      .sort((a, b) => dateOf(gameLastPlayed(b)) - dateOf(gameLastPlayed(a)));
    const fresh = allGames
      .filter((g) => !gameLastPlayed(g))
      .sort((a, b) => dateOf(b.created_at || b.add_date) - dateOf(a.created_at || a.add_date));
    return [...played, ...fresh].slice(0, 16);
  });

  const selected = $derived(gameStore.selectedGame ?? recent[0] ?? null);
  const clock = $derived(now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }));

  function metaLine(g: Game | null): string {
    if (!g) return "";
    const parts: string[] = [];
    const dev = gameDeveloperOf(g);
    if (dev) parts.push(dev);
    const year = releaseYearOf(g);
    if (year) parts.push(String(year));
    const st = STATUS[gameCompletionStatus(g)];
    if (st) parts.push(st);
    const secs = gameTotalSeconds(g);
    if (secs > 0) parts.push(formatPlayTime(secs));
    return parts.join("  ·  ");
  }

  // 确保有选中项，供详情/大屏共享
  $effect(() => {
    if (!gameStore.selectedGame && recent[0]) gameStore.selectGame(recent[0].id);
  });

  onMount(() => {
    const t = setInterval(() => (now = new Date()), 30_000);
    return () => clearInterval(t);
  });

  function onselect(id: string) { gameStore.selectGame(id); }
  function onactivate(id: string) {
    gameStore.selectGame(id);
    uiStore.libraryMode = "home";
    uiStore.currentView = "game-detail";
  }
  function onlaunch(id: string) {
    gameStore.launch(id);
    const g = allGames.find((x) => x.id === id);
    uiStore.notify(`正在启动 ${g?.name ?? "游戏"}…`);
  }
  function onfavorite() { if (selected) gameStore.toggleFavorite(selected.id); }
  function pickDock(view: string) {
    if (view === "__bigpicture") uiStore.setBigPicture(true);
    else uiStore.openDrawer(view);
  }
  function focusSearch() {
    searchInput?.focus();
  }
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
</script>

<div class="switch-home" data-testid="switch-home">
  <header class="topbar">
    <div class="brand">
      <span class="avatar">萌</span>
      <b>萌游</b>
    </div>
    <div class="spacer"></div>
    <label class="search">
      <Icon name="search" size={15} />
      <input bind:this={searchInput} type="text" placeholder="搜索游戏 / 厂商 / 标签" bind:value={gameStore.searchQuery} />
      {#if searching}
        <button class="clear" onclick={() => (gameStore.searchQuery = "")} aria-label="清空搜索">
          <Icon name="x" size={13} />
        </button>
      {/if}
    </label>
    <span class="clock">{clock}</span>
    <button class="icon-btn" title="设置" onclick={() => (uiStore.currentView = "settings")}>
      <Icon name="gear" size={17} />
    </button>
  </header>

  {#if gameStore.loadError}
    <div class="error-banner" role="alert">
      <Icon name="x" size={14} />
      <span>游戏库加载失败：{gameStore.loadError}</span>
      <button onclick={() => gameStore.load()}>重试</button>
    </div>
  {/if}

  {#if showGrid}
    <section class="all-panel" aria-label="全部游戏" data-testid="all-games-panel">
      <div class="all-head">
        {#if uiStore.libraryMode === "all" && !searching}
          <button class="back" onclick={() => (uiStore.libraryMode = "home")}>
            <Icon name="chevronLeft" size={16} /> 返回主屏
          </button>
        {/if}
        <div class="all-title">
          <h1>{allGamesTitle}</h1>
          <span class="count">{gameStore.games.length} / {allGames.length} 款 · {activeFilterLabel}</span>
        </div>
        {#if searching || gameStore.quickFilter || gameStore.filterTag || gameStore.sortBy !== "recent"}
          <button class="reset" onclick={clearGridFilters}>清除筛选</button>
        {/if}
      </div>

      <div class="filterbar" aria-label="全库筛选">
        <div class="chips" role="group" aria-label="快速筛选">
          {#each quickFilters as filter}
            <button
              class:active={(gameStore.quickFilter ?? "") === filter.id}
              onclick={() => (gameStore.quickFilter = filter.id || null)}
            >
              {filter.label}
            </button>
          {/each}
        </div>

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
                <Icon name={mode.icon} size={14} />
                <span>{mode.label}</span>
              </button>
            {/each}
          </div>

          <label>
            <span>标签</span>
            <select
              value={gameStore.filterTag ?? ""}
              onchange={(event) => (gameStore.filterTag = (event.target as HTMLSelectElement).value || null)}
            >
              <option value="">全部标签</option>
              {#each tagOptions as tag}
                <option value={tag.name}>{tag.name} ({tag.count})</option>
              {/each}
            </select>
          </label>

          <label>
            <span>排序</span>
            <select
              value={gameStore.sortBy}
              onchange={(event) => (gameStore.sortBy = (event.target as HTMLSelectElement).value)}
            >
              {#each sortOptions as option}
                <option value={option.id}>{option.label}</option>
              {/each}
            </select>
          </label>
        </div>
      </div>
    </section>
    <div class="all-grid" data-testid="all-games-grid"><GameGrid /></div>
  {:else if gameStore.loading && allGames.length === 0}
    <div class="rail-skel" aria-busy="true">
      {#each Array(7) as _}<div class="skeleton b"></div>{/each}
    </div>
  {:else if allGames.length === 0}
    <div class="empty-wrap" data-testid="switch-home-empty">
      <EmptyState
        title="还没有游戏"
        description="从旧版 MoeGame 一键迁移你的游戏库，或同步 Steam / Epic、添加本地游戏。"
        actionLabel="从旧版 MoeGame 导入"
        onAction={() => (uiStore.currentView = "migration")}
      />
      <div class="empty-actions">
        <button onclick={() => (uiStore.currentView = "steam-import")}>Steam / Epic 导入</button>
        <button onclick={() => gameStore.importGame()}>添加本地游戏</button>
      </div>
    </div>
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

      <div class="info">
        <h1 class="title">{selected?.name ?? ""}</h1>
        <p class="sub">{metaLine(selected)}</p>
        <div class="actions">
          <button class="play" onclick={() => selected && onlaunch(selected.id)}>
            <Icon name="play" size={18} /><span>开始游戏</span>
          </button>
          <button class:active={selected?.favorite} onclick={onfavorite}>
            <Icon name={selected?.favorite ? "heartFill" : "heart"} size={16} />
            <span>{selected?.favorite ? "已收藏" : "收藏"}</span>
          </button>
          <button onclick={() => selected && onactivate(selected.id)}>
            <Icon name="database" size={16} /><span>详情</span>
          </button>
        </div>
      </div>
    </section>
  {/if}

  <SystemDock items={dock} current={uiStore.currentView} onpick={pickDock} />
</div>

<style>
  .switch-home {
    height: 100%;
    display: flex;
    flex-direction: column;
    background:
      radial-gradient(ellipse at center, rgba(255,255,255,0.035) 0%, transparent 58%),
      var(--sw-bg);
    color: var(--text-primary);
    overflow: hidden;
  }

  .topbar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 28px;
    flex-shrink: 0;
  }
  .brand { display: flex; align-items: center; gap: 10px; }
  .brand .avatar {
    width: 30px; height: 30px;
    border-radius: 50%;
    background: var(--bg-elev);
    border: 1px solid var(--border-hover);
    display: grid; place-items: center;
    color: var(--accent); font-weight: 700; font-size: 13px;
  }
  .brand b { font-family: var(--font-display); font-size: 16px; }
  .spacer { flex: 1; }

  .search {
    display: flex; align-items: center; gap: 8px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    padding: 7px 14px;
    width: 280px;
    color: var(--text-muted);
  }
  .search input {
    flex: 1; border: none; background: none;
    color: var(--text-primary); outline: none;
    font-size: 13px; font-family: var(--font-ui);
  }
  .search .clear {
    border: none; background: none; cursor: pointer;
    color: var(--text-muted); display: grid; place-items: center; padding: 0;
  }
  .clock {
    font-family: var(--font-mono); font-variant-numeric: tabular-nums;
    color: var(--text-secondary); font-size: 14px;
  }
  .icon-btn {
    width: 34px; height: 34px;
    display: grid; place-items: center;
    border: none; background: var(--bg-elev);
    color: var(--text-secondary);
    border-radius: 50%; cursor: pointer;
    transition: color 0.18s ease;
  }
  .icon-btn:hover { color: var(--text-primary); }

  .error-banner {
    display: flex; align-items: center; gap: 10px;
    margin: 0 28px 8px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: rgba(248, 113, 113, 0.1);
    border: 1px solid rgba(248, 113, 113, 0.3);
    color: var(--color-error);
    font-size: 13px;
  }
  .error-banner button {
    margin-left: auto; border: none; cursor: pointer;
    background: var(--color-error); color: #fff;
    padding: 4px 12px; border-radius: var(--radius-full); font-size: 12px;
  }

  .stage { flex: 1; min-height: 0; display: flex; flex-direction: column; justify-content: center; }

  .info { padding: 0 8vw 10px; min-height: 124px; }
  .info .title {
    font-family: var(--font-display);
    font-size: clamp(26px, 3.4vw, 38px);
    font-weight: 700; line-height: 1.15;
    margin: 0 0 6px;
  }
  .info .sub { color: var(--text-muted); font-size: 13px; margin: 0 0 16px; min-height: 18px; }
  .actions { display: flex; gap: 12px; flex-wrap: wrap; }
  .actions button {
    display: inline-flex; align-items: center; gap: 8px;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    color: var(--text-secondary);
    padding: 10px 18px;
    border-radius: var(--radius-full);
    cursor: pointer; font-size: 13px; font-weight: 600;
    transition: transform 0.15s ease, background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
  }
  .actions button:hover { color: var(--text-primary); border-color: var(--border-hover); }
  .actions .play { background: var(--accent); border-color: transparent; color: #fff; }
  .actions .play:hover { background: var(--accent-hi); transform: translateY(-1px); }
  .actions button.active { color: var(--accent); }

  .rail-skel { display: flex; gap: 18px; padding: 44px 8vw; }
  .rail-skel .b { width: var(--sw-tile-width); aspect-ratio: 3 / 4; border-radius: var(--sw-tile-radius); flex: 0 0 auto; }

  .empty-wrap {
    flex: 1; min-height: 0;
    display: grid; place-content: center; justify-items: center;
    padding: 0 8vw;
  }
  .empty-actions { display: flex; gap: 10px; justify-content: center; margin-top: 14px; }
  .empty-actions button {
    border: 1px solid var(--border); background: var(--bg-elev);
    color: var(--text-secondary); cursor: pointer;
    padding: 9px 16px; border-radius: var(--radius-full); font-size: 13px; font-weight: 600;
    transition: color 0.18s ease, border-color 0.18s ease;
  }
  .empty-actions button:hover { color: var(--text-primary); border-color: var(--border-hover); }

  .all-panel {
    display: grid;
    gap: 12px;
    padding: 6px 28px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-void);
    flex-shrink: 0;
  }
  .all-head {
    min-width: 0;
    display: flex; align-items: center; gap: 16px;
  }
  .all-title { min-width: 0; display: grid; gap: 4px; }
  .all-head h1 { font-family: var(--font-display); font-size: 20px; font-weight: 700; margin: 0; }
  .all-head .count { color: var(--text-muted); font-size: 13px; }
  .all-head .back,
  .all-head .reset {
    display: inline-flex; align-items: center; gap: 4px;
    border: 1px solid var(--border); background: var(--bg-elev);
    color: var(--text-secondary); cursor: pointer;
    padding: 6px 12px; border-radius: var(--radius-full); font-size: 13px;
  }
  .all-head .reset { margin-left: auto; }
  .all-head .back:hover,
  .all-head .reset:hover { color: var(--text-primary); border-color: var(--border-hover); }
  .filterbar {
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }
  .chips {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .chips::-webkit-scrollbar { display: none; }
  .chips button {
    flex: 0 0 auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    padding: 7px 12px;
    color: var(--text-secondary);
    background: var(--bg-elev);
    font-size: 12px;
    font-weight: 650;
    cursor: pointer;
  }
  .chips button:hover { color: var(--text-primary); border-color: var(--border-hover); }
  .chips button.active {
    color: var(--accent);
    border-color: var(--accent-ring);
    background: var(--accent-lo);
  }
  .selects {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .view-switch {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-elev);
  }
  .view-switch button {
    min-height: 28px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: none;
    border-radius: 7px;
    padding: 0 9px;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 12px;
    font-weight: 650;
    cursor: pointer;
  }
  .view-switch button:hover {
    color: var(--text-primary);
  }
  .view-switch button.active {
    color: var(--accent);
    background: var(--accent-lo);
  }
  .selects label {
    min-width: 150px;
    display: grid;
    gap: 4px;
  }
  .selects span {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 650;
  }
  .selects select {
    min-height: 34px;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0 10px;
    color: var(--text-primary);
    background: var(--bg-elev);
    font: inherit;
    font-size: 12px;
    outline: none;
  }
  .selects select:focus-visible {
    border-color: var(--accent-ring);
    box-shadow: var(--focus-ring);
  }
  .all-grid { flex: 1; min-height: 0; display: flex; }

  @media (max-width: 760px) {
    .topbar {
      padding: 12px 14px;
      gap: 10px;
      flex-wrap: wrap;
    }
    .brand b { display: none; }
    .spacer { display: none; }
    .search {
      order: 3;
      width: 100%;
      padding: 7px 12px;
    }
    .clock { margin-left: auto; }
    .info {
      padding: 0 18px 8px;
      min-height: 150px;
    }
    .actions button {
      padding: 9px 13px;
    }
    .actions button span {
      max-width: 72px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .all-head {
      padding-inline: 14px;
      flex-wrap: wrap;
    }
    .all-panel {
      padding-inline: 14px;
    }
    .filterbar {
      align-items: stretch;
      flex-direction: column;
    }
    .selects {
      width: 100%;
      flex-direction: column;
      align-items: stretch;
    }
    .view-switch {
      justify-content: stretch;
      width: 100%;
    }
    .view-switch button {
      flex: 1 1 0;
      justify-content: center;
      padding-inline: 6px;
    }
    .selects label {
      min-width: 0;
    }
  }
</style>
