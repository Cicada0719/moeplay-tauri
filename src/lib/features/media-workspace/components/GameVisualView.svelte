<script lang="ts">
  import "../styles/media-workspace.css";
  import MediaArtwork from "./MediaArtwork.svelte";
  import { findAction, formatPlaytime, runAction, statusLabel } from "./viewHelpers";
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
    heading = "继续你的故事",
    eyebrow = "GAME ARCHIVE / VISUAL",
    emptyTitle = "游戏库正在等待第一部作品",
    emptyDescription = "导入游戏后，这里会以电影档案的方式呈现最近游玩、精选作品与关键媒体。",
    onAction,
    onImport,
  }: Props = $props();

  const orderedItems = $derived([...items].sort((a, b) => {
    if (a.id === selectedId) return -1;
    if (b.id === selectedId) return 1;
    return (b.metadata.totalSeconds || 0) - (a.metadata.totalSeconds || 0);
  }));
  const featured = $derived(orderedItems[0] ?? null);
  const rail = $derived(orderedItems.slice(0, 5));
  const launchAction = $derived(featured ? findAction(featured, "launch") : undefined);
  const openAction = $derived(featured ? findAction(featured, "open") : undefined);
  const favoriteAction = $derived(featured ? findAction(featured, "toggle-favorite") : undefined);
</script>

<section class="mw-visual" aria-labelledby="mw-visual-heading">
  {#if featured}
    <div class="mw-visual__backdrop" aria-hidden="true">
      <MediaArtwork src={featured.hero?.src} alt="" title={featured.title} eager />
    </div>
    <div class="mw-visual__scrim" aria-hidden="true"></div>

    <header class="mw-visual__header">
      <div>
        <p>{eyebrow}</p>
        <h2 id="mw-visual-heading">{heading}</h2>
      </div>
      <span>{String(items.length).padStart(2, "0")} TITLES</span>
    </header>

    <div class="mw-visual__composition">
      <article class="mw-visual__copy">
        <div class="mw-visual__sequence" aria-hidden="true">
          <span>01</span><i></i><span>{featured.metadata.releaseYear || "----"}</span>
        </div>
        <p class="mw-visual__status">{statusLabel(featured.metadata.completionStatus)} / {formatPlaytime(featured.metadata.totalSeconds)}</p>
        <h3>{featured.title}</h3>
        <p class="mw-visual__description">
          {featured.description || `${featured.metadata.developer || "开发者未标注"} 的作品已收录于你的个人游戏档案。选择继续游玩，或进入详情查看媒体、记录与存档。`}
        </p>
        <div class="mw-visual__meta" aria-label="游戏信息">
          <span><small>STUDIO</small>{featured.metadata.developer || "开发者未标注"}</span>
          <span><small>STATUS</small>{statusLabel(featured.metadata.completionStatus)}</span>
          <span><small>PLAYTIME</small>{formatPlaytime(featured.metadata.totalSeconds)}</span>
        </div>
        <div class="mw-visual__actions">
          {#if launchAction}
            <button class="mw-action mw-action--primary" type="button" onclick={() => runAction(featured, "launch", onAction)}>
              <span>{launchAction.label}</span><i aria-hidden="true"></i>
            </button>
          {/if}
          {#if openAction}<button class="mw-action" type="button" onclick={() => runAction(featured, "open", onAction)}>{openAction.label}</button>{/if}
          {#if favoriteAction}
            <button class="mw-action mw-action--quiet" type="button" aria-pressed={favoriteAction.active ?? featured.favorite} onclick={() => runAction(featured, "toggle-favorite", onAction)}>
              {favoriteAction.active ?? featured.favorite ? "已收藏" : favoriteAction.label}
            </button>
          {/if}
        </div>
      </article>

      <button class="mw-visual__poster" type="button" onclick={() => runAction(featured, "open", onAction)} disabled={!openAction} aria-label={`打开 ${featured.title} 的档案`}>
        <MediaArtwork src={featured.cover?.src} alt={featured.cover?.alt || `${featured.title} 封面`} title={featured.title} eager />
        <span aria-hidden="true">OPEN FILE</span>
      </button>
    </div>

    <div class="mw-visual__rail" aria-label="游戏精选">
      {#each rail as item, index (item.id)}
        <button
          type="button"
          class:active={item.id === featured.id}
          onclick={() => runAction(item, "select", onAction)}
          ondblclick={() => runAction(item, "open", onAction)}
          aria-label={`选择 ${item.title}`}
          aria-current={item.id === featured.id ? "true" : undefined}
          disabled={!findAction(item, "select")}
        >
          <span class="mw-visual__rail-index">{String(index + 1).padStart(2, "0")}</span>
          <MediaArtwork src={(item.hero || item.cover)?.src} alt="" title={item.title} />
          <span class="mw-visual__rail-copy"><strong>{item.title}</strong><small>{statusLabel(item.metadata.completionStatus)}</small></span>
        </button>
      {/each}
    </div>
  {:else}
    <div class="mw-empty mw-empty--visual">
      <div class="mw-empty__mark" aria-hidden="true"><span>MP</span><i></i></div>
      <div>
        <p>{eyebrow}</p>
        <h2 id="mw-visual-heading">{emptyTitle}</h2>
        <p>{emptyDescription}</p>
        {#if onImport}<button class="mw-action mw-action--primary" type="button" onclick={() => void onImport?.()}><span>导入游戏</span><i aria-hidden="true"></i></button>{/if}
      </div>
      <span class="mw-empty__folio">NO. 000</span>
    </div>
  {/if}
</section>
