<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import type { Game } from "../stores/games.svelte";
  import Icon from "./Icon.svelte";
  import BigPictureDetail from "./BigPictureDetail.svelte";
  import RatingRing from "./RatingRing.svelte";
  import { formatPlayTime } from "../api";
  import { fileSrc } from "../utils";
  import {
    coverOf as gameCoverOf,
    developerOf as gameDeveloperOf,
    gameCompletionStatus,
    gameLastPlayed,
    gameRating,
    gameTotalSeconds,
    heroImageOf as gameHeroImageOf,
    releaseYearOf,
    tagsOf as gameTagsOf,
  } from "../utils/game";
  import { attachGamepad } from "./switch/useGamepad.svelte";
  import defaultLibraryBackdrop from "../assets/default-library-backdrop.png";

  const STATUS: Record<string, string> = {
    not_started: "未开始", playing: "游玩中", completed: "已通关",
    on_hold: "搁置", dropped: "已弃坑", plan_to_play: "计划中", replaying: "重温中",
  };

  let focusIdx = $state(0);
  let filterAll = $state(true);
  let showDetail = $state(false);
  let railEl = $state<HTMLDivElement>();
  let now = $state(new Date());
  let bgCurrent = $state<string>(defaultLibraryBackdrop);
  let bgPrevious = $state<string | null>(null);
  let bgFading = $state(false);
  let prefersReducedMotion = $state(false);
  let bgTimer: ReturnType<typeof setTimeout> | null = null;

  const games = $derived(gameStore.games);
  const allGames = $derived(gameStore.allGames);
  const filteredGames = $derived(filterAll ? games : allGames.filter((g) => !!g.exe_path));
  const focusGame = $derived(filteredGames[focusIdx] ?? null);

  const backgroundArt = $derived(pickBackgroundArt(focusGame));
  const scoreValue = $derived(
    focusGame ? Math.round(Math.min(10, Math.max(0, rating(focusGame))) * 10) : 0
  );
  const clock = $derived(now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }));

  function rating(g: Game | null): number {
    if (!g) return 0;
    return gameRating(g);
  }
  function pickBackgroundArt(g: Game | null): string {
    if (!g) return defaultLibraryBackdrop;
    return fileSrc(gameHeroImageOf(g)) ?? defaultLibraryBackdrop;
  }
  function allTags(g: Game | null): string[] {
    return gameTagsOf(g);
  }
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
  const monogram = (g: Game) => (g.name?.trim()?.[0] ?? "?").toUpperCase();
  const desc = $derived(
    focusGame?.description?.trim() || (focusGame ? allTags(focusGame).slice(0, 6).join(" / ") : "") || "暂无简介"
  );
  const trimmedDesc = $derived(desc.length > 260 ? desc.slice(0, 260) + "…" : desc);

  const achTotal = $derived(focusGame?.play_tracker?.achievements_total ?? 0);
  const achDone = $derived(focusGame?.play_tracker?.achievements_unlocked ?? 0);
  function timeAgo(v: string | null | undefined): string {
    if (!v) return "尚未游玩";
    const days = Math.floor((Date.now() - new Date(v).getTime()) / 86400000);
    if (days <= 0) return "今天";
    if (days === 1) return "昨天";
    if (days < 7) return `${days} 天前`;
    if (days < 30) return `${Math.floor(days / 7)} 周前`;
    return `${Math.floor(days / 30)} 个月前`;
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

  // ---- nav ----
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
    const idx = focusIdx;
    queueMicrotask(() => {
      railEl?.querySelector<HTMLElement>(`[data-idx="${idx}"]`)?.scrollIntoView({
        inline: "center",
        block: "nearest",
        behavior: prefersReducedMotion ? "auto" : "smooth",
      });
    });
  });
  // keep global selection in sync (shared with detail / Switch home)
  $effect(() => {
    if (focusGame && gameStore.selectedGame?.id !== focusGame.id) gameStore.selectGame(focusGame.id);
  });
  $effect(() => {
    const next = backgroundArt;
    if (!next || next === bgCurrent) return;
    if (bgTimer) {
      clearTimeout(bgTimer);
      bgTimer = null;
    }
    if (prefersReducedMotion) {
      bgCurrent = next;
      bgPrevious = null;
      bgFading = false;
      return;
    }
    bgPrevious = bgCurrent;
    bgCurrent = next;
    bgFading = true;
    bgTimer = setTimeout(() => {
      bgPrevious = null;
      bgFading = false;
      bgTimer = null;
    }, 640);
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
  function back() { if (showDetail) { closeDetail(); return; } uiStore.setBigPicture(false); }
  function toggleFilter() { filterAll = !filterAll; focusIdx = 0; }

  function onWheel(e: WheelEvent) {
    if (showDetail || filteredGames.length === 0) return;
    if (Math.abs(e.deltaY) < 1 && Math.abs(e.deltaX) < 1) return;
    e.preventDefault();
    move(e.deltaY > 0 || e.deltaX > 0 ? 1 : -1);
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
    }
  }

  onMount(() => {
    const t = setInterval(() => (now = new Date()), 30_000);
    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const syncMotion = () => {
      prefersReducedMotion = motionQuery.matches;
      if (prefersReducedMotion) {
        bgPrevious = null;
        bgFading = false;
      }
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
  <div class="bp-bg">
    {#if bgPrevious}
      <div class="bp-bg-layer bp-bg-layer-prev" class:fade-out={bgFading} style={`background-image: url("${bgPrevious}")`}></div>
    {/if}
    <div class="bp-bg-layer bp-bg-layer-current" class:fade-in={bgFading} style={`background-image: url("${bgCurrent}")`}></div>
  </div>
  <div class="bp-scrim"></div>

  <header class="bp-top">
    <nav class="bp-nav">
      <span class="active">游戏</span>
      <span>媒体</span>
    </nav>
    <div class="bp-top-right">
      <span class="bp-count">{filteredGames.length} 款</span>
      <button class="bp-chip" onclick={toggleFilter}>{filterAll ? "全部" : "已安装"}</button>
      <span class="bp-clock">{clock}</span>
    </div>
  </header>

  <div class="bp-stage">
    {#if focusGame}
      <div class="bp-hero">
        {#if focusGame.metadata?.original_name}
          <p class="bp-jp">{focusGame.metadata.original_name}</p>
        {/if}
        <h1 class="bp-title">{focusGame.name}</h1>
        <p class="bp-meta">{metaLine(focusGame) || "未知社团"}</p>

        <div class="bp-actions">
          <button class="bp-play" onclick={launchFocus}>
            <Icon name="play" size={22} /><span>开始游戏</span>
          </button>
          <button class="bp-secondary" class:active={focusGame.favorite} onclick={toggleFav}>
            <Icon name={focusGame.favorite ? "heartFill" : "heart"} size={18} />
            <span>{focusGame.favorite ? "已收藏" : "收藏"}</span>
          </button>
          <button class="bp-secondary" onclick={openDetail}>
            <Icon name="database" size={18} /><span>详情</span>
          </button>
        </div>

        <div class="bp-tags">
          {#each allTags(focusGame).slice(0, 7) as t}
            <span class="bp-tag">{t}</span>
          {/each}
        </div>
        <p class="bp-desc">{trimmedDesc}</p>
      </div>

      <aside class="bp-cards">
        <div class="bp-card-stat bp-rating">
          <RatingRing value={scoreValue} max={100} size={92} />
          <span>评分</span>
        </div>
        <div class="bp-card-stat">
          <strong>{achTotal > 0 ? `${achDone}/${achTotal}` : "—"}</strong>
          <span>成就</span>
        </div>
        <div class="bp-card-stat">
          <strong>{formatPlayTime(gameTotalSeconds(focusGame))}</strong>
          <span>游玩时长</span>
        </div>
        <div class="bp-card-stat">
          <strong>{timeAgo(gameLastPlayed(focusGame))}</strong>
          <span>最后游玩</span>
        </div>
        <div class="bp-card-stat">
          <strong>{weekHours}h</strong>
          <span>本周</span>
        </div>
      </aside>
    {/if}
  </div>

  <div class="bp-rail-wrap">
    <div class="bp-rail" bind:this={railEl} role="listbox" aria-label="大屏游戏列表">
      {#each filteredGames as g, i (g.id)}
        <button
          class="bp-card"
          class:focus={i === focusIdx}
          data-idx={i}
          role="option"
          aria-selected={i === focusIdx}
          onclick={() => setFocus(i)}
          ondblclick={() => { setFocus(i); openDetail(); }}
          onfocus={() => setFocus(i)}
          aria-label={g.name}
          aria-current={i === focusIdx ? "true" : undefined}
          tabindex={i === focusIdx ? 0 : -1}
        >
          {#if fileSrc(gameCoverOf(g))}
            <img src={fileSrc(gameCoverOf(g))!} alt={g.name} draggable="false" loading="lazy" />
          {:else}
            <span class="bp-mono">{monogram(g)}</span>
          {/if}
        </button>
      {/each}
      {#if filteredGames.length === 0}
        <div class="bp-empty">
          <p>暂无游戏</p>
          <button class="bp-empty-action" onclick={openImport}><Icon name="download" size={16} /> Steam / Epic 导入</button>
        </div>
      {/if}
    </div>
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

  {#if showDetail && focusGame}
    <BigPictureDetail game={focusGame} onClose={closeDetail} />
  {/if}
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

  .bp-bg { position: absolute; inset: 0; z-index: 0; }
  .bp-bg-layer {
    position: absolute; inset: 0;
    background-size: cover; background-position: center 28%;
    filter: saturate(0.96) contrast(1.05);
    opacity: 1;
    will-change: opacity;
  }
  .bp-bg-layer-current.fade-in {
    animation: bpBgIn 0.6s cubic-bezier(0.45, 0, 0.2, 1) both;
  }
  .bp-bg-layer-prev.fade-out {
    animation: bpBgOut 0.6s cubic-bezier(0.45, 0, 0.2, 1) both;
  }
  @keyframes bpBgIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  @keyframes bpBgOut {
    from { opacity: 1; }
    to { opacity: 0; }
  }
  .bp-scrim {
    position: absolute; inset: 0; z-index: 1; pointer-events: none;
    background:
      linear-gradient(90deg, rgba(7,9,15,0.92) 0%, rgba(7,9,15,0.74) 28%, rgba(7,9,15,0.20) 60%, rgba(7,9,15,0.55) 100%),
      linear-gradient(180deg, rgba(7,9,15,0.55) 0%, rgba(7,9,15,0.05) 32%, rgba(7,9,15,0.72) 78%, var(--bg-void) 100%);
  }

  .bp-top {
    position: relative; z-index: 3;
    display: flex; align-items: center; justify-content: space-between;
    padding: 22px 40px 0;
  }
  .bp-nav { display: flex; gap: 22px; font-size: 16px; }
  .bp-nav span { color: var(--text-muted); cursor: default; }
  .bp-nav .active { color: var(--text-primary); font-weight: 700; }
  .bp-top-right { display: flex; align-items: center; gap: 16px; }
  .bp-count { color: var(--text-muted); font-size: 13px; }
  .bp-chip {
    border: 1px solid var(--border); background: rgba(7,9,15,0.4);
    color: var(--text-secondary); border-radius: var(--radius-full);
    padding: 5px 14px; font-size: 12px; cursor: pointer;
  }
  .bp-clock { font-family: var(--font-mono); font-variant-numeric: tabular-nums; color: var(--text-secondary); }

  .bp-stage {
    position: relative; z-index: 2;
    flex: 1; min-height: 0;
    display: flex; align-items: flex-end; justify-content: space-between;
    gap: 32px; padding: 0 40px 8px;
  }

  .bp-hero { max-width: 56%; padding-bottom: 8px; }
  .bp-jp { color: var(--text-muted); font-size: 15px; margin: 0 0 6px; }
  .bp-title {
    font-family: var(--font-display);
    font-size: clamp(38px, 5.4vw, 66px);
    font-weight: 800; line-height: 1.05; margin: 0 0 10px;
    text-shadow: 0 2px 24px rgba(0,0,0,0.5);
  }
  .bp-meta { color: var(--text-secondary); font-size: 15px; margin: 0 0 18px; }

  .bp-actions { display: flex; gap: 12px; margin-bottom: 16px; flex-wrap: wrap; }
  .bp-play {
    display: inline-flex; align-items: center; gap: 10px;
    border: none; cursor: pointer;
    background: var(--accent); color: #fff;
    font-size: 16px; font-weight: 700;
    padding: 13px 28px; border-radius: var(--radius-full);
    transition: transform 0.15s ease, background 0.18s ease;
  }
  .bp-play:hover { background: var(--accent-hi); transform: translateY(-1px); }
  .bp-secondary {
    display: inline-flex; align-items: center; gap: 8px;
    border: 1px solid var(--border-hover); cursor: pointer;
    background: rgba(7,9,15,0.45); color: var(--text-secondary);
    font-size: 14px; font-weight: 600;
    padding: 13px 20px; border-radius: var(--radius-full);
    backdrop-filter: blur(6px);
    transition: color 0.18s ease, border-color 0.18s ease;
  }
  .bp-secondary:hover { color: var(--text-primary); border-color: var(--text-muted); }
  .bp-secondary.active { color: var(--accent); }

  .bp-tags { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 14px; max-width: 640px; }
  .bp-tag {
    font-size: 12px; padding: 4px 11px; border-radius: var(--radius-full);
    background: rgba(255,255,255,0.08); color: var(--text-secondary);
  }
  .bp-desc {
    max-width: 620px; color: var(--text-secondary);
    font-size: 14px; line-height: 1.7; margin: 0;
  }

  .bp-cards {
    display: flex; flex-direction: column; gap: 10px;
    flex-shrink: 0; width: 188px;
  }
  .bp-card-stat {
    background: rgba(15,19,28,0.66);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    backdrop-filter: blur(12px);
    padding: 14px 16px;
    display: flex; flex-direction: column; gap: 2px;
  }
  .bp-card-stat strong { font-size: 18px; font-weight: 700; color: var(--text-primary); }
  .bp-card-stat span { font-size: 12px; color: var(--text-muted); }
  .bp-rating { flex-direction: row; align-items: center; gap: 12px; }
  .bp-rating span { font-size: 13px; }

  .bp-rail-wrap { position: relative; z-index: 2; padding: 6px 0 4px; }
  .bp-rail {
    display: flex; gap: 14px; align-items: flex-end;
    overflow-x: auto; overflow-y: hidden;
    padding: 16px 40px 18px;
    scrollbar-width: none;
  }
  .bp-rail::-webkit-scrollbar { display: none; }
  .bp-card {
    flex: 0 0 auto;
    width: 116px; height: 155px;
    border: none; padding: 0; cursor: pointer;
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg-elev);
    box-shadow: var(--shadow-tile);
    outline: 0;
    transition: transform 0.24s cubic-bezier(0.22,1,0.36,1), box-shadow 0.24s cubic-bezier(0.22,1,0.36,1);
    will-change: transform;
  }
  .bp-card img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .bp-mono {
    width: 100%; height: 100%; display: grid; place-items: center;
    font-family: var(--font-display); font-size: 32px; font-weight: 700;
    color: var(--text-muted);
    background: linear-gradient(135deg, rgba(232,85,127,0.18), rgba(110,120,160,0.14));
  }
  .bp-card.focus {
    transform: translateY(-6px) scale(1.06);
    box-shadow: var(--ring-ps5);
    z-index: 2;
  }
  .bp-card:focus-visible {
    box-shadow: var(--ring-ps5);
  }

  .bp-empty {
    display: flex; flex-direction: column; align-items: center; gap: 12px;
    width: 100%; padding: 28px; color: var(--text-muted);
  }
  .bp-empty-action {
    display: inline-flex; align-items: center; gap: 6px;
    border: none; cursor: pointer; background: var(--accent); color: #fff;
    padding: 9px 18px; border-radius: var(--radius-full); font-weight: 700; font-size: 13px;
  }

  .bp-hints {
    position: relative; z-index: 3;
    display: flex; align-items: center; gap: 22px;
    padding: 8px 40px 18px;
    color: var(--text-muted); font-size: 13px;
  }
  .bp-hints b {
    display: inline-grid; place-items: center; min-width: 20px; height: 20px;
    margin-right: 6px; padding: 0 5px;
    border: 1px solid var(--border-hover); border-radius: 5px;
    font-size: 11px; color: var(--text-secondary); font-family: var(--font-mono);
  }
  .bp-pos { margin-left: auto; font-family: var(--font-mono); }

  @media (prefers-reduced-motion: reduce) {
    .bp-bg-layer-current.fade-in,
    .bp-bg-layer-prev.fade-out {
      animation: none;
    }
    .bp-card,
    .bp-play,
    .bp-secondary {
      transition: none;
    }
    .bp-card.focus {
      transform: none;
    }
  }
</style>
