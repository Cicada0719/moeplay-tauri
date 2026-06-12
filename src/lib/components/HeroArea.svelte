<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";
  import { fileSrc } from "../utils";
  import { formatPlayTime } from "../api";
  import { coverOf, gameLastPlayed, gameTotalSeconds, heroImageOf } from "../utils/game";

  let containerEl = $state<HTMLDivElement>();
  let carouselEl = $state<HTMLDivElement>();
  let searchInput = $state("");

  /// 按最近游玩时间排序，未玩的补在后面
  const recentGames = $derived.by(() => {
    const withPlay = [...gameStore.allGames]
      .filter(g => gameLastPlayed(g))
      .sort((a, b) =>
        new Date(gameLastPlayed(b)!).getTime() -
        new Date(gameLastPlayed(a)!).getTime()
      );
    const noPlay = gameStore.allGames.filter(g => !gameLastPlayed(g));
    return [...withPlay, ...noPlay].slice(0, 20);
  });

  /// "继续游玩"：最近游玩的那款
  const continueGame = $derived(
    gameStore.allGames
      .filter(g => gameLastPlayed(g))
      .sort((a, b) =>
        new Date(gameLastPlayed(b)!).getTime() -
        new Date(gameLastPlayed(a)!).getTime()
      )[0] ?? null
  );

  const heroGame = $derived(gameStore.selectedGame ?? continueGame);
  const heroArt = $derived(fileSrc(heroImageOf(heroGame)) ?? "");

  const bgWall = $derived(
    gameStore.allGames
      .filter((g) => coverOf(g))
      .slice(0, 10)
      .map((g) => fileSrc(coverOf(g)) ?? "")
      .filter(Boolean)
  );

  function timeAgo(dateStr: string | null | undefined): string {
    if (!dateStr) return "";
    const days = Math.floor((Date.now() - new Date(dateStr).getTime()) / 86400000);
    if (days === 0) return "今天";
    if (days === 1) return "昨天";
    if (days < 7) return `${days} 天前`;
    if (days < 30) return `${Math.floor(days / 7)} 周前`;
    return `${Math.floor(days / 30)} 个月前`;
  }

  /// Debounced pinyin-aware search
  let debounceTimer: ReturnType<typeof setTimeout>;
  function handleSearch(e: Event) {
    const raw = (e.target as HTMLInputElement).value;
    searchInput = raw;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      if (!raw.trim()) { gameStore.searchQuery = ""; return; }
      gameStore.searchQuery = raw;
    }, 200);
  }

  function clearSearch() {
    searchInput = "";
    gameStore.searchQuery = "";
  }

  function openSteamImport() {
    uiStore.currentView = "steam-import";
  }

  /// Carousel scroll
  function scrollCarousel(dir: -1 | 1) {
    const el = carouselEl;
    if (!el) return;
    el.scrollBy({ left: dir * (90 + 12) * 4, behavior: "smooth" });
  }

  function handleCardClick(gameId: string) {
    gameStore.selectGame(gameId);
    uiStore.showDetailPanel = true;
  }

  /// GSAP 入场
  onMount(() => {
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    const node = containerEl;
    if (reduce || !node) return;
    const ctx = gsap.context(() => {
      gsap.from(node.querySelector(".hero-key-visual"), {
        autoAlpha: 0, scale: 1.03, duration: 0.9, ease: "power2.out" });
      const dockCards = carouselEl?.querySelectorAll(".dock-card");
      if (dockCards?.length) {
        gsap.from(dockCards, {
          autoAlpha: 0, y: 28, stagger: 0.04, duration: 0.55, ease: "power3.out", delay: 0.4 });
      }
    }, node);
    return () => ctx.revert();
  });
</script>

