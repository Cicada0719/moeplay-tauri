<script lang="ts">
  import type { Snippet } from "svelte";
  import { focusTrap } from "../../actions/a11y/focusTrap";
  import type { InitialFocusTarget, ReturnFocusTarget } from "../../actions/a11y/focusTrap";
  import type { PanelSide, PanelSize, UiDensity } from "./types";

  let {
    open = false,
    title = "详情",
    description,
    children,
    actions,
    footer,
    onClose,
    side = "right",
    size = "md",
    modal = true,
    trapFocus,
    closeOnBackdrop = true,
    closeOnEscape = true,
    hideCloseButton = false,
    initialFocus = "auto",
    returnFocus = true,
    density = "comfortable",
    id,
    class: className = "",
    ref = $bindable<HTMLElement | undefined>(undefined),
  }: {
    open?: boolean;
    title?: string;
    description?: string;
    children?: Snippet;
    actions?: Snippet;
    footer?: Snippet;
    onClose: () => void;
    side?: PanelSide;
    size?: PanelSize;
    modal?: boolean;
    trapFocus?: boolean;
    closeOnBackdrop?: boolean;
    closeOnEscape?: boolean;
    hideCloseButton?: boolean;
    initialFocus?: InitialFocusTarget;
    returnFocus?: ReturnFocusTarget;
    density?: UiDensity;
    id?: string;
    class?: string;
    ref?: HTMLElement | undefined;
  } = $props();

  const generatedId = $props.id();
  const panelId = $derived(id ?? generatedId);
  const titleId = $derived(`${panelId}-title`);
  const descriptionId = $derived(`${panelId}-description`);
  const resolvedTrapFocus = $derived(trapFocus ?? modal);
</script>

