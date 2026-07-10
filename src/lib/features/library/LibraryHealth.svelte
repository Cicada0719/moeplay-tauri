<script lang="ts">
  import type { LibraryHealthSnapshot } from "./contracts";

  let { health }: { health: LibraryHealthSnapshot } = $props();
  const coverage = $derived(`${Math.round(health.provenanceCoverage * 100)}%`);
</script>

<section class="library-health" data-state={health.state} aria-label="游戏库健康状态">
  <header>
    <div>
      <span class="eyebrow">Library health</span>
      <h2>{health.state === "healthy" ? "游戏库可靠" : health.state === "degraded" ? "游戏库需要修复" : "游戏库需要关注"}</h2>
    </div>
    <strong>{health.totalGames}</strong>
  </header>

  <div class="metrics">
    <div><span>缺失启动目标</span><b>{health.missingLaunchTargets}</b></div>
    <div><span>强身份重复组</span><b>{health.duplicateIdentityGroups}</b></div>
    <div><span>同名召回组</span><b>{health.titleRecallGroups}</b></div>
    <div><span>待决冲突</span><b>{health.unresolvedImportConflicts}</b></div>
    <div><span>来源覆盖</span><b>{coverage}</b></div>
  </div>

  {#if health.issues.length > 0}
    <ul>
      {#each health.issues as issue (`${issue.code}:${issue.gameIds.join(",")}`)}
        <li data-severity={issue.severity}>
          <span>{issue.severity}</span>
          <p>{issue.message}</p>
          {#if issue.gameIds.length}<code>{issue.gameIds.join(", ")}</code>{/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">未发现身份重复、启动缺失或待处理冲突。</p>
  {/if}
</section>

<style>
  .library-health { border-radius: 18px; border: 1px solid color-mix(in srgb, currentColor 14%, transparent); padding: 18px; background: color-mix(in srgb, Canvas 94%, #47d7ac 6%); }
  .library-health[data-state="degraded"] { background: color-mix(in srgb, Canvas 94%, #ff6b6b 6%); }
  header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  h2 { margin: 3px 0 0; font-size: 1.08rem; }
  header strong { font: 700 2rem/1 monospace; }
  .eyebrow { font: 600 .68rem/1 monospace; letter-spacing: .1em; text-transform: uppercase; opacity: .58; }
  .metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 8px; margin-top: 16px; }
  .metrics div { padding: 11px; border-radius: 12px; background: color-mix(in srgb, currentColor 5%, transparent); }
  .metrics span { display: block; font-size: .72rem; opacity: .62; }
  .metrics b { display: block; margin-top: 5px; font: 700 1.1rem/1 monospace; }
  ul { display: grid; gap: 8px; list-style: none; padding: 0; margin: 16px 0 0; }
  li { display: grid; grid-template-columns: auto 1fr; gap: 5px 10px; padding: 10px 12px; border-left: 3px solid #ff9f43; background: color-mix(in srgb, currentColor 4%, transparent); }
  li[data-severity="error"] { border-color: #ff6b6b; }
  li > span { font: 600 .65rem/1.4 monospace; text-transform: uppercase; opacity: .65; }
  li p { margin: 0; font-size: .82rem; }
  li code { grid-column: 2; font-size: .68rem; opacity: .55; overflow-wrap: anywhere; }
  .empty { margin: 16px 0 0; opacity: .62; font-size: .82rem; }
</style>
