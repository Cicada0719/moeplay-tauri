<script lang="ts">
  import { animeStore } from "../../stores/anime.svelte";
  import { comicStore } from "../../stores/comic.svelte";
  import { novelStore } from "../novel/store.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { openUnifiedMediaHistory } from "./open";
  import {
    buildUnifiedMediaHistory,
    mediaHistoryActionLabel,
    mediaHistoryKindLabel,
    type UnifiedMediaHistoryItem,
    type UnifiedMediaHistoryKind,
  } from "./unified";

  type HistoryFilter = "all" | UnifiedMediaHistoryKind;

  let { onOpen }: { onOpen?: (item: UnifiedMediaHistoryItem) => void | Promise<void> } = $props();
  let filter = $state<HistoryFilter>("all");
  let openingId = $state<string | null>(null);

  const items = $derived(
    buildUnifiedMediaHistory({
      anime: animeStore.history,
      comic: comicStore.readHistory,
      novel: novelStore.history,
    }),
  );
  const filteredItems = $derived(filter === "all" ? items : items.filter((item) => item.kind === filter));
  const filterOptions = $derived([
    { value: "all" as const, label: "全部", count: items.length },
    { value: "anime" as const, label: "番剧", count: items.filter((item) => item.kind === "anime").length },
    { value: "comic" as const, label: "漫画", count: items.filter((item) => item.kind === "comic").length },
    { value: "novel" as const, label: "小说", count: items.filter((item) => item.kind === "novel").length },
  ]);

  function coverOf(item: UnifiedMediaHistoryItem): string | null {
    return item.kind === "anime" && item.cover ? animeStore.getImg(item.cover) || item.cover : item.cover;
  }

  function timeLabel(timestamp: number): string {
    if (!timestamp) return "时间未知";
    const diff = Date.now() - timestamp;
    if (diff < 60_000) return "刚刚更新";
    if (diff < 3_600_000) return `${Math.max(1, Math.round(diff / 60_000))} 分钟前`;
    if (diff < 86_400_000) return `${Math.round(diff / 3_600_000)} 小时前`;
    if (diff < 7 * 86_400_000) return `${Math.round(diff / 86_400_000)} 天前`;
    return new Date(timestamp).toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" });
  }

  async function openItem(item: UnifiedMediaHistoryItem) {
    if (openingId) return;
    openingId = item.id;
    try {
      if (onOpen) await onOpen(item);
      else await openUnifiedMediaHistory({ kind: item.kind, payload: item.payload });
    } catch (error) {
      console.warn("[media-history] open failed", error);
      uiStore.notify(`打开「${item.title}」失败，请稍后重试。`, "error");
    } finally {
      openingId = null;
    }
  }
</script>

