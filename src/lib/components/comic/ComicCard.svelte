<script lang="ts">
  import type { ComicSummary } from "../../stores/comic.svelte";
  import { MediaCard } from "../ui-v2";

  let {
    comic,
    onclick,
    focusKey,
    selected = false,
    interactiveRef = $bindable<HTMLElement | undefined>(undefined),
  }: {
    comic: ComicSummary;
    onclick?: (event: MouseEvent) => void;
    focusKey?: string;
    selected?: boolean;
    interactiveRef?: HTMLElement | undefined;
  } = $props();

  const sourceLabel = $derived(comic.categories[0] ?? "");
  const subtitle = $derived(comic.author || "未知作者");
  const metaText = $derived(
    `${comic.eps_count > 0 ? `${comic.eps_count}话` : comic.categories.slice(1, 3).join(" · ") || "在线漫画"}${comic.total_views > 0 ? ` · ${(comic.total_views / 1000).toFixed(0)}k` : ""}`,
  );
</script>

<MediaCard
  title={comic.title}
  subtitle={subtitle}
  imageSrc={comic.thumb_url}
  imageAlt={`${comic.title} 封面`}
  variant="poster"
  {focusKey}
  {selected}
  ariaLabel={`打开漫画：${comic.title}`}
  onActivate={onclick}
  bind:interactiveRef
  class="comic-media-card"
>
  {#snippet badges()}
    {#if sourceLabel}<span class="comic-badge comic-badge--source">{sourceLabel}</span>{/if}
    {#if comic.finished}<span class="comic-badge comic-badge--finished">完结</span>{/if}
  {/snippet}
  {#snippet meta()}<span class="comic-meta">{metaText}</span>{/snippet}
</MediaCard>

<style>
  :global(.comic-media-card) {
    height: 100%;
  }

  .comic-badge {
    display: inline-flex;
    align-items: center;
    min-height: 1.35rem;
    max-width: 9rem;
    padding: 0.15rem 0.42rem;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--v2-color-border) 75%, transparent);
    border-radius: var(--v2-radius-sm);
    background: color-mix(in srgb, var(--v2-color-surface) 86%, transparent);
    color: var(--v2-color-text);
    font-size: var(--v2-text-xs);
    font-weight: 700;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
    backdrop-filter: blur(0.4rem);
  }

  .comic-badge--finished {
    border-color: color-mix(in srgb, var(--v2-color-accent) 58%, transparent);
    background: var(--v2-color-accent);
    color: var(--v2-color-on-accent, #fff);
  }

  .comic-meta {
    color: var(--v2-color-text-tertiary, var(--v2-color-text-secondary));
    font-family: var(--v2-font-mono, var(--v2-font-sans));
    font-size: var(--v2-text-xs);
  }
</style>
