<script lang="ts">
  import type { Snippet } from "svelte";
  import type { MediaVariant, UiDensity } from "./types";

  let {
    title,
    subtitle,
    description,
    imageSrc,
    imageAlt = "",
    imageLoading = "lazy",
    href,
    onActivate,
    badges,
    meta,
    actions,
    footer,
    variant = "poster",
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
    imageLoading?: "eager" | "lazy";
    href?: string;
    onActivate?: (event: MouseEvent) => void;
    badges?: Snippet;
    meta?: Snippet;
    actions?: Snippet;
    footer?: Snippet;
    variant?: MediaVariant;
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
  <div class="v2-media-card__media">
    {#if imageSrc && !loading}
      <img src={imageSrc} alt={imageAlt} loading={imageLoading} />
    {:else}
      <div class="v2-media-card__placeholder" aria-hidden="true"></div>
    {/if}
    {#if badges}
      <div class="v2-media-card__badges">{@render badges()}</div>
    {/if}
  </div>
  <div class="v2-media-card__copy">
    <h3 title={title}>{title}</h3>
    {#if subtitle}<p class="v2-media-card__subtitle">{subtitle}</p>{/if}
    {#if description}<p class="v2-media-card__description">{description}</p>{/if}
    {#if meta}<div class="v2-media-card__meta">{@render meta()}</div>{/if}
  </div>
{/snippet}

<article
  bind:this={ref}
  class="v2-media-card v2-media-card--{variant} {selected ? 'is-selected' : ''} {loading ? 'is-loading' : ''} {className}"
  role={resolvedRole}
  aria-busy={loading}
  data-density={density}
  data-selected={selected ? "true" : undefined}
  data-focus-key={focusKey}
  data-ui-v2="media-card"
>
  {#if href}
    <a
      bind:this={interactiveRef}
      class="v2-media-card__primary"
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
      class="v2-media-card__primary"
      type="button"
      aria-label={ariaLabel}
      aria-pressed={selected}
      disabled={isDisabled}
      onclick={onActivate}
    >{@render content()}</button>
  {:else}
    <div class="v2-media-card__primary v2-media-card__primary--static">
      {@render content()}
    </div>
  {/if}

  {#if actions}
    <div class="v2-media-card__actions" aria-label={`${title} 操作`}>
      {@render actions()}
    </div>
  {/if}
  {#if footer}<footer class="v2-media-card__footer">{@render footer()}</footer>{/if}
  {#if loading}<span class="v2-media-card__sr-only">正在加载 {title}</span>{/if}
</article>

<style>
  .v2-media-card {
    position: relative;
    display: flex;
    min-width: 0;
    height: 100%;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--v2-color-border);
    border-radius: var(--v2-radius-lg);
    background: var(--v2-color-surface);
    color: var(--v2-color-text);
    font-family: var(--v2-font-sans);
    transition: border-color var(--v2-motion-fast) var(--v2-ease-standard), box-shadow var(--v2-motion-fast) var(--v2-ease-standard), transform var(--v2-motion-fast) var(--v2-ease-standard);
  }
  .v2-media-card:hover { border-color: var(--v2-color-border-strong); }
  .v2-media-card.is-selected { border-color: var(--v2-color-accent); box-shadow: 0 0 0 1px var(--v2-color-accent); }
  .v2-media-card.is-loading { pointer-events: none; }

  .v2-media-card__primary {
    display: flex;
    min-width: 0;
    flex: 1 1 auto;
    flex-direction: column;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: start;
    text-decoration: none;
    cursor: pointer;
  }
  .v2-media-card__primary--static { cursor: default; }
  .v2-media-card__primary:focus-visible { outline: none; box-shadow: inset var(--v2-focus-ring); }
  .v2-media-card__primary:disabled,
  .v2-media-card__primary[aria-disabled="true"] { cursor: not-allowed; opacity: 0.58; }

  .v2-media-card__media { position: relative; overflow: hidden; width: 100%; aspect-ratio: 2 / 3; background: var(--v2-color-surface-subtle); }
  .v2-media-card--landscape .v2-media-card__media { aspect-ratio: 16 / 9; }
  .v2-media-card--square .v2-media-card__media { aspect-ratio: 1; }
  .v2-media-card__media img { width: 100%; height: 100%; object-fit: cover; transition: transform var(--v2-motion-base) var(--v2-ease-standard); }
  .v2-media-card__primary:hover .v2-media-card__media img { transform: scale(1.025); }
  .v2-media-card__placeholder { width: 100%; height: 100%; background: linear-gradient(135deg, var(--v2-color-surface-subtle), var(--v2-color-surface-raised)); }
  .v2-media-card.is-loading .v2-media-card__placeholder { animation: v2-media-card-pulse 1.8s ease-in-out infinite; }

  .v2-media-card__badges { position: absolute; z-index: 1; inset: var(--v2-space-2) var(--v2-space-2) auto; display: flex; flex-wrap: wrap; gap: var(--v2-space-1); pointer-events: none; }
  .v2-media-card__copy { display: grid; gap: var(--v2-space-1); min-width: 0; padding: var(--v2-space-3); }
  .v2-media-card__copy h3 { overflow: hidden; font-size: var(--v2-text-md); line-height: var(--v2-leading-tight); text-overflow: ellipsis; white-space: nowrap; }
  .v2-media-card__subtitle,
  .v2-media-card__meta { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); line-height: var(--v2-leading-normal); }
  .v2-media-card__description { display: -webkit-box; overflow: hidden; color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); line-height: var(--v2-leading-normal); -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3; }
  .v2-media-card__actions,
  .v2-media-card__footer { display: flex; align-items: center; gap: var(--v2-space-2); padding: 0 var(--v2-space-3) var(--v2-space-3); }
  .v2-media-card__actions { justify-content: flex-end; }
  .v2-media-card__footer { margin-top: auto; color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }

  .v2-media-card[data-density="compact"] .v2-media-card__copy { padding: var(--v2-space-2); }
  .v2-media-card[data-density="couch"] .v2-media-card__primary,
  :global([data-density="couch"]) .v2-media-card__primary { min-block-size: 3.5rem; }
  .v2-media-card[data-density="couch"] .v2-media-card__copy h3,
  :global([data-density="couch"]) .v2-media-card__copy h3 { font-size: 1.125rem; }
  .v2-media-card[data-density="couch"] .v2-media-card__subtitle,
  .v2-media-card[data-density="couch"] .v2-media-card__meta,
  .v2-media-card[data-density="couch"] .v2-media-card__footer,
  :global([data-density="couch"]) .v2-media-card__subtitle,
  :global([data-density="couch"]) .v2-media-card__meta,
  :global([data-density="couch"]) .v2-media-card__footer { font-size: 0.875rem; }
  .v2-media-card[data-density="couch"] .v2-media-card__description,
  :global([data-density="couch"]) .v2-media-card__description { font-size: 1rem; }
  .v2-media-card[data-density="couch"] :global(button),
  :global([data-density="couch"]) .v2-media-card :global(button) { min-block-size: 3.5rem; }

  .v2-media-card__sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
  @keyframes v2-media-card-pulse { 50% { opacity: 0.5; } }

  @media (prefers-reduced-motion: reduce) {
    .v2-media-card,
    .v2-media-card__media img { transition-duration: 0ms; }
    .v2-media-card__primary:hover .v2-media-card__media img { transform: none; }
    .v2-media-card.is-loading .v2-media-card__placeholder { animation: none; }
  }
  :global([data-motion="reduce"]) .v2-media-card,
  :global([data-motion="reduce"]) .v2-media-card__media img { transition-duration: 0ms; }
  :global([data-motion="reduce"]) .v2-media-card__primary:hover .v2-media-card__media img { transform: none; }
  :global([data-motion="reduce"]) .v2-media-card.is-loading .v2-media-card__placeholder { animation: none; }
</style>

