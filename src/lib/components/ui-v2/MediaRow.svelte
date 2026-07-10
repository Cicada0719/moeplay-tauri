<script lang="ts">
  import type { Snippet } from "svelte";
  import type { UiDensity } from "./types";

  let {
    title,
    subtitle,
    description,
    imageSrc,
    imageAlt = "",
    href,
    onActivate,
    meta,
    badge,
    actions,
    density = "comfortable",
    selected = false,
    disabled = false,
    loading = false,
    ariaLabel,
    focusKey,
    itemRole = "listitem",
    class: className = "",
    ref = $bindable<HTMLElement | undefined>(undefined),
    interactiveRef = $bindable<HTMLElement | undefined>(undefined),
  }: {
    title: string;
    subtitle?: string;
    description?: string;
    imageSrc?: string;
    imageAlt?: string;
    href?: string;
    onActivate?: (event: MouseEvent) => void;
    meta?: Snippet;
    badge?: Snippet;
    actions?: Snippet;
    density?: UiDensity;
    selected?: boolean;
    disabled?: boolean;
    loading?: boolean;
    ariaLabel?: string;
    focusKey?: string;
    itemRole?: "listitem" | "gridcell" | "article" | "none";
    class?: string;
    ref?: HTMLElement | undefined;
    interactiveRef?: HTMLElement | undefined;
  } = $props();

  const isDisabled = $derived(disabled || loading);
  const resolvedRole = $derived(itemRole === "none" ? undefined : itemRole);

  function handleAnchorClick(event: MouseEvent) {
    if (isDisabled) {
      event.preventDefault();
      return;
    }
    onActivate?.(event);
  }
</script>

