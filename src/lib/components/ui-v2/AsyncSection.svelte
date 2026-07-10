<script lang="ts">
  import type { Snippet } from "svelte";
  import AsyncState from "./AsyncState.svelte";
  import type {
    AriaLiveMode,
    AsyncAction,
    AsyncDetails,
    UiDensity,
    ViewState,
  } from "./types";

  type HeadingLevel = 2 | 3;

  let {
    title,
    description,
    state = "ready",
    children,
    loading,
    status,
    actions,
    primaryAction,
    secondaryAction,
    details,
    detailsLabel,
    ariaLive,
    preserveContent,
    loadingRows = 3,
    loadingDelayMs = 200,
    headingLevel = 2,
    density = "comfortable",
    compact = false,
    id,
    class: className = "",
  }: {
    title: string;
    description?: string;
    state?: ViewState;
    children?: Snippet;
    loading?: Snippet;
    status?: Snippet;
    actions?: Snippet;
    primaryAction?: AsyncAction;
    secondaryAction?: AsyncAction;
    details?: AsyncDetails;
    detailsLabel?: string;
    ariaLive?: AriaLiveMode;
    preserveContent?: boolean;
    loadingRows?: number;
    loadingDelayMs?: number;
    headingLevel?: HeadingLevel;
    density?: UiDensity;
    compact?: boolean;
    id?: string;
    class?: string;
  } = $props();

  const generatedId = $props.id();
  const sectionId = $derived(id ?? generatedId);
  const headingId = $derived(`${sectionId}-title`);
</script>

<section
  id={sectionId}
  class="v2-async-section {className}"
  aria-labelledby={headingId}
  aria-busy={state === "loading" || state === "refreshing"}
  data-density={density}
  data-state={state}
  data-ui-v2="async-section"
>
  <header class="v2-async-section__header">
    <div class="v2-async-section__heading">
      {#if headingLevel === 2}
        <h2 id={headingId}>{title}</h2>
      {:else}
        <h3 id={headingId}>{title}</h3>
      {/if}
      {#if description}<p>{description}</p>{/if}
    </div>
    {#if status || actions}
      <div class="v2-async-section__header-side">
        {#if status}<div class="v2-async-section__status">{@render status()}</div>{/if}
        {#if actions}<div class="v2-async-section__header-actions">{@render actions()}</div>{/if}
      </div>
    {/if}
  </header>

  <div class="v2-async-section__body">
    <AsyncState
      {state}
      {children}
      {loading}
      {primaryAction}
      {secondaryAction}
      {details}
      {detailsLabel}
      {ariaLive}
      {preserveContent}
      {loadingRows}
      {loadingDelayMs}
      {density}
      {compact}
    />
  </div>
</section>

<style>
  .v2-async-section { min-width: 0; color: var(--v2-color-text); font-family: var(--v2-font-sans); }
  .v2-async-section__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--v2-space-4);
    margin-bottom: var(--v2-space-4);
  }
  .v2-async-section__heading { min-width: 0; }
  .v2-async-section__heading h2,
  .v2-async-section__heading h3 { font-size: var(--v2-text-lg); line-height: var(--v2-leading-tight); }
  .v2-async-section__heading p { max-width: 68ch; margin-top: var(--v2-space-1); color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); line-height: var(--v2-leading-normal); }
  .v2-async-section__header-side,
  .v2-async-section__header-actions { display: flex; flex: 0 0 auto; align-items: center; gap: var(--v2-space-2); }
  .v2-async-section__status { color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); }
  .v2-async-section__body { min-width: 0; }

  .v2-async-section[data-density="couch"] .v2-async-section__heading h2,
  .v2-async-section[data-density="couch"] .v2-async-section__heading h3,
  :global([data-density="couch"]) .v2-async-section__heading h2,
  :global([data-density="couch"]) .v2-async-section__heading h3 { font-size: 1.5rem; }
  .v2-async-section[data-density="couch"] .v2-async-section__heading p,
  :global([data-density="couch"]) .v2-async-section__heading p { font-size: 1rem; }

  @media (max-width: 42rem) {
    .v2-async-section__header { flex-direction: column; }
    .v2-async-section__header-side { width: 100%; justify-content: space-between; }
  }
</style>
