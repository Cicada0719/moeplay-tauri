<script lang="ts">
  import "../styles/media-workspace.css";
  import MediaArtwork from "./MediaArtwork.svelte";
  import { findAction, runAction, sceneEntries, statusLabel } from "./viewHelpers";
  import type { MediaPresentationItem, MediaWorkspaceViewActions } from "./types";

  interface Props extends MediaWorkspaceViewActions {
    items?: readonly MediaPresentationItem[];
    selectedId?: string | null;
    heading?: string;
    eyebrow?: string;
    emptyTitle?: string;
    emptyDescription?: string;
  }

  let {
    items = [],
    selectedId = null,
    heading = "场景记忆",
    eyebrow = "GAME ARCHIVE / SCENE",
    emptyTitle = "还没有可以陈列的场景",
    emptyDescription = "导入游戏或补充封面与截图后，MoePlay 会在这里组合你的私人游戏记忆。",
    onAction,
    onImport,
  }: Props = $props();

  const entries = $derived(sceneEntries(items, selectedId, 6));
  const selectedItem = $derived(items.find((item) => item.id === selectedId) ?? entries[0]?.item ?? null);
  const selectedLaunch = $derived(selectedItem ? findAction(selectedItem, "launch") : undefined);
  const selectedOpen = $derived(selectedItem ? findAction(selectedItem, "open") : undefined);
  const selectedFavorite = $derived(selectedItem ? findAction(selectedItem, "toggle-favorite") : undefined);

  function handleKeydown(event: KeyboardEvent, item: MediaPresentationItem) {
    if (event.key === "Enter" && findAction(item, "open")) {
      event.preventDefault();
      runAction(item, "open", onAction);
    }
  }
</script>

<section class="mw-scene" aria-labelledby="mw-scene-heading">
  <header class="mw-scene__header">
    <div>
      <p>{eyebrow}</p>
      <h2 id="mw-scene-heading">{heading}</h2>
    </div>
    {#if selectedItem}
      <div class="mw-scene__selection" aria-live="polite">
        <span>SELECTED</span>
        <strong>{selectedItem.title}</strong>
        <small>{selectedItem.metadata.developer || "开发者未标注"} / {statusLabel(selectedItem.metadata.completionStatus)}</small>
      </div>
    {/if}
  </header>

  {#if entries.length}
    <div class="mw-scene__grid" data-count={entries.length}>
      {#each entries as entry, index (entry.id)}
        <article class:active={entry.item.id === selectedItem?.id} class={`mw-scene__tile mw-scene__tile--${index + 1}`}>
          <button
            type="button"
            class="mw-scene__media"
            onclick={() => runAction(entry.item, "select", onAction)}
            ondblclick={() => runAction(entry.item, "open", onAction)}
            onkeydown={(event) => handleKeydown(event, entry.item)}
            aria-label={`选择 ${entry.item.title}，按回车打开详情`}
            aria-current={entry.item.id === selectedItem?.id ? "true" : undefined}
            disabled={!findAction(entry.item, "select") && !findAction(entry.item, "open")}
          >
            <MediaArtwork src={entry.asset?.src} alt={entry.asset?.alt || ""} title={entry.item.title} eager={index < 2} />
            <span class="mw-scene__shade" aria-hidden="true"></span>
            <span class="mw-scene__caption">
              <small>{String(index + 1).padStart(2, "0")} / {(entry.asset?.role || "archive").toUpperCase()}</small>
              <strong>{entry.item.title}</strong>
            </span>
          </button>
        </article>
      {/each}
    </div>

    {#if selectedItem}
      <footer class="mw-scene__footer">
        <p><span>MEMORY INDEX</span>{selectedItem.description || "从封面、场景与游玩记录重新进入这部作品。"}</p>
        <div class="mw-scene__actions">
          {#if selectedLaunch}<button type="button" class="mw-action mw-action--primary" onclick={() => runAction(selectedItem, "launch", onAction)}><span>{selectedLaunch.label}</span><i aria-hidden="true"></i></button>{/if}
          {#if selectedOpen}<button type="button" class="mw-action" onclick={() => runAction(selectedItem, "open", onAction)}>{selectedOpen.label}</button>{/if}
          {#if selectedFavorite}<button type="button" class="mw-action mw-action--quiet" aria-pressed={selectedFavorite.active ?? selectedItem.favorite} onclick={() => runAction(selectedItem, "toggle-favorite", onAction)}>{selectedFavorite.active ?? selectedItem.favorite ? "已收藏" : selectedFavorite.label}</button>{/if}
        </div>
      </footer>
    {/if}
  {:else}
    <div class="mw-empty mw-empty--scene">
      <div class="mw-empty__mark" aria-hidden="true"><span>SCENE</span><i></i></div>
      <div><p>{eyebrow}</p><h2 id="mw-scene-heading">{emptyTitle}</h2><p>{emptyDescription}</p>{#if onImport}<button class="mw-action mw-action--primary" type="button" onclick={() => void onImport?.()}><span>导入游戏</span><i aria-hidden="true"></i></button>{/if}</div>
      <span class="mw-empty__folio">FRAME 000</span>
    </div>
  {/if}
</section>
