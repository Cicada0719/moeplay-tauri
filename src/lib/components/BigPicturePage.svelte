<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import type { Game } from "../stores/games.svelte";
  import Icon from "./Icon.svelte";
  import BigPictureDetail from "./BigPictureDetail.svelte";
  import BPSearch from "./BPSearch.svelte";
  import BigPictureBackground from "./bigpicture/BigPictureBackground.svelte";
  import BigPictureWheel from "./bigpicture/BigPictureWheel.svelte";
  import BigPictureHero from "./bigpicture/BigPictureHero.svelte";
  import BigPictureMediaTab from "./bigpicture/BigPictureMediaTab.svelte";
  import { fileSrc } from "../utils";
  import { hasHeroBackground, heroImageOf as gameHeroImageOf } from "../utils/game";
  import { attachGamepad, type GamepadAttachment } from "./switch/useGamepad.svelte";
  import { getDefaultGamepadFocusRuntime } from "../actions/a11y/gamepadFocus";
  import defaultLibraryBackdrop from "../assets/default-library-backdrop.png";
  import { animeStore } from "../stores/anime.svelte";

  type BigPictureTab = "game" | "media";
  type BigPictureZone = "top-nav" | "wheel" | "hero" | "media" | "detail" | "search" | "keyboard";

  let bpTab = $state<BigPictureTab>("game");
  let activeZone = $state<BigPictureZone>("wheel");
  let focusIdx = $state(0);
  let topFocusIdx = $state(0);
  let filterAll = $state(true);
  let showDetail = $state(false);
  let showSearch = $state(false);
  let now = $state(new Date());
  let bgCurrent = $state<string>(defaultLibraryBackdrop);
  let bgPrevious = $state<string | null>(null);
  let bgFading = $state(false);
  let prefersReducedMotion = $state(false);
  let bgTimer: ReturnType<typeof setTimeout> | null = null;
  let topNavEl = $state<HTMLElement>();
  let detailReturnFocus = $state<HTMLElement | null>(null);
  let searchReturnFocus = $state<HTMLElement | null>(null);
  let detailReturnZone = $state<BigPictureZone>("wheel");
  let searchReturnZone = $state<BigPictureZone>("top-nav");
  let topScope: GamepadAttachment | null = null;

  const games = $derived(gameStore.games);
  const allGames = $derived(gameStore.allGames);
  const filteredGames = $derived(filterAll ? games : allGames.filter((g) => g.install_dir || g.exe_path));
  const focusGame = $derived(filteredGames[focusIdx] ?? null);
  const backgroundArt = $derived(pickBackgroundArt(focusGame));
  const isHeroBg = $derived(hasHeroBackground(focusGame));
  const clock = $derived(now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }));

  const TOP_ITEMS = ["game", "media", "search", "exit", "settings"] as const;

  function pickBackgroundArt(g: Game | null): string {
    if (!g) return defaultLibraryBackdrop;
    return fileSrc(gameHeroImageOf(g)) ?? defaultLibraryBackdrop;
  }

  const weekHours = $derived.by(() => {
    const since = Date.now() - 7 * 86400000;
    let s = 0;
    for (const g of allGames) {
      for (const sess of g.play_tracker?.sessions ?? []) {
        if (new Date(sess.start_time).getTime() >= since) s += sess.duration_seconds ?? 0;
      }
    }
    return (s / 3600).toFixed(1);
  });

  function move(d: number) {
    if (filteredGames.length === 0) return;
    focusIdx = Math.max(0, Math.min(filteredGames.length - 1, focusIdx + d));
  }

  function setFocus(i: number) {
    if (i >= 0 && i < filteredGames.length) focusIdx = i;
  }

  function focusTop(index = topFocusIdx) {
    topFocusIdx = Math.max(0, Math.min(TOP_ITEMS.length - 1, index));
    queueMicrotask(() => {
      topNavEl?.querySelector<HTMLElement>(`[data-top-index="${topFocusIdx}"]`)?.focus({ preventScroll: true });
    });
  }

  function setZone(zone: BigPictureZone, focus = true) {
    activeZone = zone;
    getDefaultGamepadFocusRuntime()?.setActiveZone(zone);
    if (focus && zone === "top-nav") focusTop();
  }

  function enterContent() {
    setZone(bpTab === "game" ? "wheel" : "media");
  }

  function switchTab(tab: BigPictureTab, enter = true) {
    bpTab = tab;
    topFocusIdx = tab === "game" ? 0 : 1;
    if (tab === "media") void animeStore.loadRecommendations();
    setZone(enter ? (tab === "game" ? "wheel" : "media") : "top-nav");
  }

  function cycleTab(direction: -1 | 1) {
    switchTab(direction < 0 ? "game" : "media", true);
  }

  function moveTop(direction: number) {
    focusTop((topFocusIdx + direction + TOP_ITEMS.length) % TOP_ITEMS.length);
  }

  function activateTop() {
    switch (TOP_ITEMS[topFocusIdx]) {
      case "game": switchTab("game"); break;
      case "media": switchTab("media"); break;
      case "search": openSearch(); break;
      case "exit": exitBigPicture(); break;
      case "settings": openSettings(); break;
    }
  }

  function openDetail() {
    if (!focusGame || showSearch) return;
    detailReturnZone = activeZone === "hero" ? "hero" : "wheel";
    detailReturnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    showDetail = true;
    activeZone = "detail";
  }

  function closeDetail() {
    if (!showDetail) return;
    showDetail = false;
    setZone(detailReturnZone, false);
    queueMicrotask(() => detailReturnFocus?.isConnected && detailReturnFocus.focus({ preventScroll: true }));
  }

  function openSearch() {
    if (showDetail || showSearch) return;
    searchReturnZone = activeZone;
    searchReturnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    showSearch = true;
    activeZone = "keyboard";
  }

  function closeSearch() {
    showSearch = false;
    setZone(searchReturnZone, false);
    queueMicrotask(() => searchReturnFocus?.isConnected && searchReturnFocus.focus({ preventScroll: true }));
  }

  async function launchFocus() { if (focusGame) await gameStore.launch(focusGame.id); }
  async function toggleFav() { if (focusGame) await gameStore.toggleFavorite(focusGame.id); }

  function openScraper() {
    if (focusGame) gameStore.selectGame(focusGame.id);
    uiStore.setBigPicture(false);
    uiStore.currentView = "scraper";
  }

  function openImport() { uiStore.setBigPicture(false); uiStore.currentView = "steam-import"; }
  function openSettings() { uiStore.setBigPicture(false); uiStore.currentView = "settings"; }
  function exitBigPicture() { uiStore.setBigPicture(false); }

  function back() {
    if (showSearch) { closeSearch(); return; }
    if (showDetail) { closeDetail(); return; }
    if (activeZone === "hero" || activeZone === "media") { setZone(activeZone === "hero" ? "wheel" : "top-nav"); return; }
    if (activeZone === "top-nav") { exitBigPicture(); return; }
    setZone("top-nav");
  }

  function toggleFilter() { filterAll = !filterAll; focusIdx = 0; }

  function selectMedia(item: { type: string }) {
    uiStore.setBigPicture(false);
    uiStore.currentView = item.type === "anime" ? "anime" : "comic";
  }

  function onWheel(e: WheelEvent) {
    if (showDetail || showSearch || bpTab !== "game" || activeZone !== "wheel" || filteredGames.length === 0) return;
    if (Math.abs(e.deltaY) < 1 && Math.abs(e.deltaX) < 1) return;
    e.preventDefault();
    move(e.deltaY > 0 ? 1 : e.deltaY < 0 ? -1 : e.deltaX > 0 ? 1 : -1);
  }

  function onGlobalKeydown(e: KeyboardEvent) {
    if (showDetail || showSearch) return;
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;

    if (e.key === "/") { e.preventDefault(); openSearch(); return; }
    if (e.key === "q" || e.key === "Q") { e.preventDefault(); cycleTab(-1); return; }
    if (e.key === "e" || e.key === "E") { e.preventDefault(); cycleTab(1); return; }

    if (activeZone === "top-nav") {
      switch (e.key) {
        case "ArrowLeft": e.preventDefault(); moveTop(-1); break;
        case "ArrowRight": e.preventDefault(); moveTop(1); break;
        case "ArrowDown": e.preventDefault(); enterContent(); break;
        case "Enter": case " ": e.preventDefault(); activateTop(); break;
        case "Escape": case "b": case "B": e.preventDefault(); exitBigPicture(); break;
      }
      return;
    }

    if (bpTab === "game") {
      if (e.key === "f" || e.key === "F") { e.preventDefault(); toggleFilter(); }
      if (e.key === "y" || e.key === "Y") { e.preventDefault(); openDetail(); }
      if (e.key === "i" || e.key === "I") { e.preventDefault(); openImport(); }
    }
  }

  $effect(() => {
    if (focusIdx >= filteredGames.length) focusIdx = Math.max(0, filteredGames.length - 1);
  });

  $effect(() => {
    if (focusGame && gameStore.selectedGame?.id !== focusGame.id) gameStore.selectGame(focusGame.id);
  });

  $effect(() => {
    const zone = activeZone;
    if (!showDetail && !showSearch) getDefaultGamepadFocusRuntime()?.setActiveZone(zone);
  });

  $effect(() => {
    const next = backgroundArt;
    if (!next || next === bgCurrent) return;
    if (bgTimer) { clearTimeout(bgTimer); bgTimer = null; }
    if (prefersReducedMotion) {
      bgCurrent = next; bgPrevious = null; bgFading = false; return;
    }
    bgPrevious = bgCurrent;
    bgCurrent = next;
    bgFading = true;
    bgTimer = setTimeout(() => { bgPrevious = null; bgFading = false; bgTimer = null; }, 640);
  });

  onMount(() => {
    const t = setInterval(() => (now = new Date()), 30_000);
    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const syncMotion = () => {
      prefersReducedMotion = motionQuery.matches || document.documentElement.dataset.motion === "reduce";
      if (prefersReducedMotion) { bgPrevious = null; bgFading = false; }
    };
    syncMotion();
    motionQuery.addEventListener("change", syncMotion);

    topScope = attachGamepad({
      left: () => moveTop(-1),
      right: () => moveTop(1),
      down: () => enterContent(),
      launch: () => activateTop(),
      activate: () => activateTop(),
      back: () => exitBigPicture(),
      pageLeft: () => cycleTab(-1),
      pageRight: () => cycleTab(1),
    }, { id: "big-picture-top-nav", zone: "top-nav", priority: 20 });

    getDefaultGamepadFocusRuntime()?.setActiveZone(activeZone);
    queueMicrotask(() => setZone(activeZone));

    return () => {
      clearInterval(t);
      if (bgTimer) clearTimeout(bgTimer);
      motionQuery.removeEventListener("change", syncMotion);
      topScope?.();
      topScope = null;
    };
  });
