<script lang="ts">
  import type { Snippet } from "svelte";
  import Skeleton from "./Skeleton.svelte";

  let {
    title,
    subtitle,
    action,
    loading = false,
    empty = false,
    error,
    skeletonCount = 5,
    itemWidth = "180px",
    children,
    class: className = "",
  }: {
    title?: string;
    subtitle?: string;
    action?: Snippet;
    loading?: boolean;
    empty?: boolean;
    error?: string;
    skeletonCount?: number;
    itemWidth?: string;
    children?: Snippet;
    class?: string;
  } = $props();
</script>

<section class="ui-rail {className}" style={`--ui-rail-item-width: ${itemWidth}`}>
  {#if title || subtitle || action}
    <header class="ui-rail__header">
      <div class="ui-rail__title-group">
        {#if title}
          <h2>{title}</h2>
        {/if}
        {#if subtitle}
          <p>{subtitle}</p>
        {/if}
      </div>

      {#if action}
        <div class="ui-rail__action">
          {@render action()}
        </div>
      {/if}
    </header>
  {/if}

  {#if loading}
    <div class="ui-rail__scroller" aria-busy="true">
      {#each Array.from({ length: Math.max(0, skeletonCount) }) as _}
        <Skeleton variant="card" />
      {/each}
    </div>
  {:else if error}
    <div class="ui-rail__state is-error">{error}</div>
  {:else if empty}
    <div class="ui-rail__state"></div>
  {:else}
    <div class="ui-rail__scroller">
      {@render children?.()}
    </div>
  {/if}
</section>

<style>
  .ui-rail {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .ui-rail__header {
    min-width: 0;
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 16px;
  }

  .ui-rail__title-group {
    min-width: 0;
  }

  .ui-rail__title-group h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 17px;
    font-weight: 750;
    line-height: 1.25;
  }

  .ui-rail__title-group p {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .ui-rail__action {
    flex: 0 0 auto;
  }

  .ui-rail__scroller {
    min-width: 0;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(132px, var(--ui-rail-item-width));
    gap: 14px;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 2px 2px 12px;
    scroll-snap-type: x proximity;
    scrollbar-color: var(--border-hover, var(--border)) transparent;
  }

  .ui-rail__scroller :global(*) {
    scroll-snap-align: start;
  }

  .ui-rail__state {
    min-height: 120px;
    border: 1px dashed var(--border);
    border-radius: var(--radius-lg);
    display: grid;
    place-items: center;
    background: var(--bg-elev);
    color: var(--text-muted);
    font-size: 13px;
  }

  .ui-rail__state.is-error {
    border-color: var(--accent-ring);
    color: var(--text-secondary);
  }

  @media (max-width: 640px) {
    .ui-rail__header {
      align-items: stretch;
      flex-direction: column;
    }

    .ui-rail__action {
      align-self: flex-start;
    }
  }
</style>
