<script lang="ts">
  import type { Snippet } from "svelte";
  import type { UiDensity } from "./types";

  type HeadingLevel = 1 | 2 | 3;

  let {
    title,
    description,
    eyebrow,
    actions,
    children,
    headingLevel = 1,
    density = "comfortable",
    id,
    class: className = "",
  }: {
    title: string;
    description?: string;
    eyebrow?: string;
    actions?: Snippet;
    children?: Snippet;
    headingLevel?: HeadingLevel;
    density?: UiDensity;
    id?: string;
    class?: string;
  } = $props();
</script>

<header class="v2-page-header {className}" data-density={density} data-ui-v2="page-header">
  <div class="v2-page-header__copy">
    {#if eyebrow}<p class="v2-page-header__eyebrow">{eyebrow}</p>{/if}
    {#if headingLevel === 1}
      <h1 class="v2-page-header__title" {id}>{title}</h1>
    {:else if headingLevel === 2}
      <h2 class="v2-page-header__title" {id}>{title}</h2>
    {:else}
      <h3 class="v2-page-header__title" {id}>{title}</h3>
    {/if}
    {#if description}<p class="v2-page-header__description">{description}</p>{/if}
    {@render children?.()}
  </div>

  {#if actions}<div class="v2-page-header__actions" aria-label="页面操作">{@render actions()}</div>{/if}
</header>

<style>
  .v2-page-header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--v2-space-4); min-width: 0; }
  .v2-page-header__copy { min-width: 0; }
  .v2-page-header__eyebrow { margin-bottom: var(--v2-space-2); color: var(--v2-color-accent); font-size: var(--v2-text-xs); font-weight: 700; letter-spacing: 0.08em; line-height: var(--v2-leading-tight); text-transform: uppercase; }
  .v2-page-header__title { color: var(--v2-color-text); font-size: var(--v2-text-xl); font-weight: 700; letter-spacing: -0.02em; line-height: var(--v2-leading-tight); }
  .v2-page-header__description { max-width: 68ch; margin-top: var(--v2-space-2); color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); line-height: var(--v2-leading-normal); }
  .v2-page-header__actions { display: flex; flex: 0 0 auto; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: var(--v2-space-2); }

  .v2-page-header[data-density="couch"] { gap: var(--v2-space-6); }
  .v2-page-header[data-density="couch"] .v2-page-header__title,
  :global([data-density="couch"]) .v2-page-header__title { font-size: clamp(2rem, 3vw, 3.25rem); }
  .v2-page-header[data-density="couch"] .v2-page-header__description,
  :global([data-density="couch"]) .v2-page-header__description { font-size: 1rem; }
  .v2-page-header[data-density="couch"] .v2-page-header__eyebrow,
  :global([data-density="couch"]) .v2-page-header__eyebrow { font-size: 0.875rem; }
  .v2-page-header[data-density="couch"] :global(button),
  .v2-page-header[data-density="couch"] :global(a),
  :global([data-density="couch"]) .v2-page-header :global(button),
  :global([data-density="couch"]) .v2-page-header :global(a) { min-block-size: 3.5rem; }

  @media (max-width: 42rem) {
    .v2-page-header { flex-direction: column; }
    .v2-page-header__actions { justify-content: flex-start; }
  }
</style>
