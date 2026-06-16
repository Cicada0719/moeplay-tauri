<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import type { Game } from "../stores/games.svelte";
  import CachedImage from "./CachedImage.svelte";
  import Icon from "./Icon.svelte";
  import { coverOf, developerOf, gameCompletionStatus, gameRating, tagsOf } from "../utils/game";

  let { game }: { game: Game } = $props();
  let el = $state<HTMLDivElement>();

  const statusLabels: Record<string, string> = {
    playing: "游玩中",
    completed: "已通关",
    on_hold: "搁置",
    dropped: "已放弃",
    plan_to_play: "计划中",
    replaying: "重温中",
  };
  const statusIcons: Record<string, string> = {
    playing: "play",
    completed: "check",
    on_hold: "chevronDown",
    dropped: "x",
    plan_to_play: "star",
    replaying: "refresh",
  };

  const monogram = $derived((game.name?.trim()?.[0] ?? "?").toUpperCase());
  const coverSource = $derived(coverOf(game));
  const completionStatus = $derived(gameCompletionStatus(game));
  const showStatusBadge = $derived(Boolean(statusLabels[completionStatus]));
  const developer = $derived(developerOf(game));
  const rating = $derived(gameRating(game));
  const tags = $derived(tagsOf(game));

  const isNsfw = $derived(
    tags.some(t => /^(nsfw|18\+|r-?18|adult|成人|エロ|エロゲ)$/i.test(t.trim()))
  );
  const nsfwMode = $derived(settingsStore.settings.nsfw_display_mode ?? "show");

  // GSAP 入场：power3.out 淡入上移（结合 gsap-skill；hover 仍用 CSS 微交互）。
  onMount(() => {
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    const node = el;
    if (reduce || !node) return;
    const ctx = gsap.context(() => {
      gsap.from(node, { opacity: 0, y: 14, duration: 0.5, ease: "power3.out" });
    }, node);
    return () => ctx.revert();
  });

  function handleClick() {
    gameStore.selectGame(game.id);
    if (uiStore.currentView === "home") uiStore.libraryMode = "all";
    uiStore.currentView = "game-detail";
  }
  function toggleFavorite(e: MouseEvent) {
    e.stopPropagation();
    gameStore.toggleFavorite(game.id);
  }
</script>

<div
  bind:this={el}
  class="game-card"
  class:list={uiStore.viewMode === "list"}
  class:nsfw-hidden={isNsfw && nsfwMode === "hide"}
  onclick={handleClick}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleClick();
    }
  }}
  role="button"
  tabindex="0"
