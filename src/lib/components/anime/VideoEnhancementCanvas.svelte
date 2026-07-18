<script lang="ts">
  import { LocalVideoEnhancer, type VideoEnhancementMode, type VideoEnhancementStatus } from "../../features/anime-player/localVideoEnhancement";

  let {
    video,
    mode,
    onStatus = () => {},
  }: {
    video: HTMLVideoElement | null;
    mode: VideoEnhancementMode;
    onStatus?: (status: VideoEnhancementStatus, message?: string) => void;
  } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let localStatus = $state<VideoEnhancementStatus>("off");

  function reportStatus(status: VideoEnhancementStatus, message?: string) {
    localStatus = status;
    onStatus(status, message);
  }

  $effect(() => {
    const target = video;
    const output = canvas;
    const selectedMode = mode;
    if (!target || !output || selectedMode === "off") {
      reportStatus("off");
      return;
    }
    const enhancer = new LocalVideoEnhancer(output, target, selectedMode, reportStatus);
    enhancer.start();
    return () => enhancer.destroy();
  });
</script>

{#if mode !== "off"}
  <canvas bind:this={canvas} class="enhancement-canvas" class:ready={localStatus === "ready"} aria-hidden="true"></canvas>
{/if}

<style>
  .enhancement-canvas {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    pointer-events: none;
    opacity: 0;
    background: #000;
    transition: opacity 120ms ease;
  }
  .enhancement-canvas.ready {
    opacity: 1;
  }
</style>
