<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { Chart as ChartJS } from "chart.js/auto";
  import type { ChartData, ChartOptions, ChartType } from "chart.js";

  let {
    type,
    data,
    options,
  }: {
    type: ChartType;
    data: ChartData;
    options?: ChartOptions;
  } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let chart = $state<ChartJS | null>(null);

  onMount(() => {
    if (!canvas) return;
    chart = new ChartJS(canvas, { type, data, options });
  });

  $effect(() => {
    if (!chart) return;
    chart.data = data;
    if (options) chart.options = options;
    chart.update("none");
  });

  onDestroy(() => {
    chart?.destroy();
    chart = null;
  });
</script>

<div class="chart-wrap">
  <canvas bind:this={canvas}></canvas>
</div>

<style>
  .chart-wrap {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 0;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
