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
  let wheelLockUntil = 0;
  let topNavEl = $state<HTMLElement>();
  let detailReturnFocus = $state<HTMLElement | null>(null);
  let searchReturnFocus = $state<HTMLElement | null>(null);
  let detailReturnZone = $state<BigPictureZone>("wheel");
  let searchReturnZone = $state<BigPictureZone>("top-nav");
  let topScope: GamepadAttachment | null = null;

  const allGames = $derived(gameStore.allGames);
  const installedGames = $derived(gameStore.installedGames);
  const filteredGames = $derived(filterAll ? allGames : installedGames);
  const focusGame = $derived(filteredGames[focusIdx] ?? null);
  const backgroundArt = $derived(pickBackgroundArt(focusGame));
  const isHeroBg = $derived(hasHeroBackground(focusGame));
  const clock = $derived(now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }));

  const TOP_ITEMS = ["game", "media", "search", "exit", "settings"] as const;
  const SCENE_PALETTES = [
    { accent: "#ff4d5f", paper: "#f8f2e8", ink: "#0b0b0e" },
    { accent: "#75e0d1", paper: "#f1f4e9", ink: "#08100f" },
    { accent: "#ffd34d", paper: "#fff8df", ink: "#100e08" },
    { accent: "#9d82ff", paper: "#f4efff", ink: "#0d0a14" },
    { accent: "#ff7ab5", paper: "#fff0f5", ink: "#13090e" },
    { accent: "#7bb8ff", paper: "#eef6ff", ink: "#080e16" },
  ] as const;
  const scenePalette = $derived(SCENE_PALETTES[focusIdx % SCENE_PALETTES.length]);

  function pickBackgroundArt(game: Game | null): string {
    if (!game) return defaultLibraryBackdrop;
    return fileSrc(gameHeroImageOf(game)) ?? defaultLibraryBackdrop;
  }

  const weekHours = $derived.by(() => {
    const since = Date.now() - 7 * 86400000;
    let seconds = 0;
    for (const game of allGames) {
      for (const session of game.play_tracker?.sessions ?? []) {
        if (new Date(session.start_time).getTime() >= since) seconds += session.duration_seconds ?? 0;
      }
    }
    return (seconds / 3600).toFixed(1);
  });

  function move(delta: number) {
    if (filteredGames.length === 0) return;
    focusIdx = Math.max(0, Math.min(filteredGames.length - 1, focusIdx + delta));
  }
  function setFocus(index: number) { if (index >= 0 && index < filteredGames.length) focusIdx = index; }
  function focusTop(index = topFocusIdx) {
    topFocusIdx = Math.max(0, Math.min(TOP_ITEMS.length - 1, index));
    queueMicrotask(() => topNavEl?.querySelector<HTMLElement>(`[data-top-index="${topFocusIdx}"]`)?.focus({ preventScroll: true }));
  }
  function setZone(zone: BigPictureZone, focus = true) {
    activeZone = zone;
    getDefaultGamepadFocusRuntime()?.setActiveZone(zone);
    if (focus && zone === "top-nav") focusTop();
  }
  function enterContent() { setZone(bpTab === "game" ? "wheel" : "media"); }
  function switchTab(tab: BigPictureTab, enter = true) {
    bpTab = tab;
    topFocusIdx = tab === "game" ? 0 : 1;
    if (tab === "media") void animeStore.loadRecommendations();
    setZone(enter ? (tab === "game" ? "wheel" : "media") : "top-nav");
  }
  function cycleTab(direction: -1 | 1) { switchTab(direction < 0 ? "game" : "media", true); }
  function moveTop(direction: number) { focusTop((topFocusIdx + direction + TOP_ITEMS.length) % TOP_ITEMS.length); }
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
  function selectMedia(item: { type: string }) { uiStore.setBigPicture(false); uiStore.currentView = item.type === "anime" ? "anime" : "comic"; }

  function onWheel(event: WheelEvent) {
    if (showDetail || showSearch || bpTab !== "game" || filteredGames.length === 0) return;
    if (Math.abs(event.deltaY) < 8 && Math.abs(event.deltaX) < 8) return;
    event.preventDefault();
    const time = performance.now();
    if (time < wheelLockUntil) return;
    wheelLockUntil = time + (prefersReducedMotion ? 120 : 340);
    const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
    move(delta > 0 ? 1 : -1);
    setZone("wheel");
  }

  function onGlobalKeydown(event: KeyboardEvent) {
    if (showDetail || showSearch) return;
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement) return;
    if (event.key === "/") { event.preventDefault(); openSearch(); return; }
    if (event.key === "q" || event.key === "Q") { event.preventDefault(); cycleTab(-1); return; }
    if (event.key === "e" || event.key === "E") { event.preventDefault(); cycleTab(1); return; }
    if (activeZone === "top-nav") {
      switch (event.key) {
        case "ArrowLeft": event.preventDefault(); moveTop(-1); break;
        case "ArrowRight": event.preventDefault(); moveTop(1); break;
        case "ArrowDown": event.preventDefault(); enterContent(); break;
        case "Enter": case " ": event.preventDefault(); activateTop(); break;
        case "Escape": case "b": case "B": event.preventDefault(); exitBigPicture(); break;
      }
      return;
    }
    if (bpTab === "game") {
      if (event.key === "f" || event.key === "F") { event.preventDefault(); toggleFilter(); }
      if (event.key === "y" || event.key === "Y") { event.preventDefault(); openDetail(); }
      if (event.key === "i" || event.key === "I") { event.preventDefault(); openImport(); }
    }
  }

  $effect(() => { if (focusIdx >= filteredGames.length) focusIdx = Math.max(0, filteredGames.length - 1); });
  $effect(() => { if (focusGame && gameStore.selectedGame?.id !== focusGame.id) gameStore.selectGame(focusGame.id); });
  $effect(() => { const zone = activeZone; if (!showDetail && !showSearch) getDefaultGamepadFocusRuntime()?.setActiveZone(zone); });
  $effect(() => {
    const next = backgroundArt;
    if (!next || next === bgCurrent) return;
    if (bgTimer) { clearTimeout(bgTimer); bgTimer = null; }
    if (prefersReducedMotion) { bgCurrent = next; bgPrevious = null; bgFading = false; return; }
    bgPrevious = bgCurrent; bgCurrent = next; bgFading = true;
    bgTimer = setTimeout(() => { bgPrevious = null; bgFading = false; bgTimer = null; }, 720);
  });

  onMount(() => {
    const timer = setInterval(() => (now = new Date()), 30_000);
    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const syncMotion = () => {
      prefersReducedMotion = motionQuery.matches || document.documentElement.dataset.motion === "reduce";
      if (prefersReducedMotion) { bgPrevious = null; bgFading = false; }
    };
    syncMotion();
    motionQuery.addEventListener("change", syncMotion);
    topScope = attachGamepad({
      left: () => moveTop(-1), right: () => moveTop(1), down: () => enterContent(),
      launch: () => activateTop(), activate: () => activateTop(), back: () => exitBigPicture(),
      pageLeft: () => cycleTab(-1), pageRight: () => cycleTab(1),
      filter: () => { if (bpTab === "game") toggleFilter(); },
    }, { id: "big-picture-top-nav", zone: "top-nav", priority: 20 });
    getDefaultGamepadFocusRuntime()?.setActiveZone(activeZone);
    queueMicrotask(() => setZone(activeZone));
    return () => {
      clearInterval(timer);
      if (bgTimer) clearTimeout(bgTimer);
      motionQuery.removeEventListener("change", syncMotion);
      topScope?.(); topScope = null;
    };
  });
