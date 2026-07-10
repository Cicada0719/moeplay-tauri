<script lang="ts">
  import type { ActivityEventView } from "./contracts";
  let { events = [], loading = false, loadingMore = false, error = null, hasMore = false, onLoadMore, onEdit, onDelete }: { events?: ActivityEventView[]; loading?: boolean; loadingMore?: boolean; error?: string | null; hasMore?: boolean; onLoadMore?: () => void; onEdit?: (event: ActivityEventView) => void; onDelete?: (event: ActivityEventView) => void } = $props();
  const eventLabel = (event: ActivityEventView) => `${event.resourceKind} ${event.eventType}`;
</script>
<section class="activity-timeline" aria-label="Activity timeline" aria-busy={loading || loadingMore}>
  <header><h2>Timeline</h2><span>{events.length} events</span></header>
  {#if loading}<p class="state">Loading activity…</p>
  {:else if error}<p class="state error" role="alert">{error}</p>
  {:else if events.length === 0}<p class="state">No activity matches these filters.</p>
  {:else}<ol>{#each events as event (event.id)}<li><time datetime={event.startedAt}>{event.startedAt}</time><div><strong>{eventLabel(event)}</strong><small>{event.durationQuality.replace("_", " ")}{#if event.durationSeconds !== null} · {event.durationSeconds}s{/if}</small></div><div class="actions">{#if onEdit}<button type="button" onclick={() => onEdit?.(event)}>Edit</button>{/if}{#if onDelete}<button type="button" onclick={() => onDelete?.(event)}>Delete</button>{/if}</div></li>{/each}</ol>{#if hasMore}<button class="more" type="button" onclick={onLoadMore} disabled={loadingMore}>{loadingMore ? "Loading…" : "Load more"}</button>{/if}{/if}
</section>
<style>
  .activity-timeline { display:grid; gap:.75rem; } header { display:flex; justify-content:space-between; align-items:baseline; } h2 { margin:0; font-size:1.05rem; } header span,small,.state,time { color:var(--v2-color-text-secondary,#a7a7ba); } ol { margin:0; padding:0; display:grid; list-style:none; border:1px solid var(--v2-color-border,#3b3b46); border-radius:.7rem; overflow:hidden; } li { display:grid; grid-template-columns:minmax(10rem,1fr) minmax(10rem,2fr) auto; gap:.75rem; align-items:center; padding:.75rem; border-bottom:1px solid var(--v2-color-border,#3b3b46); } li:last-child { border-bottom:0; } li div { display:grid; gap:.2rem; } button { min-height:2rem; border:1px solid var(--v2-color-border,#555); border-radius:.35rem; background:var(--v2-color-surface,#22222c); color:inherit; padding:.2rem .5rem; } .actions { display:flex; gap:.4rem; } .more { justify-self:center; } .error { color:var(--v2-color-danger,#f08080); } @media(max-width:42rem){li{grid-template-columns:1fr}.actions{justify-content:flex-start}}
</style>
