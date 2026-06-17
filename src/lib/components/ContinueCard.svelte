<script lang="ts">
  import type { ContinueItem } from "../stores/continue.svelte";
  import CachedImage from "./CachedImage.svelte";
  import Icon from "./Icon.svelte";

  let {
    item,
    onclick,
  }: {
    item: ContinueItem;
    onclick?: () => void;
  } = $props();

  const typeIcon = $derived(
    item.type === "game" ? "🎮" : item.type === "anime" ? "📺" : "📖"
  );

  function timeAgo(ts: number): string {
    const diffMs = Date.now() - ts;
    const diffMin = Math.floor(diffMs / 60000);
    const diffHr = Math.floor(diffMs / 3600000);
    const diffDay = Math.floor(diffMs / 86400000);
    if (diffMin < 1) return "刚刚";
    if (diffMin < 60) return `${diffMin}分钟前`;
    if (diffHr < 24) return `${diffHr}小时前`;
    if (diffDay < 7) return `${diffDay}天前`;
    return new Date(ts).toLocaleDateString("zh-CN", { month: "short", day: "numeric" });
  }
</script>

<button class="cc-card" onclick={onclick}>
  <div class="cc-cover">
    {#if item.cover}
      <CachedImage source={item.cover} cacheKey={item.id} alt={item.title} />
    {:else}
      <div class="cc-placeholder">{item.title[0]}</div>
    {/if}
    <span class="cc-type">{typeIcon}</span>
  </div>
  <div class="cc-info">
    <span class="cc-title">{item.title}</span>
    <span class="cc-meta">{item.progressLabel} · {timeAgo(item.lastActivity)}</span>
  </div>
  <div class="cc-action">
    <Icon name="play" size={14} />
  </div>
</button>

<style>
  .cc-card {
    display: flex; align-items: center; gap: 10px;
    padding: 8px 10px; border: 1px solid var(--border); border-radius: 10px;
    background: var(--bg-elev, rgba(255,255,255,0.03));
    cursor: pointer; transition: all 0.15s; width: 100%; text-align: left;
  }
  .cc-card:hover {
    border-color: var(--accent); background: rgba(232,85,127,0.05);
    transform: translateX(2px);
  }
  .cc-cover {
    width: 44px; height: 59px; flex-shrink: 0; border-radius: 6px; overflow: hidden;
    background: rgba(255,255,255,0.06); position: relative;
  }
  .cc-cover :global(.cached-image) { width: 100%; height: 100%; object-fit: cover; }
  .cc-placeholder {
    width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;
    font-size: 18px; font-weight: 700; color: var(--text-muted);
  }
  .cc-type {
    position: absolute; top: 2px; right: 2px; font-size: 10px;
  }
  .cc-info { flex: 1; min-width: 0; }
  .cc-title {
    display: block; font-size: 13px; font-weight: 600; color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cc-meta { font-size: 11px; color: var(--text-muted); }
  .cc-action {
    display: flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; border-radius: 50%;
    background: rgba(232,85,127,0.1); color: var(--accent);
    flex-shrink: 0; opacity: 0; transition: opacity 0.15s;
  }
  .cc-card:hover .cc-action { opacity: 1; }
</style>