</script>

<svelte:window onkeydown={onGlobalKeydown} />

<section
  class="bp"
  onwheel={onWheel}
  data-active-zone={activeZone}
  data-tab={bpTab}
  aria-label="大屏模式"
  style={`--scene-accent:${scenePalette.accent};--scene-paper:${scenePalette.paper};--scene-ink:${scenePalette.ink}`}
>
  <BigPictureBackground current={bgCurrent} previous={bgPrevious} fading={bgFading} isCover={!isHeroBg} />
  <div class="bp-grid" aria-hidden="true"></div>
  <div class="bp-layout" aria-hidden="true"></div>

  <header class="bp-top" bind:this={topNavEl}>
    <div class="bp-brand" aria-label="MoePlay 大屏模式">
      <span class="bp-brand-mark">萌</span>
      <span><strong>MOEPLAY</strong><small>SCROLLING THEATRE</small></span>
    </div>

    <nav class="bp-nav" aria-label="大屏分类">
      <button class:active={bpTab === "game"} data-top-index="0" tabindex={activeZone === "top-nav" && topFocusIdx === 0 ? 0 : -1} aria-current={bpTab === "game" ? "page" : undefined} aria-label="游戏" onclick={() => switchTab("game")} onfocus={() => { topFocusIdx = 0; activeZone = "top-nav"; }}>
        <span>01</span><b>游戏舞台</b>
      </button>
      <button class:active={bpTab === "media"} data-top-index="1" tabindex={activeZone === "top-nav" && topFocusIdx === 1 ? 0 : -1} aria-current={bpTab === "media" ? "page" : undefined} aria-label="媒体" onclick={() => switchTab("media")} onfocus={() => { topFocusIdx = 1; activeZone = "top-nav"; }}>
        <span>02</span><b>媒体展廊</b>
      </button>
    </nav>

    <div class="bp-tools">
      <button data-top-index="2" tabindex={activeZone === "top-nav" && topFocusIdx === 2 ? 0 : -1} onclick={openSearch} onfocus={() => { topFocusIdx = 2; activeZone = "top-nav"; }} aria-label="搜索游戏"><Icon name="search" size={18} /><small>SEARCH</small></button>
      <div class="bp-time"><strong>{clock}</strong><small>{String(focusIdx + 1).padStart(2,"0")} / {String(filteredGames.length).padStart(2,"0")}</small></div>
      <button data-top-index="3" tabindex={activeZone === "top-nav" && topFocusIdx === 3 ? 0 : -1} onclick={exitBigPicture} onfocus={() => { topFocusIdx = 3; activeZone = "top-nav"; }} aria-label="退出大屏"><Icon name="shrink" size={18} /></button>
      <button data-top-index="4" tabindex={activeZone === "top-nav" && topFocusIdx === 4 ? 0 : -1} onclick={openSettings} onfocus={() => { topFocusIdx = 4; activeZone = "top-nav"; }} aria-label="设置"><Icon name="gear" size={18} /></button>
    </div>
  </header>

  {#if bpTab === "game"}
    <main class="bp-game-view">
      {#if focusGame}
        <BigPictureHero game={focusGame} {weekHours} active={activeZone === "hero" && !showDetail && !showSearch} onLaunch={launchFocus} onFavorite={toggleFav} onDetail={openDetail} onMoveToWheel={() => setZone("wheel")} onMoveToTop={() => setZone("top-nav")} onTabPrevious={() => cycleTab(-1)} onTabNext={() => cycleTab(1)} onToggleFilter={toggleFilter} />
      {/if}
      <BigPictureWheel games={filteredGames} {focusIdx} {filterAll} {prefersReducedMotion} active={activeZone === "wheel" && !showDetail && !showSearch} onSelect={setFocus} onActivate={(index) => { setFocus(index); openDetail(); }} onLaunch={(index) => { setFocus(index); void launchFocus(); }} onFavorite={(index) => { setFocus(index); void toggleFav(); }} onMoveToHero={() => setZone("hero")} onMoveToTop={() => setZone("top-nav")} onBack={back} onTabPrevious={() => cycleTab(-1)} onTabNext={() => cycleTab(1)} onToggleFilter={toggleFilter} onOpenImport={openImport} />
    </main>
    <footer class="bp-hints" aria-label="手柄快捷操作">
      <span><b>LS / ← →</b>切换作品</span><span><b class="key-a">A</b>打开档案</span><span><b>Start</b>启动</span><span><b class="key-x">X</b>收藏</span><span><b>LB RB</b>切换展厅</span><span><b>View</b>{filterAll ? "本机安装" : "全部作品"}</span><span><b>B</b>菜单</span><span class="bp-pos">{filteredGames.length ? String(focusIdx + 1).padStart(2,"0") : "00"} / {String(filteredGames.length).padStart(2,"0")}</span>
    </footer>
  {:else}
    <main class="bp-media-view"><BigPictureMediaTab active={activeZone === "media" && !showDetail && !showSearch} onSelectMedia={selectMedia} onMoveToTop={() => setZone("top-nav")} onBack={back} onTabPrevious={() => cycleTab(-1)} onTabNext={() => cycleTab(1)} /></main>
    <footer class="bp-hints" aria-label="手柄快捷操作"><span><b class="key-a">A</b>打开</span><span><b>↑ ↓ ← →</b>浏览</span><span><b>LB RB</b>切换展厅</span><span><b>B</b>菜单</span></footer>
  {/if}

  {#if showDetail && focusGame}<BigPictureDetail game={focusGame} onClose={closeDetail} returnFocus={detailReturnFocus} />{/if}
  <BPSearch bind:open={showSearch} returnFocus={searchReturnFocus} onzonechange={(zone) => { activeZone = zone; }} onclose={closeSearch} onselect={(game) => { switchTab("game", true); setFocus(filteredGames.findIndex((item) => item.id === game.id)); }} />
</section>

<style>
  .bp {
    --bp-safe-x: clamp(28px, 3.2vw, 74px);
    position: relative; z-index: 1; flex: 1; width: 100%; height: 100%; min-width: 0; min-height: 0;
    overflow: hidden; color: var(--scene-paper); background: #08090c; user-select: none; isolation: isolate;
  }
  .bp-layout { position:absolute; inset:0; z-index:3; pointer-events:none; }
  .bp-grid { position:absolute; inset:0; z-index:2; pointer-events:none; opacity:.16; background-image: linear-gradient(rgba(255,255,255,.12) 1px,transparent 1px), linear-gradient(90deg,rgba(255,255,255,.12) 1px,transparent 1px); background-size: 8.333vw 8.333vw; mask-image: linear-gradient(90deg,black,transparent 45%); }

  .bp-top { position:absolute; z-index:50; top:0; left:0; right:0; display:grid; grid-template-columns: 1fr auto 1fr; align-items:center; min-height:clamp(70px,8.5vh,96px); padding: max(12px,env(safe-area-inset-top)) var(--bp-safe-x) 0; }
  .bp-brand { display:flex; align-items:center; gap:12px; }
  .bp-brand-mark { display:grid; place-items:center; width:clamp(36px,3vw,48px); aspect-ratio:1; color:#08090c; background:var(--scene-accent); font:900 clamp(17px,1.4vw,23px) serif; transition:background .6s ease; }
  .bp-brand > span:last-child { display:grid; gap:2px; }
  .bp-brand strong { font:900 clamp(14px,1.1vw,20px) var(--font-display); letter-spacing:.09em; }
  .bp-brand small { color:rgba(255,255,255,.4); font:750 7px var(--font-mono); letter-spacing:.24em; }

  .bp-nav { display:flex; align-items:center; gap:clamp(18px,2.4vw,42px); }
  .bp-nav button { position:relative; display:flex; align-items:baseline; gap:8px; padding:10px 1px; border:0; color:rgba(255,255,255,.4); background:transparent; cursor:pointer; }
  .bp-nav button::after { content:""; position:absolute; left:0; right:100%; bottom:2px; height:2px; background:var(--scene-accent); transition:right .25s ease; }
  .bp-nav button.active { color:white; }
  .bp-nav button.active::after { right:0; }
  .bp-nav span { color:var(--scene-accent); font:850 8px var(--font-mono); }
  .bp-nav b { font:800 clamp(11px,.85vw,15px) var(--font-ui); }
  .bp-nav button:focus-visible,.bp-tools button:focus-visible { outline:2px solid var(--scene-accent); outline-offset:5px; }

  .bp-tools { justify-self:end; display:flex; align-items:center; gap:9px; }
  .bp-tools button { display:grid; place-items:center; min-width:42px; height:42px; padding:0 11px; border:1px solid rgba(255,255,255,.14); color:rgba(255,255,255,.72); background:rgba(7,8,12,.28); backdrop-filter:blur(12px); cursor:pointer; }
  .bp-tools button:first-child { display:flex; gap:8px; }
  .bp-tools button small { font:750 7px var(--font-mono); letter-spacing:.14em; }
  .bp-time { display:grid; justify-items:end; min-width:68px; margin:0 7px; }
  .bp-time strong { font:850 clamp(14px,1.1vw,20px) var(--font-mono); }
  .bp-time small { color:var(--scene-accent); font:750 7px var(--font-mono); letter-spacing:.14em; }

  .bp-game-view,.bp-media-view { position:absolute; inset:0; z-index:10; overflow:hidden; }
  .bp-hints { position:absolute; z-index:60; left:var(--bp-safe-x); right:var(--bp-safe-x); bottom:max(12px,env(safe-area-inset-bottom)); display:flex; align-items:center; gap:clamp(14px,1.5vw,26px); min-height:34px; color:rgba(255,255,255,.5); font-size:clamp(9px,.68vw,11px); white-space:nowrap; }
  .bp-hints::before { content:""; position:absolute; left:0; right:0; top:-11px; height:1px; background:linear-gradient(90deg,var(--scene-accent),rgba(255,255,255,.14) 12%,rgba(255,255,255,.14)); }
  .bp-hints span { display:flex; align-items:center; gap:6px; }
  .bp-hints b { display:inline-grid; place-items:center; min-width:22px; height:22px; padding:0 6px; border:1px solid rgba(255,255,255,.18); color:white; background:rgba(7,8,12,.56); font:800 8px var(--font-mono); }
  .bp-hints .key-a { border-color:rgba(101,214,158,.6); color:#8be8b8; }
  .bp-hints .key-x { border-color:rgba(105,165,255,.6); color:#90bcff; }
  .bp-pos { margin-left:auto; color:var(--scene-accent); font:850 10px var(--font-mono); letter-spacing:.12em; }

  :global(:root[data-input-mode="gamepad"] section.bp :where(button,a,input,select,textarea,[tabindex]):focus-visible) {
    outline-color: var(--scene-accent) !important;
    box-shadow: 0 0 0 2px rgba(7,8,12,.82), 0 0 24px color-mix(in srgb,var(--scene-accent) 48%,transparent) !important;
  }

  @media (max-width:1180px) {
    .bp-top { grid-template-columns:auto 1fr auto; }
    .bp-nav { justify-self:center; }
    .bp-tools button small,.bp-brand small { display:none; }
    .bp-tools button:first-child { min-width:42px; }
    .bp-hints span:nth-child(n+6) { display:none; }
  }
  @media (max-width:850px) { .bp-brand > span:last-child,.bp-time { display:none; } .bp-nav { gap:14px; } }
  @media (max-height:760px) { .bp-top { min-height:64px; padding-top:8px; } .bp-hints { bottom:7px; } .bp-hints span:nth-child(5) { display:none; } }
  @media (min-width:2800px) { .bp { --bp-safe-x:clamp(70px,4vw,150px); } }
  @media (prefers-reduced-motion:reduce) { .bp-brand-mark,.bp-nav button::after { transition:none; } }
  :global([data-motion="reduce"]) .bp-brand-mark,:global([data-motion="reduce"]) .bp-nav button::after { transition:none; }
</style>