<section class="unified-history" data-testid="unified-media-history" aria-label="统一媒体历史">
  <header class="unified-history__header">
    <div>
      <span class="unified-history__kicker">LOCAL MEDIA INDEX</span>
      <h3>统一历史</h3>
      <p>番剧、漫画、小说共用一条时间线；选择后直接恢复到上次位置。</p>
    </div>
    <div class="unified-history__filters" role="group" aria-label="统一历史类型筛选">
      {#each filterOptions as option (option.value)}
        <button type="button" aria-pressed={filter === option.value} onclick={() => (filter = option.value)}>
          {option.label}<span>{option.count}</span>
        </button>
      {/each}
    </div>
  </header>

  {#if filteredItems.length === 0}
    <div class="unified-history__empty" role="status">
      <strong>{filter === "all" ? "还没有媒体历史" : `还没有${mediaHistoryKindLabel(filter)}历史`}</strong>
      <span>打开任意一集番剧、漫画章节或小说章节后，这里会自动记录。</span>
    </div>
  {:else}
    <div class="unified-history__grid" role="list" aria-label="统一媒体历史条目">
      {#each filteredItems as item (item.id)}
        <article class="unified-history__item" class:opening={openingId === item.id} role="listitem">
          <button
            type="button"
            class="unified-history__button"
            aria-label={`${mediaHistoryActionLabel(item.kind)} ${item.title}`}
            aria-busy={openingId === item.id}
            disabled={openingId !== null}
            onclick={() => void openItem(item)}
          >
            <span class="unified-history__art">
              {#if coverOf(item)}<img src={coverOf(item) ?? undefined} alt="" loading="lazy" decoding="async" />{:else}<span>{item.title.slice(0, 1)}</span>{/if}
              {#if item.progress !== null}<i style={`--progress:${item.progress * 100}%`}></i>{/if}
            </span>
            <span class="unified-history__copy">
              <span class="unified-history__kind">{mediaHistoryKindLabel(item.kind)} · {item.sourceLabel}</span>
              <strong>{item.title}</strong>
              <span class="unified-history__position">{item.positionLabel}</span>
              <span class="unified-history__time">{timeLabel(item.updatedAt)} <b>↗</b></span>
            </span>
          </button>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .unified-history { display: grid; gap: var(--v2-space-4); }
  .unified-history__header { display: flex; align-items: end; justify-content: space-between; gap: var(--v2-space-4); }
  .unified-history__kicker { color: var(--v2-color-accent); font: 700 var(--v2-text-xs)/1 var(--v2-font-mono); letter-spacing: .13em; }
  .unified-history h3 { margin: .45rem 0 0; font-size: clamp(1.25rem, 2vw, 1.8rem); letter-spacing: -.04em; }
  .unified-history__header p { margin: .4rem 0 0; color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); }
  .unified-history__filters { display: flex; flex-wrap: wrap; justify-content: end; gap: var(--v2-space-2); }
  .unified-history__filters button { display: inline-flex; align-items: center; gap: .4rem; min-height: 2.5rem; padding: .4rem .7rem; border: 1px solid var(--v2-color-border); border-radius: 999px; background: var(--v2-color-surface-subtle); color: var(--v2-color-text-secondary); font: inherit; font-size: var(--v2-text-xs); font-weight: 800; cursor: pointer; }
  .unified-history__filters button[aria-pressed="true"] { border-color: var(--v2-color-accent); background: color-mix(in srgb, var(--v2-color-accent) 14%, var(--v2-color-surface)); color: var(--v2-color-text); }
  .unified-history__filters span { min-width: 1.15rem; padding: .1rem .25rem; border-radius: 999px; background: color-mix(in srgb, var(--v2-color-text) 10%, transparent); font: 700 .66rem/1 var(--v2-font-mono); text-align: center; }
  .unified-history__grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 17rem), 1fr)); gap: var(--v2-space-3); }
  .unified-history__item { min-width: 0; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-lg); background: var(--v2-color-surface-subtle); transition: border-color 180ms ease, transform 180ms ease, box-shadow 180ms ease; }
  .unified-history__item:hover, .unified-history__item:has(button:focus-visible) { border-color: var(--v2-color-border-strong); transform: translateY(-1px); box-shadow: 0 10px 24px rgb(0 0 0 / .12); }
  .unified-history__item.opening { border-color: var(--v2-color-accent); }
  .unified-history__button { display: flex; align-items: stretch; width: 100%; min-height: 7.2rem; padding: 0; border: 0; border-radius: inherit; background: transparent; color: var(--v2-color-text); font: inherit; text-align: left; cursor: pointer; overflow: hidden; }
  .unified-history__button:focus-visible { outline: none; box-shadow: inset var(--v2-focus-ring); }
  .unified-history__button:disabled { cursor: wait; }
  .unified-history__art { position: relative; display: grid; place-items: center; flex: 0 0 5.8rem; min-height: 7.2rem; overflow: hidden; background: linear-gradient(135deg, var(--v2-color-surface-raised), var(--v2-color-surface)); color: var(--v2-color-text-dim); font-size: 2.4rem; font-weight: 800; }
  .unified-history__art img { width: 100%; height: 100%; object-fit: cover; }
  .unified-history__art::after { position: absolute; inset: 0; background: linear-gradient(90deg, transparent 45%, rgb(0 0 0 / .2)); content: ""; pointer-events: none; }
  .unified-history__art i { position: absolute; right: .4rem; bottom: .4rem; left: .4rem; height: .22rem; background: linear-gradient(90deg, var(--v2-color-accent) var(--progress), rgb(255 255 255 / .24) var(--progress)); }
  .unified-history__copy { display: grid; align-content: center; min-width: 0; gap: .32rem; padding: .75rem .8rem; }
  .unified-history__kind, .unified-history__time { overflow: hidden; color: var(--v2-color-text-secondary); font: 700 .66rem/1.2 var(--v2-font-mono); text-overflow: ellipsis; white-space: nowrap; }
  .unified-history__kind { color: var(--v2-color-accent); }
  .unified-history__copy strong, .unified-history__position { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .unified-history__copy strong { font-size: var(--v2-text-md); }
  .unified-history__position { color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); }
  .unified-history__time b { float: right; color: var(--v2-color-accent); font-size: .9rem; }
  .unified-history__empty { display: grid; gap: .35rem; min-height: 7rem; place-items: center; align-content: center; padding: 1rem; border: 1px dashed var(--v2-color-border); border-radius: var(--v2-radius-lg); color: var(--v2-color-text-secondary); text-align: center; }
  .unified-history__empty strong { color: var(--v2-color-text); }
  .unified-history__empty span { font-size: var(--v2-text-sm); }
  @media (max-width: 46rem) { .unified-history__header { align-items: start; flex-direction: column; } .unified-history__filters { justify-content: start; width: 100%; } .unified-history__filters button { flex: 1 1 auto; justify-content: center; } }
  @media (prefers-reduced-motion: reduce) { .unified-history__item { transition: none; } }
  :global([data-motion="reduce"]) .unified-history__item { transition: none; }
</style>
