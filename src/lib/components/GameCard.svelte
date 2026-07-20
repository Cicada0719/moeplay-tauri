<script lang="ts">
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import { navigateTo, openOverlay, closeOverlay } from "../stores/router.svelte";
  import type { Game } from "../stores/games.svelte";
  import { fileSrc } from "../utils";
  import {
    coverOf,
    developerOf,
    gameCompletionStatus,
    gameRating,
    releaseYearOf,
    tagsOf,
  } from "../utils/game";
  import { focusTrap } from "../actions/a11y/focusTrap";
  import { portal } from "../actions/portal";
  import { MediaCard } from "./ui-v2";
  import Icon from "./Icon.svelte";

  let {
    game,
    selected: selectedProp,
    disabled = false,
    loading = false,
    itemRole,
  }: {
    game: Game;
    selected?: boolean;
    disabled?: boolean;
    loading?: boolean;
    itemRole?: "listitem" | "gridcell" | "article" | "none";
  } = $props();

  let hostEl = $state<HTMLDivElement>();
  let interactiveEl = $state<HTMLElement>();
  let deleteTrigger = $state<HTMLButtonElement>();
  let deleteOpen = $state(false);
  let deleteLoading = $state(false);
  let favoriteLoading = $state(false);

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

  const focusKey = $derived(`game-card-${game.id}`);
  const monogram = $derived((game.name?.trim()?.[0] ?? "?").toUpperCase());
  const coverSource = $derived(coverOf(game));
  const imageSource = $derived(coverSource ? (fileSrc(coverSource) ?? coverSource) : undefined);
  const completionStatus = $derived(gameCompletionStatus(game));
  const showStatusBadge = $derived(Boolean(statusLabels[completionStatus]));
  const developer = $derived(developerOf(game));
  const year = $derived(releaseYearOf(game));
  const subtitle = $derived([developer && developer !== "未知社团" ? developer : "", year ? String(year) : ""].filter(Boolean).join(" · "));
  const rating = $derived(gameRating(game));
  const tags = $derived(tagsOf(game));
  const isNsfw = $derived(tags.some((tag) => /^(nsfw|18\+|r-?18|adult|成人|エロ|エロゲ)$/i.test(tag.trim())));
  const nsfwMode = $derived(settingsStore.settings.nsfw_display_mode ?? "show");
  const inSelectionMode = $derived(gameStore.selectionMode);
  const isSelected = $derived(selectedProp ?? gameStore.isSelected(game.id));
  const isList = $derived(uiStore.viewMode === "list");
  const resolvedItemRole = $derived(itemRole ?? (isList ? "listitem" : "gridcell"));
  const deleteOverlayId = $derived(`delete-game-${game.id}`);

  $effect(() => {
    const node = interactiveEl;
    if (!node) return;
    node.dataset.focusKey = focusKey;
    node.dataset.gameId = game.id;
  });

  $effect(() => {
    if (!deleteOpen) return;
    openOverlay(
      { id: deleteOverlayId, kind: "dialog", returnFocusKey: focusKey },
      () => { deleteOpen = false; },
    );
    return () => closeOverlay(deleteOverlayId);
  });

  function activate(event: MouseEvent) {
    if (inSelectionMode || event.ctrlKey || event.metaKey) {
      gameStore.toggleSelection(game.id);
      return;
    }

    gameStore.selectGame(game.id);
    uiStore.libraryMode = "all";
    navigateTo("game-detail", {
      entity: { kind: "game", id: game.id },
      focus: "start",
    });
  }

  function handleHostKeydown(event: KeyboardEvent) {
    if (event.shiftKey && event.key === "Delete" && !disabled && !loading) {
      event.preventDefault();
      event.stopPropagation();
      openDeleteDialog();
    }
  }

  async function toggleFavorite(event: MouseEvent) {
    event.stopPropagation();
    if (favoriteLoading) return;
    favoriteLoading = true;
    try {
      await gameStore.toggleFavorite(game.id);
    } catch (error) {
      uiStore.notify(`收藏操作失败：${error}`, "error");
    } finally {
      favoriteLoading = false;
    }
  }

  function openDeleteDialog(event?: MouseEvent) {
    event?.stopPropagation();
    if (deleteLoading) return;
    deleteOpen = true;
  }

  function closeDeleteDialog() {
    if (deleteLoading) return;
    deleteOpen = false;
  }

  async function confirmDelete() {
    if (deleteLoading) return;
    deleteLoading = true;
    try {
      await gameStore.deleteGame(game.id);
      deleteOpen = false;
      uiStore.notify(`已删除 ${game.name}`, "success");
    } catch (error) {
      uiStore.notify(`删除失败：${error}`, "error");
    } finally {
      deleteLoading = false;
    }
  }
