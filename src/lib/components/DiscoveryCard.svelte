<script lang="ts">
  import { openUrl, buildSourceUrl, type ScrapeResult } from "../api";
  import CachedImage from "./CachedImage.svelte";
  import Icon from "./Icon.svelte";
  import Card from "./ui/Card.svelte";
  import Tag from "./ui/Tag.svelte";
  import Button from "./ui/Button.svelte";

  let {
    result,
    onSelect,
  }: {
    result: ScrapeResult;
    onSelect?: (r: ScrapeResult) => void;
  } = $props();

  const sourceUrl = $derived(buildSourceUrl(result));

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") onSelect?.(result);
  }
</script>

<Card
  class="discovery-card"
  padding="none"
  hoverable
  onclick={() => onSelect?.(result)}
  onkeydown={handleKeydown}
  ariaLabel={`查看 ${result.title}`}
>
  {#if result.cover || result.background}
    <div class="cover-wrap">
      <CachedImage source={result.cover ?? result.background} cacheKey={`discovery:${result.source}:${result.source_id || result.title}`} alt={result.title} />
      <Tag variant="accent" size="sm" class="source-badge">{result.source}</Tag>
      {#if sourceUrl}
        <span class="url-hint" title="点击查看详情"><Icon name="arrowLeft" size={12} /></span>
      {/if}
    </div>
  {:else}
    <div class="cover-placeholder">
      <Icon name="gamepad" size={32} />
      <Tag variant="accent" size="sm" class="source-badge">{result.source}</Tag>
      {#if sourceUrl}
        <span class="url-hint" title="点击查看详情"><Icon name="arrowLeft" size={12} /></span>
      {/if}
    </div>
  {/if}

  <div class="meta">
    <strong class="title">{result.title}</strong>
    <div class="meta-row">
      {#if result.release_year}
        <Tag variant="muted" size="sm">{result.release_year}</Tag>
      {/if}
      {#if result.rating}
        <Tag variant="muted" size="sm"><Icon name="star" size={12} /> {result.rating.toFixed(1)}</Tag>
      {/if}
      {#if result.detail?.age_rating}
        <Tag variant="accent" size="sm">{result.detail.age_rating}</Tag>
      {/if}
    </div>
    {#if result.detail?.developer}
      <span class="dev text-muted">{result.detail.developer}</span>
    {/if}
    {#if result.description}
      <p class="desc">{result.description}</p>
    {:else}
      <p class="desc empty">暂无简介</p>
    {/if}
    {#if result.tags.length}
      <div class="tags">
        {#each result.tags.slice(0, 6) as t}
          <Tag variant="neutral" size="sm">{t}</Tag>
        {/each}
        {#if result.tags.length > 6}
          <Tag variant="muted" size="sm">+{result.tags.length - 6}</Tag>
        {/if}
      </div>
    {/if}
    {#if result.source === "kungal" || result.source === "touchgal"}
      <Tag variant="accent" size="sm" class="dl-badge"><Icon name="download" size={10} /> 下载</Tag>
    {/if}
  </div>

  <div class="card-actions">
    <Button variant="ghost" size="sm" press={(e) => { e.stopPropagation(); onSelect?.(result); }}>
      <Icon name="eye" size={14} /> 查看详情
    </Button>
    {#if sourceUrl}
      <Button variant="quiet" size="sm" press={(e) => { e.stopPropagation(); openUrl(sourceUrl!); }} ariaLabel="打开源站">
        <Icon name="globe" size={14} />
      </Button>
    {/if}
  </div>
</Card>

<style>
  :global(.discovery-card) {
    overflow: hidden;
    display: flex;
    flex-direction: column;
    cursor: pointer;
    transition: transform 0.2s, box-shadow 0.2s;
  }
  :global(.discovery-card):hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-hover);
  }
  .cover-wrap { position: relative; aspect-ratio: 3/4; background: var(--aura-inset); }
  .cover-wrap :global(.cached-image) {
    width: 100%; aspect-ratio: 3/4; object-fit: cover; display: block;
  }
  .cover-placeholder {
    width: 100%; aspect-ratio: 3/4; display: flex; align-items: center; justify-content: center;
    background: var(--bg-hover); color: var(--text-muted); position: relative;
  }
  :global(.source-badge) {
    position: absolute; top: 8px; left: 8px;
    text-transform: uppercase;
  }
  .meta { padding: 14px; display: flex; flex-direction: column; gap: 8px; flex: 1; }
  .meta-row { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .title { font-size: 0.95rem; font-weight: 600; color: var(--text-primary); line-height: 1.3; }
  .dev { font-size: 0.75rem; }
  .desc {
    font-size: 0.8rem; color: var(--text-secondary); line-height: 1.5;
    display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical;
    overflow: hidden; margin-top: 2px;
  }
  .desc.empty { font-style: italic; opacity: 0.5; }
  .tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: auto; padding-top: 6px; }
  :global(.dl-badge) {
    align-self: flex-start;
    display: inline-flex; align-items: center; gap: 3px;
  }
  .card-actions {
    display: flex; gap: 6px; padding: 0 14px 12px 14px; margin-top: auto;
  }
  .url-hint {
    position: absolute; top: 8px; right: 8px;
    width: 22px; height: 22px; border-radius: 50%;
    background: rgba(0,0,0,0.5); color: #fff; display: flex; align-items: center; justify-content: center;
    opacity: 0; transition: opacity 0.2s;
  }
  .cover-wrap:hover .url-hint, .cover-placeholder:hover .url-hint { opacity: 0.85; }
</style>
