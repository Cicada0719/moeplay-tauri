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
    hasHeroBackground,
    heroImageOf as gameHeroImageOf,
    isInstalled,
    releaseYearOf,
    tagsOf as gameTagsOf,
  } from "../utils/game";
  import { attachGamepad } from "./switch/useGamepad.svelte";
  import defaultLibraryBackdrop from "../assets/default-library-backdrop.png";
  import { animeStore, COLLECT_TYPES } from "../stores/anime.svelte";
  import { comicStore } from "../stores/comic.svelte";

  const STATUS: Record<string, string> = {
    not_started: "未开始", playing: "游玩中", completed: "已通关",
    on_hold: "搁置", dropped: "已弃坑", plan_to_play: "计划中", replaying: "重温中",
  };

  let bpTab = $state<"game" | "media">("game");
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
  const filteredGames = $derived(filterAll ? games : allGames.filter((g) => isInstalled(g)));
  const focusGame = $derived(filteredGames[focusIdx] ?? null);

  const backgroundArt = $derived(pickBackgroundArt(focusGame));
  const isHeroBg = $derived(hasHeroBackground(focusGame));
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
  const trimmedDesc = $derived(desc.length > 180 ? desc.slice(0, 180) + "…" : desc);

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
        inline: "nearest",
        block: "center", // 竖向卡轮：聚焦卡居中，配合两侧缩放景深
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
  function openSettings() { uiStore.setBigPicture(false); uiStore.currentView = "settings"; }
  function back() { if (showDetail) { closeDetail(); return; } uiStore.setBigPicture(false); }
  function toggleFilter() { filterAll = !filterAll; focusIdx = 0; }

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
      <div class="bp-bg-layer bp-bg-layer-prev" class:fade-out={bgFading} class:is-cover={!isHeroBg} style={`background-image: url("${bgPrevious}")`}></div>
    {/if}
    <div class="bp-bg-layer bp-bg-layer-current" class:fade-in={bgFading} class:is-cover={!isHeroBg} style={`background-image: url("${bgCurrent}")`}></div>
  </div>
  <div class="bp-scrim"></div>

  <div class="bp-layout">
    {#if bpTab === "game"}
    <aside class="bp-sidebar">
      <header class="bp-sidebar-head">
        <div class="bp-sidebar-titles">
          <span class="bp-sidebar-kicker">游戏库</span>
          <span class="bp-sidebar-count"><b>{filteredGames.length}</b><i>款</i></span>
        </div>
        <button
          class="bp-filter"
          data-on={filterAll ? "all" : "installed"}
          onclick={toggleFilter}
          aria-label={filterAll ? "当前：全部，点击仅看已安装" : "当前：已安装，点击查看全部"}
        >
          <span class="bp-filter-opt">全部</span>
          <span class="bp-filter-opt">已装</span>
        </button>
      </header>

      <div class="bp-wheel" bind:this={railEl} role="listbox" aria-label="大屏游戏列表">
        {#each filteredGames as g, i (g.id)}
          {@const off = i - focusIdx}
          {@const coff = Math.max(-4, Math.min(4, off))}
          <button
            class="bp-card"
            class:focus={i === focusIdx}
            style="--off:{off}; --aoff:{Math.min(Math.abs(off), 4)}; --coff:{coff}"
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
            <span class="bp-card-art">
              {#if fileSrc(gameCoverOf(g))}
                <img src={fileSrc(gameCoverOf(g))!} alt={g.name} draggable="false" loading="lazy" />
              {:else}
                <span class="bp-mono">{monogram(g)}</span>
              {/if}
              {#if isInstalled(g)}
                <span class="bp-card-flag" title="已安装"></span>
              {/if}
              <span class="bp-card-name">{g.name}</span>
            </span>
          </button>
        {/each}
        {#if filteredGames.length === 0}
          <div class="bp-empty">
            <p>暂无游戏</p>
            <button class="bp-empty-action" onclick={openImport}><Icon name="download" size={16} /> Steam / Epic 导入</button>
          </div>
        {/if}
      </div>

      {#if filteredGames.length > 1}
        <div class="bp-progress" aria-hidden="true">
          <span class="bp-progress-thumb" style="--p:{focusIdx / (filteredGames.length - 1)}"></span>
        </div>
      {/if}
    </aside>
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
          <div class="bp-hero">
            {#if focusGame.metadata?.original_name}
              <p class="bp-jp">{focusGame.metadata.original_name}</p>
            {/if}
            <h1 class="bp-title">{focusGame.name}</h1>
            <p class="bp-meta">{metaLine(focusGame)}</p>

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

            <div class="bp-stats-row">
              <div class="bp-stat">
                <RatingRing value={scoreValue} max={100} size={52} />
              </div>
              <div class="bp-stat">
                <strong>{achTotal > 0 ? `${achDone}/${achTotal}` : "—"}</strong>
                <span>成就</span>
              </div>
              <div class="bp-stat">
                <strong>{formatPlayTime(gameTotalSeconds(focusGame))}</strong>
                <span>时长</span>
              </div>
              <div class="bp-stat">
                <strong>{timeAgo(gameLastPlayed(focusGame))}</strong>
                <span>最后</span>
              </div>
              <div class="bp-stat">
                <strong>{weekHours}h</strong>
                <span>本周</span>
              </div>
            </div>
          </div>
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
      <div class="bp-media">
        <div class="bp-media-dual">
          <!-- ── 动漫区 ── -->
          <section class="bp-media-panel" role="button" tabindex="0"
            onclick={() => { uiStore.setBigPicture(false); uiStore.currentView = "anime"; }}
            onkeydown={(e) => { if (e.key === 'Enter') { uiStore.setBigPicture(false); uiStore.currentView = "anime"; } }}>
            <div class="bp-media-panel-head">
              <Icon name="film" size={20} />
              <h2>动漫</h2>
              <span class="bp-media-panel-badge">{animeStore.collection.length} 追番 · {animeStore.history.length} 历史</span>
            </div>
            <div class="bp-media-panel-body">
              {#if animeStore.recTrending.length > 0}
                <div class="bp-cover-rail">
                  {#each animeStore.recTrending.slice(0, 8) as sub (sub.id)}
                    <div class="bp-cover-thumb">
                      {#if animeStore.getImg(sub.image)}
                        <img src={animeStore.getImg(sub.image)} alt={sub.name_cn || sub.name} />
                      {:else}
                        <div class="bp-cover-placeholder"><Icon name="film" size={20} /></div>
                      {/if}
                      {#if sub.rating > 0}
                        <span class="bp-cover-score">{sub.rating.toFixed(1)}</span>
                      {/if}
                    </div>
                  {/each}
                </div>
              {:else if animeStore.collection.length > 0}
                <div class="bp-cover-rail">
                  {#each animeStore.collection.slice(0, 8) as item (item.key)}
                    <div class="bp-cover-thumb">
                      <div class="bp-cover-placeholder"><Icon name="film" size={20} /></div>
                    </div>
                  {/each}
                </div>
              {:else}
                <p class="bp-media-panel-hint">浏览番剧推荐、管理追番和观看记录</p>
              {/if}
            </div>
            <div class="bp-media-panel-foot">
              <span>进入动漫</span>
              <Icon name="chevronRight" size={14} />
            </div>
          </section>

          <!-- ── 漫画区 ── -->
          <section class="bp-media-panel" role="button" tabindex="0"
            onclick={() => { uiStore.setBigPicture(false); uiStore.currentView = "comic"; }}
            onkeydown={(e) => { if (e.key === 'Enter') { uiStore.setBigPicture(false); uiStore.currentView = "comic"; } }}>
            <div class="bp-media-panel-head">
              <Icon name="book" size={20} />
              <h2>漫画</h2>
              {#if comicStore.isLoggedIn}
                <span class="bp-media-panel-badge">{comicStore.favorites.length} 收藏</span>
              {/if}
            </div>
            <div class="bp-media-panel-body">
              {#if comicStore.isLoggedIn && comicStore.favorites.length > 0}
                <div class="bp-cover-rail">
                  {#each comicStore.favorites.slice(0, 8) as fav (fav._id)}
                    <div class="bp-cover-thumb">
                      {#if fav.thumb?.fileServer}
                        <img src="{fav.thumb.fileServer}/static/{fav.thumb.path}" alt={fav.title} />
                      {:else}
                        <div class="bp-cover-placeholder"><Icon name="book" size={20} /></div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {:else if comicStore.isLoggedIn}
                <p class="bp-media-panel-hint">已登录哔咔，浏览漫画分类和排行</p>
              {:else}
                <p class="bp-media-panel-hint">登录哔咔账号，浏览和收藏漫画</p>
              {/if}
            </div>
            <div class="bp-media-panel-foot">
              <span>{comicStore.isLoggedIn ? "进入漫画" : "前往登录"}</span>
              <Icon name="chevronRight" size={14} />
            </div>
          </section>
        </div>
      </div>

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

  /* ── Background ── */
  .bp-bg { position: absolute; inset: 0; z-index: 0; }
  .bp-bg-layer {
    position: absolute; inset: 0;
    background-size: cover;
    background-position: center center;
    background-repeat: no-repeat;
    background-color: var(--bg-void);
    opacity: 1;
    will-change: opacity;
  }
  /* 只有竖封面、无真实横向背景图时：完整居中显示封面，锐利不裁，两侧用底色 */
  .bp-bg-layer.is-cover {
    background-size: contain;
  }
  .bp-bg-layer-current.fade-in {
    animation: bpBgIn 0.6s cubic-bezier(0.45, 0, 0.2, 1) both;
  }
  .bp-bg-layer-prev.fade-out {
    animation: bpBgOut 0.6s cubic-bezier(0.45, 0, 0.2, 1) both;
  }
  @keyframes bpBgIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes bpBgOut { from { opacity: 1; } to { opacity: 0; } }

  .bp-scrim {
    position: absolute; inset: 0; z-index: 1; pointer-events: none;
    background:
      linear-gradient(90deg, rgba(7,9,15,0.50) 0%, rgba(7,9,15,0.22) 18%, transparent 50%, transparent 100%),
      linear-gradient(180deg, rgba(7,9,15,0.18) 0%, transparent 30%, rgba(7,9,15,0.35) 80%, var(--bg-void) 100%);
  }

  /* ── Layout: left sidebar + right main ── */
  .bp-layout {
    position: relative; z-index: 2;
    display: flex;
    flex: 1; min-height: 0;
    width: 100%; height: 100%;
  }

  /* ── Left sidebar — “封面卡轮” vertical cover wheel ── */
  .bp-sidebar {
    position: relative;
    width: 194px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: linear-gradient(180deg, rgba(11, 14, 22, 0.68) 0%, rgba(7, 9, 15, 0.62) 100%);
    backdrop-filter: blur(22px) saturate(1.15);
    border-right: 1px solid rgba(255, 255, 255, 0.07);
    /* 液态玻璃：左上内高光 + 右缘投影，制造一块有厚度的面板 */
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.06),
      inset 1px 0 0 rgba(255, 255, 255, 0.04),
      18px 0 46px -26px rgba(0, 0, 0, 0.7);
  }

  .bp-sidebar-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px;
    padding: 18px 16px 12px;
    flex-shrink: 0;
  }
  .bp-sidebar-titles { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .bp-sidebar-kicker {
    font-size: 10px; font-weight: 800; letter-spacing: 0.2em;
    text-transform: uppercase; color: var(--text-muted);
  }
  .bp-sidebar-count { display: flex; align-items: baseline; gap: 4px; }
  .bp-sidebar-count b {
    font-family: var(--font-display); font-size: 23px; font-weight: 800;
    line-height: 1; color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .bp-sidebar-count i { font-style: normal; font-size: 11px; color: var(--text-muted); }

  /* 滑块式分段筛选：全部 ↔ 已装 */
  .bp-filter {
    position: relative; display: inline-flex; align-items: center;
    padding: 3px; border-radius: var(--radius-full);
    border: 1px solid var(--border); background: rgba(7, 9, 15, 0.5);
    cursor: pointer; overflow: hidden; flex-shrink: 0;
  }
  .bp-filter::before {
    content: ""; position: absolute; top: 3px; bottom: 3px; left: 3px;
    width: calc(50% - 3px); border-radius: var(--radius-full);
    background: var(--accent);
    box-shadow: 0 2px 8px -2px rgba(232, 85, 127, 0.6);
    transition: transform 0.28s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bp-filter[data-on="installed"]::before { transform: translateX(100%); }
  .bp-filter-opt {
    position: relative; z-index: 1;
    min-width: 34px; text-align: center; padding: 4px 4px;
    font-size: 10.5px; font-weight: 800; color: var(--text-muted);
    transition: color 0.2s ease;
  }
  .bp-filter[data-on="all"] .bp-filter-opt:first-child,
  .bp-filter[data-on="installed"] .bp-filter-opt:last-child { color: #fff; }

  .bp-wheel {
    flex: 1; min-height: 0;
    display: flex; flex-direction: column;
    gap: 14px;
    overflow-y: auto; overflow-x: hidden;
    padding: 14px 22px 16vh;     /* 底部大留白让最后一张也能滚到中线 */
    scroll-padding-block: 50%;
    perspective: 1000px;          /* 卡轮景深 */
    scrollbar-width: none;
  }
  .bp-wheel::-webkit-scrollbar { display: none; }

  .bp-card {
    position: relative;
    flex: 0 0 auto;
    width: 100%;
    border: none; padding: 0; margin: 0; cursor: pointer;
    background: none; outline: 0;
    transform-style: preserve-3d;
    /* 距聚焦越远：越小、越暗、越向后倾（--aoff 绝对距离，--coff 带符号、已封顶 ±4） */
    opacity: calc(1 - var(--aoff, 0) * 0.16);
    transform:
      rotateX(calc(var(--coff, 0) * -2deg))
      scale(calc(1 - var(--aoff, 0) * 0.07));
    transition:
      transform 0.34s cubic-bezier(0.22, 1, 0.36, 1),
      opacity 0.34s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bp-card-art {
    position: relative; display: block;
    width: 100%; aspect-ratio: 3 / 4;
    border-radius: var(--radius-md); overflow: hidden;
    background: var(--bg-elev);
    box-shadow: var(--shadow-tile);
  }
  .bp-card-art img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .bp-mono {
    width: 100%; height: 100%; display: grid; place-items: center;
    font-family: var(--font-display); font-size: 28px; font-weight: 800;
    color: var(--text-muted);
    background: linear-gradient(135deg, rgba(232, 85, 127, 0.2), rgba(110, 120, 160, 0.14));
  }
  /* 已安装小绿点 */
  .bp-card-flag {
    position: absolute; top: 7px; right: 7px;
    width: 7px; height: 7px; border-radius: 50%;
    background: #5fd39a; box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.45);
  }
  /* 聚焦卡浮出名牌（仅聚焦时显示，不占布局） */
  .bp-card-name {
    position: absolute; left: 0; right: 0; bottom: 0;
    padding: 18px 9px 7px;
    font-size: 11px; font-weight: 800; line-height: 1.2; text-align: left;
    color: #fff;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.86));
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    opacity: 0; transform: translateY(6px);
    transition: opacity 0.24s ease, transform 0.24s ease;
  }

  .bp-card.focus {
    opacity: 1;
    transform: scale(1.08);
    z-index: 3;
  }
  .bp-card.focus .bp-card-name { opacity: 1; transform: none; }
  /* 聚焦光环 + 呼吸辉光（画在卡外层，不被 art 的 overflow 裁掉） */
  .bp-card.focus::after {
    content: ""; position: absolute; inset: 0;
    border-radius: var(--radius-md); pointer-events: none;
    animation: bpFocusBreath 2.8s ease-in-out infinite;
  }
  @keyframes bpFocusBreath {
    0%, 100% { box-shadow: 0 0 0 2px var(--accent), 0 14px 32px -14px rgba(232, 85, 127, 0.5); }
    50% { box-shadow: 0 0 0 2px var(--accent), 0 18px 44px -12px rgba(232, 85, 127, 0.78); }
  }
  .bp-card:hover { opacity: 1; }
  .bp-card:focus-visible { outline: none; }
  .bp-card:focus-visible .bp-card-art { box-shadow: var(--ring-switch); }

  /* 右缘自定义位置指示条 */
  .bp-progress {
    position: absolute; right: 4px; top: 70px; bottom: 16px;
    width: 3px; border-radius: 2px;
    background: rgba(255, 255, 255, 0.06);
    pointer-events: none;
  }
  .bp-progress-thumb {
    position: absolute; left: 0; right: 0; height: 36px;
    border-radius: 2px;
    background: linear-gradient(180deg, var(--accent-hi), var(--accent));
    top: calc(var(--p, 0) * (100% - 36px));
    transition: top 0.3s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .bp-empty {
    display: flex; flex-direction: column; align-items: center; gap: 12px;
    padding: 28px 8px; color: var(--text-muted); text-align: center;
  }
  .bp-empty-action {
    display: inline-flex; align-items: center; gap: 6px;
    border: none; cursor: pointer; background: var(--accent); color: #fff;
    padding: 9px 14px; border-radius: var(--radius-full); font-weight: 700; font-size: 12px;
  }

  /* ── Right main area ── */
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

  /* ── Stage: info pinned to bottom-right ── */
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

  .bp-hero {
    max-width: 64%;
    /* clamp height so the block never grows past the stage (avoids bottom clipping) */
    max-height: 100%;
    text-align: left;
    margin-top: auto;
    padding-bottom: 4px;
  }
  .bp-jp { color: var(--text-muted); font-size: 13px; margin: 0 0 3px; }
  .bp-title {
    font-family: var(--font-display);
    font-size: clamp(26px, 3.4vw, 44px);
    font-weight: 800; line-height: 1.08; margin: 0 0 6px;
    text-shadow: 0 2px 24px rgba(0,0,0,0.5);
  }
  .bp-meta { color: var(--text-secondary); font-size: 13px; margin: 0 0 10px; }

  .bp-actions { display: flex; gap: 10px; margin-bottom: 10px; flex-wrap: wrap; }
  .bp-play {
    display: inline-flex; align-items: center; gap: 10px;
    border: none; cursor: pointer;
    background: var(--accent); color: #fff;
    font-size: 14px; font-weight: 700;
    padding: 10px 22px; border-radius: var(--radius-full);
    transition: transform 0.15s ease, background 0.18s ease;
  }
  .bp-play:hover { background: var(--accent-hi); transform: translateY(-1px); }
  .bp-secondary {
    display: inline-flex; align-items: center; gap: 8px;
    border: 1px solid var(--border-hover); cursor: pointer;
    background: rgba(7,9,15,0.45); color: var(--text-secondary);
    font-size: 13px; font-weight: 600;
    padding: 10px 16px; border-radius: var(--radius-full);
    backdrop-filter: blur(6px);
    transition: color 0.18s ease, border-color 0.18s ease;
  }
  .bp-secondary:hover { color: var(--text-primary); border-color: var(--text-muted); }
  .bp-secondary.active { color: var(--accent); }

  .bp-tags { display: flex; gap: 7px; flex-wrap: wrap; margin-bottom: 8px; max-width: 580px; }
  .bp-tag {
    font-size: 11px; padding: 3px 10px; border-radius: var(--radius-full);
    background: rgba(255,255,255,0.08); color: var(--text-secondary);
  }
  .bp-desc {
    max-width: 560px; color: var(--text-secondary);
    font-size: 12.5px; line-height: 1.55; margin: 0 0 10px;
  }

  /* ── Inline stats row ── */
  .bp-stats-row {
    display: flex; gap: 14px; align-items: center;
    padding: 8px 14px;
    background: rgba(15,19,28,0.55);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    backdrop-filter: blur(10px);
    max-width: fit-content;
    flex-wrap: wrap;
  }
  .bp-stat {
    display: flex; flex-direction: column; align-items: center; gap: 1px;
    min-width: 48px;
  }
  .bp-stat strong { font-size: 14px; font-weight: 700; color: var(--text-primary); }
  .bp-stat span { font-size: 10.5px; color: var(--text-muted); }

  /* ── Footer hints ── */
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

  /* ── Media tab — dual panel ── */
  .bp-media {
    flex: 1; min-height: 0;
    display: flex; flex-direction: column;
    padding: 28px 36px 12px;
  }
  .bp-media-dual {
    flex: 1; min-height: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
  }
  .bp-media-panel {
    display: flex; flex-direction: column;
    background: rgba(10, 12, 20, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 20px;
    backdrop-filter: blur(16px);
    overflow: hidden;
    cursor: pointer;
    transition: border-color 0.22s ease, transform 0.22s ease;
    outline: none;
  }
  .bp-media-panel:hover, .bp-media-panel:focus-visible {
    border-color: var(--accent-ring, rgba(232,85,127,0.45));
    transform: translateY(-2px);
  }
  .bp-media-panel:focus-visible {
    box-shadow: var(--ring-switch);
  }
  .bp-media-panel:active { transform: translateY(0) scale(0.995); }
  .bp-media-panel-head {
    display: flex; align-items: center; gap: 10px;
    padding: 22px 24px 0;
    color: var(--text-primary);
  }
  .bp-media-panel-head h2 {
    font-size: 20px; font-weight: 800; margin: 0;
    font-family: var(--font-display);
  }
  .bp-media-panel-badge {
    margin-left: auto;
    font-size: 12px; color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .bp-media-panel-body {
    flex: 1; min-height: 0;
    padding: 18px 24px;
    display: flex; align-items: center;
  }
  .bp-media-panel-hint {
    margin: 0; color: var(--text-muted); font-size: 14px; line-height: 1.6;
  }
  .bp-cover-rail {
    display: flex; gap: 10px;
    overflow: hidden;
    width: 100%;
  }
  .bp-cover-thumb {
    flex: 0 0 auto;
    width: 90px; aspect-ratio: 3 / 4;
    border-radius: var(--radius-md);
    overflow: hidden;
    background: rgba(255, 255, 255, 0.04);
    position: relative;
  }
  .bp-cover-thumb img {
    width: 100%; height: 100%; object-fit: cover; display: block;
  }
  .bp-cover-placeholder {
    width: 100%; height: 100%;
    display: grid; place-items: center;
    color: var(--text-muted);
  }
  .bp-cover-score {
    position: absolute; top: 4px; right: 4px;
    font-size: 10px; font-weight: 700;
    padding: 2px 5px; border-radius: 4px;
    background: rgba(0, 0, 0, 0.65);
    color: #fbbf24;
    font-family: var(--font-mono);
  }
  .bp-media-panel-foot {
    display: flex; align-items: center; justify-content: flex-end; gap: 6px;
    padding: 14px 24px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    color: var(--accent);
    font-size: 13px; font-weight: 650;
  }

  @media (prefers-reduced-motion: reduce) {
    .bp-bg-layer-current.fade-in,
    .bp-bg-layer-prev.fade-out { animation: none; }
    .bp-play, .bp-secondary,
    .bp-card, .bp-card-name, .bp-filter::before, .bp-progress-thumb { transition: none; }
    /* 保留景深缩放（静态状态），仅去掉聚焦卡的呼吸动画 */
    .bp-card.focus::after {
      animation: none;
      box-shadow: 0 0 0 2px var(--accent), 0 14px 32px -14px rgba(232, 85, 127, 0.5);
    }
  }
</style>