</script>

<svelte:window onkeydown={(event) => {
  if (hostEl?.contains(document.activeElement)) handleHostKeydown(event);
}} />

<div
  bind:this={hostEl}
  class="game-card-host"
  class:game-card-host--list={isList}
  data-testid={`game-card-${game.id}`}
>
  {#snippet badges()}
    {#if inSelectionMode}
      <span class="select-check" class:checked={isSelected} aria-label={isSelected ? "已选择" : "未选择"}>
        {#if isSelected}<Icon name="check" size={14} />{/if}
      </span>
    {:else if showStatusBadge}
      <span class="status-badge" title={statusLabels[completionStatus]}>
        <Icon name={statusIcons[completionStatus] || "diamond"} size={12} />
        <span class="sr-only">{statusLabels[completionStatus]}</span>
      </span>
    {/if}
    {#if !imageSource}
      <span class="cover-monogram" aria-hidden="true">{monogram}</span>
    {/if}
    {#if isNsfw && nsfwMode !== "show"}
      <span class="nsfw-shield" data-mode={nsfwMode}>NSFW</span>
    {/if}
  {/snippet}

  {#snippet meta()}
    {#if rating > 0}
      <span class="rating"><Icon name="star" size={11} /> {rating.toFixed(1)}</span>
    {/if}
    {#if tags.length > 0}<span class="tag">{tags[0]}</span>{/if}
  {/snippet}

  {#snippet actions()}
    {#if !inSelectionMode}
      <button
        type="button"
        class="card-action favorite"
        data-gamepad-secondary-action
        data-gamepad-activate={game.favorite ? "取消收藏" : "收藏"}
        class:active={game.favorite}
        disabled={favoriteLoading || disabled || loading}
        aria-label={game.favorite ? `取消收藏 ${game.name}` : `收藏 ${game.name}`}
        aria-busy={favoriteLoading}
        title={game.favorite ? "取消收藏" : "收藏"}
        onclick={toggleFavorite}
      >
        <Icon name={game.favorite ? "heartFill" : "heart"} size={16} />
      </button>
      <button
        bind:this={deleteTrigger}
        type="button"
        class="card-action delete"
        disabled={deleteLoading || disabled || loading}
        aria-label={`删除 ${game.name}`}
        aria-haspopup="dialog"
        aria-expanded={deleteOpen}
        title="删除游戏"
        onclick={openDeleteDialog}
      >
        <Icon name="trash" size={16} />
      </button>
    {/if}
  {/snippet}

  <MediaCard
    title={game.name}
    subtitle={subtitle || undefined}
    imageSrc={imageSource}
    imageAlt={imageSource ? game.name : ""}
    onActivate={activate}
    badges={badges}
    meta={meta}
    actions={actions}
    variant={isList ? "landscape" : "poster"}
    density={uiStore.viewMode === "compact" ? "compact" : "comfortable"}
    selected={isSelected}
    {disabled}
    {loading}
    gamepadActivateLabel={inSelectionMode ? (isSelected ? "取消选择" : "选择") : "打开档案"}
    ariaLabel={inSelectionMode ? `${isSelected ? "取消选择" : "选择"} ${game.name}` : `打开 ${game.name} 详情`}
    itemRole={resolvedItemRole}
    class={`game-card ${isList ? "game-card--list" : ""} ${isNsfw && nsfwMode === "blur" ? "game-card--blur" : ""} ${isNsfw && nsfwMode === "hide" ? "game-card--hidden-cover" : ""}`}
    bind:interactiveRef={interactiveEl}
  />

  {#if deleteOpen}
    <div class="delete-dialog-root" data-testid={`delete-dialog-${game.id}`} use:portal>
      <button class="delete-dialog-backdrop" type="button" tabindex="-1" data-gamepad-skip="true" aria-label="取消删除" onclick={closeDeleteDialog}></button>
      <dialog
        open
        class="delete-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={`delete-title-${game.id}`}
        aria-describedby={`delete-description-${game.id}`}
        aria-busy={deleteLoading}
        tabindex="-1"
        use:focusTrap={{
          initialFocus: "[data-safe-cancel]",
          returnFocus: () => deleteTrigger,
          closeOnEscape: true,
          onEscape: closeDeleteDialog,
        }}
      >
        <span class="delete-dialog-icon" aria-hidden="true"><Icon name="trash" size={24} /></span>
        <div>
          <h2 id={`delete-title-${game.id}`}>从游戏库删除？</h2>
          <p id={`delete-description-${game.id}`}>
            将移除「{game.name}」的库记录，但不会删除本地游戏文件。此操作无法在应用内撤销。
          </p>
        </div>
        <footer>
          <button type="button" class="safe-cancel" data-safe-cancel disabled={deleteLoading} onclick={closeDeleteDialog}>取消</button>
          <button type="button" class="danger-confirm" disabled={deleteLoading} aria-busy={deleteLoading} onclick={confirmDelete}>
            {deleteLoading ? "正在删除…" : "确认删除"}
          </button>
        </footer>
      </dialog>
    </div>
  {/if}
</div>

<style>
  .game-card-host { min-width: 0; height: 100%; }
  .game-card-host--list { min-height: 6rem; }

  :global(.game-card.v2-media-card) {
    --v2-media-card-accent: var(--accent);
    height: 100%;
    background: color-mix(in srgb, var(--bg-elev) 92%, transparent);
    border-color: var(--border);
  }
  :global(.game-card.v2-media-card:hover) { border-color: var(--border-hover); }
  :global(.game-card.v2-media-card.is-selected) {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-ring, rgba(232, 85, 127, 0.4));
  }
  :global(.game-card .v2-media-card__media) { aspect-ratio: 3 / 4; background: var(--bg-hover); }
  :global(.game-card .v2-media-card__media img) { object-fit: cover; }
  :global(.game-card .v2-media-card__copy) { padding: 0.75rem 0.75rem 0.9rem; }
  :global(.game-card .v2-media-card__copy h3) { font-size: 0.875rem; }
  :global(.game-card .v2-media-card__subtitle) { min-height: 1rem; font-size: 0.75rem; }
  :global(.game-card .v2-media-card__meta) { gap: 0.5rem; }
  :global(.game-card .v2-media-card__actions) { top: 0.5rem; right: 0.5rem; }

  :global(.game-card--list.v2-media-card) { min-height: 6rem; display: grid; grid-template-columns: 4.5rem minmax(0, 1fr); }
  :global(.game-card--list .v2-media-card__primary) { display: contents; }
  :global(.game-card--list .v2-media-card__media) { width: 4.5rem; height: 6rem; aspect-ratio: auto; }
  :global(.game-card--list .v2-media-card__copy) { align-self: center; padding: 0.75rem 3.5rem 0.75rem 1rem; }
  :global(.game-card--list .v2-media-card__actions) { top: 50%; transform: translateY(-50%); }

  :global(.game-card--blur .v2-media-card__media img) { filter: blur(18px); transform: scale(1.08); }
  :global(.game-card--hidden-cover .v2-media-card__media img) { visibility: hidden; }

  .select-check,
  .status-badge,
  .cover-monogram,
  .nsfw-shield { display: inline-flex; align-items: center; justify-content: center; }
  .select-check {
    width: 1.6rem; height: 1.6rem; border: 2px solid rgba(255,255,255,.5); border-radius: 0.45rem;
    background: rgba(0,0,0,.48); color: white;
  }
  .select-check.checked { border-color: var(--accent); background: var(--accent); }
  .status-badge { min-width: 1.7rem; min-height: 1.7rem; border-radius: 0.45rem; background: rgba(8,11,18,.68); color: white; }
  .cover-monogram {
    position: absolute; inset: 0; font: 700 2.5rem/1 var(--font-display, var(--font-ui));
    color: var(--text-muted); background: radial-gradient(circle at 50% 20%, rgba(0,255,153,.16), transparent 65%);
  }
  .nsfw-shield { position: absolute; inset: 0; letter-spacing: .14em; font-size: .7rem; font-weight: 800; color: rgba(255,255,255,.8); }
  .nsfw-shield[data-mode="hide"] { background: var(--bg-elev); }

  .rating { display: inline-flex; align-items: center; gap: 0.25rem; color: var(--accent-pink); font: 650 0.75rem/1 var(--font-mono); }
  .tag { max-width: 7rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 0.2rem 0.5rem; border-radius: 999px; background: var(--bg-hover); color: var(--text-muted); font-size: 0.7rem; }

  .card-action {
    display: inline-grid; place-items: center; width: 2rem; min-height: 2rem; padding: 0;
    border: 1px solid rgba(255,255,255,.12); border-radius: 999px; background: rgba(8,11,18,.72);
    color: var(--text-secondary); cursor: pointer; backdrop-filter: blur(8px);
  }
  .card-action:hover:not(:disabled) { color: white; transform: translateY(-1px); }
  .card-action.favorite.active { color: var(--accent-pink); }
  .card-action.delete:hover:not(:disabled) { color: var(--color-error); border-color: color-mix(in srgb, var(--color-error) 45%, transparent); }
  .card-action:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  .card-action:disabled { cursor: wait; opacity: .55; }

  .delete-dialog-root { position: fixed; inset: 0; z-index: 1500; display: grid; place-items: center; }
  .delete-dialog-backdrop { position: absolute; inset: 0; width: 100%; border: 0; background: rgba(2,5,3,.78); backdrop-filter: blur(8px); }
  .delete-dialog {
    position: relative; z-index: 1; width: min(calc(100vw - 2rem), 28rem); display: grid; grid-template-columns: auto 1fr;
    gap: 1rem; padding: 1.25rem; border: 1px solid rgba(255,113,132,.35); border-radius: var(--v2-radius-lg, 1rem);
    background: var(--v2-color-surface-raised, var(--bg-elev)); color: var(--text-primary); box-shadow: 0 1.5rem 5rem rgba(0,0,0,.55); outline: none;
  }
  .delete-dialog-icon { display: grid; place-items: center; width: 3rem; height: 3rem; border-radius: .8rem; background: color-mix(in srgb, var(--color-error) 12%, transparent); color: var(--color-error); }
  .delete-dialog h2 { margin: 0; font-size: 1.1rem; }
  .delete-dialog p { margin: .45rem 0 0; color: var(--text-secondary); font-size: .85rem; line-height: 1.65; }
  .delete-dialog footer { grid-column: 1 / -1; display: flex; justify-content: flex-end; gap: .65rem; margin-top: .25rem; }
  .delete-dialog footer button { min-height: 2.5rem; padding: 0 1rem; border-radius: .65rem; font: 650 .85rem/1 var(--font-ui); cursor: pointer; }
  .safe-cancel { border: 1px solid var(--border); background: var(--bg-hover); color: var(--text-primary); }
  .danger-confirm { border: 1px solid color-mix(in srgb, var(--color-error) 45%, transparent); background: #c83f55; color: white; }
  .delete-dialog footer button:focus-visible { outline: none; box-shadow: 0 0 0 3px var(--accent-ring); }
  .delete-dialog footer button:disabled { cursor: wait; opacity: .6; }

  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0; }

  @media (prefers-reduced-motion: reduce) {
    .card-action { transition: none; }
  }
  :global([data-motion="reduce"]) .card-action { transition: none; }
</style>
