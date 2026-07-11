<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import type { ConceptMediaAsset, MotionQuality } from "../contracts";
  import { createMediaStage, type MediaStage } from "../webgl";
  import MediaStageFallback from "./MediaStageFallback.svelte";

  export let assets: ConceptMediaAsset[] = [];
  export let activeIndex = 0;
  export let velocity = 0;
  export let quality: MotionQuality = "full";
  export let reducedMotion = false;
  export let alt = "动态媒体舞台";

  let canvas: HTMLCanvasElement;
  let stage: MediaStage | null = null;
  let failed = false;
  let mounted = false;

  $: imageAssets = assets.filter((asset) => asset.mediaType === "image");
  $: if (mounted && stage && !failed) stage.setActive(activeIndex, velocity);

  onMount(() => {
    mounted = true;
    if (reducedMotion || quality === "reduced") {
      failed = true;
      return;
    }

    stage = createMediaStage({
      quality,
      reducedMotion,
      onContextLost: () => (failed = true),
      onContextRestored: () => (failed = false),
    });
    stage.mount(canvas, imageAssets)
      .then(() => stage?.setActive(activeIndex, velocity))
      .catch(() => {
        failed = true;
        stage?.dispose();
        stage = null;
      });
  });

  onDestroy(() => {
    mounted = false;
    stage?.dispose();
    stage = null;
  });
</script>

<div class="webgl-media-stage" data-testid="webgl-media-stage" data-fallback={String(failed)} aria-label={alt}>
  {#if failed || imageAssets.length === 0}
    <MediaStageFallback assets={imageAssets} {activeIndex} {velocity} quality={reducedMotion ? "reduced" : quality} {alt} />
  {:else}
    <canvas bind:this={canvas} aria-hidden="true"></canvas>
  {/if}
</div>

<style>
  .webgl-media-stage,
  canvas {
    width: 100%;
    height: 100%;
    display: block;
  }

  .webgl-media-stage {
    position: relative;
    overflow: hidden;
    background: #090909;
  }
</style>
