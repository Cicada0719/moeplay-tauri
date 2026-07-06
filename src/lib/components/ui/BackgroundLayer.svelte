<script lang="ts">
  let {
    src,
    fallback,
    alt = "",
    overlay = true,
    class: className = "",
  }: {
    src?: string;
    fallback?: string;
    alt?: string;
    overlay?: boolean;
    class?: string;
  } = $props();

  let current = $state("");
  let previous = $state<string | null>(null);
  let transitioning = $state(false);
  let hasLoaded = $state(false);

  function resolveUrl(): string {
    return src ?? fallback ?? "";
  }

  function loadImage(url: string): Promise<void> {
    if (!url) return Promise.resolve();
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => resolve();
      img.onerror = () => resolve();
      img.src = url;
    });
  }

  $effect(() => {
    const next = resolveUrl();
    if (next === current) {
      previous = null;
      return;
    }
    if (!hasLoaded) {
      // First image: show immediately, no fade from blank.
      loadImage(next).then(() => {
        if (next !== resolveUrl()) return;
        current = next;
        hasLoaded = true;
      });
      return;
    }
    // Preload before swapping so we never flash a blank frame.
    loadImage(next).then(() => {
      if (next !== resolveUrl()) return; // src changed again while loading
      previous = current;
      current = next;
      transitioning = true;
      // Clear previous after the crossfade finishes.
      setTimeout(() => {
        previous = null;
        transitioning = false;
      }, 420);
    });
  });
</script>

<div
  class="ui-bg-layer {className}"
  style:background-color="var(--bg-deep)"
  aria-hidden="true"
  role="img"
  aria-label={alt}
>
  {#if previous}
    <div
      class="ui-bg-layer__slide ui-bg-layer__slide--prev"
      class:is-fading={transitioning}
      style:background-image={previous ? `url(${JSON.stringify(previous)})` : undefined}
    ></div>
  {/if}
  <div
    class="ui-bg-layer__slide ui-bg-layer__slide--curr"
    class:is-fading={transitioning}
    style:background-image={current ? `url(${JSON.stringify(current)})` : undefined}
  ></div>
  {#if overlay}
    <div class="ui-bg-layer__overlay"></div>
  {/if}
</div>

<style>
  .ui-bg-layer {
    position: absolute;
    inset: 0;
    z-index: 0;
    background-position: center;
    background-size: cover;
    background-repeat: no-repeat;
    pointer-events: none;
    overflow: hidden;
  }

  .ui-bg-layer__slide {
    position: absolute;
    inset: 0;
    z-index: 0;
    background-position: inherit;
    background-size: inherit;
    background-repeat: inherit;
    opacity: 1;
    transition: opacity 0.4s ease;
  }

  .ui-bg-layer__slide--curr.is-fading {
    opacity: 0;
    animation: bg-fade-in 0.4s ease forwards;
  }

  .ui-bg-layer__slide--prev.is-fading {
    opacity: 1;
    animation: bg-fade-out 0.4s ease forwards;
  }

  @keyframes bg-fade-in {
    to { opacity: 1; }
  }
  @keyframes bg-fade-out {
    to { opacity: 0; }
  }

  .ui-bg-layer__overlay {
    position: absolute;
    inset: 0;
    z-index: 1;
    background: rgba(0, 0, 0, 0.48);
  }
</style>
