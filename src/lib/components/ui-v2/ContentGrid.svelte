<script lang="ts">
  import type { Snippet } from "svelte";
  import type { ContentGridElement, ContentGridGap, UiDensity } from "./types";

  let {
    children,
    as = "div",
    label,
    labelledBy,
    role = "list",
    busy = false,
    density = "comfortable",
    minItemWidth = "10rem",
    columns,
    gap = "md",
    class: className = "",
    ref = $bindable<HTMLElement | undefined>(undefined),
  }: {
    children?: Snippet;
    as?: ContentGridElement;
    label?: string;
    labelledBy?: string;
    role?: "list" | "grid" | "region" | "presentation" | "none";
    busy?: boolean;
    density?: UiDensity;
    minItemWidth?: string;
    columns?: number;
    gap?: ContentGridGap;
    class?: string;
    ref?: HTMLElement | undefined;
  } = $props();

  const resolvedRole = $derived(role === "none" ? undefined : role);
  const gridStyle = $derived(
    `--v2-grid-card-min:${minItemWidth};${columns ? `--v2-grid-columns:${Math.max(1, Math.floor(columns))};` : ""}`,
  );
</script>

<svelte:element
  this={as}
  bind:this={ref}
  class="v2-content-grid v2-content-grid--{gap} {columns ? 'v2-content-grid--fixed' : ''} {className}"
  role={resolvedRole}
  aria-label={labelledBy ? undefined : label}
  aria-labelledby={labelledBy}
  aria-busy={busy}
  data-density={density}
  data-ui-v2="content-grid"
  style={gridStyle}
>
  {@render children?.()}
</svelte:element>

<style>
  .v2-content-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--v2-grid-card-min)), 1fr));
    min-width: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .v2-content-grid--fixed {
    grid-template-columns: repeat(var(--v2-grid-columns), minmax(0, 1fr));
  }

  .v2-content-grid--sm { gap: var(--v2-space-2); }
  .v2-content-grid--md { gap: var(--v2-space-4); }
  .v2-content-grid--lg { gap: var(--v2-space-6); }

  .v2-content-grid[data-density="compact"] { gap: var(--v2-space-3); }
  .v2-content-grid[data-density="couch"] {
    --v2-grid-card-min: max(13rem, var(--v2-grid-card-min));
    gap: var(--v2-space-6);
  }

  .v2-content-grid[data-density="couch"] :global(a),
  .v2-content-grid[data-density="couch"] :global(button),
  :global([data-density="couch"]) .v2-content-grid :global(a),
  :global([data-density="couch"]) .v2-content-grid :global(button) {
    min-block-size: 3.5rem;
  }

  @container (max-width: 42rem) {
    .v2-content-grid {
      --v2-grid-card-min: min(100%, 9rem);
      gap: var(--v2-space-3);
    }
  }
</style>
