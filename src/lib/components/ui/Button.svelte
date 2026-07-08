<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Snippet } from "svelte";

  type ButtonVariant = "primary" | "secondary" | "ghost" | "quiet";
  type ButtonSize = "sm" | "md" | "lg";
  type ButtonType = "button" | "submit" | "reset";

  let {
    variant = "primary",
    size = "md",
    type = "button",
    disabled = false,
    loading = false,
    fullWidth = false,
    title,
    ariaLabel,
    onclick,
    onClick,
    press,
    children,
    class: className = "",
  }: {
    variant?: ButtonVariant;
    size?: ButtonSize;
    type?: ButtonType;
    disabled?: boolean;
    loading?: boolean;
    fullWidth?: boolean;
    title?: string;
    ariaLabel?: string;
    onclick?: (event: MouseEvent) => void;
    onClick?: (event: MouseEvent) => void;
    press?: (event: MouseEvent) => void;
    children?: Snippet;
    class?: string;
  } = $props();

  const isDisabled = $derived(disabled || loading);
  const dispatch = createEventDispatcher<{ click: MouseEvent }>();

  function handleClick(event: MouseEvent) {
    onclick?.(event);
    onClick?.(event);
    press?.(event);
    dispatch("click", event);
  }

  function clickAction(node: HTMLButtonElement, handler: (event: MouseEvent) => void) {
    let current = handler;
    const listener = (event: MouseEvent) => current(event);
    node.addEventListener("click", listener);

    return {
      update(next: (event: MouseEvent) => void) {
        current = next;
      },
      destroy() {
        node.removeEventListener("click", listener);
      },
    };
  }
</script>

<button
  class="ui-button ui-button--{variant} ui-button--{size} {fullWidth ? 'is-full' : ''} {className}"
  {type}
  disabled={isDisabled}
  {title}
  aria-label={ariaLabel}
  aria-busy={loading}
  use:clickAction={handleClick}
>
  {#if loading}
    <span class="ui-button__loader" aria-hidden="true"></span>
  {/if}
  <span class="ui-button__content">
    {@render children?.()}
  </span>
</button>

<style>
  .ui-button {
    min-width: 0;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-family: var(--font-ui);
    font-weight: 650;
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
    user-select: none;
    transition:
      transform 0.14s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      background 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      border-color 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      color 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1)),
      opacity 0.18s var(--ease, cubic-bezier(0.16, 1, 0.3, 1));
  }

  .ui-button--sm {
    min-height: 32px;
    padding: 0 12px;
    font-size: 12px;
  }

  .ui-button--md {
    min-height: 38px;
    padding: 0 16px;
    font-size: 13px;
  }

  .ui-button--lg {
    min-height: 46px;
    padding: 0 20px;
    font-size: 14px;
  }

  .ui-button.is-full {
    width: 100%;
  }

  .ui-button:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-ring);
  }

  .ui-button:active:not(:disabled) {
    transform: translateY(1px);
  }

  .ui-button:disabled {
    cursor: not-allowed;
    opacity: 0.56;
  }

  .ui-button--primary {
    background: var(--accent);
    color: white;
  }

  .ui-button--primary:hover:not(:disabled) {
    background: var(--accent-hi, var(--accent));
  }

  .ui-button--secondary {
    background: var(--bg-elev);
    border-color: var(--border);
    color: var(--text-primary);
  }

  .ui-button--secondary:hover:not(:disabled) {
    background: var(--bg-hover, var(--bg-elev));
    border-color: var(--border-hover, var(--border));
  }

  .ui-button--ghost {
    background: transparent;
    border-color: var(--border);
    color: var(--text-secondary);
  }

  .ui-button--ghost:hover:not(:disabled) {
    background: var(--bg-hover, var(--bg-elev));
    color: var(--text-primary);
  }

  .ui-button--quiet {
    background: transparent;
    color: var(--text-secondary);
  }

  .ui-button--quiet:hover:not(:disabled) {
    background: var(--bg-hover, var(--bg-elev));
    color: var(--text-primary);
  }

  .ui-button__content {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ui-button__loader {
    width: 12px;
    height: 12px;
    border-radius: var(--radius-full, 9999px);
    border: 2px solid currentColor;
    border-right-color: transparent;
    opacity: 0.82;
    animation: ui-button-spin 0.8s linear infinite;
  }

  @keyframes ui-button-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ui-button {
      transition: none;
    }

    .ui-button__loader {
      animation: none;
    }
  }
</style>