<div class="hero-area" bind:this={containerEl}>
  <!-- Layer 0: Background wall (2×5 thumbnails at 5%) -->
  <div class="bg-wall">
    {#each bgWall as src}
      <img src={src} alt="" loading="lazy" />
    {/each}
  </div>

  <!-- Layer 1: Key visual -->
  <div class="hero-key-visual">
    {#if heroArt}
      <img src={heroArt} alt={heroGame?.name ?? ""} />
    {/if}
  </div>

  <!-- Layer 2: Bottom gradient scrim -->
  <div class="hero-scrim"></div>

  <!-- Search box (z=20) -->
  <div class="search-box">
    <Icon name="search" size={16} className="search-icon" />
    <input
      type="text"
      placeholder="搜索游戏、角色、剧本、标签..."
      value={searchInput}
      oninput={handleSearch}
      class="search-input"
    />
    {#if searchInput}
      <button class="clear-btn" onclick={clearSearch} aria-label="清除搜索">
        <Icon name="x" size={14} />
      </button>
    {/if}
  </div>

  <!-- 继续游玩卡（底部左侧，z=15） -->
  {#if continueGame}
    <div class="continue-card">
      <p class="continue-label">继续游玩</p>
      <p class="continue-name">{continueGame.name}</p>
      <div class="continue-meta">
        <span>{formatPlayTime(gameTotalSeconds(continueGame))}</span>
        <span class="sep">·</span>
        <span>{timeAgo(gameLastPlayed(continueGame))}</span>
      </div>
      <button
        class="continue-btn"
        onclick={() => { handleCardClick(continueGame!.id); gameStore.launch(continueGame!.id); }}
      >
        <Icon name="play" size={12} />
        继续
      </button>
    </div>
  {/if}

  <!-- Layer 3: Foreground carousel Dock -->
  <div class="carousel-wrap">
    {#if recentGames.length > 1}
      <button class="carousel-arrow left" onclick={() => scrollCarousel(-1)} aria-label="上一个">
        <Icon name="chevronLeft" size={20} />
      </button>
    {/if}

    <div class="carousel-track" bind:this={carouselEl}>
      {#if recentGames.length === 0}
        <div class="dock-empty">
          <Icon name="gamepad" size={28} />
          <span>准备建立你的游戏库</span>
          <button class="dock-empty-cta" onclick={() => gameStore.importGame()}>
            <Icon name="plus" size={14} /> 添加第一款游戏
          </button>
          <button class="dock-empty-cta secondary" onclick={openSteamImport}>
            <Icon name="download" size={14} /> 从 Steam 导入
          </button>
        </div>
      {:else}
        {#each recentGames as game (game.id)}
          {@const isSelected = heroGame?.id === game.id}
          <button
            class="dock-card"
            class:selected={isSelected}
            onclick={() => handleCardClick(game.id)}
            aria-label={game.name}
            title={game.name}
          >
            {#if fileSrc(coverOf(game))}
              <img src={fileSrc(coverOf(game))!} alt={game.name} loading="lazy" />
            {:else}
              <span class="dock-monogram">{(game.name?.trim()?.[0] ?? "?").toUpperCase()}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>

    {#if recentGames.length > 1}
      <button class="carousel-arrow right" onclick={() => scrollCarousel(1)} aria-label="下一个">
        <Icon name="chevronRight" size={20} />
      </button>
    {/if}
  </div>
</div>

<style>
  .hero-area {
    position: relative;
    height: clamp(300px, 38vh, 420px);
    overflow: hidden;
    flex-shrink: 0;
    margin: 0 18px;
  }

  /* ── Layer 0: 背景缩略图墙 ── */
  .bg-wall {
    position: absolute; inset: -20px;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    grid-template-rows: repeat(2, 1fr);
    gap: 8px;
    opacity: 0.05;
    pointer-events: none;
  }
  .bg-wall img {
    width: 100%; height: 100%;
    object-fit: cover;
    border-radius: var(--radius-sm);
  }

  /* ── Layer 1: 关键立绘 ── */
  .hero-key-visual {
    position: absolute; inset: 0;
    opacity: 0.82;
  }
  .hero-key-visual img {
    width: 100%; height: 100%;
    object-fit: cover;
    object-position: center 30%;
  }

  /* ── Layer 2: 底部压暗遮罩 ── */
  .hero-scrim {
    position: absolute; inset: 0;
    background: linear-gradient(
      180deg,
      transparent 0%,
      transparent 40%,
      rgba(11, 16, 32, 0.35) 60%,
      rgba(11, 16, 32, 0.88) 82%,
      var(--bg-deep) 100%
    );
    pointer-events: none;
  }

  /* ── Search box (z=20) ── */
  .search-box {
    position: absolute; top: 18px; left: 18px; z-index: 20;
    display: flex; align-items: center; gap: 10px;
    width: 410px; height: 40px;
    padding: 0 16px; border-radius: var(--radius-md);
    background: rgba(17, 24, 39, 0.72);
    border: 1px solid var(--border);
    backdrop-filter: blur(10px);
    transition: border-color 0.2s;
  }
  .search-box:focus-within {
    border-color: var(--accent-pink-ring);
  }
  .search-box :global(.search-icon) {
    color: var(--text-muted); flex-shrink: 0;
  }
  .search-input {
    flex: 1; border: none; background: transparent; color: var(--text-primary);
    font-size: 0.9rem; outline: none; font-family: var(--font-ui);
  }
  .search-input::placeholder { color: var(--text-muted); }
  .clear-btn {
    background: none; border: none; cursor: pointer; color: var(--text-muted);
    padding: 2px; border-radius: 50%; display: flex;
  }
  .clear-btn:hover { color: var(--text-primary); }

  /* ── Carousel Dock ── */
  .carousel-wrap {
    position: absolute; bottom: 12px; left: 0; right: 0; z-index: 10;
    display: flex; align-items: center; gap: 4px;
    padding: 0 12px;
  }

  .carousel-track {
    flex: 1;
    display: flex; gap: 12px;
    overflow-x: auto; overflow-y: hidden;
    scroll-behavior: smooth;
    scrollbar-width: none;
    -ms-overflow-style: none;
    padding: 6px 4px 8px;
    align-items: end;
  }
  .carousel-track::-webkit-scrollbar { display: none; }

  .carousel-arrow {
    flex-shrink: 0;
    width: 26px; height: 48px;
    display: grid; place-items: center;
    border: 1px solid var(--border);
    background: rgba(11, 16, 32, 0.55);
    color: var(--text-secondary);
    border-radius: var(--radius-md);
    cursor: pointer;
    backdrop-filter: blur(6px);
    transition: border-color 0.2s, color 0.2s;
  }
  .carousel-arrow:hover { border-color: var(--accent-pink-ring); color: var(--accent-pink); }

  .dock-card {
    flex-shrink: 0;
    width: 90px; height: 120px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: rgba(21, 25, 50, 0.85);
    overflow: hidden;
    cursor: pointer;
    padding: 0;
    transition: transform 0.3s cubic-bezier(0.22, 1, 0.36, 1),
                border-color 0.22s ease,
                box-shadow 0.22s ease;
  }
  .dock-card:hover {
    transform: translateY(-4px);
    border-color: var(--accent-pink-ring);
    box-shadow: var(--shadow-accent);
  }
  .dock-card.selected {
    transform: translateY(-6px) scale(1.08);
    border: 2px solid var(--accent-pink);
    box-shadow: 0 0 0 2px var(--accent-pink-ring), var(--shadow-accent);
    z-index: 2;
  }

  .dock-card img {
    width: 100%; height: 100%;
    object-fit: cover;
  }

  .dock-monogram {
    width: 100%; height: 100%;
    display: grid; place-items: center;
    font-family: var(--font-display); font-size: 28px; font-weight: 700;
    color: var(--text-muted);
    background: var(--bg-elev);
  }

  /* ── 继续游玩卡 ── */
  .continue-card {
    position: absolute; bottom: 148px; left: 22px; z-index: 15;
    display: flex; flex-direction: column; gap: 4px;
    padding: 14px 18px;
    border-radius: var(--radius-lg);
    background: rgba(11, 16, 32, 0.72);
    border: 1px solid var(--border-glass);
    backdrop-filter: blur(14px);
    max-width: 260px;
  }
  .continue-label {
    font-size: 10px; font-weight: 700; letter-spacing: 0.1em;
    text-transform: uppercase; color: var(--accent-pink);
  }
  .continue-name {
    font-size: 15px; font-weight: 700; color: var(--text-primary);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .continue-meta {
    font-size: 12px; color: var(--text-muted);
    display: flex; align-items: center; gap: 6px;
  }
  .sep { opacity: 0.35; }
  .continue-btn {
    align-self: flex-start;
    display: inline-flex; align-items: center; gap: 5px;
    margin-top: 8px;
    padding: 6px 14px; border-radius: var(--radius-full);
    background: var(--accent-pink); color: #fff;
    font-size: 12px; font-weight: 700;
    border: none; cursor: pointer;
    transition: background 0.18s, transform 0.15s;
  }
  .continue-btn:hover { background: var(--accent-pink-hi); transform: scale(1.04); }

  .dock-empty {
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px;
    width: 100%; height: 200px;
    color: var(--text-muted); font-size: 16px; font-weight: 600;
  }
  .dock-empty-cta {
    margin-top: 4px;
    display: inline-flex; align-items: center; gap: 6px;
    padding: 8px 18px; border-radius: var(--radius-full);
    background: var(--accent-pink); color: #fff;
    font-size: 12px; font-weight: 700;
    border: none; cursor: pointer;
    transition: background 0.18s, transform 0.15s;
  }
  .dock-empty-cta:hover { background: var(--accent-pink-hi); transform: scale(1.04); }
  .dock-empty-cta.secondary {
    background: rgba(96, 165, 250, 0.16);
    color: var(--text-primary);
    border: 1px solid rgba(96, 165, 250, 0.34);
  }
  .dock-empty-cta.secondary:hover {
    background: rgba(96, 165, 250, 0.24);
  }
</style>
