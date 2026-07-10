<script lang="ts">
  import type { Snippet } from "svelte";
  import type {
    AriaLiveMode,
    AsyncAction,
    AsyncDetails,
    UiDensity,
    ViewState,
  } from "./types";

  type StateCopy = { title: string; description: string };

  const defaultCopy: Record<Exclude<ViewState, "ready">, StateCopy> = {
    loading: { title: "正在加载", description: "内容正在准备中，请稍候。" },
    refreshing: { title: "正在刷新", description: "保留当前内容，同时获取最新数据。" },
    empty: { title: "暂无内容", description: "这里暂时还没有可显示的内容。" },
    "no-results": { title: "没有匹配结果", description: "请清除筛选或尝试其他搜索条件。" },
    error: { title: "加载失败", description: "暂时无法获取内容，请稍后重试。" },
    offline: { title: "当前处于离线状态", description: "本地内容仍可使用；联网后可重新同步。" },
    stale: { title: "显示的是较早的数据", description: "当前内容仍可使用，你可以刷新到最新状态。" },
    partial: { title: "部分内容未能加载", description: "已显示可用内容；你可以稍后重试以获取完整结果。" },
  };

  let {
    state: viewState = "ready",
    children,
    loading,
    icon,
    title,
    description,
    primaryAction,
    secondaryAction,
    details,
    detailsLabel = "技术详情",
    ariaLive,
    preserveContent,
    loadingRows = 3,
    loadingDelayMs = 200,
    compact = false,
    density = "comfortable",
    class: className = "",
  }: {
    state?: ViewState;
    children?: Snippet;
    loading?: Snippet;
    icon?: Snippet;
    title?: string;
    description?: string;
    primaryAction?: AsyncAction;
    secondaryAction?: AsyncAction;
    details?: AsyncDetails;
    detailsLabel?: string;
    ariaLive?: AriaLiveMode;
    preserveContent?: boolean;
    loadingRows?: number;
    loadingDelayMs?: number;
    compact?: boolean;
    density?: UiDensity;
    class?: string;
  } = $props();

  let showLoadingVisual = $state(false);

  const copy = $derived(viewState === "ready" ? undefined : defaultCopy[viewState]);
  const resolvedTitle = $derived(title ?? copy?.title);
  const resolvedDescription = $derived(description ?? copy?.description);
  const defaultPreserve = $derived(
    viewState === "refreshing" || viewState === "stale" || viewState === "partial" || viewState === "offline",
  );
  const shouldPreserve = $derived((preserveContent ?? defaultPreserve) && Boolean(children));
  const resolvedLive = $derived<AriaLiveMode>(
    ariaLive ?? (viewState === "error" ? "assertive" : viewState === "ready" ? "off" : "polite"),
  );
  const liveRole = $derived(viewState === "error" && resolvedLive === "assertive" ? "alert" : "status");

  $effect(() => {
    if (viewState !== "loading") {
      showLoadingVisual = false;
      return;
    }

    if (loadingDelayMs <= 0) {
      showLoadingVisual = true;
      return;
    }

    showLoadingVisual = false;
    const timer = window.setTimeout(() => {
      showLoadingVisual = true;
    }, loadingDelayMs);
    return () => window.clearTimeout(timer);
  });

  function actionClass(action: AsyncAction, primary: boolean) {
    return `v2-async-state__button ${primary ? "v2-async-state__button--primary" : "v2-async-state__button--secondary"} ${action.loading ? "is-loading" : ""}`;
  }
</script>

