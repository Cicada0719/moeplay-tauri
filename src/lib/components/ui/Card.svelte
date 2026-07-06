<script lang="ts">
  import type { Snippet } from "svelte";

  type CardPadding = "none" | "sm" | "md" | "lg";

  let {
    children,
    class: className = "",
    padding = "md",
    hoverable = false,
    focusable = false,
    onclick,
    onkeydown,
    role,
    ariaLabel,
    ref = $bindable<HTMLElement | undefined>(undefined),
  }: {
    children?: Snippet;
    class?: string;
    padding?: CardPadding;
    hoverable?: boolean;
    focusable?: boolean;
    onclick?: (e: MouseEvent) => void;
    onkeydown?: (e: KeyboardEvent) => void;
    role?: string;
    ariaLabel?: string;
    ref?: HTMLElement | undefined;
  } = $props();

  const isButton = $derived(Boolean(onclick) && role === undefined);
  const classes = $derived(
    `ui-card ui-card--pad-${padding} ${hoverable ? "ui-card--hoverable" : ""} ${focusable ? "ui-card--focusable" : ""} ${className}`,
  );
</script>

{#if isButton}
  <button bind:this={ref as any} class={classes} type="button" aria-label={ariaLabel} onclick={onclick} onkeydown={onkeydown}>
    {@render children?.()}
  </button>
{:else}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    bind:this={ref as any}
    class={classes}
    role={role ?? (focusable ? "button" : undefined)}
    aria-label={ariaLabel}
    onclick={onclick}
    onkeydown={onkeydown}
    tabindex={focusable ? 0 : undefined}
  >
    {@render children?.()}
  </div>
{/if}
