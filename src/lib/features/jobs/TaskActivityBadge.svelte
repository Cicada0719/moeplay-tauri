<script lang="ts">
  export let activeCount = 0;
  export let failedCount = 0;
  export let label = "后台任务状态";

  $: active = Math.max(0, Math.trunc(activeCount));
  $: failed = Math.max(0, Math.trunc(failedCount));
  $: summary = `进行中 ${active}，失败 ${failed}`;
</script>

<span
  class:has-active={active > 0}
  class:has-failed={failed > 0}
  class="task-activity-badge"
  role="status"
  aria-label={`${label}：${summary}`}
  aria-live="polite"
  aria-atomic="true"
  data-testid="task-activity-badge"
>
  <span class="task-activity-badge__item" aria-hidden="true">进行中 <b>{active}</b></span>
  <span class="task-activity-badge__separator" aria-hidden="true">·</span>
  <span class="task-activity-badge__item" aria-hidden="true">失败 <b>{failed}</b></span>
</span>

<style>
  .task-activity-badge { display: inline-flex; min-height: 1.75rem; align-items: center; gap: .35rem; padding: .2rem .55rem; border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted); background: var(--bg-elevated); font: 700 11px/1 var(--font-mono, monospace); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .task-activity-badge.has-active { border-color: color-mix(in srgb, var(--accent) 48%, var(--border)); color: var(--text-secondary); }
  .task-activity-badge.has-failed { border-color: color-mix(in srgb, var(--color-error, #f87171) 52%, var(--border)); color: var(--color-error, #f87171); }
  .task-activity-badge__item { display: inline-flex; gap: .25rem; }
  .task-activity-badge b { color: var(--text-primary); }
  .task-activity-badge.has-failed .task-activity-badge__item:last-child b { color: currentColor; }
  .task-activity-badge__separator { color: var(--text-muted); }
</style>
