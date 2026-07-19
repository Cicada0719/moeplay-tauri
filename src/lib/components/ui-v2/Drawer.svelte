<script lang="ts">
  import type { Snippet } from "svelte";
  import { focusTrap } from "../../actions/a11y/focusTrap";
  import type { InitialFocusTarget, ReturnFocusTarget } from "../../actions/a11y/focusTrap";
  import type { DrawerSide, PanelSize, UiDensity } from "./types";

  let {
    open = false,
    title = "面板",
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
    side?: DrawerSide;
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
  const drawerId = $derived(id ?? generatedId);
  const titleId = $derived(`${drawerId}-title`);
  const descriptionId = $derived(`${drawerId}-description`);
  const resolvedTrapFocus = $derived(trapFocus ?? modal);
</script>

{#if open}
  {#if modal}
    <button
      class="v2-drawer__backdrop"
      type="button"
      tabindex="-1"
      data-gamepad-skip="true"
      aria-label={`关闭${title}`}
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
    id={drawerId}
    class="v2-drawer v2-drawer--{side} v2-drawer--{size} {className}"
    role="dialog"
    aria-modal={modal ? "true" : undefined}
    aria-labelledby={titleId}
    aria-describedby={description ? descriptionId : undefined}
    tabindex="-1"
    data-density={density}
    data-ui-v2="drawer"
  >
    <header class="v2-drawer__header">
      <div class="v2-drawer__heading">
        <h2 id={titleId}>{title}</h2>
        {#if description}<p id={descriptionId}>{description}</p>{/if}
      </div>
      <div class="v2-drawer__header-actions">
        {@render actions?.()}
        {#if !hideCloseButton}
          <button class="v2-drawer__close" type="button" aria-label={`关闭${title}`} onclick={onClose}>
            <span aria-hidden="true">×</span>
          </button>
        {/if}
      </div>
    </header>

    <div class="v2-drawer__body">{@render children?.()}</div>
    {#if footer}<footer class="v2-drawer__footer">{@render footer()}</footer>{/if}
  </div>
{/if}

<style>
  .v2-drawer__backdrop {
    position: fixed;
    z-index: 110;
    inset: 0;
    border: 0;
    background: var(--v2-color-scrim);
    cursor: default;
    animation: v2-drawer-backdrop-in var(--v2-motion-base) var(--v2-ease-standard) both;
  }

  .v2-drawer {
    position: fixed;
    z-index: 111;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-color: var(--v2-color-border);
    border-style: solid;
    background: var(--v2-color-surface-raised);
    box-shadow: var(--v2-shadow-panel);
    color: var(--v2-color-text);
    font-family: var(--v2-font-sans);
    outline: none;
  }
  .v2-drawer--right,
  .v2-drawer--left { top: 0; bottom: 0; width: min(100vw, 30rem); min-width: min(100vw, 20rem); }
  .v2-drawer--right { right: 0; border-width: 0 0 0 1px; animation: v2-drawer-right-in var(--v2-motion-slow) var(--v2-ease-standard) both; }
  .v2-drawer--left { left: 0; border-width: 0 1px 0 0; animation: v2-drawer-left-in var(--v2-motion-slow) var(--v2-ease-standard) both; }
  .v2-drawer--bottom { right: 0; bottom: 0; left: 0; max-height: min(80vh, 48rem); border-width: 1px 0 0; border-radius: var(--v2-radius-lg) var(--v2-radius-lg) 0 0; animation: v2-drawer-bottom-in var(--v2-motion-slow) var(--v2-ease-standard) both; }
  .v2-drawer--sm.v2-drawer--right,
  .v2-drawer--sm.v2-drawer--left { width: min(100vw, 24rem); }
  .v2-drawer--lg.v2-drawer--right,
  .v2-drawer--lg.v2-drawer--left { width: min(100vw, 42rem); }
  .v2-drawer--sm.v2-drawer--bottom { max-height: min(50vh, 28rem); }
  .v2-drawer--lg.v2-drawer--bottom { max-height: min(92vh, 64rem); }
  .v2-drawer:focus-visible { box-shadow: var(--v2-shadow-panel), inset var(--v2-focus-ring); }

  .v2-drawer__header,
  .v2-drawer__footer { display: flex; flex: 0 0 auto; align-items: flex-start; justify-content: space-between; gap: var(--v2-space-3); padding: var(--v2-space-4) var(--v2-space-5); border-color: var(--v2-color-border); border-style: solid; }
  .v2-drawer__header { border-width: 0 0 1px; }
  .v2-drawer__footer { align-items: center; border-width: 1px 0 0; }
  .v2-drawer__heading { min-width: 0; }
  .v2-drawer__heading h2 { font-size: var(--v2-text-lg); line-height: var(--v2-leading-tight); }
  .v2-drawer__heading p { margin-top: var(--v2-space-1); color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); line-height: var(--v2-leading-normal); }
  .v2-drawer__header-actions { display: flex; flex: 0 0 auto; align-items: center; gap: var(--v2-space-2); }
  .v2-drawer__close { display: inline-grid; width: 2.5rem; min-height: 2.5rem; place-items: center; padding: 0; border: 1px solid transparent; border-radius: var(--v2-radius-md); background: transparent; color: var(--v2-color-text-secondary); font: 1.5rem/1 var(--v2-font-sans); cursor: pointer; transition: background var(--v2-motion-fast) var(--v2-ease-standard), color var(--v2-motion-fast) var(--v2-ease-standard); }
  .v2-drawer__close:hover { background: var(--v2-color-surface-subtle); color: var(--v2-color-text); }
  .v2-drawer__close:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .v2-drawer__body { min-height: 0; flex: 1 1 auto; overflow: auto; padding: var(--v2-space-5); overscroll-behavior: contain; }

  .v2-drawer[data-density="couch"] .v2-drawer__close,
  :global([data-density="couch"]) .v2-drawer__close { width: 3.5rem; min-height: 3.5rem; font-size: 2rem; }
  .v2-drawer[data-density="couch"] .v2-drawer__heading h2,
  :global([data-density="couch"]) .v2-drawer__heading h2 { font-size: 1.5rem; }
  .v2-drawer[data-density="couch"] .v2-drawer__heading p,
  :global([data-density="couch"]) .v2-drawer__heading p { font-size: 1rem; }
  .v2-drawer[data-density="couch"] :global(button),
  .v2-drawer[data-density="couch"] :global(a),
  :global([data-density="couch"]) .v2-drawer :global(button),
  :global([data-density="couch"]) .v2-drawer :global(a) { min-block-size: 3.5rem; }

  @keyframes v2-drawer-backdrop-in { from { opacity: 0; } }
  @keyframes v2-drawer-right-in { from { opacity: 0; transform: translate3d(1rem, 0, 0); } }
  @keyframes v2-drawer-left-in { from { opacity: 0; transform: translate3d(-1rem, 0, 0); } }
  @keyframes v2-drawer-bottom-in { from { opacity: 0; transform: translate3d(0, 1rem, 0); } }
  @keyframes v2-drawer-fade-in { from { opacity: 0; } }

  @media (max-width: 32rem) {
    .v2-drawer--right,
    .v2-drawer--left { width: 100vw; min-width: 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .v2-drawer,
    .v2-drawer__backdrop { animation-name: v2-drawer-fade-in; animation-duration: 80ms; }
    .v2-drawer__close { transition-duration: 0ms; }
  }
  :global([data-motion="reduce"]) .v2-drawer,
  :global([data-motion="reduce"]) .v2-drawer__backdrop { animation: none; }
  :global([data-motion="reduce"]) .v2-drawer__close { transition-duration: 0ms; }
</style>

