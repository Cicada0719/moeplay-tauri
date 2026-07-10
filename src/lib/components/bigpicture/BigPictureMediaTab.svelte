<script lang="ts">
  import { onMount } from "svelte";
  import { animeStore } from "../../stores/anime.svelte";
  import { comicStore } from "../../stores/comic.svelte";
  import Icon from "../Icon.svelte";
  import BPMediaRail from "../BPMediaRail.svelte";
  import { attachGamepad, type GamepadAttachment } from "../switch/useGamepad.svelte";

  interface MediaItem {
    id: string;
    title: string;
    cover: string | null;
    progress?: number;
    progressLabel?: string;
    type: "anime" | "comic";
  }

  let {
    active = false,
    onSelectMedia,
    onMoveToTop,
    onBack,
    onTabPrevious,
    onTabNext,
  }: {
    active?: boolean;
    onSelectMedia: (item: { type: string }) => void;
    onMoveToTop: () => void;
    onBack: () => void;
    onTabPrevious: () => void;
    onTabNext: () => void;
  } = $props();

  let rootEl = $state<HTMLDivElement>();
  let focusIdx = $state(0);
  let scope: GamepadAttachment | null = null;

  const continueAnime = $derived<MediaItem[]>(
    animeStore.history.filter((h) => h.lastEpisode > 0).slice(0, 10).map((h) => ({
      id: `anime-${h.key}`,
      title: h.name,
      cover: h.image ? animeStore.getImg(h.image) || h.image : null,
      progress: undefined,
      progressLabel: `第${h.lastEpisode}话`,
      type: "anime" as const,
    }))
  );

  const continueComics = $derived<MediaItem[]>(
    comicStore.readHistory.slice(0, 10).map((h) => ({
      id: `comic-${h.id || h.title}`,
      title: h.title,
      cover: null,
      progressLabel: h.last_title || undefined,
      type: "comic" as const,
    }))
  );

  const animeStart = $derived(0);
  const comicStart = $derived(continueAnime.length);
  const panelStart = $derived(continueAnime.length + continueComics.length);
  const itemCount = $derived(panelStart + 2);

  const rows = $derived.by(() => {
    const result: number[][] = [];
    if (continueAnime.length) result.push(Array.from({ length: continueAnime.length }, (_, i) => animeStart + i));
    if (continueComics.length) result.push(Array.from({ length: continueComics.length }, (_, i) => comicStart + i));
    result.push([panelStart, panelStart + 1]);
    return result;
  });

  function rowPosition(index: number) {
    for (let row = 0; row < rows.length; row += 1) {
      const col = rows[row].indexOf(index);
      if (col >= 0) return { row, col };
    }
    return { row: rows.length - 1, col: 0 };
  }

  function setFocus(index: number) {
    focusIdx = Math.max(0, Math.min(itemCount - 1, index));
    if (!active) return;
    queueMicrotask(() => rootEl?.querySelector<HTMLElement>(`[data-media-index="${focusIdx}"]`)?.focus({ preventScroll: true }));
  }

  function moveHorizontal(delta: number) {
    const { row, col } = rowPosition(focusIdx);
    const rowItems = rows[row];
    setFocus(rowItems[Math.max(0, Math.min(rowItems.length - 1, col + delta))]);
  }

  function moveVertical(delta: number) {
    const { row, col } = rowPosition(focusIdx);
    const targetRow = row + delta;
    if (targetRow < 0) { onMoveToTop(); return; }
    if (targetRow >= rows.length) return;
    const target = rows[targetRow];
    setFocus(target[Math.min(col, target.length - 1)]);
  }

  function activateFocused() {
    rootEl?.querySelector<HTMLButtonElement>(`[data-media-index="${focusIdx}"]`)?.click();
  }

  function onMediaKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowLeft": event.preventDefault(); moveHorizontal(-1); break;
      case "ArrowRight": event.preventDefault(); moveHorizontal(1); break;
      case "ArrowUp": event.preventDefault(); moveVertical(-1); break;
      case "ArrowDown": event.preventDefault(); moveVertical(1); break;
      case "Escape": event.preventDefault(); onBack(); break;
      case "Home": event.preventDefault(); setFocus(0); break;
      case "End": event.preventDefault(); setFocus(itemCount - 1); break;
    }
  }

  $effect(() => {
    if (focusIdx >= itemCount) focusIdx = Math.max(0, itemCount - 1);
    if (active) setFocus(focusIdx);
  });

  onMount(() => {
    scope = attachGamepad({
      left: () => moveHorizontal(-1),
      right: () => moveHorizontal(1),
      up: () => moveVertical(-1),
      down: () => moveVertical(1),
      launch: () => activateFocused(),
      activate: () => activateFocused(),
      back: () => onBack(),
      pageLeft: () => onTabPrevious(),
      pageRight: () => onTabNext(),
    }, { id: "big-picture-media", zone: "media", priority: 20 });
    return () => { scope?.(); scope = null; };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="bp-media"
  bind:this={rootEl}
  data-focus-zone="media"
  data-active={active ? "true" : "false"}
  onkeydown={onMediaKeydown}
  role="region"
  aria-label="媒体内容"
>
  {#if continueAnime.length > 0}
    <BPMediaRail
      title="继续观看"
      items={continueAnime}
      startIndex={animeStart}
      activeIndex={focusIdx}
      zoneActive={active}
      onfocusitem={setFocus}
      onselect={onSelectMedia}
    />
  {/if}
  {#if continueComics.length > 0}
    <BPMediaRail
      title="继续阅读"
      items={continueComics}
      startIndex={comicStart}
      activeIndex={focusIdx}
      zoneActive={active}
      onfocusitem={setFocus}
      onselect={onSelectMedia}
    />
  {/if}
  <div class="bp-media-dual">
    <button
      class="bp-media-panel"
      class:zone-focus={active && focusIdx === panelStart}
      data-media-index={panelStart}
      tabindex={active && focusIdx === panelStart ? 0 : -1}
      onclick={() => onSelectMedia({ type: "anime" })}
      onfocus={() => setFocus(panelStart)}
    >
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
                {#if animeStore.getImg(sub.image)}<img src={animeStore.getImg(sub.image)} alt={sub.name_cn || sub.name} />
                {:else}<div class="bp-cover-placeholder"><Icon name="film" size={20} /></div>{/if}
                {#if sub.rating > 0}<span class="bp-cover-score">{sub.rating.toFixed(1)}</span>{/if}
              </div>
            {/each}
          </div>
        {:else if animeStore.collection.length > 0}
          <div class="bp-cover-rail">
            {#each animeStore.collection.slice(0, 8) as item (item.key)}
              <div class="bp-cover-thumb"><div class="bp-cover-placeholder"><Icon name="film" size={20} /></div></div>
            {/each}
          </div>
        {:else}<p class="bp-media-panel-hint">浏览番剧推荐、管理追番和观看记录</p>{/if}
      </div>
      <div class="bp-media-panel-foot"><span>进入动漫</span><Icon name="chevronRight" size={14} /></div>
    </button>

    <button
      class="bp-media-panel"
      class:zone-focus={active && focusIdx === panelStart + 1}
      data-media-index={panelStart + 1}
      tabindex={active && focusIdx === panelStart + 1 ? 0 : -1}
      onclick={() => onSelectMedia({ type: "comic" })}
      onfocus={() => setFocus(panelStart + 1)}
    >
      <div class="bp-media-panel-head">
        <Icon name="book" size={20} />
        <h2>漫画</h2>
        {#if comicStore.isLoggedIn}<span class="bp-media-panel-badge">{comicStore.favorites.length} 收藏</span>{/if}
      </div>
      <div class="bp-media-panel-body">
        {#if comicStore.isLoggedIn && comicStore.favorites.length > 0}
          <div class="bp-cover-rail">
            {#each comicStore.favorites.slice(0, 8) as fav (fav.id)}
              <div class="bp-cover-thumb">
                {#if fav.thumb_url}<img src={fav.thumb_url} alt={fav.title} />
                {:else}<div class="bp-cover-placeholder"><Icon name="book" size={20} /></div>{/if}
              </div>
            {/each}
          </div>
        {:else if comicStore.isLoggedIn}<p class="bp-media-panel-hint">已登录哔咔，浏览漫画分类和排行</p>
        {:else}<p class="bp-media-panel-hint">登录哔咔账号，浏览和收藏漫画</p>{/if}
      </div>
      <div class="bp-media-panel-foot"><span>{comicStore.isLoggedIn ? "进入漫画" : "前往登录"}</span><Icon name="chevronRight" size={14} /></div>
    </button>
  </div>
</div>

<style>
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
  .bp-media-panel:hover, .bp-media-panel:focus-visible, .bp-media-panel.zone-focus {
    border-color: var(--accent-ring, rgba(232,85,127,0.45));
    transform: translateY(-2px);
  }
  .bp-media-panel:focus-visible { box-shadow: var(--ring-switch); }
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
</style>
