<script lang="ts">
  // 历史列表单行（spec §4 步骤 4：封面缩略图、标题、章节信息、相对时间、来源、删除/批量勾选）
  import type { HistoryItem } from '../../history/types';
  import Icon from '../Icon.svelte';
  import { CONTENT_TYPE_ICONS, CONTENT_TYPE_LABELS, formatRelativeTime, highlight, progressLabel } from './utils';

  let {
    item,
    keyword = '',
    batchMode = false,
    selected = false,
    onActivate,
    onDelete,
    onToggleSelect,
  }: {
    item: HistoryItem;
    keyword?: string;
    batchMode?: boolean;
    selected?: boolean;
    onActivate?: (item: HistoryItem) => void;
    onDelete?: (item: HistoryItem) => void;
    onToggleSelect?: (id: string) => void;
  } = $props();

  const segments = $derived(highlight(item.title, keyword));
  const time = $derived(formatRelativeTime(item.updatedAt));
  const progress = $derived(progressLabel(item));
</script>

<article class="history-row" role="listitem" data-testid="history-row" data-history-id={item.id}>
  {#if batchMode}
    <label class="row-check">
      <input
        type="checkbox"
        checked={selected}
        onchange={() => onToggleSelect?.(item.id)}
        aria-label={`选择 ${item.title}`}
        data-testid="row-check"
      />
    </label>
  {/if}

  <button
    type="button"
    class="row-main"
    onclick={() => onActivate?.(item)}
    aria-label={`继续阅读 ${item.title}`}
    data-testid="row-activate"
  >
    {#if item.cover}
      <img class="row-cover" src={item.cover} alt="" loading="lazy" />
    {:else}
      <span class="row-cover row-cover-fallback" aria-hidden="true">
        <Icon name={CONTENT_TYPE_ICONS[item.contentType]} size={18} />
      </span>
    {/if}

    <span class="row-copy">
      <strong class="row-title" data-testid="row-title">
        {#each segments as seg, i}
          {seg.pre}<mark class="row-mark">{seg.hit}</mark>{i === segments.length - 1 ? seg.post : ''}
        {/each}
      </strong>
      <span class="row-sub">
        {item.chapterTitle || '未记录章节'}
        <em>· {CONTENT_TYPE_LABELS[item.contentType]}</em>
      </span>
      <span class="row-progress" data-testid="row-progress">{progress}</span>
    </span>

    <time class="row-time" datetime={new Date(item.updatedAt).toISOString()}>{time}</time>
    <span class="row-source">{item.sourceId}</span>
  </button>

  {#if !batchMode}
    <button
      type="button"
      class="row-delete"
      onclick={() => onDelete?.(item)}
      aria-label={`删除 ${item.title}`}
      data-testid="row-delete"
      title="删除这条记录"
    >
      <Icon name="trash" size={15} />
    </button>
  {/if}
</article>

<style>
  .history-row {
    display: flex;
    align-items: stretch;
    min-width: 0;
    height: 72px;
    border-bottom: 1px solid var(--v2-color-border, rgba(255, 255, 255, 0.08));
    background: transparent;
  }
  .history-row:hover { background: var(--v2-color-surface-subtle, rgba(255, 255, 255, 0.03)); }

  .row-check { display: grid; flex: 0 0 auto; place-items: center; padding: 0 0.75rem; }
  .row-check input { width: 1.1rem; height: 1.1rem; accent-color: var(--v2-color-accent, var(--accent)); }

  .row-main {
    display: flex;
    min-width: 0;
    flex: 1 1 auto;
    align-items: center;
    gap: var(--v2-space-3, 0.75rem);
    padding: 0.5rem 0.5rem 0.5rem 0.75rem;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .row-main:focus-visible { outline: none; box-shadow: inset var(--v2-focus-ring, 0 0 0 2px var(--accent)); }

  .row-cover {
    width: 2.75rem;
    aspect-ratio: 3 / 4;
    flex: 0 0 auto;
    border-radius: var(--v2-radius-sm, 0.25rem);
    object-fit: cover;
    background: var(--v2-color-surface-subtle, rgba(255, 255, 255, 0.06));
  }
  .row-cover-fallback { display: grid; place-items: center; color: var(--v2-color-text-secondary, var(--text-muted)); }

  .row-copy { display: grid; min-width: 0; flex: 1 1 auto; gap: 0.15rem; }
  .row-title {
    overflow: hidden;
    font-size: var(--v2-text-sm, 0.875rem);
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-mark {
    padding: 0 0.1em;
    border-radius: 0.2em;
    background: color-mix(in srgb, var(--v2-color-accent, var(--accent)) 30%, transparent);
    color: inherit;
  }
  .row-sub { overflow: hidden; color: var(--v2-color-text-secondary, var(--text-muted)); font-size: var(--v2-text-xs, 0.75rem); text-overflow: ellipsis; white-space: nowrap; }
  .row-sub em { font-style: normal; opacity: 0.8; }
  .row-progress { color: var(--v2-color-accent, var(--accent)); font-size: var(--v2-text-xs, 0.75rem); font-weight: 650; }

  .row-time { flex: 0 0 auto; align-self: center; color: var(--v2-color-text-secondary, var(--text-muted)); font-size: var(--v2-text-xs, 0.75rem); }
  .row-source {
    display: none;
    max-width: 7rem;
    overflow: hidden;
    align-self: center;
    color: var(--v2-color-text-secondary, var(--text-muted));
    font-size: var(--v2-text-xs, 0.75rem);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-delete {
    display: grid;
    flex: 0 0 auto;
    align-self: center;
    place-items: center;
    width: 2.5rem;
    height: 2.5rem;
    margin-right: 0.5rem;
    border: 0;
    border-radius: var(--v2-radius-md, 0.5rem);
    background: transparent;
    color: var(--v2-color-text-secondary, var(--text-muted));
    cursor: pointer;
  }
  .row-delete:hover { background: color-mix(in srgb, #f87171 14%, transparent); color: #fca5a5; }
  .row-delete:focus-visible { outline: none; box-shadow: var(--v2-focus-ring, 0 0 0 2px var(--accent)); }

  @media (min-width: 52rem) {
    .row-source { display: block; }
  }
</style>
