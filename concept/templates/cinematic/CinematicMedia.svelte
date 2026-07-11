<script lang="ts">
  import type { ConceptMediaAsset } from "../../contracts";

  export let asset: ConceptMediaAsset | undefined;
  export let alt = "";
  export let eager = false;
  export let decorative = false;

  $: position = asset
    ? `${Math.round(asset.focalPoint.x * 100)}% ${Math.round(asset.focalPoint.y * 100)}%`
    : "50% 50%";
</script>

<div class:cinematic-media--empty={!asset} class="cinematic-media">
  {#if asset?.mediaType === "video"}
    <video
      src={asset.src}
      poster={asset.placeholder}
      muted
      loop
      playsinline
      preload={eager ? "metadata" : "none"}
      aria-label={decorative ? undefined : alt}
      aria-hidden={decorative ? "true" : undefined}
      style:object-position={position}
    ></video>
  {:else if asset}
    <img
      src={asset.src}
      alt={decorative ? "" : alt}
      aria-hidden={decorative ? "true" : undefined}
      loading={eager ? "eager" : "lazy"}
      decoding="async"
      style:object-position={position}
    />
  {:else}
    <span class="cinematic-media__empty" aria-hidden="true">NO FRAME</span>
  {/if}
</div>
