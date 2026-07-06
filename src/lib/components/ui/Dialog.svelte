<script lang="ts">
  import type { Snippet } from "svelte";
  import Overlay from "./Overlay.svelte";
  import { focusTrap } from "../../actions/focus-trap.svelte";

  let {
    open = false,
    onClose,
    title,
    ariaLabel,
    children,
  }: {
    open?: boolean;
    onClose: () => void;
    title?: string;
    ariaLabel?: string;
    children?: Snippet;
  } = $props();

  const label = $derived(ariaLabel ?? title);
</script>

{#if open}
  <div class="ui-dialog-root">
    <Overlay onClose={onClose} ariaLabel={label ? `关闭 ${label}` : "关闭"} />
    <div
      class="ui-dialog-panel"
      role="dialog"
      aria-modal="true"
      aria-label={label}
      tabindex="-1"
      use:focusTrap
    >
      {@render children?.()}
    </div>
  </div>
{/if}

<svelte:window
  onkeydown={(e) => {
    if (open && e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }}
/>

<style>
  .ui-dialog-root {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    pointer-events: none;
    animation: fade-in 0.15s ease;
  }

  .ui-dialog-panel {
    position: relative;
    pointer-events: auto;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    outline: none;
  }

  .ui-dialog-panel:focus-visible {
    box-shadow: 0 0 0 2px var(--accent-ring);
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ui-dialog-root {
      animation: none;
    }
  }
</style>
