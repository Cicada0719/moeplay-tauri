<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore, type Game } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { fileSrc } from "../utils";
  import { formatPlayTime } from "../api";
  import CachedImage from "./CachedImage.svelte";
  import EmptyState from "./EmptyState.svelte";
  import GameGrid from "./GameGrid.svelte";
  import Icon from "./Icon.svelte";
  import Rail from "./ui/Rail.svelte";
  import defaultLibraryBackdrop from "../assets/default-library-backdrop.png";
  import {
    coverOf as gameCoverOf,
    developerOf as gameDeveloperOf,
    gameCompletionStatus,
    gameLastPlayed,
    gameRating,
    gameTotalSeconds,
    heroImageOf as gameHeroImageOf,
    platformOf as gamePlatformOf,
    releaseYearOf,
    tagsOf as gameTagsOf,
  } from "../utils/game";
  import WhatToPlay from "./WhatToPlay.svelte";
  import SmartCollectionEditor from "./SmartCollectionEditor.svelte";

  let showWhatToPlay = $state(false);
  let showCollectionEditor = $state(false);
  let editingCollection = $state<import("../stores/games.svelte").SmartCollection | null>(null);
  let initialCollectionFilters = $state<import("../stores/games.svelte").SmartCollection["filters"] | undefined>();

  let railEl = $state<HTMLDivElement>();
  let now = $state(new Date());

  const games = $derived(gameStore.games);
  const allGames = $derived(gameStore.allGames);
  const focusGame = $derived.by(() => {
    const selected = gameStore.selectedGame;
    if (selected && games.some((game) => game.id === selected.id)) return selected;
    return games[0] ?? allGames[0] ?? null;
  });
  const focusIndex = $derived(focusGame ? games.findIndex((game) => game.id === focusGame.id) : -1);

  const topGames = $derived.by(() =>
    [...games]
      .sort((a, b) => {
        const aPlayed = dateValue(lastPlayedOf(a));
        const bPlayed = dateValue(lastPlayedOf(b));
        if (aPlayed !== bPlayed) return bPlayed - aPlayed;
        return secondsOf(b) - secondsOf(a);
      })
      .slice(0, 18)
  );

  const recentPlayed = $derived.by(() =>
    allGames
      .filter((game) => lastPlayedOf(game))
      .sort((a, b) => dateValue(lastPlayedOf(b)) - dateValue(lastPlayedOf(a)))
      .slice(0, 4)
  );

  const recentAdded = $derived.by(() =>
    [...allGames]
      .sort((a, b) => createdValue(b) - createdValue(a))
      .slice(0, 14)
  );

  const favoriteGames = $derived.by(() =>
    allGames
      .filter((game) => game.favorite)
      .sort((a, b) => dateValue(lastPlayedOf(b)) - dateValue(lastPlayedOf(a)))
      .slice(0, 14)
  );

  const platformRails = $derived.by(() => {
    const groups = new Map<string, Game[]>();
    for (const game of allGames) {
      const key = platformOf(game);
      groups.set(key, [...(groups.get(key) ?? []), game]);
    }
    return [...groups.entries()]
      .sort((a, b) => b[1].length - a[1].length)
      .slice(0, 3)
      .map(([label, items]) => ({
        label,
        items: items
          .sort((a, b) => dateValue(lastPlayedOf(b)) - dateValue(lastPlayedOf(a)))
          .slice(0, 12),
      }));
  });

  const totalGames = $derived(allGames.length);
  const installedCount = $derived(allGames.filter((game) => Boolean(game.exe_path)).length);
  const completedCount = $derived(allGames.filter((game) => normalizeStatus(game) === "completed").length);
  const playingCount = $derived(allGames.filter((game) => normalizeStatus(game) === "playing").length);
  const favoriteCount = $derived(allGames.filter((game) => game.favorite).length);
  const totalSeconds = $derived(allGames.reduce((sum, game) => sum + secondsOf(game), 0));
  const totalHours = $derived(Math.round(totalSeconds / 3600));
  const weekSeconds = $derived(sumRecentSeconds(7));
  const weekHours = $derived((weekSeconds / 3600).toFixed(1));
  const completionRate = $derived(totalGames ? Math.round((completedCount / totalGames) * 100) : 0);
  const metadataCoverage = $derived(totalGames ? Math.round((allGames.filter(hasCoreMetadata).length / totalGames) * 100) : 0);
  const achievementRate = $derived(achievementProgress(focusGame));
  const focusRating = $derived(ratingOf(focusGame));
  const heroImage = $derived(heroImageOf(focusGame) ?? defaultLibraryBackdrop);
  const clock = $derived(now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }));
  const heroStyle = $derived(heroImage ? `background-image: url("${heroImage}")` : "");

  const navItems = [
    { id: "home", label: "游戏" },
    { id: "stats", label: "统计" },
    { id: "downloads", label: "下载" },
    { id: "backup", label: "存档" },
    { id: "scraper", label: "资料" },
    { id: "settings", label: "设置" },
  ];

  const filterChips = [
    { id: null, label: "全部" },
    { id: "recent", label: "最近" },
    { id: "installed", label: "已安装" },
    { id: "playing", label: "游玩中" },
    { id: "completed", label: "已通关" },
    { id: "favorite", label: "收藏" },
    { id: "missing_metadata", label: "待补全" },
  ] as const;

  const sortOptions = [
    { id: "recent", label: "最近游玩" },
    { id: "name", label: "名称" },
    { id: "rating", label: "评分" },
    { id: "playtime", label: "游玩时长" },
    { id: "added", label: "添加时间" },
  ];

  onMount(() => {
    const timer = window.setInterval(() => {
      now = new Date();
    }, 30_000);

    return () => window.clearInterval(timer);
  });

  $effect(() => {
    if (!gameStore.selectedGame && games[0]) {
      gameStore.selectGame(games[0].id);
    }
  });

  $effect(() => {
    if (!railEl || !focusGame) return;
    const node = railEl.querySelector<HTMLElement>(`[data-game-id="${focusGame.id}"]`);
    node?.scrollIntoView({ inline: "center", block: "nearest", behavior: "smooth" });
  });

  function normalizeStatus(game: Game): string {
    return gameCompletionStatus(game);
  }

  function secondsOf(game: Game): number {
    return gameTotalSeconds(game);
  }

  function lastPlayedOf(game: Game): string | null {
    return gameLastPlayed(game);
  }

  function dateValue(value: string | null | undefined): number {
    return value ? new Date(value).getTime() || 0 : 0;
  }

  function createdValue(game: Game): number {
    return dateValue(game.created_at || game.add_date || game.updated_at);
  }

  function ratingOf(game: Game | null): number {
    if (!game) return 0;
    return gameRating(game);
  }

  function yearOf(game: Game | null): string {
    if (!game) return "----";
    return String(releaseYearOf(game) ?? "----");
  }

  function developerOf(game: Game | null): string {
    if (!game) return "";
    return gameDeveloperOf(game);
  }

  function platformOf(game: Game | null): string {
    if (!game) return "Local";
    return gamePlatformOf(game);
  }

  function coverOf(game: Game | null): string | null {
    return fileSrc(coverSourceOf(game));
  }

  function coverSourceOf(game: Game | null): string | null {
    return gameCoverOf(game);
  }

  function heroImageOf(game: Game | null): string | null {
    return fileSrc(gameHeroImageOf(game));
  }

  function monogramOf(game: Game | null): string {
    return (game?.name?.trim()?.[0] ?? "M").toUpperCase();
  }

  function allTags(game: Game | null): string[] {
    return gameTagsOf(game);
  }

  function hasCoreMetadata(game: Game): boolean {
    return Boolean(
      gameCoverOf(game) &&
      game.description &&
      allTags(game).length > 0 &&
      gameDeveloperOf(game)
    );
  }

  function achievementProgress(game: Game | null): number {
    const total = game?.play_tracker?.achievements_total ?? 0;
    const unlocked = game?.play_tracker?.achievements_unlocked ?? 0;
    return total > 0 ? Math.round((unlocked / total) * 100) : 0;
  }

  function sumRecentSeconds(days: number): number {
    const since = Date.now() - days * 86400000;
    return allGames.reduce((sum, game) => {
      const sessions = game.play_tracker?.sessions ?? [];
      return sum + sessions.reduce((inner, session) => {
        const start = dateValue(session.start_time);
        return start >= since ? inner + (session.duration_seconds ?? 0) : inner;
      }, 0);
    }, 0);
  }

  function timeAgo(value: string | null | undefined): string {
    if (!value) return "尚未游玩";
    const diff = Math.max(0, Date.now() - dateValue(value));
    const days = Math.floor(diff / 86400000);
    if (days === 0) return "今天";
    if (days === 1) return "昨天";
    if (days < 7) return `${days} 天前`;
    if (days < 30) return `${Math.floor(days / 7)} 周前`;
    return `${Math.floor(days / 30)} 个月前`;
  }

  function statusLabel(game: Game | null): string {
    const status = game ? normalizeStatus(game) : "";
    if (status === "completed") return "已通关";
    if (status === "playing") return "游玩中";
    if (status === "on_hold") return "搁置";
    if (status === "dropped") return "已弃坑";
    if (status === "replaying") return "重温中";
    if (status === "plan_to_play") return "计划中";
    return "未开始";
  }

  function selectGame(game: Game) {
    gameStore.selectGame(game.id);
  }

  function moveFocus(delta: number) {
    if (!games.length) return;
    const current = focusIndex >= 0 ? focusIndex : 0;
    const next = Math.max(0, Math.min(games.length - 1, current + delta));
    selectGame(games[next]);
  }

  async function launchFocus() {
    if (!focusGame) return;
    await gameStore.launch(focusGame.id);
    uiStore.notify(`正在启动 ${focusGame.name}...`, "info");
  }

  async function toggleFavorite() {
    if (!focusGame) return;
    await gameStore.toggleFavorite(focusGame.id);
  }

  function openScrape() {
    if (!focusGame) return;
    uiStore.openScrapeDialog(focusGame.id);
  }

  function openDetail() {
    if (!focusGame) return;
    gameStore.selectGame(focusGame.id);
    uiStore.currentView = "game-detail";
  }

  function clearSearch() {
    gameStore.searchQuery = "";
  }

  async function batchToggleFavorite() {
    const n = await gameStore.batchToggleFavorite();
    uiStore.notify(`已切换 ${n} 个游戏的收藏状态`, "success");
  }
  async function batchToggleHidden() {
    const n = await gameStore.batchToggleHidden();
    uiStore.notify(`已切换 ${n} 个游戏的隐藏状态`, "success");
  }
  async function promptBatchTag() {
    const tag = prompt("请输入要添加的标签：");
    if (!tag?.trim()) return;
    const n = await gameStore.batchAddTag(tag.trim());
    uiStore.notify(`已为 ${n} 个游戏添加标签「${tag.trim()}」`, "success");
  }
  async function confirmBatchDelete() {
    const count = gameStore.selectedIds.size;
    if (!confirm(`确定要删除选中的 ${count} 个游戏吗？此操作不可撤销。`)) return;
    const n = await gameStore.batchDelete();
    uiStore.notify(`已删除 ${n} 个游戏`, "success");
  }
  async function batchSetStatus(status: string) {
    const n = await gameStore.batchSetStatus(status as any);
    uiStore.notify(`已设置 ${n} 个游戏的状态`, "success");
  }
  function toggleSelectionMode() {
    if (gameStore.selectionMode) {
      gameStore.clearSelection();
    } else {
      // Enter selection mode by toggling first game
      if (allGames.length > 0) gameStore.toggleSelection(allGames[0].id);
    }
  }

  function saveCurrentFilter() {
    editingCollection = null;
    initialCollectionFilters = {
      quickFilter: gameStore.quickFilter,
      filterTag: gameStore.filterTag,
      searchQuery: gameStore.searchQuery || undefined,
      sortBy: gameStore.sortBy,
    };
    showCollectionEditor = true;
  }
  function editCollection(col: import("../stores/games.svelte").SmartCollection) {
    editingCollection = col;
    initialCollectionFilters = undefined;
    showCollectionEditor = true;
  }
