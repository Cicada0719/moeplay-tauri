<script lang="ts">
  import type { ActivityEventType, ActivityFilters as FilterState, ActivityResourceKind } from "./contracts";
  let { filters, loading = false, onChange, onClear }: { filters: FilterState; loading?: boolean; onChange: (filters: FilterState) => void; onClear?: () => void } = $props();
  const kinds: Array<ActivityResourceKind | ""> = ["", "game", "anime", "comic"];
  const eventTypes: Array<ActivityEventType | ""> = ["", "started", "progressed", "completed", "rated", "favorited", "imported", "failed"];
  function update<K extends keyof FilterState>(key: K, value: FilterState[K] | "") { onChange({ ...filters, [key]: value || null }); }
</script>

<section class="activity-filters" aria-label="Activity filters" aria-busy={loading}>
  <label>Kind <select value={filters.resourceKind ?? ""} onchange={(event) => update("resourceKind", event.currentTarget.value as ActivityResourceKind | "")}>{#each kinds as kind}<option value={kind}>{kind || "All kinds"}</option>{/each}</select></label>
  <label>Event <select value={filters.eventType ?? ""} onchange={(event) => update("eventType", event.currentTarget.value as ActivityEventType | "")}>{#each eventTypes as type}<option value={type}>{type || "All events"}</option>{/each}</select></label>
  <label>From <input type="datetime-local" value={filters.startedAtFrom ?? ""} onchange={(event) => update("startedAtFrom", event.currentTarget.value || "")} /></label>
  <label>To <input type="datetime-local" value={filters.startedAtTo ?? ""} onchange={(event) => update("startedAtTo", event.currentTarget.value || "")} /></label>
  {#if onClear}<button type="button" onclick={onClear} disabled={loading}>Clear</button>{/if}
</section>

<style>
  .activity-filters { display:flex; flex-wrap:wrap; gap:.75rem; align-items:end; padding:.75rem; border:1px solid var(--v2-color-border, #31313c); border-radius:.75rem; background:var(--v2-color-surface, #1d1d25); }
  label { display:grid; gap:.3rem; color:var(--v2-color-text-secondary, #b8b8c8); font-size:.8rem; }
  select,input,button { min-height:2rem; border-radius:.4rem; border:1px solid var(--v2-color-border, #454552); background:var(--v2-color-surface-subtle, #242430); color:inherit; padding:.25rem .5rem; }
</style>
