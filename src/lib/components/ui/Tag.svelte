<script lang="ts">
  import type { Snippet } from "svelte";

  type TagVariant = "neutral" | "accent" | "muted";
  type TagSize = "sm" | "md";

  let {
    variant = "neutral",
    size = "sm",
    active = false,
    title,
    onclick,
    children,
    class: className = "",
  }: {
    variant?: TagVariant;
    size?: TagSize;
    active?: boolean;
    title?: string;
    onclick?: (event: MouseEvent) => void;
    children?: Snippet;
    class?: string;
  } = $props();
</script>

{#if onclick}
  <button
    class="ui-tag ui-tag--{variant} ui-tag--{size} {active ? 'is-active' : ''} {className}"
    type="button"
    {title}
    {onclick}
  >
    {@render children?.()}
  </button>
{:else}
  <span class="ui-tag ui-tag--{variant} ui-tag--{size} {active ? 'is-active' : ''} {className}" {title}>
    {@render children?.()}
  </span>
{/if}

<style>
  .ui-tag {
    width: max-content;
    max-width: 100%;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-full, 9999px);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-family: var(--font-ui);
    font-weight: 600;
    line-height: 1.2;
    white-space: nowrap;
    color: var(--text-secondary);
    background: transparent;
    overflow: hidden;
    text-overflow: ellipsis;
    transition:
      background 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      border-color 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      color 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1));
  }

  button.ui-tag {
    cursor: pointer;
  }

  button.ui-tag:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-ring);
  }

  button.ui-tag:hover {
    border-color: var(--border-hover, var(--border));
    color: var(--text-primary);
  }

  .ui-tag--sm {
    min-height: 24px;
    padding: 0 9px;
    font-size: 11px;
  }

  .ui-tag--md {
    min-height: 30px;
    padding: 0 12px;
    font-size: 12px;
  }

  .ui-tag--accent,
  .ui-tag.is-active {
    border-color: transparent;
    background: var(--accent-lo, var(--bg-elev));
    color: var(--accent);
  }

  .ui-tag--muted {
    background: var(--bg-elev);
    color: var(--text-muted);
  }

  @media (prefers-reduced-motion: reduce) {
    .ui-tag {
      transition: none;
    }
  }
</style>
