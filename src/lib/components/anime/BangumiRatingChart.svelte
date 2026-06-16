<script lang="ts">
  let { count = [], total = 0 }: { count?: number[]; total?: number } = $props();
  
  const maxVal = $derived(Math.max(...count.slice(1), 1));
  const bars = $derived(Array.from({ length: 10 }, (_, i) => {
    const val = count[i + 1] || 0;
    return { score: i + 1, value: val, height: (val / maxVal) * 100 };
  }));
</script>

<div class="chart">
  <div class="chart-label">评分透视:</div>
  <div class="chart-body">
    {#each bars as bar}
      <div class="bar-col">
        <div class="bar" style="height: {bar.height}%"></div>
        <span class="bar-label">{bar.score}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .chart { display: flex; flex-direction: column; gap: 6px; }
  .chart-label { font-size: 11px; color: #9ca3af; }
  .chart-body {
    display: flex; gap: 4px; align-items: flex-end; height: 60px;
    padding: 4px 0;
  }
  .bar-col {
    flex: 1; display: flex; flex-direction: column; align-items: center; gap: 3px;
  }
  .bar {
    width: 100%; min-height: 2px;
    background: #d1d5db; border-radius: 2px 2px 0 0;
    transition: height 0.3s;
  }
  .bar-label { font-size: 10px; color: #6b7280; }
</style>
