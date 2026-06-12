<script lang="ts">
  let {
    title = "暂无数据",
    description = "",
    actionLabel = "",
    onAction = null,
  }: {
    title?: string;
    description?: string;
    actionLabel?: string;
    onAction?: (() => void) | null;
  } = $props();
</script>

<div class="empty-state">
  <svg class="mark" viewBox="0 0 96 72" aria-hidden="true">
    <path d="M20 52h56" />
    <path d="M30 52V24a6 6 0 0 1 6-6h24a6 6 0 0 1 6 6v28" />
    <path d="M38 30h20M38 39h12" />
    <path d="M24 52l8-14M72 52l-8-14" />
  </svg>
  <h3>{title}</h3>
  {#if description}
    <p>{description}</p>
  {/if}
  {#if actionLabel && onAction}
    <button onclick={onAction}>{actionLabel}</button>
  {/if}
</div>

<style>
  .empty-state {
    min-height: 260px;
    display: grid;
    place-items: center;
    align-content: center;
    gap: 12px;
    color: var(--text-muted);
    text-align: center;
    border: 1px dashed var(--border-hover);
    border-radius: var(--radius-lg);
    background: rgba(255, 255, 255, 0.035);
    box-shadow: var(--glass-highlight);
  }

  .mark {
    width: 88px;
    height: 66px;
    color: var(--accent);
    background: var(--accent-lo);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 12px;
  }
  .mark path { fill: none; stroke: currentColor; stroke-width: 1.75; stroke-linecap: round; stroke-linejoin: round; }

  h3 {
    color: var(--text-primary);
    font-size: 17px;
    font-weight: 650;
  }

  p {
    max-width: 380px;
    font-size: 13px;
    line-height: 1.65;
  }

  button {
    border: 0;
    border-radius: var(--radius-full);
    padding: 10px 18px;
    background: var(--accent);
    color: #fff;
    cursor: pointer;
    font-weight: 650;
    transition: transform 0.12s ease, background 0.18s ease;
  }
  button:hover { background: var(--accent-hi); }
  button:active { transform: translateY(1px); }
</style>
