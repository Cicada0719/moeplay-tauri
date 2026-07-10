<script lang="ts">
  import type { ContinueCandidate } from "./contracts";
  let { candidates = [], loading = false, error = null, onSelect }: { candidates?: ContinueCandidate[]; loading?: boolean; error?: string | null; onSelect?: (candidate: ContinueCandidate) => void } = $props();
  const format = (seconds: number | null) => seconds === null ? null : `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
</script>

<section class="continue-rail" aria-label="Continue" aria-busy={loading}>
  <header><h2>Continue</h2><span>{candidates.length} items</span></header>
  {#if loading}<p class="state">Loading continue candidates…</p>
  {:else if error}<p class="state error" role="alert">{error}</p>
  {:else if candidates.length === 0}<p class="state">Nothing to continue yet. Save progress in a game, anime, or comic.</p>
  {:else}<div class="cards">{#each candidates as candidate (candidate.resourceKind + candidate.resourceId + (candidate.providerId ?? ""))}<button class="card" type="button" onclick={() => onSelect?.(candidate)}>
    {#if candidate.artworkUrl}<img src={candidate.artworkUrl} alt="" />{:else}<span class="artwork" aria-hidden="true">{candidate.resourceKind.slice(0, 1).toUpperCase()}</span>{/if}
    <span class="copy"><strong>{candidate.title}</strong><small>{candidate.resourceKind} · {candidate.durationQuality.replace("_", " ")}</small>{#if format(candidate.exactDurationSeconds) || format(candidate.estimatedDurationSeconds)}<small>{format(candidate.exactDurationSeconds) ?? format(candidate.estimatedDurationSeconds)}</small>{/if}</span>
  </button>{/each}</div>{/if}
</section>
<style>
  .continue-rail { display:grid; gap:.75rem; } header { display:flex; justify-content:space-between; align-items:baseline; } h2 { margin:0; font-size:1.05rem; } header span,small,.state { color:var(--v2-color-text-secondary,#a7a7ba); } .cards { display:grid; grid-auto-flow:column; grid-auto-columns:minmax(13rem,18rem); overflow-x:auto; gap:.75rem; padding-bottom:.25rem; } .card { display:flex; gap:.65rem; text-align:left; padding:.6rem; color:inherit; border:1px solid var(--v2-color-border,#3b3b46); border-radius:.7rem; background:var(--v2-color-surface,#1e1e27); } img,.artwork { flex:0 0 3rem; width:3rem; height:4rem; object-fit:cover; border-radius:.4rem; background:#454560; display:grid; place-items:center; } .copy { display:grid; gap:.2rem; min-width:0; } strong { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; } .error { color:var(--v2-color-danger,#f08080); }
</style>
