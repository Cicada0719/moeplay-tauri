<script lang="ts">
  import { formatDistanceToNowStrict } from "date-fns";
  import { zhCN } from "date-fns/locale";
  import type { ContinueItem } from "../stores/continue.svelte";
  import CachedImage from "./CachedImage.svelte";
  import Icon from "./Icon.svelte";
  import Card from "./ui/Card.svelte";
  import Tag from "./ui/Tag.svelte";

  let {
    item,
    onclick,
  }: {
    item: ContinueItem;
    onclick?: () => void;
  } = $props();

  const typeLabel = $derived(
    item.type === "game" ? "游戏" : item.type === "anime" ? "番剧" : "漫画"
  );
  const typeIcon = $derived(
    item.type === "game" ? "gamepad" : item.type === "anime" ? "play" : "book"
  );

  function timeAgo(ts: number): string {
    try {
      return formatDistanceToNowStrict(new Date(ts), { addSuffix: true, locale: zhCN });
    } catch {
      return "";
    }
  }
</script>

<Card class="cc-card" hoverable focusable onclick={onclick} ariaLabel={`继续 ${item.title}`}>
  <div class="cc-cover">
    {#if item.cover}
      <CachedImage source={item.cover} cacheKey={item.id} alt={item.title} />
    {:else}
      <div class="cc-placeholder">{item.title[0]}</div>
    {/if}
    <Tag variant="accent" size="sm" class="cc-type">{typeLabel}</Tag>
  </div>
  <div class="cc-info">
    <span class="cc-title">{item.title}</span>
    {#if item.subtitle}
      <span class="cc-subtitle">{item.subtitle}</span>
    {/if}
    {#if item.progress > 0}
      <div class="cc-progress">
        <div class="cc-progress-bar" style="width: {item.progress}%"></div>
      </div>
    {/if}
    <span class="cc-meta">{item.progressLabel} · {timeAgo(item.lastActivity)}</span>
  </div>
  <div class="cc-action" aria-hidden="true" title={item.actionLabel}>
    <Icon name={typeIcon} size={14} />
  </div>
</Card>

<style>
  :global(.cc-card) {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 12px; width: 100%; text-align: left;
  }
  .cc-cover {
    width: 48px; height: 64px; flex-shrink: 0; border-radius: 8px; overflow: hidden;
    background: rgba(255,255,255,0.06); position: relative;
  }
  .cc-cover :global(.cached-image) { width: 100%; height: 100%; object-fit: cover; }
  .cc-placeholder {
    width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;
    font-size: 18px; font-weight: 700; color: var(--text-muted);
  }
  :global(.cc-type) {
    position: absolute; top: 2px; right: 2px;
  }
  .cc-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .cc-title {
    display: block; font-size: 14px; font-weight: 600; color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cc-subtitle {
    font-size: 11px; color: var(--text-secondary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cc-progress {
    height: 3px; width: 100%; max-width: 160px;
    background: rgba(255,255,255,0.1); border-radius: 2px; margin-top: 2px;
  }
  .cc-progress-bar {
    height: 100%; background: var(--accent); border-radius: 2px;
  }
  .cc-meta { font-size: 11px; color: var(--text-muted); }
  .cc-action {
    display: flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; border-radius: 50%;
    background: rgba(232,85,127,0.1); color: var(--accent);
    flex-shrink: 0; opacity: 0; transition: opacity 0.15s;
  }
  :global(.cc-card):hover .cc-action { opacity: 1; }
</style>
