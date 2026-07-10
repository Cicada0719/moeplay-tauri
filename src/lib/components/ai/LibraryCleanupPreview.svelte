<script lang="ts">
  import { onMount } from "svelte";
  import Button from "../ui/Button.svelte";
  import Icon from "../Icon.svelte";
  import { GenerationGuard, getAiClient, isAbortError, isAiUnavailableError, type AiClient } from "../../features/ai";
  import type { AiChangeSetPreview } from "../../features/ai/change-set";
  import type { NormalizedChangeSetPreview } from "../../features/ai/types";
  import { operationTarget, operationTitle, validateChangeSetPreview } from "../../features/ai/validation";

  let {
    client = getAiClient(),
    initialPreview = null,
    autoLoad = false,
  }: {
    client?: AiClient;
    initialPreview?: AiChangeSetPreview | unknown | null;
    autoLoad?: boolean;
  } = $props();

  let preview = $state<NormalizedChangeSetPreview | null>(null);
  let phase = $state<"idle" | "loading" | "ready" | "invalid" | "offline" | "error" | "cancelled" | "applied" | "undoing" | "undone">("idle");
  let errorMessage = $state("");
  let selectedIndexes = $state<number[]>([]);
  let confirmed = $state(false);
  let undoToken = $state<string | null>(null);
  const guard = new GenerationGuard();

  const selectedCount = $derived(selectedIndexes.length);
  const canApply = $derived(phase === "ready" && preview !== null && selectedCount > 0 && confirmed);

  function acceptPreview(raw: unknown) {
    const validation = validateChangeSetPreview(raw);
    selectedIndexes = [];
    confirmed = false;
    undoToken = null;
    if (!validation.ok) {
      preview = null;
      phase = "invalid";
      errorMessage = validation.errors.join(" ");
      return;
    }
    preview = validation.value;
    phase = "ready";
    errorMessage = "";
  }

  async function loadPreview() {
    const request = guard.begin();
    phase = "loading";
    preview = null;
    selectedIndexes = [];
    confirmed = false;
    errorMessage = "";
    try {
      const result = await client.previewLibraryCleanup({ scope: "game_library", limit: 100, generation: request.generation }, request.signal);
      if (!guard.isCurrent(request.generation)) return;
      acceptPreview(result);
    } catch (error) {
      if (!guard.isCurrent(request.generation) || isAbortError(error)) return;
      phase = isAiUnavailableError(error) ? "offline" : "error";
      errorMessage = phase === "offline" ? "AI 整理建议当前不可用。不会对资料库做任何修改。" : "整理建议生成失败。没有变更被应用。";
    }
  }

  function cancelLoad() {
    guard.cancel();
    phase = "cancelled";
    errorMessage = "已取消生成；迟到结果不会进入确认流程。";
  }

  function toggleOperation(index: number, checked: boolean) {
    selectedIndexes = checked
      ? Array.from(new Set([...selectedIndexes, index])).sort((a, b) => a - b)
      : selectedIndexes.filter((entry) => entry !== index);
    confirmed = false;
  }

  async function applySelected() {
    if (!canApply || !preview) return;
    phase = "loading";
    errorMessage = "";
    try {
      const result = await client.applyChangeSet({
        changeSetId: preview.id,
        selectedOperationIndexes: selectedIndexes,
        confirmed: true,
      });
      undoToken = result.undoToken;
      phase = "applied";
      confirmed = false;
    } catch {
      phase = "error";
      errorMessage = "应用变更失败。后端必须保持原子性；请刷新后确认当前资料库状态。";
    }
  }

  async function undo() {
    if (!undoToken) return;
    phase = "undoing";
    try {
      await client.undoChangeSet(undoToken);
      undoToken = null;
      selectedIndexes = [];
      phase = "undone";
    } catch {
      phase = "error";
      errorMessage = "撤销失败，请保留 undo token 并重试。";
    }
  }

  onMount(() => {
    if (initialPreview !== null) acceptPreview(initialPreview);
    else if (autoLoad) void loadPreview();
    return () => guard.cancel();
  });
</script>