{#snippet content()}
  <div class="v2-media-row__media">
    {#if imageSrc && !loading}<img src={imageSrc} alt={imageAlt} loading="lazy" />{:else}<span aria-hidden="true"></span>{/if}
  </div>
  <div class="v2-media-row__copy">
    <div class="v2-media-row__title-line">
      <h3 title={title}>{title}</h3>
      {#if badge}<div class="v2-media-row__badge">{@render badge()}</div>{/if}
    </div>
    {#if subtitle}<p class="v2-media-row__subtitle">{subtitle}</p>{/if}
    {#if description}<p class="v2-media-row__description">{description}</p>{/if}
    {#if meta}<div class="v2-media-row__meta">{@render meta()}</div>{/if}
  </div>
{/snippet}

<article
  bind:this={ref}
  class="v2-media-row {selected ? 'is-selected' : ''} {loading ? 'is-loading' : ''} {className}"
  role={resolvedRole}
  aria-busy={loading}
  data-density={density}
  data-selected={selected ? "true" : undefined}
  data-focus-key={focusKey}
  data-ui-v2="media-row"
>
  {#if href}
    <a
      bind:this={interactiveRef}
      class="v2-media-row__primary"
      href={href}
      aria-label={ariaLabel}
      aria-disabled={isDisabled ? "true" : undefined}
      aria-current={selected ? "true" : undefined}
      tabindex={isDisabled ? -1 : undefined}
      onclick={handleAnchorClick}
    >{@render content()}</a>
  {:else if onActivate}
    <button
      bind:this={interactiveRef}
      class="v2-media-row__primary"
      type="button"
      aria-label={ariaLabel}
      aria-pressed={selected}
      disabled={isDisabled}
      onclick={onActivate}
    >{@render content()}</button>
  {:else}
    <div class="v2-media-row__primary v2-media-row__primary--static">{@render content()}</div>
  {/if}
  {#if actions}<div class="v2-media-row__actions" aria-label={`${title} 操作`}>{@render actions()}</div>{/if}
  {#if loading}<span class="v2-media-row__sr-only">正在加载 {title}</span>{/if}
</article>

<style>
  .v2-media-row {
    display: flex;
    align-items: stretch;
    min-width: 0;
    overflow: hidden;
    border: 1px solid var(--v2-color-border);
    border-radius: var(--v2-radius-lg);
    background: var(--v2-color-surface);
    color: var(--v2-color-text);
    font-family: var(--v2-font-sans);
    transition: border-color var(--v2-motion-fast) var(--v2-ease-standard), box-shadow var(--v2-motion-fast) var(--v2-ease-standard);
  }
  .v2-media-row:hover { border-color: var(--v2-color-border-strong); }
  .v2-media-row.is-selected { border-color: var(--v2-color-accent); box-shadow: 0 0 0 1px var(--v2-color-accent); }
  .v2-media-row.is-loading { pointer-events: none; }

  .v2-media-row__primary {
    display: flex;
    align-items: center;
    gap: var(--v2-space-3);
    min-width: 0;
    flex: 1 1 auto;
    padding: var(--v2-space-3);
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: start;
    text-decoration: none;
    cursor: pointer;
  }
  .v2-media-row__primary--static { cursor: default; }
  .v2-media-row__primary:focus-visible { outline: none; box-shadow: inset var(--v2-focus-ring); }
  .v2-media-row__primary:disabled,
  .v2-media-row__primary[aria-disabled="true"] { cursor: not-allowed; opacity: 0.58; }
  .v2-media-row__media { flex: 0 0 auto; width: 5rem; aspect-ratio: 4 / 3; overflow: hidden; border-radius: var(--v2-radius-md); background: var(--v2-color-surface-subtle); }
  .v2-media-row__media img,
  .v2-media-row__media span { display: block; width: 100%; height: 100%; object-fit: cover; }
  .v2-media-row__media span { background: linear-gradient(135deg, var(--v2-color-surface-subtle), var(--v2-color-surface-raised)); }
  .v2-media-row.is-loading .v2-media-row__media span { animation: v2-media-row-pulse 1.8s ease-in-out infinite; }
  .v2-media-row__copy { display: grid; gap: var(--v2-space-1); min-width: 0; flex: 1 1 auto; }
  .v2-media-row__title-line { display: flex; align-items: center; gap: var(--v2-space-2); min-width: 0; }
  .v2-media-row__title-line h3 { overflow: hidden; font-size: var(--v2-text-md); line-height: var(--v2-leading-tight); text-overflow: ellipsis; white-space: nowrap; }
  .v2-media-row__badge { flex: 0 0 auto; }
  .v2-media-row__subtitle,
  .v2-media-row__meta { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); line-height: var(--v2-leading-normal); }
  .v2-media-row__description { overflow: hidden; color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); line-height: var(--v2-leading-normal); text-overflow: ellipsis; white-space: nowrap; }
  .v2-media-row__actions { display: flex; flex: 0 0 auto; align-items: center; gap: var(--v2-space-2); padding: var(--v2-space-3); }

  .v2-media-row[data-density="compact"] .v2-media-row__primary { padding: var(--v2-space-2); }
  .v2-media-row[data-density="couch"] .v2-media-row__primary,
  :global([data-density="couch"]) .v2-media-row__primary { min-block-size: 3.5rem; padding: var(--v2-space-4); }
  .v2-media-row[data-density="couch"] .v2-media-row__title-line h3,
  :global([data-density="couch"]) .v2-media-row__title-line h3 { font-size: 1.125rem; }
  .v2-media-row[data-density="couch"] .v2-media-row__subtitle,
  .v2-media-row[data-density="couch"] .v2-media-row__meta,
  :global([data-density="couch"]) .v2-media-row__subtitle,
  :global([data-density="couch"]) .v2-media-row__meta { font-size: 0.875rem; }
  .v2-media-row[data-density="couch"] .v2-media-row__description,
  :global([data-density="couch"]) .v2-media-row__description { font-size: 1rem; }
  .v2-media-row[data-density="couch"] :global(button),
  :global([data-density="couch"]) .v2-media-row :global(button) { min-block-size: 3.5rem; }

  .v2-media-row__sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
  @keyframes v2-media-row-pulse { 50% { opacity: 0.5; } }

  @media (max-width: 34rem) {
    .v2-media-row__media { width: 4rem; }
    .v2-media-row__actions { padding-inline-start: 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .v2-media-row { transition-duration: 0ms; }
    .v2-media-row.is-loading .v2-media-row__media span { animation: none; }
  }
  :global([data-motion="reduce"]) .v2-media-row { transition-duration: 0ms; }
  :global([data-motion="reduce"]) .v2-media-row.is-loading .v2-media-row__media span { animation: none; }
</style>
