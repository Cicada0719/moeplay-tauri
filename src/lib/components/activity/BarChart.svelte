<script lang="ts">
  import type { DashboardChartPoint } from "./dashboard-model";

  let {
    label,
    points = [],
    summary,
    orientation = "vertical",
    emptyMessage = "暂无趋势数据",
  }: {
    label: string;
    points?: DashboardChartPoint[];
    summary: string;
    orientation?: "vertical" | "horizontal";
    emptyMessage?: string;
  } = $props();

  const maxValue = $derived(Math.max(1, ...points.map((point) => point.value)));
</script>

<figure class="activity-chart" data-orientation={orientation} data-ui-pattern="chart">
  <figcaption>{label}</figcaption>
  <p class="activity-chart__summary">{summary}</p>
  {#if points.length > 0}
    <div class="activity-chart__plot" role="img" aria-label={`${label}。${summary}`}>
      {#each points as point (point.key)}
        <div class="activity-chart__item" title={`${point.label}：${point.valueLabel}`}>
          <span
            class="activity-chart__bar"
            style={orientation === "vertical"
              ? `--activity-chart-size:${Math.max(point.value > 0 ? 8 : 3, Math.round(point.value / maxValue * 100))}%`
              : `--activity-chart-size:${Math.max(point.value > 0 ? 6 : 2, Math.round(point.value / maxValue * 100))}%`}
          ></span>
          <small>{point.label}</small>
          {#if orientation === "horizontal"}<b>{point.valueLabel}</b>{/if}
        </div>
      {/each}
    </div>
  {:else}
    <p class="activity-chart__empty">{emptyMessage}</p>
  {/if}
</figure>

<style>
  .activity-chart {
    display: grid;
    gap: var(--v2-space-3);
    min-width: 0;
    margin: 0;
    padding: var(--v2-space-4);
    border: 1px solid var(--v2-color-border);
    border-radius: var(--v2-radius-lg);
    background: var(--v2-color-surface);
    color: var(--v2-color-text);
  }
  figcaption { font-size: var(--v2-text-md); font-weight: 800; }
  .activity-chart__summary { margin: 0; color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); line-height: var(--v2-leading-normal); }
  .activity-chart__plot { display: grid; gap: var(--v2-space-2); min-width: 0; }
  .activity-chart[data-orientation="vertical"] .activity-chart__plot { grid-template-columns: repeat(auto-fit, minmax(1.5rem, 1fr)); align-items: end; min-height: 10rem; }
  .activity-chart[data-orientation="vertical"] .activity-chart__item { display: grid; grid-template-rows: 8rem auto; align-items: end; justify-items: center; gap: var(--v2-space-1); min-width: 0; }
  .activity-chart[data-orientation="vertical"] .activity-chart__bar { width: min(1.25rem, 70%); height: var(--activity-chart-size); min-height: .2rem; border-radius: var(--v2-radius-sm) var(--v2-radius-sm) 0 0; background: linear-gradient(180deg, var(--v2-color-accent), color-mix(in srgb, var(--v2-color-accent) 45%, transparent)); }
  .activity-chart[data-orientation="vertical"] small { overflow: hidden; max-width: 100%; color: var(--v2-color-text-secondary); font-size: .65rem; text-overflow: ellipsis; white-space: nowrap; }
  .activity-chart[data-orientation="horizontal"] .activity-chart__item { display: grid; grid-template-columns: 3.5rem minmax(0, 1fr) auto; align-items: center; gap: var(--v2-space-2); }
  .activity-chart[data-orientation="horizontal"] .activity-chart__bar { grid-column: 2; width: var(--activity-chart-size); min-width: .25rem; height: .5rem; border-radius: 999px; background: var(--v2-color-accent); }
  .activity-chart[data-orientation="horizontal"] small { grid-column: 1; grid-row: 1; color: var(--v2-color-text-secondary); }
  .activity-chart[data-orientation="horizontal"] b { grid-column: 3; grid-row: 1; font-family: var(--v2-font-mono); font-size: var(--v2-text-xs); }
  .activity-chart__empty { margin: 0; padding-block: var(--v2-space-6); color: var(--v2-color-text-secondary); text-align: center; }
  @media (prefers-reduced-motion: reduce) { .activity-chart__bar { transition: none; } }
  :global([data-motion="reduce"]) .activity-chart__bar { transition: none; }
</style>