</script>

<svelte:window onkeydown={onGlobalKeydown} />

<section class="bp" onwheel={onWheel} data-active-zone={activeZone} aria-label="大屏模式">
  <BigPictureBackground current={bgCurrent} previous={bgPrevious} fading={bgFading} isCover={!isHeroBg} />

  <div class="bp-layout">
    {#if bpTab === "game"}
      <BigPictureWheel
        games={filteredGames}
        {focusIdx}
        {filterAll}
        {prefersReducedMotion}
        active={activeZone === "wheel" && !showDetail && !showSearch}
        onSelect={setFocus}
        onActivate={(i) => { setFocus(i); openDetail(); }}
        onLaunch={(i) => { setFocus(i); void launchFocus(); }}
        onFavorite={(i) => { setFocus(i); void toggleFav(); }}
        onMoveToHero={() => setZone("hero")}
        onMoveToTop={() => setZone("top-nav")}
        onBack={back}
        onTabPrevious={() => cycleTab(-1)}
        onTabNext={() => cycleTab(1)}
        onToggleFilter={toggleFilter}
        onOpenImport={openImport}
      />
    {/if}

    <div class="bp-main">
      <header class="bp-top" bind:this={topNavEl}>
        <nav class="bp-nav" aria-label="大屏分类">
          <button
            class:active={bpTab === "game"}
            data-top-index="0"
            tabindex={activeZone === "top-nav" && topFocusIdx === 0 ? 0 : -1}
            aria-current={bpTab === "game" ? "page" : undefined}
            onclick={() => switchTab("game")}
            onfocus={() => { topFocusIdx = 0; activeZone = "top-nav"; }}
          >游戏</button>
          <button
            class:active={bpTab === "media"}
            data-top-index="1"
            tabindex={activeZone === "top-nav" && topFocusIdx === 1 ? 0 : -1}
            aria-current={bpTab === "media" ? "page" : undefined}
            onclick={() => switchTab("media")}
            onfocus={() => { topFocusIdx = 1; activeZone = "top-nav"; }}
          >媒体</button>
        </nav>
        <div class="bp-top-right">
          <button
            class="bp-search"
            data-top-index="2"
            tabindex={activeZone === "top-nav" && topFocusIdx === 2 ? 0 : -1}
            onclick={openSearch}
            onfocus={() => { topFocusIdx = 2; activeZone = "top-nav"; }}
            aria-label="搜索游戏"
            title="搜索（/）"
          ><Icon name="search" size={17} /><span>搜索</span></button>
          <button
            class="bp-exit"
            data-top-index="3"
            tabindex={activeZone === "top-nav" && topFocusIdx === 3 ? 0 : -1}
            onclick={exitBigPicture}
            onfocus={() => { topFocusIdx = 3; activeZone = "top-nav"; }}
            title="退出大屏（Esc / 手柄 B）"
          >
            <Icon name="chevronLeft" size={16} />
            <span>退出大屏</span>
          </button>
          <span class="bp-clock">{clock}</span>
          <button
            class="bp-settings"
            data-top-index="4"
            tabindex={activeZone === "top-nav" && topFocusIdx === 4 ? 0 : -1}
            onclick={openSettings}
            onfocus={() => { topFocusIdx = 4; activeZone = "top-nav"; }}
            title="设置"
            aria-label="设置"
          >
            <Icon name="gear" size={18} />
          </button>
        </div>
      </header>

      {#if bpTab === "game"}
        <div class="bp-stage">
          {#if focusGame}
            <BigPictureHero
              game={focusGame}
              {weekHours}
              active={activeZone === "hero" && !showDetail && !showSearch}
              onLaunch={launchFocus}
              onFavorite={toggleFav}
              onDetail={openDetail}
              onMoveToWheel={() => setZone("wheel")}
              onMoveToTop={() => setZone("top-nav")}
              onTabPrevious={() => cycleTab(-1)}
              onTabNext={() => cycleTab(1)}
            />
          {/if}
        </div>

        <footer class="bp-hints">
          <span><b>A</b> 启动 / 激活</span>
          <span><b>B</b> 返回</span>
          <span><b>X</b> 收藏</span>
          <span><b>Y</b> 详情</span>
          <span><b>LB/RB</b> 切换分类</span>
          <span><b>/</b> 搜索</span>
          <span><b>F</b> {filterAll ? "已安装" : "全部"}</span>
          <span class="bp-pos">{filteredGames.length ? focusIdx + 1 : 0} / {filteredGames.length}</span>
        </footer>
      {:else}
        <BigPictureMediaTab
          active={activeZone === "media" && !showDetail && !showSearch}
          onSelectMedia={selectMedia}
          onMoveToTop={() => setZone("top-nav")}
          onBack={back}
          onTabPrevious={() => cycleTab(-1)}
          onTabNext={() => cycleTab(1)}
        />

        <footer class="bp-hints">
          <span><b>A / Enter</b> 打开</span>
          <span><b>B / Esc</b> 返回导航</span>
          <span><b>LB/RB</b> 切换分类</span>
        </footer>
      {/if}
    </div>
  </div>

  {#if showDetail && focusGame}
    <BigPictureDetail game={focusGame} onClose={closeDetail} returnFocus={detailReturnFocus} />
  {/if}

  <BPSearch
    bind:open={showSearch}
    returnFocus={searchReturnFocus}
    onzonechange={(zone) => { activeZone = zone; }}
    onclose={closeSearch}
    onselect={(g) => {
      switchTab("game", true);
      setFocus(filteredGames.findIndex((f) => f.id === g.id));
    }}
  />
</section>

<style>
  .bp {
    position: relative;
    z-index: 1;
    flex: 1;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    background: var(--bg-void);
    color: var(--text-primary);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    user-select: none;
  }

  .bp-layout {
    position: relative; z-index: 2;
    display: flex;
    flex: 1; min-height: 0;
    width: 100%; height: 100%;
  }

  .bp-main {
    flex: 1; min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .bp-top {
    display: flex; align-items: center; justify-content: space-between;
    padding: 18px 36px 0;
    flex-shrink: 0;
  }
  .bp-nav { display: flex; gap: 22px; font-size: 16px; }
  .bp-nav button {
    background: none; border: none; padding: 4px 2px;
    color: var(--text-muted); cursor: pointer; font-size: inherit;
    border-bottom: 2px solid transparent; transition: all 0.18s ease;
  }
  .bp-nav button:hover { color: var(--text-secondary); }
  .bp-nav button.active { color: var(--text-primary); font-weight: 700; border-bottom-color: var(--accent); }
  .bp-clock { font-family: var(--font-mono); font-variant-numeric: tabular-nums; color: var(--text-secondary); }
  .bp-top-right { display: flex; align-items: center; gap: 14px; }
  .bp-search,
  .bp-exit {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 7px 16px 7px 11px;
    border: 1px solid var(--accent-ring, rgba(232,85,127,0.45));
    border-radius: var(--radius-full);
    background: var(--accent-lo, rgba(232,85,127,0.14));
    color: var(--accent, #e8557f);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 650;
    cursor: pointer;
    backdrop-filter: blur(6px);
    transition: background 0.18s ease, color 0.18s ease, transform 0.18s ease;
  }
  .bp-search:hover,
  .bp-exit:hover {
    background: var(--accent, #e8557f);
    color: #fff;
    transform: translateX(-2px);
  }
  .bp-search:active,
  .bp-exit:active { transform: translateX(-2px) scale(0.97); }
  .bp-settings {
    display: grid; place-items: center;
    width: 34px; height: 34px;
    border: 1px solid var(--border-hover);
    border-radius: 50%;
    background: rgba(7, 9, 15, 0.45);
    color: var(--text-secondary);
    cursor: pointer;
    backdrop-filter: blur(6px);
    transition: color 0.18s ease, border-color 0.18s ease;
  }
  .bp-settings:hover { color: var(--text-primary); border-color: var(--text-muted); }
  .bp-search:focus-visible, .bp-exit:focus-visible, .bp-settings:focus-visible, .bp-nav button:focus-visible { outline: none; box-shadow: var(--ring-switch, 0 0 0 3px rgba(232,85,127,.45)); }

  .bp-stage {
    flex: 1; min-height: 0;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    padding: 0 36px 12px;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: rgba(255,255,255,0.08) transparent;
  }
  .bp-stage::-webkit-scrollbar { width: 3px; }
  .bp-stage::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }

  .bp-hints {
    display: flex; align-items: center; gap: 18px;
    padding: 8px 36px 14px;
    color: var(--text-muted); font-size: 12px;
    flex-shrink: 0;
  }
  .bp-hints b {
    display: inline-grid; place-items: center; min-width: 18px; height: 18px;
    margin-right: 5px; padding: 0 4px;
    border: 1px solid var(--border-hover); border-radius: 4px;
    font-size: 10px; color: var(--text-secondary); font-family: var(--font-mono);
  }
  .bp-pos { margin-left: auto; font-family: var(--font-mono); }

  @media (prefers-reduced-motion: reduce) {
    .bp-nav button, .bp-exit, .bp-settings { transition: none; }
  }
  :global([data-motion="reduce"]) .bp-nav button,
  :global([data-motion="reduce"]) .bp-exit,
  :global([data-motion="reduce"]) .bp-settings { transition: none; }
</style>