>
  <div class="cover" class:cover-blur={isNsfw && nsfwMode === "blur"}>
    {#if coverSource}
      <CachedImage source={coverSource} cacheKey={`game-cover-${game.id}`} alt={game.name} loading="lazy" />
    {:else}
      <div class="cover-placeholder"><span class="monogram">{monogram}</span></div>
    {/if}

    {#if showStatusBadge}
      <span class="status-badge" title={statusLabels[completionStatus]}>
        <Icon name={statusIcons[completionStatus] || "diamond"} size={12} />
      </span>
    {/if}

    <button class="fav-btn" class:active={game.favorite} onclick={toggleFavorite} aria-label="收藏">
      <Icon name={game.favorite ? "heartFill" : "heart"} size={16} />
    </button>

    <div class="gradient-overlay"></div>
  </div>

  <div class="info">
    <h3 class="title">{game.name}</h3>
    {#if developer && developer !== '未知社团'}
      <p class="developer">{developer}</p>
    {/if}
    <div class="meta">
      {#if rating > 0}
        <span class="rating">
          <Icon name="star" size={11} />
          <span class="rating-num">{rating.toFixed(1)}</span>
        </span>
      {/if}
      {#if tags.length > 0}
        <span class="tag">{tags[0]}</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .game-card {
    background: var(--bg-elev, var(--bg-card));
    border-radius: var(--radius-lg, 14px);
    border: 1px solid var(--border);
    overflow: hidden;
    cursor: pointer;
    /* 克制的 ease-out（无 overshoot 弹跳，仅 transform/box-shadow） */
    transition: transform 0.26s cubic-bezier(0.22, 1, 0.36, 1),
                box-shadow 0.26s ease, border-color 0.26s ease;
    text-align: left;
    padding: 0;
    width: 100%;
    will-change: transform;
  }
  .game-card:hover {
    transform: translateY(-4px);
    box-shadow: var(--shadow-hover);
    border-color: var(--border-hover);
  }
  /* 玫红焦点环（键盘/手柄/选中），对应设计稿 */
  .game-card:focus-visible {
    outline: none;
    border-color: transparent;
    box-shadow: 0 0 0 2px var(--accent-pink-ring), var(--shadow-hover);
  }

  .cover {
    position: relative;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: var(--bg-hover);
  }
  .cover :global(.cached-image) {
    width: 100%; height: 100%;
    object-fit: cover;
    transition: transform 0.4s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .game-card:hover .cover :global(.cached-image) { transform: scale(1.04); }

  .cover-placeholder {
    width: 100%; height: 100%;
    display: flex; align-items: center; justify-content: center;
    background: linear-gradient(135deg, rgba(255,126,173,.22), rgba(167,139,250,.22));
  }
  .monogram {
    font-family: var(--font-display, var(--font-ui));
    font-size: 40px; font-weight: 700;
    color: var(--text-muted);
    opacity: 0.5;
  }

  .status-badge {
    position: absolute; top: 10px; left: 10px;
    display: inline-flex; align-items: center;
    color: var(--text-primary);
    background: rgba(10, 13, 20, 0.55);
    padding: 4px 7px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    backdrop-filter: blur(6px);
  }

  .fav-btn {
    position: absolute; top: 8px; right: 8px;
    display: inline-flex; align-items: center; justify-content: center;
    border: none; background: rgba(10, 13, 20, 0.45);
    color: var(--text-secondary);
    width: 28px; height: 28px;
    border-radius: var(--radius-full);
    cursor: pointer;
    transition: transform 0.18s ease, color 0.18s ease, background 0.18s ease;
    backdrop-filter: blur(6px);
  }
  .fav-btn:hover { transform: scale(1.12); color: var(--text-primary); }
  .fav-btn.active { color: var(--accent-pink); }

  .gradient-overlay {
    position: absolute; inset: auto 0 0 0; height: 56px;
    background: linear-gradient(transparent, rgba(8, 11, 18, 0.6));
    pointer-events: none;
  }

  .info { padding: 12px 12px 14px; }
  .title {
    margin: 0 0 4px;
    font-size: 14px; font-weight: 600;
    color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .developer {
    margin: 0 0 8px;
    font-size: 12px; color: var(--text-muted);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .meta { display: flex; gap: 8px; align-items: center; }
  /* 评分：玫红 + 等宽数字，无渐变、无 emoji */
  .rating {
    display: inline-flex; align-items: center; gap: 4px;
    color: var(--accent-pink);
  }
  .rating-num {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 12px; font-weight: 600;
  }
  .tag {
    font-size: 11px; padding: 2px 8px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    color: var(--text-muted);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 84px;
  }

  /* 列表视图 */
  .game-card.list { display: flex; align-items: center; padding: 10px; gap: 14px; }
  .game-card.list .cover { width: 56px; height: 74px; aspect-ratio: auto; flex-shrink: 0; border-radius: var(--radius-md); }
  .game-card.list .info { flex: 1; padding: 0; }
  .game-card.list .gradient-overlay,
  .game-card.list .status-badge,
  .game-card.list .fav-btn { display: none; }

  /* NSFW */
  .nsfw-hidden { display: none; }
  .cover.cover-blur :global(.cached-image) { filter: blur(18px); transform: scale(1.06); }
  .cover.cover-blur::after {
    content: "NSFW";
    position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
    font-size: 11px; font-weight: 800; letter-spacing: 0.14em;
    color: rgba(255,255,255,.7);
    pointer-events: none;
  }
</style>
