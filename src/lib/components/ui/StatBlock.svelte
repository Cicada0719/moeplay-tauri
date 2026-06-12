<script lang="ts">
  import type { Snippet } from "svelte";
  import Skeleton from "./Skeleton.svelte";

  let {
    label,
    value,
    unit,
    hint,
    loading = false,
    error = false,
    children,
    class: className = "",
  }: {
    label?: string;
    value?: string | number;
    unit?: string;
    hint?: string;
    loading?: boolean;
    error?: boolean;
    children?: Snippet;
    class?: string;
  } = $props();
</script>

<section class="ui-stat {error ? 'is-error' : ''} {className}" aria-busy={loading}>
  {#if loading}
    <Skeleton variant="text" width="42%" />
    <Skeleton variant="text" width="76%" height="30px" />
    <Skeleton variant="text" width="54%" />
  {:else}
    {#if label}
      <span class="ui-stat__label">{label}</span>
    {/if}

    <div class="ui-stat__value">
      {#if value !== undefined && value !== null}
        <span>{value}</span>
      {/if}
      {#if unit}
        <small>{unit}</small>
      {/if}
    </div>

    {#if hint}
      <p class="ui-stat__hint">{hint}</p>
    {/if}

    {@render children?.()}
  {/if}
</section>

<style>
  .ui-stat {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-elev);
    padding: 16px;
    color: var(--text-primary);
  }

  .ui-stat.is-error {
    border-color: var(--accent-ring);
    background: var(--accent-lo, var(--bg-elev));
  }

  .ui-stat__label {
    display: block;
    margin-bottom: 8px;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.2;
  }

  .ui-stat__value {
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 6px;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: clamp(24px, 3vw, 36px);
    font-weight: 750;
    line-height: 1;
  }

  .ui-stat__value span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .ui-stat__value small {
    color: var(--text-muted);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 600;
  }

  .ui-stat__hint {
    margin: 10px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }
</style>
