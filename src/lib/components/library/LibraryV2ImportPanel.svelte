<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import {
    ImportDiff,
    createLibraryFeatureStore,
    tauriLibraryApi,
    type ApplyImportResponse,
    type ImportAction,
    type ImportCandidate,
    type ImportDecision,
    type LibraryApi,
    type LibraryFeatureState,
    type PreviewImportRequest,
  } from "../../features/library";
  import Icon from "../Icon.svelte";
  import { Button } from "../ui";

  let {
    request,
    title = "Library v2 导入预览",
    api = tauriLibraryApi,
    onApplied,
    onClose,
  }: {
    request: PreviewImportRequest | null;
    title?: string;
    api?: LibraryApi;
    onApplied?: (result: ApplyImportResponse) => void | Promise<void>;
    onClose?: () => void;
  } = $props();

  const feature = createLibraryFeatureStore(untrack(() => api));
  let snapshot = $state<LibraryFeatureState>(feature.getSnapshot());
  let decisions = $state<Record<string, ImportDecision>>({});
  let decisionPreviewId = $state("");
  let requestSignature = $state("");
  let applyKey = $state("");

  const preview = $derived(snapshot.preview);
  const candidates = $derived(preview?.candidates ?? []);
  const decisionList = $derived(candidates.map((candidate) => decisions[candidate.id] ?? defaultDecision(candidate)));
  const summary = $derived.by(() => {
    const counts: Record<ImportAction, number> = { create: 0, update: 0, merge: 0, conflict: 0, ignore: 0 };
    for (const decision of decisionList) counts[decision.action] += 1;
    return counts;
  });
  const unresolved = $derived(
    decisionList.filter((decision) =>
      decision.action === "conflict"
      || ((decision.action === "update" || decision.action === "merge") && !decision.targetGameId),
    ).length,
  );
  const plannedWrites = $derived(summary.create + summary.update + summary.merge);
  const applySummary = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const result of snapshot.applyResult?.results ?? []) {
      counts.set(result.status, (counts.get(result.status) ?? 0) + 1);
    }
    return [...counts.entries()];
  });

  const unsubscribe = feature.subscribe((next) => {
    snapshot = next;
    if (next.preview && next.preview.previewId !== decisionPreviewId) {
      decisionPreviewId = next.preview.previewId;
      decisions = Object.fromEntries(next.preview.candidates.map((candidate) => [candidate.id, defaultDecision(candidate)]));
      applyKey = makeApplyKey(next.preview.previewId);
    }
  });

  $effect(() => {
    const signature = request ? JSON.stringify(request) : "";
    if (!request || signature === requestSignature) return;
    requestSignature = signature;
    decisionPreviewId = "";
    decisions = {};
    feature.clear();
    void feature.preview(request);
  });

  onDestroy(() => {
    feature.cancelAll();
    unsubscribe();
  });

  function defaultDecision(candidate: ImportCandidate): ImportDecision {
    return {
      candidateId: candidate.id,
      action: candidate.action,
      targetGameId: candidate.targetGameId,
    };
  }

  function makeApplyKey(previewId: string): string {
    const suffix = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    return `${previewId}:${suffix}`;
  }

  function updateDecision(decision: ImportDecision) {
    decisions = { ...decisions, [decision.candidateId]: decision };
  }

  async function applyPreview() {
    if (!preview || unresolved > 0 || snapshot.isApplying) return;
    await feature.apply(decisionList, applyKey);
    const result = feature.getSnapshot().applyResult;
    if (result) await onApplied?.(result);
  }

  function retryPreview() {
    if (!request || snapshot.isPreviewing) return;
    feature.clear();
    decisionPreviewId = "";
    decisions = {};
    void feature.preview(request);
  }

  function closePanel() {
    feature.cancelAll();
    onClose?.();
  }

  function statusLabel(status: string): string {
    const labels: Record<string, string> = {
      created: "新增",
      updated: "更新",
      merged: "合并",
      no_changes: "无变化",
      ignored: "忽略",
      conflict: "冲突",
      failed: "失败",
      already_applied: "已应用",
    };
    return labels[status] ?? status;
  }
</script>

