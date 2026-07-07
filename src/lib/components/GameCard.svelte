<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import type { Game } from "../stores/games.svelte";
  import CachedImage from "./CachedImage.svelte";
  import Icon from "./Icon.svelte";
  import { Card } from "./ui";
  import {
    coverOf,
    developerOf,
    gameCompletionStatus,
    gameRating,
    releaseYearOf,
    tagsOf,
  } from "../utils/game";

  let { game }: { game: Game } = $props();
  let el = $state<HTMLElement>();
  let coverFailed = $state(false);

  const statusLabels: Record<string, string> = {
    playing: "游玩中",
    completed: "已通关",
    on_hold: "搁置",
    dropped: "已放弃",
    plan_to_play: "计划中",
    replaying: "重温中",
    not_started: "未开始",
  };
  const statusIcons: Record<string, string> = {
    playing: "play",
    completed: "check",
    on_hold: "chevronDown",
    dropped: "x",
    plan_to_play: "star",
    replaying: "refresh",
    not_started: "circle",
  };

  const monogram = $derived((game.name?.trim()?.[0] ?? "?").toUpperCase());
  const coverSource = $derived(coverOf(game));
  const completionStatus = $derived(gameCompletionStatus(game));
  const showStatusBadge = $derived(Boolean(statusLabels[completionStatus]));
  const developer = $derived(developerOf(game));
  const year = $derived(releaseYearOf(game));
  const rating = $derived(gameRating(game));
  const tags = $derived(tagsOf(game));
  const isNsfw = $derived(
    tags.some(t => /^(nsfw|18\+|r-?18|adult|成人|エロ|エロゲ)$/i.test(t.trim()))
  );
  const nsfwMode = $derived(settingsStore.settings.nsfw_display_mode ?? "show");
  const inSelectionMode = $derived(gameStore.selectionMode);
  const isSelected = $derived(gameStore.isSelected(game.id));
  const isList = $derived(uiStore.viewMode === "list");

  onMount(() => {
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    const node = el;
    if (reduce || !node) return;
    const ctx = gsap.context(() => {
      gsap.from(node, { opacity: 0, y: 14, duration: 0.5, ease: "power3.out" });
    }, node);
    return () => ctx.revert();
  });

  function handleClick(e: MouseEvent | KeyboardEvent) {
    if (e instanceof KeyboardEvent && e.key !== "Enter" && e.key !== " ") return;
    if (inSelectionMode || (e instanceof MouseEvent && (e.ctrlKey || e.metaKey))) {
      gameStore.toggleSelection(game.id);
      return;
    }
    gameStore.selectGame(game.id);
    if (uiStore.currentView === "home") uiStore.libraryMode = "all";
    uiStore.currentView = "game-detail";
  }
  function handleKeydown(e: KeyboardEvent) {
    if (e.shiftKey && e.key === "Delete") {
      e.preventDefault();
      e.stopPropagation();
      confirmDelete();
      return;
    }
    handleClick(e);
  }
  function toggleFavorite(e: MouseEvent) {
    e.stopPropagation();
    gameStore.toggleFavorite(game.id);
  }
  async function confirmDelete() {
    if (!window.confirm(`确定要从游戏库中删除「${game.name}」吗？\n删除后不会移除本地文件。`)) return;
    try {
      await gameStore.deleteGame(game.id);
      uiStore.notify(`已删除 ${game.name}`, "success");
    } catch (e) {
      uiStore.notify(`删除失败：${e}`, "error");
    }
  }
</script>

<Card
  bind:ref={el}
  class="game-card {isList ? 'game-card--list' : ''} {isSelected ? 'game-card--selected' : ''}"
  padding="none"
  hoverable
  focusable
  role="button"
  ariaLabel={game.name}
  onclick={handleClick}
  onkeydown={handleKeydown}
