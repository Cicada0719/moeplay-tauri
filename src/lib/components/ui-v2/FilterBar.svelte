<script lang="ts">
  import type { Snippet } from "svelte";
  import Button from "../ui/Button.svelte";
  import type { UiDensity } from "./types";

  let {
    controls,
    actions,
    children,
    label = "筛选条件",
    activeCount = 0,
    onClear,
    clearLabel = "清除筛选",
    busy = false,
    density = "comfortable",
    class: className = "",
  }: {
    controls?: Snippet;
    actions?: Snippet;
    children?: Snippet;
    label?: string;
    activeCount?: number;
    onClear?: () => void;
    clearLabel?: string;
    busy?: boolean;
    density?: UiDensity;
    class?: string;
  } = $props();

  const hasActiveFilters = $derived(activeCount > 0);
</script>

<section class="v2-filter-bar {className}" aria-label={label} aria-busy={busy} data-density={density} data-ui-v2="filter-bar">
  <div class="v2-filter-bar__controls">
    {#if controls}{@render controls()}{:else}{@render children?.()}{/if}
  </div>

  <div class="v2-filter-bar__actions">
    {#if hasActiveFilters}<span class="v2-filter-bar__summary" aria-live="polite">已启用 {activeCount} 个筛选</span>{/if}
    {#if onClear && hasActiveFilters}<Button variant="quiet" size="sm" onclick={onClear} disabled={busy}>{clearLabel}</Button>{/if}
    {@render actions?.()}
  </div>
</section>

<style>
  .v2-filter-bar { display: flex; align-items: center; justify-content: space-between; gap: var(--v2-space-3); min-width: 0; padding: var(--v2-space-3); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-lg); background: var(--v2-color-surface); }
  .v2-filter-bar__controls,
  .v2-filter-bar__actions { display: flex; align-items: center; gap: var(--v2-space-2); min-width: 0; }
  .v2-filter-bar__controls { flex: 1 1 auto; flex-wrap: wrap; }
  .v2-filter-bar__actions { flex: 0 0 auto; flex-wrap: wrap; justify-content: flex-end; }
  .v2-filter-bar__summary { color: var(--v2-color-text-muted); font-size: var(--v2-text-xs); white-space: nowrap; }

  .v2-filter-bar[data-density="compact"] { padding: var(--v2-space-2); }
  .v2-filter-bar[data-density="couch"] { gap: var(--v2-space-4); padding: var(--v2-space-4); }
  .v2-filter-bar[data-density="couch"] .v2-filter-bar__summary,
  :global([data-density="couch"]) .v2-filter-bar__summary { font-size: 0.875rem; }
  .v2-filter-bar[data-density="couch"] :global(button),
  .v2-filter-bar[data-density="couch"] :global(a),
  .v2-filter-bar[data-density="couch"] :global(input),
  .v2-filter-bar[data-density="couch"] :global(select),
  :global([data-density="couch"]) .v2-filter-bar :global(button),
  :global([data-density="couch"]) .v2-filter-bar :global(a),
  :global([data-density="couch"]) .v2-filter-bar :global(input),
  :global([data-density="couch"]) .v2-filter-bar :global(select) { min-block-size: 3.5rem; font-size: 1rem; }

  @media (max-width: 42rem) {
    .v2-filter-bar { align-items: flex-start; flex-direction: column; }
    .v2-filter-bar__actions { justify-content: flex-start; }
  }
</style>
