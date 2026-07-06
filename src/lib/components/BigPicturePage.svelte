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
  import { attachGamepad } from "./switch/useGamepad.svelte";
  import defaultLibraryBackdrop from "../assets/default-library-backdrop.png";
  import { animeStore } from "../stores/anime.svelte";

  let bpTab = $state<"game" | "media">("game");
  let focusIdx = $state(0);
  let filterAll = $state(true);
  let showDetail = $state(false);
  let showSearch = $state(false);
  let now = $state(new Date());
  let bgCurrent = $state<string>(defaultLibraryBackdrop);
  let bgPrevious = $state<string | null>(null);
  let bgFading = $state(false);
  let prefersReducedMotion = $state(false);
  let bgTimer: ReturnType<typeof setTimeout> | null = null;

  const games = $derived(gameStore.games);
  const allGames = $derived(gameStore.allGames);
  const filteredGames = $derived(filterAll ? games : allGames.filter((g) => g.install_dir || g.exe_path));
  const focusGame = $derived(filteredGames[focusIdx] ?? null);

  const backgroundArt = $derived(pickBackgroundArt(focusGame));
  const isHeroBg = $derived(hasHeroBackground(focusGame));
  const clock = $derived(now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }));

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
  $effect(() => {
    if (focusIdx >= filteredGames.length) focusIdx = Math.max(0, filteredGames.length - 1);
  });
  $effect(() => {
    if (focusGame && gameStore.selectedGame?.id !== focusGame.id) gameStore.selectGame(focusGame.id);
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

  function openDetail() { if (focusGame) showDetail = true; }
  function closeDetail() { showDetail = false; }
  async function launchFocus() { if (focusGame) await gameStore.launch(focusGame.id); }
  async function toggleFav() { if (focusGame) await gameStore.toggleFavorite(focusGame.id); }
  function openScraper() {
    if (focusGame) gameStore.selectGame(focusGame.id);
    uiStore.setBigPicture(false);
    uiStore.currentView = "scraper";
  }
  function openImport() { uiStore.setBigPicture(false); uiStore.currentView = "steam-import"; }
  function openSettings() { uiStore.setBigPicture(false); uiStore.currentView = "settings"; }
  function back() { if (showDetail) { closeDetail(); return; } uiStore.setBigPicture(false); }
  function toggleFilter() { filterAll = !filterAll; focusIdx = 0; }
  function selectMedia(item: { type: string }) {
    uiStore.setBigPicture(false);
    uiStore.currentView = item.type === "anime" ? "anime" : "comic";
  }

  function onWheel(e: WheelEvent) {
    if (showDetail || filteredGames.length === 0) return;
    if (Math.abs(e.deltaY) < 1 && Math.abs(e.deltaX) < 1) return;
    e.preventDefault();
    move(e.deltaY > 0 ? 1 : e.deltaY < 0 ? -1 : e.deltaX > 0 ? 1 : -1);
  }
  function onKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case "ArrowRight": case "d": case "D": case "ArrowDown": e.preventDefault(); move(1); break;
      case "ArrowLeft": case "a": case "A": case "ArrowUp": e.preventDefault(); move(-1); break;
      case "PageDown": e.preventDefault(); move(6); break;
      case "PageUp": e.preventDefault(); move(-6); break;
      case "Home": e.preventDefault(); setFocus(0); break;
      case "End": e.preventDefault(); setFocus(filteredGames.length - 1); break;
      case " ": case "l": case "L": e.preventDefault(); launchFocus(); break;
      case "Enter": e.preventDefault(); openDetail(); break;
      case "Escape": case "b": case "B": e.preventDefault(); back(); break;
      case "f": case "F": e.preventDefault(); toggleFilter(); break;
      case "x": case "X": e.preventDefault(); toggleFav(); break;
      case "y": case "Y": e.preventDefault(); openScraper(); break;
      case "i": case "I": e.preventDefault(); openImport(); break;
      case "/": e.preventDefault(); showSearch = true; break;
    }
  }

  onMount(() => {
    const t = setInterval(() => (now = new Date()), 30_000);
    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const syncMotion = () => {
      prefersReducedMotion = motionQuery.matches;
      if (prefersReducedMotion) { bgPrevious = null; bgFading = false; }
    };
    syncMotion();
    motionQuery.addEventListener("change", syncMotion);
    const detach = attachGamepad({
      left: () => move(-1), right: () => move(1),
      pageLeft: () => move(-6), pageRight: () => move(6),
      launch: () => launchFocus(), back: () => back(),
      favorite: () => toggleFav(), activate: () => openDetail(),
    });
    return () => {
      clearInterval(t);
      if (bgTimer) clearTimeout(bgTimer);
      motionQuery.removeEventListener("change", syncMotion);
      detach();
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

<section class="bp" onwheel={onWheel}>
  <BigPictureBackground current={bgCurrent} previous={bgPrevious} fading={bgFading} isCover={!isHeroBg} />

  <div class="bp-layout">
    {#if bpTab === "game"}
      <BigPictureWheel
        games={filteredGames}
        {focusIdx}
        {filterAll}
        {prefersReducedMotion}
        onSelect={setFocus}
        onActivate={(i) => { setFocus(i); openDetail(); }}
        onToggleFilter={toggleFilter}
        onOpenImport={openImport}
      />
    {/if}

    <div class="bp-main">
      <header class="bp-top">
        <nav class="bp-nav">
          <button class:active={bpTab === "game"} onclick={() => { bpTab = "game"; }}>游戏</button>
          <button class:active={bpTab === "media"} onclick={() => { bpTab = "media"; animeStore.loadRecommendations(); }}>媒体</button>
        </nav>
        <div class="bp-top-right">
          <button class="bp-exit" onclick={back} title="退出大屏（Esc / 手柄 B）">
            <Icon name="chevronLeft" size={16} />
            <span>退出大屏</span>
          </button>
          <span class="bp-clock">{clock}</span>
          <button class="bp-settings" onclick={openSettings} title="设置" aria-label="设置">
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
              onLaunch={launchFocus}
              onFavorite={toggleFav}
              onDetail={openDetail}
            />
          {/if}
        </div>

        <footer class="bp-hints">
          <span><b>A</b> 启动</span>
          <span><b>B</b> 返回</span>
          <span><b>X</b> 收藏</span>
          <span><b>Y</b> 刮削</span>
          <span><b>Enter</b> 详情</span>
          <span><b>F</b> {filterAll ? "已安装" : "全部"}</span>
          <span class="bp-pos">{filteredGames.length ? focusIdx + 1 : 0} / {filteredGames.length}</span>
        </footer>
      {:else}
        <BigPictureMediaTab onSelectMedia={selectMedia} />

        <footer class="bp-hints">
          <span><b>B</b> 返回</span>
          <span><b>Enter</b> 打开</span>
        </footer>
      {/if}
    </div>
  </div>

  {#if showDetail && focusGame}
    <BigPictureDetail game={focusGame} onClose={closeDetail} />
  {/if}

  <BPSearch bind:open={showSearch} onselect={(g) => { setFocus(filteredGames.findIndex(f => f.id === g.id)); }} />
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
  .bp-exit:hover {
    background: var(--accent, #e8557f);
    color: #fff;
    transform: translateX(-2px);
  }
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
</style>