<section class="v2-panel" aria-labelledby="library-v2-preview-title" aria-busy={snapshot.isPreviewing || snapshot.isApplying}>
  <header class="panel-head">
    <div>
      <span class="eyebrow">Safe Import</span>
      <h2 id="library-v2-preview-title">{title}</h2>
      <p>先计算身份匹配与字段 provenance，再由你确认真正写入。</p>
    </div>
    <Button variant="quiet" size="sm" press={closePanel} ariaLabel="关闭 Library v2 预览">
      <Icon name="x" size={16} />关闭
    </Button>
  </header>

  <div class="zero-write" class:verified={preview?.writeCount === 0} role="status" aria-live="polite">
    <Icon name="check" size={16} />
    <span>
      <strong>预览阶段零写入</strong>
      {#if preview}
        后端写入计数：{preview.writeCount}
      {:else}
        正在等待 diff
      {/if}
    </span>
  </div>

  {#if snapshot.error}
    <div class="state-box error" role="alert">
      <div><strong>Library v2 操作失败</strong><span>{snapshot.error}</span></div>
      <Button variant="secondary" size="sm" press={retryPreview}>重试预览</Button>
    </div>
  {/if}

  {#if snapshot.isPreviewing && !preview}
    <div class="state-box loading" role="status">
      <span class="spinner" aria-hidden="true"></span>
      <div><strong>正在生成导入 diff</strong><span>比对平台 ID、启动路径、同名召回与字段来源。</span></div>
      <Button variant="secondary" size="sm" press={() => feature.cancelPreview()}>取消</Button>
    </div>
  {:else if preview && candidates.length === 0}
    <div class="state-box empty">
      <Icon name="database" size={20} />
      <div><strong>没有可预览候选</strong><span>重新扫描平台或选择至少一个目录候选。</span></div>
    </div>
  {:else if preview}
    <div class="summary" aria-label="导入 diff 汇总">
      <div data-kind="create"><span>新增</span><b>{summary.create}</b></div>
      <div data-kind="update"><span>更新</span><b>{summary.update + summary.merge}</b></div>
      <div data-kind="conflict"><span>冲突</span><b>{summary.conflict}</b></div>
      <div data-kind="ignore"><span>忽略</span><b>{summary.ignore}</b></div>
    </div>

    <div class="diff-list">
      {#each candidates as candidate (candidate.id)}
        {@const selected = decisions[candidate.id] ?? defaultDecision(candidate)}
        <ImportDiff
          {candidate}
          selectedAction={selected.action}
          selectedTarget={selected.targetGameId}
          onDecision={updateDecision}
        />
      {/each}
    </div>

    <footer class="apply-bar">
      <div class="apply-copy" aria-live="polite">
        <strong>{plannedWrites} 项预计写入</strong>
        {#if unresolved > 0}
          <span class="warning">仍有 {unresolved} 项冲突或缺少目标，请先选择 create / merge / ignore。</span>
        {:else}
          <span>已解析所有决策；忽略项不会写入。</span>
        {/if}
      </div>
      {#if snapshot.isApplying}
        <Button variant="secondary" press={() => feature.cancelApply()}>取消应用</Button>
      {/if}
      <Button press={applyPreview} loading={snapshot.isApplying} disabled={unresolved > 0 || plannedWrites === 0 || !!snapshot.applyResult}>
        <Icon name="download" size={16} />确认应用
      </Button>
    </footer>
  {/if}

  {#if snapshot.applyResult}
    <div class="apply-result" role="status" aria-live="polite">
      <div class="result-head">
        <div>
          <span class="eyebrow">Apply Result</span>
          <h3>应用完成</h3>
        </div>
        <span>{snapshot.applyResult.replayed ? "幂等重放" : "首次执行"}</span>
      </div>
      <div class="result-tags">
        {#each applySummary as [status, count]}
          <span>{statusLabel(status)} {count}</span>
        {/each}
        <span>provenance 变更 {snapshot.applyResult.provenanceChanges.length}</span>
      </div>
      <ul>
        {#each snapshot.applyResult.results as result (result.itemIdempotencyKey)}
          <li data-status={result.status}>
            <strong>{statusLabel(result.status)}</strong>
            <span>{result.message}</span>
            {#if result.appliedFields.length}<code>写入：{result.appliedFields.join(", ")}</code>{/if}
            {#if result.preservedFields.length}<code>保留：{result.preservedFields.join(", ")}</code>{/if}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</section>

<style>
  .v2-panel {
    position: relative;
    z-index: 2;
    display: grid;
    gap: 14px;
    margin: 14px 28px 0;
    padding: 18px;
    border: 1px solid color-mix(in srgb, var(--accent) 36%, var(--border));
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-panel) 94%, var(--accent) 6%);
    box-shadow: 0 18px 54px rgba(0, 0, 0, .22);
  }
  .panel-head, .apply-bar, .result-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  h2, h3 { margin: 0; color: var(--text-primary); }
  h2 { font-size: 19px; }
  h3 { font-size: 16px; }
  .panel-head p { margin: 5px 0 0; color: var(--text-muted); font-size: 12px; }
  .eyebrow { display: block; margin-bottom: 5px; color: var(--accent); font: 650 10px/1 var(--font-mono); text-transform: uppercase; }
  .zero-write {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-elev) 88%, transparent);
    font-size: 12px;
  }
  .zero-write.verified { color: var(--color-success); border-color: color-mix(in srgb, var(--color-success) 35%, var(--border)); }
  .zero-write span { display: flex; gap: 8px; flex-wrap: wrap; }
  .state-box {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 72px;
    padding: 13px;
    border: 1px dashed var(--border-hover);
    border-radius: 12px;
    color: var(--text-secondary);
  }
  .state-box > div { min-width: 0; display: grid; gap: 4px; }
  .state-box span { color: var(--text-muted); font-size: 12px; overflow-wrap: anywhere; }
  .state-box :global(.ui-button) { margin-left: auto; }
  .state-box.error { border-style: solid; border-color: color-mix(in srgb, var(--color-error) 45%, var(--border)); }
  .spinner { width: 18px; height: 18px; flex: 0 0 auto; border: 2px solid var(--border-hover); border-top-color: var(--accent); border-radius: 50%; animation: spin .8s linear infinite; }
  .summary { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
  .summary div { padding: 11px 12px; border: 1px solid var(--border); border-radius: 10px; background: var(--bg-elev); }
  .summary span { display: block; color: var(--text-muted); font-size: 11px; }
  .summary b { display: block; margin-top: 5px; font: 750 18px/1 var(--font-mono); }
  .summary [data-kind="conflict"] b { color: #fbbf24; }
  .summary [data-kind="create"] b { color: var(--color-success); }
  .diff-list { display: grid; gap: 10px; max-height: 48vh; overflow: auto; padding-right: 3px; overscroll-behavior: contain; }
  .apply-bar { align-items: center; padding-top: 2px; }
  .apply-copy { min-width: 0; display: grid; gap: 3px; }
  .apply-copy span { color: var(--text-muted); font-size: 11px; }
  .apply-copy .warning { color: #fbbf24; }
  .apply-result { display: grid; gap: 12px; padding: 14px; border: 1px solid color-mix(in srgb, var(--color-success) 34%, var(--border)); border-radius: 12px; background: color-mix(in srgb, var(--color-success) 7%, transparent); }
  .result-head > span { color: var(--text-muted); font: 600 11px/1 var(--font-mono); }
  .result-tags { display: flex; flex-wrap: wrap; gap: 7px; }
  .result-tags span { padding: 5px 8px; border-radius: 999px; background: var(--bg-elev); color: var(--text-secondary); font-size: 11px; }
  ul { display: grid; gap: 7px; margin: 0; padding: 0; list-style: none; }
  li { display: grid; grid-template-columns: auto 1fr; gap: 3px 9px; padding: 9px 10px; border-left: 3px solid var(--accent); background: color-mix(in srgb, var(--bg-elev) 82%, transparent); }
  li[data-status="failed"], li[data-status="conflict"] { border-left-color: var(--color-error); }
  li strong { font-size: 11px; }
  li span { color: var(--text-secondary); font-size: 11px; }
  li code { grid-column: 2; color: var(--text-muted); font-size: 10px; overflow-wrap: anywhere; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 760px) {
    .v2-panel { margin-inline: 14px; padding: 14px; }
    .panel-head, .apply-bar { align-items: stretch; flex-direction: column; }
    .summary { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>
