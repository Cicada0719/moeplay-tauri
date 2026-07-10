<script lang="ts">
  import type { ImportAction, ImportCandidate, ImportDecision } from "./contracts";

  let {
    candidate,
    selectedAction = candidate.action,
    selectedTarget = candidate.targetGameId,
    onDecision,
  }: {
    candidate: ImportCandidate;
    selectedAction?: ImportAction;
    selectedTarget?: string | null;
    onDecision?: (decision: ImportDecision) => void;
  } = $props();

  const changedFields = $derived(candidate.fieldDiff.filter((diff) => diff.disposition !== "unchanged"));
  const mergeTargets = $derived(
    Array.from(new Map(candidate.matches.map((match) => [match.gameId, match.gameTitle])).entries()),
  );

  function decide(action: ImportAction, targetGameId = selectedTarget) {
    onDecision?.({
      candidateId: candidate.id,
      action,
      targetGameId: action === "merge" || action === "update" ? targetGameId : null,
    });
  }

  function show(value: unknown): string {
    if (value === null || value === undefined || value === "") return "—";
    if (typeof value === "string") return value;
    try { return JSON.stringify(value); } catch { return String(value); }
  }
</script>

<article class="import-diff" data-action={selectedAction}>
  <header>
    <div>
      <span class="source">{candidate.source}</span>
      <h3>{candidate.record.title}</h3>
      <p>{candidate.reason.message}</p>
    </div>
    <span class:conflict={selectedAction === "conflict"} class="action">{selectedAction}</span>
  </header>

  {#if candidate.reason.recalledGameIds.length > 0}
    <div class="recall" role="note">
      同名召回：{candidate.reason.recalledGameIds.length} 项。仅供人工判断，不会自动合并。
    </div>
  {/if}

  {#if onDecision}
    <div class="decision-bar" aria-label="导入决策">
      {#each ["create", "update", "merge", "ignore"] as action}
        <button
          type="button"
          class:active={selectedAction === action}
          onclick={() => decide(action as ImportAction)}
        >{action}</button>
      {/each}
      {#if selectedAction === "merge" || selectedAction === "update"}
        <select
          aria-label="目标游戏"
          value={selectedTarget ?? ""}
          onchange={(event) => decide(selectedAction, event.currentTarget.value || null)}
        >
          <option value="">选择目标</option>
          {#each mergeTargets as [gameId, title]}
            <option value={gameId}>{title} · {gameId}</option>
          {/each}
        </select>
      {/if}
    </div>
  {/if}

  <div class="identity-grid">
    <span>启动路径</span><code>{candidate.identity.launchPath ?? "—"}</code>
    <span>平台 ID</span><code>{candidate.identity.platformId ? `${candidate.identity.platformId.source}:${candidate.identity.platformId.id}` : "—"}</code>
    <span>标题指纹</span><code>{candidate.identity.titleFingerprint || "—"}</code>
  </div>

  <div class="table-wrap">
    <table>
      <thead><tr><th>字段</th><th>当前</th><th>导入</th><th>策略</th></tr></thead>
      <tbody>
        {#each changedFields as diff (diff.field)}
          <tr class:preserved={!diff.willApply}>
            <th>{diff.field}</th>
            <td>{show(diff.current)}</td>
            <td>{show(diff.incoming)}</td>
            <td><span class="policy">{diff.disposition}</span></td>
          </tr>
        {:else}
          <tr><td colspan="4" class="empty">没有字段变化</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
</article>

<style>
  .import-diff { border: 1px solid color-mix(in srgb, currentColor 16%, transparent); border-radius: 16px; padding: 16px; background: color-mix(in srgb, Canvas 94%, currentColor 6%); }
  header { display: flex; justify-content: space-between; gap: 18px; align-items: flex-start; }
  h3 { margin: 3px 0; font-size: 1rem; }
  p { margin: 0; opacity: .7; font-size: .86rem; }
  .source { font: 600 .68rem/1.2 monospace; letter-spacing: .08em; text-transform: uppercase; opacity: .6; }
  .action, .policy { border-radius: 999px; padding: 4px 8px; font: 600 .68rem/1 monospace; background: color-mix(in srgb, #47d7ac 18%, transparent); }
  .action.conflict { background: color-mix(in srgb, #ff9f43 24%, transparent); }
  .recall { margin-top: 12px; padding: 9px 11px; border-left: 3px solid #ff9f43; background: color-mix(in srgb, #ff9f43 9%, transparent); font-size: .82rem; }
  .decision-bar { display: flex; flex-wrap: wrap; gap: 7px; margin-top: 12px; }
  button, select { border: 1px solid color-mix(in srgb, currentColor 18%, transparent); border-radius: 9px; background: transparent; color: inherit; padding: 7px 10px; }
  button.active { border-color: #47d7ac; background: color-mix(in srgb, #47d7ac 14%, transparent); }
  .identity-grid { display: grid; grid-template-columns: minmax(92px, auto) 1fr; gap: 6px 12px; margin: 14px 0; font-size: .78rem; }
  .identity-grid span { opacity: .6; }
  code { overflow-wrap: anywhere; }
  .table-wrap { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; font-size: .78rem; }
  th, td { padding: 9px 8px; text-align: left; border-top: 1px solid color-mix(in srgb, currentColor 10%, transparent); vertical-align: top; }
  tr.preserved td { opacity: .65; }
  .empty { text-align: center; opacity: .55; }
</style>