{#if viewState === "ready"}
  {@render children?.()}
{:else if viewState === "loading"}
  <section
    class="v2-async-state v2-async-state--loading {compact ? 'v2-async-state--compact' : ''} {className}"
    aria-busy="true"
    aria-live={resolvedLive}
    aria-label={resolvedTitle}
    data-density={density}
    data-state={viewState}
    data-ui-v2="async-state"
  >
    <span class="v2-async-state__sr-only">{resolvedTitle}</span>
    {#if showLoadingVisual}
      {#if loading}
        {@render loading()}
      {:else}
        <div class="v2-async-state__skeleton" aria-hidden="true">
          {#each Array(Math.max(1, loadingRows)) as _, index (index)}
            <span style={`--v2-skeleton-index:${index}`}></span>
          {/each}
        </div>
      {/if}
    {/if}
  </section>
{:else if shouldPreserve}
  <section
    class="v2-async-state__notice v2-async-state__notice--{viewState} {className}"
    role={liveRole}
    aria-live={resolvedLive}
    aria-busy={viewState === "refreshing"}
    data-density={density}
    data-state={viewState}
    data-ui-v2="async-state"
  >
    <div class="v2-async-state__notice-copy">
      <strong>{resolvedTitle}</strong>
      {#if resolvedDescription}<span>{resolvedDescription}</span>{/if}
    </div>
    <div class="v2-async-state__actions">
      {#if secondaryAction}
        <button
          type="button"
          class={actionClass(secondaryAction, false)}
          disabled={secondaryAction.disabled || secondaryAction.loading}
          aria-label={secondaryAction.ariaLabel}
          aria-busy={secondaryAction.loading}
          onclick={secondaryAction.onSelect}
        >{secondaryAction.loading ? "处理中…" : secondaryAction.label}</button>
      {/if}
      {#if primaryAction}
        <button
          type="button"
          class={actionClass(primaryAction, true)}
          disabled={primaryAction.disabled || primaryAction.loading}
          aria-label={primaryAction.ariaLabel}
          aria-busy={primaryAction.loading}
          onclick={primaryAction.onSelect}
        >{primaryAction.loading ? "处理中…" : primaryAction.label}</button>
      {/if}
    </div>
  </section>
  {@render children?.()}
{:else}
  <section
    class="v2-async-state v2-async-state--{viewState} {compact ? 'v2-async-state--compact' : ''} {className}"
    role={liveRole}
    aria-live={resolvedLive}
    data-density={density}
    data-state={viewState}
    data-ui-v2="async-state"
  >
    {#if icon}
      <div class="v2-async-state__icon" aria-hidden="true">{@render icon()}</div>
    {:else}
      <div class="v2-async-state__marker" aria-hidden="true"></div>
    {/if}
    <h2 class="v2-async-state__title">{resolvedTitle}</h2>
    {#if resolvedDescription}
      <p class="v2-async-state__description">{resolvedDescription}</p>
    {/if}
    {#if primaryAction || secondaryAction}
      <div class="v2-async-state__actions">
        {#if primaryAction}
          <button
            type="button"
            class={actionClass(primaryAction, true)}
            disabled={primaryAction.disabled || primaryAction.loading}
            aria-label={primaryAction.ariaLabel}
            aria-busy={primaryAction.loading}
            onclick={primaryAction.onSelect}
          >{primaryAction.loading ? "处理中…" : primaryAction.label}</button>
        {/if}
        {#if secondaryAction}
          <button
            type="button"
            class={actionClass(secondaryAction, false)}
            disabled={secondaryAction.disabled || secondaryAction.loading}
            aria-label={secondaryAction.ariaLabel}
            aria-busy={secondaryAction.loading}
            onclick={secondaryAction.onSelect}
          >{secondaryAction.loading ? "处理中…" : secondaryAction.label}</button>
        {/if}
      </div>
    {/if}
    {#if details}
      <details class="v2-async-state__details">
        <summary>{detailsLabel}</summary>
        {#if typeof details === "string"}
          <pre>{details}</pre>
        {:else}
          {@render details()}
        {/if}
      </details>
    {/if}
  </section>
{/if}

<style>
  .v2-async-state {
    display: grid;
    justify-items: center;
    place-content: center;
    min-height: 12rem;
    padding: var(--v2-space-8);
    border: 1px solid var(--v2-color-border);
    border-radius: var(--v2-radius-lg);
    background: var(--v2-color-surface);
    color: var(--v2-color-text);
    font-family: var(--v2-font-sans);
    text-align: center;
  }

  .v2-async-state--compact { min-height: 8rem; padding: var(--v2-space-5); }

  .v2-async-state--loading {
    display: block;
    min-height: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }

  .v2-async-state__skeleton { display: grid; gap: var(--v2-space-3); width: 100%; }
  .v2-async-state__skeleton span {
    display: block;
    min-height: 3.5rem;
    border-radius: var(--v2-radius-md);
    background: var(--v2-color-surface-subtle);
    opacity: 0.72;
    animation: v2-async-pulse 1.8s ease-in-out infinite;
    animation-delay: calc(var(--v2-skeleton-index) * 70ms);
  }

  .v2-async-state__marker {
    width: 0.65rem;
    height: 0.65rem;
    margin-bottom: var(--v2-space-3);
    border-radius: var(--v2-radius-full);
    background: var(--v2-color-info);
    box-shadow: 0 0 0 0.35rem color-mix(in srgb, var(--v2-color-info) 14%, transparent);
  }

  .v2-async-state--empty .v2-async-state__marker,
  .v2-async-state--no-results .v2-async-state__marker { background: var(--v2-color-text-muted); }
  .v2-async-state--error .v2-async-state__marker { background: var(--v2-color-danger); }
  .v2-async-state--offline .v2-async-state__marker { background: var(--v2-color-warning); }

  .v2-async-state__icon { margin-bottom: var(--v2-space-3); color: var(--v2-color-text-secondary); }
  .v2-async-state__title { font-size: var(--v2-text-lg); line-height: var(--v2-leading-tight); }
  .v2-async-state__description {
    max-width: 48ch;
    margin-top: var(--v2-space-2);
    color: var(--v2-color-text-secondary);
    font-size: var(--v2-text-sm);
    line-height: var(--v2-leading-normal);
  }

  .v2-async-state__actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: var(--v2-space-2);
    margin-top: var(--v2-space-4);
  }

  .v2-async-state__button {
    min-block-size: 2.5rem;
    padding: 0 var(--v2-space-4);
    border: 1px solid var(--v2-color-border);
    border-radius: var(--v2-radius-md);
    background: var(--v2-color-surface-raised);
    color: var(--v2-color-text);
    font: inherit;
    font-weight: 650;
    cursor: pointer;
    transition: background var(--v2-motion-fast) var(--v2-ease-standard), transform var(--v2-motion-fast) var(--v2-ease-standard);
  }
  .v2-async-state__button--primary { border-color: transparent; background: var(--v2-color-accent); color: white; }
  .v2-async-state__button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .v2-async-state__button:active:not(:disabled) { transform: translateY(1px); }
  .v2-async-state__button:disabled { cursor: not-allowed; opacity: 0.56; }

  .v2-async-state__notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--v2-space-3);
    margin-bottom: var(--v2-space-4);
    padding: var(--v2-space-3) var(--v2-space-4);
    border: 1px solid var(--v2-color-border);
    border-inline-start: 3px solid var(--v2-color-info);
    border-radius: var(--v2-radius-md);
    background: var(--v2-color-surface-subtle);
    color: var(--v2-color-text);
  }
  .v2-async-state__notice--partial,
  .v2-async-state__notice--offline { border-inline-start-color: var(--v2-color-warning); }
  .v2-async-state__notice--error { border-inline-start-color: var(--v2-color-danger); }
  .v2-async-state__notice-copy { display: grid; gap: var(--v2-space-1); min-width: 0; }
  .v2-async-state__notice-copy strong { font-size: var(--v2-text-sm); }
  .v2-async-state__notice-copy span { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); line-height: var(--v2-leading-normal); }
  .v2-async-state__notice .v2-async-state__actions { flex: 0 0 auto; margin-top: 0; }

  .v2-async-state__details { width: min(100%, 42rem); margin-top: var(--v2-space-4); color: var(--v2-color-text-secondary); text-align: start; }
  .v2-async-state__details summary { cursor: pointer; font-size: var(--v2-text-sm); }
  .v2-async-state__details pre { overflow: auto; margin-top: var(--v2-space-2); padding: var(--v2-space-3); border-radius: var(--v2-radius-sm); background: var(--v2-color-canvas); font: 0.75rem/1.5 var(--v2-font-mono); white-space: pre-wrap; }

  .v2-async-state[data-density="couch"] .v2-async-state__button,
  :global([data-density="couch"]) .v2-async-state__button { min-block-size: 3.5rem; font-size: 1rem; }
  .v2-async-state[data-density="couch"] .v2-async-state__description,
  :global([data-density="couch"]) .v2-async-state__description { font-size: 1rem; }
  .v2-async-state[data-density="couch"] .v2-async-state__notice-copy span,
  :global([data-density="couch"]) .v2-async-state__notice-copy span { font-size: 0.875rem; }

  .v2-async-state__sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @keyframes v2-async-pulse { 50% { opacity: 0.48; } }

  @media (max-width: 42rem) {
    .v2-async-state__notice { align-items: flex-start; flex-direction: column; }
    .v2-async-state__notice .v2-async-state__actions { justify-content: flex-start; }
  }

  @media (prefers-reduced-motion: reduce) {
    .v2-async-state__skeleton span { animation: none; }
    .v2-async-state__button { transition-duration: 0ms; }
  }

  :global([data-motion="reduce"]) .v2-async-state__skeleton span { animation: none; }
  :global([data-motion="reduce"]) .v2-async-state__button { transition-duration: 0ms; }
</style>


