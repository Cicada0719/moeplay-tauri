<script lang="ts">
  let {
    label,
    value,
    detail,
    tone = "default",
    class: className = "",
  }: {
    label: string;
    value: string | number;
    detail?: string;
    tone?: "default" | "accent" | "success" | "warning";
    class?: string;
  } = $props();
</script>

<article class="activity-stat {className}" data-tone={tone} data-ui-pattern="stat-block">
  <span class="activity-stat__label">{label}</span>
  <strong>{value}</strong>
  {#if detail}<small>{detail}</small>{/if}
</article>

<style>
  .activity-stat {
    display: grid;
    align-content: start;
    gap: var(--v2-space-2);
    min-width: 0;
    min-height: 7.5rem;
    padding: var(--v2-space-4);
    border: 1px solid var(--v2-color-border);
    border-radius: var(--v2-radius-lg);
    background: linear-gradient(145deg, var(--v2-color-surface), var(--v2-color-surface-subtle));
    color: var(--v2-color-text);
    font-family: var(--v2-font-sans);
  }
  .activity-stat[data-tone="accent"] { border-color: color-mix(in srgb, var(--v2-color-accent) 52%, var(--v2-color-border)); }
  .activity-stat[data-tone="success"] { border-color: color-mix(in srgb, #53d69b 48%, var(--v2-color-border)); }
  .activity-stat[data-tone="warning"] { border-color: color-mix(in srgb, #f0bf70 48%, var(--v2-color-border)); }
  .activity-stat__label { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); font-weight: 700; letter-spacing: .04em; }
  strong { overflow-wrap: anywhere; font-family: var(--v2-font-mono); font-size: clamp(1.35rem, 2.4vw, 2rem); line-height: var(--v2-leading-tight); }
  small { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); line-height: var(--v2-leading-normal); }
  :global([data-density="couch"]) .activity-stat { min-height: 10rem; padding: var(--v2-space-6); }
  :global([data-density="couch"]) .activity-stat__label,
  :global([data-density="couch"]) .activity-stat small { font-size: .875rem; }
  :global([data-density="couch"]) .activity-stat strong { font-size: 2.25rem; }
</style>
