<script lang="ts">
  import type { Snippet } from "svelte";
  import type { ShellElement, ShellWidth, UiDensity } from "./types";

  let {
    children,
    as = "main",
    role,
    class: className = "",
    ariaLabel = "主内容",
    width = "content",
    density = "comfortable",
    scrollable = true,
    labelledBy,
    focusable = true,
    ref = $bindable<HTMLElement | undefined>(undefined),
  }: {
    children?: Snippet;
    as?: ShellElement;
    role?: "main" | "region" | "none";
    class?: string;
    ariaLabel?: string;
    width?: ShellWidth;
    density?: UiDensity;
    scrollable?: boolean;
    labelledBy?: string;
    focusable?: boolean;
    ref?: HTMLElement | undefined;
  } = $props();

  const resolvedRole = $derived(
    role === "none" ? undefined : role ?? (as === "main" ? undefined : "region"),
  );
</script>

<svelte:element
  this={as}
  bind:this={ref}
  class="v2-page-shell v2-page-shell--{width} {scrollable ? 'v2-page-shell--scrollable' : ''} {className}"
  role={resolvedRole}
  aria-label={labelledBy ? undefined : ariaLabel}
  aria-labelledby={labelledBy}
  tabindex={focusable ? -1 : undefined}
  data-density={density}
  data-ui-v2="page-shell"
>
  <div class="v2-page-shell__inner">
    {@render children?.()}
  </div>
</svelte:element>

<style>
  .v2-page-shell {
    container-type: inline-size;
    min-width: 0;
    min-height: 0;
    width: 100%;
    color: var(--v2-color-text);
    font-family: var(--v2-font-sans);
    outline: none;
  }

  .v2-page-shell--scrollable {
    overflow: auto;
    overscroll-behavior: contain;
  }

  .v2-page-shell__inner {
    width: 100%;
    min-height: 100%;
    margin-inline: auto;
    padding: var(--v2-page-gutter);
  }

  .v2-page-shell--full .v2-page-shell__inner { max-width: none; }
  .v2-page-shell--content .v2-page-shell__inner { max-width: var(--v2-page-max); }
  .v2-page-shell--narrow .v2-page-shell__inner { max-width: 56rem; }

  .v2-page-shell[data-density="compact"] .v2-page-shell__inner {
    padding: max(var(--v2-space-4), calc(var(--v2-page-gutter) * 0.75));
  }

  .v2-page-shell[data-density="couch"] .v2-page-shell__inner {
    padding:
      max(var(--v2-space-6), env(safe-area-inset-top))
      max(var(--v2-space-8), env(safe-area-inset-right))
      max(var(--v2-space-6), env(safe-area-inset-bottom))
      max(var(--v2-space-8), env(safe-area-inset-left));
  }

</style>
