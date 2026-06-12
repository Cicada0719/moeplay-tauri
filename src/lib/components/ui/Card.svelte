<script lang="ts">
  import type { Snippet } from "svelte";

  type CardVariant = "elevated" | "flat" | "glass";
  type CardPadding = "none" | "sm" | "md" | "lg";

  let {
    variant = "elevated",
    padding = "md",
    interactive = false,
    selected = false,
    children,
    header,
    footer,
    onclick,
    class: className = "",
  }: {
    variant?: CardVariant;
    padding?: CardPadding;
    interactive?: boolean;
    selected?: boolean;
    children?: Snippet;
    header?: Snippet;
    footer?: Snippet;
    onclick?: (event: MouseEvent) => void;
    class?: string;
  } = $props();

  const classes = $derived(
    `ui-card ui-card--${variant} ui-card--pad-${padding} ${
      interactive || onclick ? "is-interactive" : ""
    } ${selected ? "is-selected" : ""} ${className}`,
  );
</script>

{#if onclick}
  <button class={classes} type="button" {onclick}>
    {#if header}
      <div class="ui-card__header">
        {@render header()}
      </div>
    {/if}

    <div class="ui-card__body">
      {@render children?.()}
    </div>

    {#if footer}
      <div class="ui-card__footer">
        {@render footer()}
      </div>
    {/if}
  </button>
{:else}
  <article class={classes}>
    {#if header}
      <div class="ui-card__header">
        {@render header()}
      </div>
    {/if}

    <div class="ui-card__body">
      {@render children?.()}
    </div>

    {#if footer}
      <div class="ui-card__footer">
        {@render footer()}
      </div>
    {/if}
  </article>
{/if}

<style>
  .ui-card {
    min-width: 0;
    width: 100%;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    display: block;
    overflow: hidden;
    text-align: left;
    transition:
      transform 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      background 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      border-color 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      box-shadow 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1));
  }

  button.ui-card {
    appearance: none;
    font: inherit;
    padding: 0;
  }

  .ui-card--elevated {
    background: var(--bg-elev);
    box-shadow: var(--shadow-card, none);
  }

  .ui-card--flat {
    background: transparent;
  }

  .ui-card--glass {
    background: rgba(255, 255, 255, 0.045);
    backdrop-filter: blur(18px);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.08);
  }

  .ui-card--pad-none .ui-card__body {
    padding: 0;
  }

  .ui-card--pad-sm .ui-card__body,
  .ui-card__header,
  .ui-card__footer {
    padding: 12px;
  }

  .ui-card--pad-md .ui-card__body {
    padding: 16px;
  }

  .ui-card--pad-lg .ui-card__body {
    padding: 20px;
  }

  .ui-card__header,
  .ui-card__footer {
    color: var(--text-secondary);
  }

  .ui-card__header {
    border-bottom: 1px solid var(--border);
  }

  .ui-card__footer {
    border-top: 1px solid var(--border);
  }

  .ui-card.is-interactive {
    cursor: pointer;
  }

  .ui-card.is-interactive:hover,
  .ui-card.is-interactive:focus-visible {
    border-color: var(--border-hover, var(--border));
    transform: translateY(-1px);
  }

  .ui-card.is-interactive:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-ring);
  }

  .ui-card.is-selected {
    border-color: var(--accent);
    background: var(--accent-lo, var(--bg-elev));
  }

  @media (prefers-reduced-motion: reduce) {
    .ui-card {
      transition: none;
    }

    .ui-card.is-interactive:hover,
    .ui-card.is-interactive:focus-visible {
      transform: none;
    }
  }
</style>