<section class="cleanup" aria-labelledby="cleanup-title">
  <div class="cleanup-head">
    <div>
      <p class="eyebrow">CONFIRMABLE CHANGE SET</p>
      <h2 id="cleanup-title">资料库整理预览</h2>
      <p>所有操作默认不选中。只有逐项选择并明确确认后，后端才能应用变更。</p>
    </div>
    {#if phase === "loading"}
      <Button variant="ghost" size="sm" press={cancelLoad}>取消生成</Button>
    {:else if phase !== "applied" && phase !== "undoing"}
      <Button variant="secondary" size="sm" press={loadPreview}>生成整理建议</Button>
    {/if}
  </div>

  {#if phase === "loading"}
    <div class="state-banner"><span class="spinner"></span>正在生成并验证 change set…</div>
  {:else if ["invalid", "offline", "error", "cancelled"].includes(phase)}
    <div class="state-banner" class:error={phase === "invalid" || phase === "error"} class:offline={phase === "offline"} role="alert">
      <Icon name={phase === "offline" ? "info" : phase === "cancelled" ? "square" : "x"} size={15} />
      <span>{errorMessage}</span>
    </div>
  {:else if phase === "undone"}
    <div class="state-banner success"><Icon name="check" size={15} /><span>已撤销刚才应用的整理操作。</span></div>
  {/if}

  {#if preview}
    <div class="preview-summary">
      <div><strong>{preview.summary}</strong><span>change set {preview.id}</span></div>
      <div class="confidence"><span>置信度</span><strong>{Math.round(preview.confidence * 100)}%</strong></div>
    </div>

    <div class="operation-list" aria-label="整理操作列表">
      {#each preview.operations as entry, index (entry.id)}
        <label class:selected={selectedIndexes.includes(index)} class="operation-row">
          <input
            type="checkbox"
            aria-label={`选择操作 ${index + 1}`}
            checked={selectedIndexes.includes(index)}
            disabled={phase !== "ready"}
            onchange={(event) => toggleOperation(index, event.currentTarget.checked)}
          />
          <span class="operation-index">{String(index + 1).padStart(2, "0")}</span>
          <span class="operation-copy">
            <strong>{operationTitle(entry.operation)}</strong>
            <small>{operationTarget(entry.operation)}</small>
            <p>{entry.operation.reason}</p>
          </span>
        </label>
      {:else}
        <p class="empty-copy">当前没有可预览的整理操作。</p>
      {/each}
    </div>

    {#if phase === "ready" && preview.operations.length > 0}
      <div class="confirmation-box">
        <label>
          <input type="checkbox" bind:checked={confirmed} disabled={selectedCount === 0} aria-label="确认应用所选操作" />
          <span><strong>我已检查所选操作</strong><small>确认后仅应用 {selectedCount} 条已选择操作；未选中的操作不会执行。</small></span>
        </label>
        <Button variant="primary" size="sm" disabled={!canApply} press={applySelected}>确认并应用 {selectedCount} 条</Button>
      </div>
    {:else if phase === "applied"}
      <div class="applied-box">
        <div><Icon name="check" size={16} /><span><strong>变更已应用</strong><small>已应用 {selectedCount} 条操作，可立即撤销。</small></span></div>
        <Button variant="secondary" size="sm" disabled={!undoToken} press={undo}>撤销操作</Button>
      </div>
    {:else if phase === "undoing"}
      <div class="state-banner"><span class="spinner"></span>正在撤销变更…</div>
    {/if}
  {/if}
</section>

<style>
  .cleanup { display: grid; gap: 14px; }
  .cleanup-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .eyebrow { margin: 0 0 5px; color: var(--accent); font-family: var(--font-mono); font-size: 10px; font-weight: 700; letter-spacing: .13em; }
  h2, p { margin: 0; }
  h2 { color: var(--text-primary); font-size: 1.12rem; }
  .cleanup-head p:not(.eyebrow) { max-width: 70ch; margin-top: 5px; color: var(--text-secondary); font-size: 12px; line-height: 1.5; }
  .state-banner { min-height: 42px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; display: flex; align-items: center; gap: 9px; background: var(--bg-elev); color: var(--text-secondary); font-size: 11px; }
  .state-banner.error { border-color: color-mix(in srgb, var(--color-error, #f87171) 35%, var(--border)); color: var(--color-error, #f87171); }
  .state-banner.offline { border-color: color-mix(in srgb, var(--color-warning, #fbbf24) 35%, var(--border)); color: var(--color-warning, #fbbf24); }
  .state-banner.success { border-color: color-mix(in srgb, var(--color-success, #4ade80) 35%, var(--border)); color: var(--color-success, #4ade80); }
  .spinner { width: 14px; height: 14px; border: 2px solid currentColor; border-right-color: transparent; border-radius: 50%; animation: spin .8s linear infinite; }
  .preview-summary { padding: 13px 14px; border: 1px solid var(--border); border-radius: 8px; display: flex; justify-content: space-between; gap: 16px; background: var(--bg-deep); }
  .preview-summary > div:first-child { min-width: 0; display: grid; gap: 4px; }
  .preview-summary strong { color: var(--text-primary); font-size: 12px; }
  .preview-summary span { color: var(--text-muted); font-family: var(--font-mono); font-size: 9px; }
  .confidence { flex-shrink: 0; display: grid; justify-items: end; gap: 2px; }
  .confidence strong { font-family: var(--font-mono); font-size: 1rem; }
  .operation-list { border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
  .operation-row { min-height: 84px; padding: 12px; display: grid; grid-template-columns: auto auto 1fr; align-items: start; gap: 10px; background: var(--bg-card); cursor: pointer; }
  .operation-row + .operation-row { border-top: 1px solid var(--border); }
  .operation-row.selected { background: color-mix(in srgb, var(--accent) 6%, var(--bg-card)); }
  .operation-row > input, .confirmation-box input { margin-top: 3px; accent-color: var(--accent); }
  .operation-index { color: var(--text-muted); font-family: var(--font-mono); font-size: 10px; }
  .operation-copy { min-width: 0; display: grid; gap: 3px; }
  .operation-copy strong { color: var(--text-primary); font-size: 12px; }
  .operation-copy small { color: var(--accent); font-family: var(--font-mono); font-size: 9px; overflow-wrap: anywhere; }
  .operation-copy p { margin-top: 2px; color: var(--text-secondary); font-size: 10px; line-height: 1.5; }
  .confirmation-box, .applied-box { padding: 12px; border: 1px solid var(--border); border-radius: 8px; display: flex; align-items: center; justify-content: space-between; gap: 14px; background: var(--bg-card); }
  .confirmation-box > label, .applied-box > div { display: flex; align-items: flex-start; gap: 9px; }
  .confirmation-box span, .applied-box span { display: grid; gap: 3px; }
  .confirmation-box strong, .applied-box strong { color: var(--text-primary); font-size: 11px; }
  .confirmation-box small, .applied-box small { color: var(--text-muted); font-size: 9px; line-height: 1.4; }
  .applied-box > div { align-items: center; color: var(--color-success, #4ade80); }
  .empty-copy { padding: 20px; color: var(--text-muted); font-size: 11px; text-align: center; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 600px) { .cleanup-head, .confirmation-box, .applied-box { align-items: stretch; flex-direction: column; } }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>
