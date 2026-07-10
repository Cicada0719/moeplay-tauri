<script lang="ts">
  import type { Snippet } from "svelte";
  import AsyncState from "./AsyncState.svelte";
  import type { AriaLiveMode, UiDensity, ViewState } from "./types";

  let {
    state = "ready",
    children,
    loading,
    title,
    description,
    onRetry,
    retryLabel = "重试",
    loadingRows = 3,
    ariaLive,
    preserveContent,
    density = "comfortable",
    class: className = "",
  }: {
    state?: ViewState;
    children?: Snippet;
    loading?: Snippet;
    title?: string;
    description?: string;
    onRetry?: () => void;
    retryLabel?: string;
    loadingRows?: number;
    ariaLive?: AriaLiveMode;
    preserveContent?: boolean;
    density?: UiDensity;
    class?: string;
  } = $props();

  const primaryAction = $derived(
    onRetry && state !== "empty"
      ? { label: retryLabel, onSelect: () => onRetry() }
      : undefined,
  );
</script>

<AsyncState
  {state}
  {children}
  {loading}
  {title}
  {description}
  {primaryAction}
  {loadingRows}
  loadingDelayMs={0}
  {ariaLive}
  {preserveContent}
  {density}
  class={className}
/>
