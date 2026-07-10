<script lang="ts">
  import { formatDistanceToNowStrict } from "date-fns";
  import { zhCN } from "date-fns/locale";
  import type { ContinueItem } from "../stores/continue.svelte";
  import { fileSrc } from "../utils";
  import { MediaRow } from "./ui-v2";

  let { item, onclick, emphasized = false }: { item: ContinueItem; onclick?: () => void; emphasized?: boolean } = $props();
  const typeLabel = $derived(item.type === "game" ? "游戏" : item.type === "anime" ? "番剧" : "漫画");
  const imageSrc = $derived(item.type === "game" ? fileSrc(item.cover) : item.cover);

  function timeAgo(ts: number): string {
    try { return formatDistanceToNowStrict(new Date(ts), { addSuffix: true, locale: zhCN }); } catch { return ""; }
  }
</script>

{#snippet badge()}<span class="continue-card__badge">{typeLabel}</span>{/snippet}
{#snippet meta()}
  <span class="continue-card__meta-copy">{item.progressLabel || item.actionLabel || "继续"} · {timeAgo(item.lastActivity)}</span>
  {#if item.progress > 0}
    <span class="continue-card__progress" role="progressbar" aria-label={`${item.title} 进度`} aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(item.progress)}><span style={`width:${Math.max(0, Math.min(100, item.progress))}%`}></span></span>
  {/if}
{/snippet}

<MediaRow title={item.title} subtitle={item.subtitle} description={item.actionLabel} imageSrc={imageSrc ?? undefined} imageAlt="" ariaLabel={`继续 ${item.title}`} onActivate={onclick} badge={badge} meta={meta} focusKey={`continue-${item.type}-${item.id}`} class={emphasized ? "continue-card continue-card--emphasized" : "continue-card"} />

<style>
  .continue-card__badge { display: inline-flex; padding: .2rem .45rem; border-radius: 999px; background: color-mix(in srgb, var(--v2-color-accent) 18%, transparent); color: var(--v2-color-accent); font-size: .68rem; font-weight: 800; }
  .continue-card__meta-copy { display: block; }
  .continue-card__progress { display: block; width: min(12rem, 100%); height: .25rem; margin-top: var(--v2-space-2); overflow: hidden; border-radius: 999px; background: var(--v2-color-border); }
  .continue-card__progress > span { display: block; height: 100%; border-radius: inherit; background: var(--v2-color-accent); }
  :global(.continue-card--emphasized) { border-color: color-mix(in srgb, var(--v2-color-accent) 55%, var(--v2-color-border)); background: linear-gradient(135deg, color-mix(in srgb, var(--v2-color-accent) 12%, var(--v2-color-surface)), var(--v2-color-surface)); }
  @media (prefers-reduced-motion: reduce) { .continue-card__progress > span { transition: none; } }
  :global([data-motion="reduce"]) .continue-card__progress > span { transition: none; }
</style>
