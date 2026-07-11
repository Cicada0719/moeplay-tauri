<script lang="ts">
  import type { ConceptMediaAsset, MotionQuality } from "../contracts";

  let {
    assets = [],
    activeIndex = 0,
    velocity = 0,
    quality = "reduced",
    alt = "",
    class: className = "",
  }: {
    assets?: ConceptMediaAsset[];
    activeIndex?: number;
    velocity?: number;
    quality?: MotionQuality;
    alt?: string;
    class?: string;
  } = $props();

  const imageAssets = $derived(assets.filter((asset) => asset.mediaType === "image"));
  const safeIndex = $derived(imageAssets.length === 0
    ? -1
    : Math.max(0, Math.min(imageAssets.length - 1, Math.trunc(activeIndex))));
  const active = $derived(safeIndex < 0 ? undefined : imageAssets[safeIndex]);
  const focalPosition = $derived(active
    ? `${active.focalPoint.x * 100}% ${active.focalPoint.y * 100}%`
    : "50% 50%");
  const shift = $derived(quality === "reduced" ? 0 : Math.max(-1, Math.min(1, velocity)) * 1.25);
</script>

<div
  class={`media-stage-fallback ${className}`}
  data-quality={quality}
  aria-label={alt || active?.contentId || "Media preview"}
  role="img"
>
  {#if active}
    {#key active.id}
      <img
        src={active.src}
        alt=""
        draggable="false"
        style:object-position={focalPosition}
        style:transform={`scale(1.015) translate3d(${shift}%, 0, 0)`}
      />
    {/key}
  {:else}
    <div class="media-stage-fallback__empty" aria-hidden="true"></div>
  {/if}
</div>

<style>
  .media-stage-fallback {
    position: relative;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    isolation: isolate;
    background: #111;
  }

  img,
  .media-stage-fallback__empty {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  img {
    display: block;
    object-fit: cover;
    user-select: none;
    pointer-events: none;
    will-change: transform, opacity;
    animation: media-stage-fallback-in 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  .media-stage-fallback__empty {
    background: linear-gradient(135deg, #171717, #080808);
  }

  [data-quality="reduced"] img {
    animation: none;
    will-change: auto;
  }

  @keyframes media-stage-fallback-in {
    from { opacity: 0; transform: scale(1.035); }
    to { opacity: 1; }
  }

  @media (prefers-reduced-motion: reduce) {
    img {
      animation: none;
      transform: none !important;
      will-change: auto;
    }
  }
</style>