{#if open}
  {#if modal}
    <button
      class="v2-detail-panel__backdrop"
      type="button"
      tabindex="-1"
      aria-label="关闭详情面板"
      onclick={() => closeOnBackdrop && onClose()}
    ></button>
  {/if}

  <div
    bind:this={ref}
    use:focusTrap={{
      enabled: open,
      trapFocus: resolvedTrapFocus,
      closeOnEscape,
      initialFocus,
      returnFocus,
      onEscape: () => onClose(),
    }}
    id={panelId}
    class="v2-detail-panel v2-detail-panel--{side} v2-detail-panel--{size} {className}"
    role="dialog"
    aria-modal={modal ? "true" : undefined}
    aria-labelledby={titleId}
    aria-describedby={description ? descriptionId : undefined}
    tabindex="-1"
    data-density={density}
    data-ui-v2="detail-panel"
  >
    <header class="v2-detail-panel__header">
      <div class="v2-detail-panel__heading">
        <h2 id={titleId} class="v2-detail-panel__title">{title}</h2>
        {#if description}<p id={descriptionId} class="v2-detail-panel__description">{description}</p>{/if}
      </div>
      <div class="v2-detail-panel__header-actions">
        {@render actions?.()}
        {#if !hideCloseButton}
          <button class="v2-detail-panel__close" type="button" aria-label="关闭详情面板" onclick={onClose}>
            <span aria-hidden="true">×</span>
          </button>
        {/if}
      </div>
    </header>

    <div class="v2-detail-panel__body">{@render children?.()}</div>
    {#if footer}<footer class="v2-detail-panel__footer">{@render footer()}</footer>{/if}
  </div>
{/if}

<style>
  .v2-detail-panel__backdrop { position: fixed; z-index: 100; inset: 0; border: 0; background: var(--v2-color-scrim); cursor: default; animation: v2-detail-backdrop-in var(--v2-motion-base) var(--v2-ease-standard) both; }
  .v2-detail-panel { position: fixed; z-index: 101; top: 0; bottom: 0; display: flex; flex-direction: column; width: min(100vw, 34rem); min-width: min(100vw, 20rem); overflow: hidden; border-color: var(--v2-color-border); border-style: solid; background: var(--v2-color-surface-raised); box-shadow: var(--v2-shadow-panel); color: var(--v2-color-text); font-family: var(--v2-font-sans); outline: none; }
  .v2-detail-panel--right { right: 0; border-width: 0 0 0 1px; animation: v2-detail-right-in var(--v2-motion-slow) var(--v2-ease-standard) both; }
  .v2-detail-panel--left { left: 0; border-width: 0 1px 0 0; animation: v2-detail-left-in var(--v2-motion-slow) var(--v2-ease-standard) both; }
  .v2-detail-panel--sm { width: min(100vw, 26rem); }
  .v2-detail-panel--lg { width: min(100vw, 44rem); }
  .v2-detail-panel:focus-visible { box-shadow: var(--v2-shadow-panel), inset var(--v2-focus-ring); }

  .v2-detail-panel__header,
  .v2-detail-panel__footer { display: flex; flex: 0 0 auto; align-items: flex-start; justify-content: space-between; gap: var(--v2-space-3); padding: var(--v2-space-4) var(--v2-space-5); border-color: var(--v2-color-border); border-style: solid; }
  .v2-detail-panel__header { border-width: 0 0 1px; }
  .v2-detail-panel__footer { align-items: center; border-width: 1px 0 0; }
  .v2-detail-panel__heading { min-width: 0; }
  .v2-detail-panel__title { font-size: var(--v2-text-lg); line-height: var(--v2-leading-tight); }
  .v2-detail-panel__description { margin-top: var(--v2-space-1); color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); line-height: var(--v2-leading-normal); }
  .v2-detail-panel__header-actions { display: flex; flex: 0 0 auto; align-items: center; gap: var(--v2-space-2); }
  .v2-detail-panel__close { display: inline-grid; width: 2.5rem; min-height: 2.5rem; place-items: center; padding: 0; border: 1px solid transparent; border-radius: var(--v2-radius-md); background: transparent; color: var(--v2-color-text-secondary); font: 1.5rem/1 var(--v2-font-sans); cursor: pointer; transition: background var(--v2-motion-fast) var(--v2-ease-standard), color var(--v2-motion-fast) var(--v2-ease-standard); }
  .v2-detail-panel__close:hover { background: var(--v2-color-surface-subtle); color: var(--v2-color-text); }
  .v2-detail-panel__close:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .v2-detail-panel__body { min-height: 0; flex: 1 1 auto; overflow: auto; padding: var(--v2-space-5); overscroll-behavior: contain; }

  .v2-detail-panel[data-density="couch"] .v2-detail-panel__close,
  :global([data-density="couch"]) .v2-detail-panel__close { width: 3.5rem; min-height: 3.5rem; font-size: 2rem; }
  .v2-detail-panel[data-density="couch"] .v2-detail-panel__title,
  :global([data-density="couch"]) .v2-detail-panel__title { font-size: 1.5rem; }
  .v2-detail-panel[data-density="couch"] .v2-detail-panel__description,
  :global([data-density="couch"]) .v2-detail-panel__description { font-size: 1rem; }
  .v2-detail-panel[data-density="couch"] :global(button),
  .v2-detail-panel[data-density="couch"] :global(a),
  :global([data-density="couch"]) .v2-detail-panel :global(button),
  :global([data-density="couch"]) .v2-detail-panel :global(a) { min-block-size: 3.5rem; }

  @keyframes v2-detail-backdrop-in { from { opacity: 0; } }
  @keyframes v2-detail-right-in { from { opacity: 0; transform: translate3d(1rem, 0, 0); } }
  @keyframes v2-detail-left-in { from { opacity: 0; transform: translate3d(-1rem, 0, 0); } }
  @keyframes v2-detail-fade-in { from { opacity: 0; } }

  @media (max-width: 32rem) { .v2-detail-panel { width: 100vw; min-width: 0; } }
  @media (prefers-reduced-motion: reduce) {
    .v2-detail-panel,
    .v2-detail-panel__backdrop { animation-name: v2-detail-fade-in; animation-duration: 80ms; }
    .v2-detail-panel__close { transition-duration: 0ms; }
  }
  :global([data-motion="reduce"]) .v2-detail-panel,
  :global([data-motion="reduce"]) .v2-detail-panel__backdrop { animation: none; }
  :global([data-motion="reduce"]) .v2-detail-panel__close { transition-duration: 0ms; }
</style>

