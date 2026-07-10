<script lang="ts">
  import type { ComicSummary } from "../../stores/comic.svelte";
  import { Card } from "../ui";

  let { comic, onclick }: { comic: ComicSummary; onclick?: () => void } = $props();
  const sourceLabel = $derived(comic.categories[0] ?? "");
</script>

<Card class="comic-card" padding="none" hoverable focusable role="button" ariaLabel={comic.title} {onclick}>
  <div class="thumb-wrap">
    <img
      src={comic.thumb_url}
      alt={comic.title}
      loading="lazy"
      class="thumb"
      onerror={(e) => { (e.currentTarget as HTMLImageElement).src = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='170' viewBox='0 0 120 170'%3E%3Crect fill='%231a1d27' width='120' height='170'/%3E%3Ctext x='60' y='90' text-anchor='middle' fill='%23666' font-size='28'%3E📚%3C/text%3E%3C/svg%3E"; }}
    />
    {#if sourceLabel}<span class="badge-source">{sourceLabel}</span>{/if}
    {#if comic.finished}
      <span class="badge-finished">完结</span>
    {/if}
  </div>
  <div class="info">
    <p class="title" title={comic.title}>{comic.title}</p>
    <p class="author">{comic.author || "未知作者"}</p>
    <p class="meta">
      {comic.eps_count > 0 ? `${comic.eps_count}话` : comic.categories.slice(1, 3).join(" · ") || "在线漫画"}
      {#if comic.total_views > 0} · {(comic.total_views / 1000).toFixed(0)}k{/if}
    </p>
  </div>
</Card>

<style>
  :global(.comic-card) {
    cursor: pointer;
    text-align: left;
    width: 100%;
  }

  .thumb-wrap {
    position: relative;
    width: 100%;
    padding-top: 140%;
    background: var(--bg-deep);
    overflow: hidden;
  }
  .thumb {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .badge-source {
    position: absolute;
    top: 6px;
    left: 6px;
    max-width: calc(100% - 62px);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: rgba(10,12,18,0.78);
    color: rgba(255,255,255,0.88);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 9px;
    font-weight: 700;
    backdrop-filter: blur(5px);
  }
  .badge-finished {
    position: absolute;
    top: 6px;
    right: 6px;
    background: var(--accent);
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: var(--font-ui);
  }

  .info {
    padding: 8px 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title {
    font-size: 12.5px;
    font-weight: 650;
    color: var(--text-primary);
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    margin: 0;
  }
  .author {
    font-size: 11px;
    color: var(--text-muted);
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: 10px;
    color: var(--text-dim, var(--text-muted));
    margin: 0;
    font-family: var(--font-mono);
  }
</style>