>
  <div class="cover" class:cover-blur={isNsfw && nsfwMode === "blur"} class:cover-hidden={isNsfw && nsfwMode === "hide"}>
    {#if coverSource}
      <CachedImage source={coverSource} cacheKey={`game-cover-${game.id}`} alt={game.name} loading="lazy" />
    {:else}
      <div class="cover-placeholder"><span class="monogram">{monogram}</span></div>
    {/if}

    {#if inSelectionMode}
      <span class="select-check" class:checked={isSelected}>
        {#if isSelected}
          <Icon name="check" size={14} />
        {/if}
      </span>
    {/if}

    {#if showStatusBadge && !inSelectionMode}
      <span class="status-badge" title={statusLabels[completionStatus]}>
        <Icon name={statusIcons[completionStatus] || "diamond"} size={12} />
      </span>
    {/if}

    {#if !inSelectionMode}
      <div class="card-actions">
        <button class="fav-btn" class:active={game.favorite} onclick={toggleFavorite} aria-label="收藏" title="收藏">
          <Icon name={game.favorite ? "heartFill" : "heart"} size={16} />
        </button>
        <button class="del-btn" onclick={(e) => { e.stopPropagation(); confirmDelete(); }} aria-label="删除游戏" title="删除游戏">
          <Icon name="trash" size={16} />
        </button>
      </div>
    {/if}

    <div class="gradient-overlay"></div>
  </div>

  <div class="info">
    <h3 class="title">{game.name}</h3>
    <div class="meta-line">
      {#if developer && developer !== "未知社团"}
        <span class="developer">{developer}</span>
      {/if}
      {#if year}
        <span class="year">{year}</span>
      {/if}
    </div>
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
</Card>

<style>
  :global(.game-card) {
    overflow: hidden;
    cursor: pointer;
    text-align: left;
    width: 100%;
    position: relative;
  }
  :global(.game-card--selected) {
    border-color: var(--accent) !important;
    box-shadow: 0 0 0 2px var(--accent-ring, rgba(232,85,127,0.4)) !important;
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
  :global(.game-card:hover) .cover :global(.cached-image) { transform: scale(1.04); }

  .cover-placeholder {
    width: 100%; height: 100%;
    display: flex; align-items: center; justify-content: center;
    background:
      radial-gradient(120% 100% at 50% 0%, rgba(0, 255, 153, 0.16), transparent 60%),
      linear-gradient(135deg, rgba(0, 200, 120, 0.18), rgba(0, 90, 70, 0.20));
  }
  .monogram {
    font-family: var(--font-display, var(--font-ui));
    font-size: 40px; font-weight: 700;
    color: var(--text-muted);
    opacity: 0.5;
  }

  .select-check {
    position: absolute; top: 8px; left: 8px; z-index: 5;
    width: 24px; height: 24px; border-radius: 6px;
    border: 2px solid rgba(255,255,255,0.4); background: rgba(0,0,0,0.3);
    display: flex; align-items: center; justify-content: center;
    color: #fff; transition: all 0.15s;
  }
  .select-check.checked {
    border-color: var(--accent); background: var(--accent);
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

  /* Fixed action cluster — always visible, anchored next to the heart (top-right) */
  .card-actions {
    position: absolute; top: 8px; right: 8px; z-index: 6;
    display: flex; align-items: center; gap: 6px;
  }
  .fav-btn, .del-btn {
    display: inline-flex; align-items: center; justify-content: center;
    border: none; background: rgba(10, 13, 20, 0.5);
    color: var(--text-secondary);
    width: 28px; height: 28px;
    border-radius: var(--radius-full);
    cursor: pointer;
    transition: transform 0.18s ease, color 0.18s ease, background 0.18s ease;
    backdrop-filter: blur(6px);
  }
  .fav-btn:hover, .del-btn:hover { transform: scale(1.12); color: var(--text-primary); }
  .fav-btn.active { color: var(--accent-pink); }
  .del-btn:hover { color: #f87171; background: rgba(248, 113, 113, 0.18); }

  .gradient-overlay {
    position: absolute; inset: auto 0 0 0; height: 56px;
    background: linear-gradient(transparent, rgba(8, 11, 18, 0.6));
    pointer-events: none;
  }

  .info { padding: 12px 12px 14px; }
  .title {
    margin: 0 0 6px;
    font-size: 14px; font-weight: 600;
    color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .meta-line {
    display: flex; gap: 8px; align-items: center;
    margin-bottom: 8px;
    min-height: 16px;
  }
  .developer, .year {
    margin: 0;
    font-size: 12px; color: var(--text-muted);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .year::before { content: "·"; margin-right: 8px; }
  .meta { display: flex; gap: 8px; align-items: center; }
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

  /* List view */
  :global(.game-card--list) { display: flex; align-items: center; padding: 10px; gap: 14px; }
  :global(.game-card--list) .cover { width: 56px; height: 74px; aspect-ratio: auto; flex-shrink: 0; border-radius: var(--radius-md); }
  :global(.game-card--list) .info { flex: 1; padding: 0; }
  :global(.game-card--list) .gradient-overlay,
  :global(.game-card--list) .status-badge,
  :global(.game-card--list) .card-actions { display: none; }

  /* NSFW */
  .cover-hidden { display: none; }
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