</script>

{#snippet railTile(game: Game)}
  <button
    class="game-tile"
    class:active={focusGame?.id === game.id}
    data-game-id={game.id}
    data-testid="game-rail-tile"
    onclick={() => selectGame(game)}
    ondblclick={() => gameStore.launch(game.id)}
    title={game.name}
    type="button"
  >
    <span class="tile-art">
      {#if coverSourceOf(game)}
        <CachedImage source={coverSourceOf(game)} cacheKey={`home-rail-${game.id}`} alt={game.name} loading="lazy" />
      {:else}
        <span>{monogramOf(game)}</span>
      {/if}
    </span>
    <span class="tile-copy">
      <strong>{game.name}</strong>
      <small>{timeAgo(lastPlayedOf(game))} / {formatPlayTime(secondsOf(game))}</small>
    </span>
    {#if game.favorite}
      <span class="favorite-dot"><Icon name="heartFill" size={12} /></span>
    {/if}
  </button>
{/snippet}

<section class="console-dashboard" class:has-art={Boolean(heroImage)} data-testid="library-shell">
  <div class="hero-backdrop" style={heroStyle}>
    <div class="hero-shade"></div>
  </div>

  <header class="console-topbar">
    <div class="brand-group">
      <button class="brand-mark" onclick={() => (uiStore.currentView = "home")} type="button" title="游戏主页">
        M
      </button>
      <nav class="console-nav" aria-label="主导航">
        {#each navItems as item}
          <button
            class:active={uiStore.currentView === item.id}
            onclick={() => (uiStore.currentView = item.id)}
            type="button"
          >
            {item.label}
          </button>
        {/each}
      </nav>
    </div>

    <div class="system-group">
      <label class="search-box" aria-label="搜索游戏">
        <Icon name="search" size={15} />
        <input
          type="text"
          placeholder="搜索游戏、标签、厂商、平台"
          bind:value={gameStore.searchQuery}
        />
        {#if gameStore.searchQuery}
          <button class="clear-search" onclick={clearSearch} aria-label="清空搜索" type="button">
            <Icon name="x" size={13} />
          </button>
        {/if}
      </label>

      <button class="system-btn" onclick={() => gameStore.importGame()} title="添加本地游戏" type="button">
        <Icon name="plus" size={18} />
      </button>
      <span class="clock">{clock}</span>
    </div>
  </header>

  {#if gameStore.loadError}
    <div class="load-error-banner" role="alert">
      <Icon name="x" size={14} />
      <span>游戏库加载失败：{gameStore.loadError}</span>
      <button onclick={() => gameStore.load()} type="button">重试</button>
    </div>
  {/if}

  {#if gameStore.selectionMode}
    <div class="batch-bar">
      <span class="batch-count">已选 {gameStore.selectedIds.size} 个游戏</span>
      <div class="batch-actions">
        <button class="batch-btn" onclick={() => gameStore.selectAll()} title="全选">
          <Icon name="check" size={14} /> 全选
        </button>
        <button class="batch-btn" onclick={batchToggleFavorite} title="批量收藏/取消">
          <Icon name="heart" size={14} /> 收藏
        </button>
        <button class="batch-btn" onclick={batchToggleHidden} title="批量隐藏">
          <Icon name="eyeOff" size={14} /> 隐藏
        </button>
        <button class="batch-btn" onclick={promptBatchTag} title="批量打标签">
          <Icon name="tag" size={14} /> 标签
        </button>
        <select class="batch-select" onchange={(e) => batchSetStatus((e.target as HTMLSelectElement).value)} title="设置状态">
          <option value="">设置状态...</option>
          <option value="playing">游玩中</option>
          <option value="completed">已通关</option>
          <option value="on_hold">搁置</option>
          <option value="dropped">已放弃</option>
          <option value="plan_to_play">计划中</option>
          <option value="replaying">重温中</option>
        </select>
        <button class="batch-btn danger" onclick={confirmBatchDelete} title="批量删除">
          <Icon name="trash" size={14} /> 删除
        </button>
      </div>
      <button class="batch-cancel" onclick={() => gameStore.clearSelection()}>
        <Icon name="x" size={14} /> 取消
      </button>
    </div>
  {/if}

  {#if gameStore.loading && allGames.length === 0}
    <div class="loading-panel glass-card" data-testid="library-loading">
      <Icon name="refresh" size={20} />
      <span>正在读取游戏库...</span>
    </div>
  {:else if !gameStore.loading && allGames.length === 0}
    <div class="empty-console" data-testid="library-empty">
      <EmptyState
        title="准备建立你的主机桌面"
        description="先导入本地游戏，或同步 Steam / Epic 库。导入后这里会显示继续游玩、统计、存档和资源状态。"
        actionLabel="添加第一款游戏"
        onAction={() => gameStore.importGame()}
      />
      <div class="empty-actions">
        <button onclick={() => (uiStore.currentView = "steam-import")} type="button">Steam / Epic 导入</button>
        <button onclick={() => (uiStore.currentView = "emulator")} type="button">导入模拟器游戏</button>
      </div>
    </div>
  {:else if uiStore.viewMode === "list" || uiStore.viewMode === "compact"}
    <div class="library-fallback">
      <div class="fallback-head">
        <div>
          <span class="section-kicker">Library View</span>
          <h1>{uiStore.viewMode === "list" ? "列表视图" : "封面墙"}</h1>
        </div>
        <button onclick={() => (uiStore.viewMode = "grid")} type="button">返回主机主页</button>
      </div>
      <GameGrid />
    </div>
  {:else}
    <main class="dashboard-main" data-testid="library-dashboard">
      <section class="command-row">
        <div class="filter-strip" aria-label="快捷筛选">
          {#each filterChips as chip}
            <button
              class:active={gameStore.quickFilter === chip.id && !gameStore.activeCollectionId}
              onclick={() => { gameStore.activateCollection(null); gameStore.quickFilter = chip.id; }}
              type="button"
            >
              {chip.label}
            </button>
          {/each}
          {#if gameStore.smartCollections.length > 0}
            <span class="filter-divider"></span>
          {/if}
          {#each gameStore.smartCollections as col}
            <button
              class:active={gameStore.activeCollectionId === col.id}
              onclick={() => gameStore.activateCollection(col.id)}
              oncontextmenu={(e) => { e.preventDefault(); editCollection(col); }}
              type="button"
              title="{col.name}（右键编辑）"
            >
              {col.name}
            </button>
          {/each}
          <button class="filter-add" onclick={saveCurrentFilter} title="将当前筛选保存为合集" type="button">
            +
          </button>
        </div>

        <div class="command-actions">
          <select
            value={gameStore.sortBy}
            onchange={(event) => (gameStore.sortBy = (event.target as HTMLSelectElement).value)}
            title="排序"
          >
            {#each sortOptions as option}
              <option value={option.id}>{option.label}</option>
            {/each}
          </select>
          <button class:active={uiStore.viewMode === "grid"} onclick={() => (uiStore.viewMode = "grid")} type="button">主屏</button>
          <button onclick={() => (uiStore.viewMode = "compact")} type="button">封面墙</button>
          <button onclick={() => (uiStore.viewMode = "list")} type="button">列表</button>
          <button class:active={gameStore.selectionMode} onclick={toggleSelectionMode} type="button" title="多选模式 (Ctrl+点击)">多选</button>
          <button onclick={() => (showWhatToPlay = true)} type="button" title="今天玩什么？">🎲</button>
        </div>
      </section>

      <section class="focus-section">
        <article class="focus-copy">
          <div class="focus-meta">
            <span>{statusLabel(focusGame)}</span>
            <span>{platformOf(focusGame)}</span>
            <span>{yearOf(focusGame)}</span>
            {#if achievementRate > 0}
              <span>成就 {achievementRate}%</span>
            {/if}
          </div>

          <h1>{focusGame?.name}</h1>

          <p class="subtitle">{focusGame?.metadata?.original_name || developerOf(focusGame)}</p>
          <p class="description">
            {focusGame?.description || "这款游戏还没有简介。可以从资料入口补全封面、背景、标签、厂商、评分与剧情摘要。"}
          </p>

          <div class="tag-strip">
            {#each allTags(focusGame).slice(0, 5) as tag}
              <button onclick={() => (gameStore.filterTag = tag)} type="button">{tag}</button>
            {/each}
          </div>

          <div class="hero-actions">
            <button class="primary-action" onclick={launchFocus} type="button">
              <Icon name="play" size={17} />
              <span>启动游戏</span>
            </button>
            <button onclick={toggleFavorite} class:active={focusGame?.favorite} type="button">
              <Icon name={focusGame?.favorite ? "heartFill" : "heart"} size={16} />
              <span>{focusGame?.favorite ? "已收藏" : "收藏"}</span>
            </button>
            <button onclick={openScrape} type="button">
              <Icon name="search" size={16} />
              <span>资料补全</span>
            </button>
            <button onclick={openDetail} type="button">
              <Icon name="database" size={16} />
              <span>详情</span>
            </button>
          </div>
        </article>

        <aside class="stats-panel" aria-label="游戏统计">
          <div class="stats-head">
            <span class="section-kicker">Game Stats</span>
            <strong>总览</strong>
          </div>

          <div class="completion-block">
            <div>
              <span>库完成度</span>
              <strong>{completionRate}%</strong>
            </div>
            <div class="progress-line" aria-label="库完成度">
              <i style={`width:${completionRate}%`}></i>
            </div>
          </div>

          <div class="stat-grid">
            <div class="stat-card">
              <strong>{totalHours}h</strong>
              <span>总游玩</span>
            </div>
            <div class="stat-card">
              <strong>{weekHours}h</strong>
              <span>本周</span>
            </div>
            <div class="stat-card">
              <strong>{completedCount}</strong>
              <span>已通关</span>
            </div>
            <div class="stat-card">
              <strong>{installedCount}</strong>
              <span>已安装</span>
            </div>
          </div>

          <div class="focus-stats">
            <div>
              <span>当前评分</span>
              <strong>{focusRating ? focusRating.toFixed(1) : "--"}</strong>
            </div>
            <div>
              <span>当前时长</span>
              <strong>{focusGame ? formatPlayTime(secondsOf(focusGame)) : "--"}</strong>
            </div>
            <div>
              <span>资料覆盖</span>
              <strong>{metadataCoverage}%</strong>
            </div>
          </div>
        </aside>
      </section>

      <section class="lower-section">
        <div class="continue-area">
          <div class="section-head">
            <div>
              <span class="section-kicker">Continue Playing</span>
              <strong>继续游玩</strong>
            </div>
            <div class="rail-controls">
              <button onclick={() => moveFocus(-1)} disabled={focusIndex <= 0} type="button">上一款</button>
              <button onclick={() => moveFocus(1)} disabled={focusIndex >= games.length - 1} type="button">下一款</button>
            </div>
          </div>

          <div class="game-rail" bind:this={railEl}>
            {#each topGames as game (game.id)}
              <button
                class="game-tile"
                class:active={focusGame?.id === game.id}
                data-game-id={game.id}
                data-testid="game-rail-tile"
                onclick={() => selectGame(game)}
                ondblclick={() => gameStore.launch(game.id)}
                title={game.name}
                type="button"
              >
                <span class="tile-art">
                  {#if coverSourceOf(game)}
                    <CachedImage source={coverSourceOf(game)} cacheKey={`home-continue-${game.id}`} alt={game.name} loading="lazy" />
                  {:else}
                    <span>{monogramOf(game)}</span>
                  {/if}
                </span>
                <span class="tile-copy">
                  <strong>{game.name}</strong>
                  <small>{timeAgo(lastPlayedOf(game))} / {formatPlayTime(secondsOf(game))}</small>
                </span>
                {#if game.favorite}
                  <span class="favorite-dot"><Icon name="heartFill" size={12} /></span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <aside class="activity-panel" aria-label="最近活动">
          <div class="section-head">
            <div>
              <span class="section-kicker">Activity</span>
              <strong>最近活动</strong>
            </div>
          </div>

          <div class="activity-list">
            <button class="activity-item" onclick={() => (uiStore.currentView = "stats")} type="button">
              <span class="activity-icon"><Icon name="chart" size={17} /></span>
              <span>
                <strong>本周游玩 {weekHours}h</strong>
                <small>{playingCount} 款正在游玩</small>
              </span>
              <em>统计</em>
            </button>
            <button class="activity-item" onclick={() => (uiStore.currentView = "backup")} type="button">
              <span class="activity-icon"><Icon name="save" size={17} /></span>
              <span>
                <strong>存档快照</strong>
                <small>为当前游戏管理本地/云端快照</small>
              </span>
              <em>存档</em>
            </button>
            <button class="activity-item" onclick={() => (uiStore.currentView = "downloads")} type="button">
              <span class="activity-icon"><Icon name="download" size={17} /></span>
              <span>
                <strong>资源与补丁</strong>
                <small>查找下载、补丁和导入任务</small>
              </span>
              <em>下载</em>
            </button>
            <button class="activity-item" onclick={() => (uiStore.currentView = "steam-import")} type="button">
              <span class="activity-icon"><Icon name="globe" size={17} /></span>
              <span>
                <strong>平台库同步</strong>
                <small>Steam / Epic / 本机扫描</small>
              </span>
              <em>导入</em>
            </button>
          </div>
        </aside>
      </section>

      <section class="rail-matrix" aria-label="游戏内容轨">
        <Rail title="最近添加" subtitle={`${recentAdded.length} 款新入库作品`} empty={recentAdded.length === 0} itemWidth="156px">
          {#each recentAdded as game (game.id)}
            {@render railTile(game)}
          {/each}
        </Rail>

        <Rail title="收藏" subtitle={`${favoriteGames.length} 款已收藏`} empty={favoriteGames.length === 0} itemWidth="156px">
          {#each favoriteGames as game (game.id)}
            {@render railTile(game)}
          {/each}
        </Rail>

        {#each platformRails as rail (rail.label)}
          <Rail title={`平台 · ${rail.label}`} subtitle={`${rail.items.length} 款`} empty={rail.items.length === 0} itemWidth="156px">
            {#each rail.items as game (game.id)}
              {@render railTile(game)}
            {/each}
          </Rail>
        {/each}
      </section>
    </main>

    <footer class="console-statusbar" aria-label="主机操作提示">
      <span>A 启动</span>
      <span>X 收藏</span>
      <span>Y 资料补全</span>
      <span>I Steam 导入</span>
      <span>LB/RB 切换游戏</span>
      <span class="status-count"><strong>{totalGames}</strong> 游戏 / <strong>{favoriteCount}</strong> 收藏</span>
    </footer>
  {/if}
</section>

<WhatToPlay bind:open={showWhatToPlay} />
<SmartCollectionEditor bind:open={showCollectionEditor} bind:collection={editingCollection} initialFilters={initialCollectionFilters} />

<style>
  .console-dashboard {
    flex: 1;
    min-height: 0;
    position: relative;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    overflow: hidden;
    background:
      linear-gradient(135deg, #05070c, #111722 54%, #070a10),
      var(--bg-deep);
    color: #f4f7fb;
  }

  .hero-backdrop {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(circle at 70% 36%, rgba(69, 163, 255, 0.25), transparent 28%),
      linear-gradient(135deg, #101824, #384452 52%, #11151c);
    background-size: cover;
    background-position: center;
    transform: scale(1.02);
  }

  .has-art .hero-backdrop {
    filter: saturate(0.88) brightness(0.82);
  }

  .hero-shade {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(90deg, rgba(5, 7, 12, 0.98), rgba(5, 7, 12, 0.58) 42%, rgba(5, 7, 12, 0.90)),
      linear-gradient(180deg, rgba(5, 7, 12, 0.72), rgba(5, 7, 12, 0.22) 30%, rgba(5, 7, 12, 0.92) 82%);
  }

  .console-topbar,
  .console-statusbar {
    position: relative;
    z-index: 4;
  }

  .console-topbar {
    min-height: 76px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 0 34px;
    background: linear-gradient(180deg, rgba(3, 5, 8, 0.72), rgba(3, 5, 8, 0.18));
    backdrop-filter: blur(20px);
  }

  .brand-group,
  .system-group,
  .console-nav,
  .focus-meta,
  .hero-actions,
  .filter-strip,
  .command-actions,
  .rail-controls {
    display: flex;
    align-items: center;
  }

  .brand-group {
    min-width: 0;
    gap: 18px;
  }

  .brand-mark {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 8px;
    background: linear-gradient(135deg, #287bff, #54d3ff);
    color: #ffffff;
    font-size: 20px;
    font-weight: 900;
    cursor: pointer;
  }

  .console-nav {
    gap: 4px;
    min-width: 0;
  }

  .console-nav button,
  .filter-strip button,
  .command-actions button,
  .hero-actions button,
  .rail-controls button,
  .system-btn,
  .clear-search,
  .empty-actions button,
  .fallback-head button {
    border: 1px solid transparent;
    color: rgba(244, 247, 251, 0.66);
    background: transparent;
    cursor: pointer;
    font: inherit;
  }

  .console-nav button {
    height: 38px;
    border-radius: 8px;
    padding: 0 13px;
    font-size: 14px;
    font-weight: 760;
  }

  .console-nav button:hover,
  .console-nav button.active {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.12);
  }

  .system-group {
    justify-content: flex-end;
    gap: 10px;
    min-width: 0;
  }

  .search-box {
    width: min(34vw, 440px);
    min-width: 240px;
    height: 40px;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 12px;
    border: 1px solid rgba(255, 255, 255, 0.13);
    border-radius: 8px;
    background: rgba(8, 12, 18, 0.58);
    color: rgba(244, 247, 251, 0.48);
  }

  .search-box input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: #ffffff;
    font: inherit;
    font-size: 14px;
  }

  .search-box input::placeholder {
    color: rgba(244, 247, 251, 0.42);
  }

  .clear-search {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
  }

  .system-btn {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-color: rgba(255, 255, 255, 0.13);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.08);
  }

  .system-btn:hover {
    color: #ffffff;
    border-color: rgba(69, 163, 255, 0.58);
    background: rgba(69, 163, 255, 0.18);
  }

  .clock {
    min-width: 50px;
    color: rgba(244, 247, 251, 0.68);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .load-error-banner {
    position: absolute;
    top: 82px;
    left: 34px;
    right: 34px;
    z-index: 10;
    min-height: 42px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border: 1px solid rgba(255, 105, 116, 0.35);
    border-radius: 8px;
    color: #ff9aa2;
    background: rgba(70, 12, 20, 0.34);
  }

  .load-error-banner button {
    margin-left: auto;
    border: 1px solid rgba(255, 105, 116, 0.35);
    border-radius: 8px;
    padding: 5px 12px;
    color: #ffffff;
    background: rgba(255, 105, 116, 0.14);
    cursor: pointer;
  }

  .batch-bar {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 16px; margin: 0 34px;
    border: 1px solid var(--accent-ring, rgba(232,85,127,0.4));
    border-radius: 10px;
    background: var(--accent-lo, rgba(232,85,127,0.08));
    animation: fade-in 0.2s ease;
  }
  .batch-count {
    font-size: 13px; font-weight: 650; color: var(--accent);
    white-space: nowrap;
  }
  .batch-actions { display: flex; gap: 6px; flex: 1; }
  .batch-btn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 6px 12px; border: 1px solid rgba(255,255,255,0.1);
    border-radius: 6px; background: rgba(255,255,255,0.04);
    color: var(--text-secondary); font-size: 12px; cursor: pointer;
  }
  .batch-btn:hover { border-color: var(--accent); color: var(--accent); }
  .batch-btn.danger { border-color: rgba(248,113,113,0.3); color: #f87171; }
  .batch-btn.danger:hover { background: rgba(248,113,113,0.1); }
  .batch-cancel {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 6px 12px; border: 1px solid rgba(255,255,255,0.1);
    border-radius: 6px; background: transparent;
    color: var(--text-muted); font-size: 12px; cursor: pointer;
  }
  .batch-cancel:hover { color: var(--text-primary); }
  .batch-select {
    padding: 6px 8px; border: 1px solid rgba(255,255,255,0.1);
    border-radius: 6px; background: rgba(255,255,255,0.04);
    color: var(--text-secondary); font-size: 12px; cursor: pointer;
  }
  .batch-select:hover { border-color: var(--accent); }
  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }

  .loading-panel,
  .empty-console {
    position: relative;
    z-index: 2;
    place-self: center;
  }

  .loading-panel {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 18px 22px;
  }

  .empty-console {
    width: min(520px, calc(100vw - 48px));
    display: grid;
    gap: 14px;
  }

  .empty-actions {
    display: flex;
    justify-content: center;
    gap: 10px;
  }

  .empty-actions button,
  .fallback-head button {
    min-height: 38px;
    border-color: rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    padding: 0 14px;
    color: #ffffff;
    background: rgba(255, 255, 255, 0.08);
  }

  .dashboard-main {
    position: relative;
    z-index: 2;
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(290px, 45vh) minmax(180px, auto) auto;
    gap: 24px;
    padding: 12px 38px 58px;
    overflow: hidden;
  }

  .command-row,
  .focus-section,
  .lower-section {
    min-width: 0;
  }

  .rail-matrix {
    min-width: 0;
    display: grid;
    gap: 22px;
    padding: 4px 0 18px;
    overflow: hidden auto;
  }

  .rail-matrix :global(.ui-rail__scroller) {
    grid-auto-columns: 156px;
  }

  .command-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .filter-strip {
    gap: 8px;
    min-width: 0;
    overflow: auto;
    scrollbar-width: none;
  }

  .filter-strip::-webkit-scrollbar {
    display: none;
  }

  .filter-strip button,
  .command-actions button,
  .command-actions select {
    height: 34px;
    flex: 0 0 auto;
    border: 1px solid rgba(255, 255, 255, 0.13);
    border-radius: 8px;
    padding: 0 12px;
    color: rgba(244, 247, 251, 0.72);
    background: rgba(8, 12, 18, 0.46);
  }

  .filter-strip button.active,
  .command-actions button.active,
  .filter-strip button:hover,
  .command-actions button:hover {
    color: #ffffff;
    border-color: rgba(69, 163, 255, 0.52);
    background: rgba(69, 163, 255, 0.16);
  }

  .filter-divider {
    width: 1px; height: 20px; flex-shrink: 0;
    background: rgba(255,255,255,0.12);
  }
  .filter-add {
    width: 34px !important; padding: 0 !important;
    font-size: 16px; font-weight: 700;
    border-style: dashed !important;
    opacity: 0.6;
  }
  .filter-add:hover { opacity: 1; }

  .command-actions {
    gap: 8px;
    flex: 0 0 auto;
  }

  .command-actions select {
    outline: 0;
  }

  .command-actions option {
    color: #ffffff;
    background: #101722;
  }

  .focus-section {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(300px, 360px);
    align-items: end;
    gap: 30px;
  }

  .focus-copy {
    min-width: 0;
  }

  .focus-meta {
    flex-wrap: wrap;
    gap: 10px;
  }

  .focus-meta span,
  .tag-strip button {
    min-height: 30px;
    display: inline-flex;
    align-items: center;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    padding: 0 10px;
    color: #ffffff;
    background: rgba(255, 255, 255, 0.08);
    font-size: 12px;
    font-weight: 760;
  }

  .focus-copy h1 {
    margin: 14px 0 10px;
    max-width: 850px;
    color: #ffffff;
    font-size: clamp(42px, 6.6vw, 92px);
    line-height: 0.96;
    font-weight: 880;
    letter-spacing: 0;
    text-wrap: balance;
  }

  .subtitle {
    color: rgba(244, 247, 251, 0.82);
    font-size: 16px;
    font-weight: 700;
  }

  .description {
    max-width: 760px;
    margin-top: 12px;
    color: rgba(244, 247, 251, 0.68);
    line-height: 1.72;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .tag-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    min-height: 30px;
    margin-top: 16px;
  }

  .tag-strip button {
    cursor: pointer;
  }

  .hero-actions {
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 22px;
  }

  .hero-actions button {
    min-height: 44px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border-color: rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    padding: 0 15px;
    color: #ffffff;
    background: rgba(255, 255, 255, 0.08);
    font-weight: 800;
  }

  .hero-actions button:hover,
  .hero-actions button.active {
    border-color: rgba(69, 163, 255, 0.54);
    background: rgba(69, 163, 255, 0.17);
  }

  .hero-actions .primary-action {
    border-color: rgba(69, 163, 255, 0.68);
    background: rgba(69, 163, 255, 0.30);
  }

  .stats-panel,
  .activity-panel {
    border: 1px solid rgba(255, 255, 255, 0.13);
    border-radius: 8px;
    background: rgba(8, 12, 18, 0.58);
    backdrop-filter: blur(22px);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06), 0 28px 80px rgba(0, 0, 0, 0.24);
  }

  .stats-panel {
    display: grid;
    gap: 16px;
    padding: 20px;
  }

  .section-kicker {
    color: rgba(244, 247, 251, 0.46);
    font-size: 11px;
    font-weight: 850;
    text-transform: uppercase;
  }

  .stats-head {
    display: grid;
    gap: 4px;
  }

  .stats-head strong,
  .section-head strong {
    color: #ffffff;
    font-size: 20px;
  }

  .completion-block {
    display: grid;
    gap: 10px;
  }

  .completion-block > div:first-child {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: rgba(244, 247, 251, 0.58);
  }

  .completion-block strong {
    color: #ffffff;
    font-family: var(--font-mono);
    font-size: 26px;
  }

  .progress-line {
    height: 8px;
    overflow: hidden;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.13);
  }

  .progress-line i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, #45a3ff, #4fd49b);
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .stat-card,
  .focus-stats > div {
    min-width: 0;
    display: grid;
    gap: 4px;
    padding: 12px;
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.065);
  }

  .stat-card strong,
  .focus-stats strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #ffffff;
    font-family: var(--font-mono);
    font-size: 24px;
    line-height: 1.1;
  }

  .stat-card span,
  .focus-stats span {
    color: rgba(244, 247, 251, 0.50);
    font-size: 12px;
  }

  .focus-stats {
    display: grid;
    gap: 8px;
  }

  .lower-section {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(300px, 360px);
    gap: 22px;
    overflow: hidden;
  }

  .continue-area,
  .activity-panel {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 12px;
  }

  .section-head {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 14px;
  }

  .section-head > div:first-child {
    display: grid;
    gap: 4px;
  }

  .rail-controls {
    gap: 8px;
  }

  .rail-controls button {
    min-height: 34px;
    border-color: rgba(255, 255, 255, 0.13);
    border-radius: 8px;
    padding: 0 12px;
    color: rgba(244, 247, 251, 0.72);
    background: rgba(8, 12, 18, 0.46);
  }

  .rail-controls button:hover:not(:disabled) {
    color: #ffffff;
    border-color: rgba(69, 163, 255, 0.52);
  }

  .rail-controls button:disabled {
    opacity: 0.38;
    cursor: not-allowed;
  }

  .game-rail {
    min-height: 0;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 156px;
    align-items: start;
    gap: 16px;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 18px 4px 30px;
    scroll-snap-type: x mandatory;
  }

  .game-tile {
    height: 216px;
    min-width: 0;
    position: relative;
    display: grid;
    grid-template-rows: 148px minmax(0, 1fr);
    gap: 10px;
    border: 1px solid rgba(255, 255, 255, 0.13);
    border-radius: 8px;
    padding: 8px;
    color: #ffffff;
    background: rgba(255, 255, 255, 0.075);
    cursor: pointer;
    text-align: left;
    scroll-snap-align: center;
    transition: transform 0.22s ease, border-color 0.22s ease, box-shadow 0.22s ease, background 0.22s ease;
  }

  .game-tile:hover,
  .game-tile.active {
    border-color: rgba(69, 163, 255, 0.82);
    background: rgba(255, 255, 255, 0.10);
  }

  .game-tile.active {
    transform: translateY(-18px);
    box-shadow: 0 0 0 2px rgba(69, 163, 255, 0.22), 0 28px 80px rgba(0, 0, 0, 0.42);
  }

  .tile-art {
    display: grid;
    place-items: center;
    border-radius: 6px;
    overflow: hidden;
    background: linear-gradient(135deg, #253246, #46566c);
    color: rgba(255, 255, 255, 0.82);
    font-size: 34px;
    font-weight: 900;
  }

  .tile-art :global(.cached-image) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
  }

  .tile-copy {
    min-width: 0;
    display: grid;
    align-content: start;
    gap: 4px;
  }

  .tile-copy strong,
  .tile-copy small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tile-copy strong {
    color: #ffffff;
    font-size: 14px;
  }

  .tile-copy small {
    color: rgba(244, 247, 251, 0.48);
    font-size: 11px;
  }

  .favorite-dot {
    position: absolute;
    top: 14px;
    right: 14px;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: #ff7d8d;
    background: rgba(3, 5, 8, 0.62);
    backdrop-filter: blur(10px);
  }

  .activity-panel {
    padding: 16px;
  }

  .activity-list {
    min-height: 0;
    display: grid;
    gap: 10px;
    align-content: start;
    overflow: auto;
  }

  .activity-item {
    min-height: 62px;
    display: grid;
    grid-template-columns: 42px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 8px;
    padding: 10px;
    color: #ffffff;
    background: rgba(255, 255, 255, 0.055);
    cursor: pointer;
    text-align: left;
  }

  .activity-item:hover {
    border-color: rgba(69, 163, 255, 0.46);
    background: rgba(69, 163, 255, 0.12);
  }

  .activity-icon {
    width: 42px;
    height: 42px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: #93cfff;
    background: rgba(69, 163, 255, 0.16);
  }

  .activity-item span:nth-child(2) {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .activity-item strong,
  .activity-item small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-item small {
    color: rgba(244, 247, 251, 0.48);
  }

  .activity-item em {
    color: rgba(244, 247, 251, 0.48);
    font-style: normal;
    font-size: 12px;
  }

  .console-statusbar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    min-height: 46px;
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 0 36px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    color: rgba(244, 247, 251, 0.56);
    background: rgba(5, 7, 12, 0.56);
    backdrop-filter: blur(18px);
    font-size: 12px;
  }

  .status-count {
    margin-left: auto;
    color: rgba(244, 247, 251, 0.66);
  }

  .status-count strong {
    color: #ffffff;
    font-family: var(--font-mono);
  }

  .library-fallback {
    position: relative;
    z-index: 2;
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 14px;
    padding: 92px 34px 34px;
  }

  .fallback-head {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 16px;
  }

  .fallback-head h1 {
    margin: 4px 0 0;
    font-size: 38px;
    line-height: 1;
  }

  :global(.library-fallback .game-grid) {
    border: 1px solid rgba(255, 255, 255, 0.13);
    border-radius: 8px;
    background: rgba(8, 12, 18, 0.54);
    backdrop-filter: blur(18px);
  }

  @media (max-width: 1180px) {
    .console-nav button:nth-child(n + 5),
    .command-actions button {
      display: none;
    }

    .focus-section,
    .lower-section {
      grid-template-columns: minmax(0, 1fr);
    }

    .stats-panel {
      grid-template-columns: minmax(190px, 0.7fr) minmax(280px, 1fr);
      align-items: center;
    }

    .focus-stats {
      grid-column: 1 / -1;
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .activity-panel {
      min-height: 220px;
    }
  }

  @media (max-width: 820px) {
    .console-topbar {
      min-height: 112px;
      align-items: flex-start;
      flex-direction: column;
      justify-content: center;
      padding: 12px 16px;
    }

    .brand-group,
    .system-group,
    .command-row {
      width: 100%;
    }

    .console-nav {
      overflow: auto;
    }

    .system-group {
      justify-content: stretch;
    }

    .search-box {
      min-width: 0;
      width: 100%;
    }

    .clock,
    .command-actions,
    .console-statusbar {
      display: none;
    }

    .dashboard-main {
      grid-template-rows: auto auto minmax(220px, 1fr);
      gap: 18px;
      padding: 124px 16px 18px;
      overflow: auto;
    }

    .focus-section,
    .lower-section,
    .stat-grid,
    .focus-stats,
    .stats-panel {
      grid-template-columns: 1fr;
    }

    .focus-copy h1 {
      font-size: 42px;
    }

    .description {
      -webkit-line-clamp: 4;
      line-clamp: 4;
    }

    .game-rail {
      grid-auto-columns: 132px;
    }

    .game-tile {
      height: 194px;
      grid-template-rows: 126px minmax(0, 1fr);
    }
  }
</style>
