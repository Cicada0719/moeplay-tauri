<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { LoadedRule } from "../api/rules";
  import { importRule, removeCustomRule } from "../api/rules";
  import { sourceSwitchState } from "../stores/sourceSwitch";

  let {
    rules = [],
    activeRuleId = null,
  }: {
    rules?: LoadedRule[];
    activeRuleId?: string | null;
  } = $props();

  const dispatch = createEventDispatcher<{
    select: { ruleId: string };
    import: { rule: LoadedRule };
    remove: { ruleId: string };
  }>();

  let importError: string | null = $state(null);
  let busy = $state(false);

  function onSelect(ruleId: string) {
    if ($sourceSwitchState.switching) return;
    dispatch("select", { ruleId });
  }

  async function onImport() {
    importError = null;
    busy = true;
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "规则文件", extensions: ["json", "yaml", "yml"] }],
      });
      if (!selected) return;
      const rule = await importRule(selected as string);
      dispatch("import", { rule });
    } catch (e) {
      const err = e as { message?: string; line?: number | null };
      importError = err?.line
        ? `${err.message}（第 ${err.line} 行）`
        : String(err?.message ?? e);
    } finally {
      busy = false;
    }
  }

  async function onRemove(ruleId: string) {
    try {
      await removeCustomRule(ruleId);
      dispatch("remove", { ruleId });
    } catch (e) {
      importError = String(e);
    }
  }
</script>

<div class="source-list" data-testid="source-list">
  <div class="source-list__header">
    <h3>播放源</h3>
    <button
      type="button"
      class="import-btn"
      onclick={onImport}
      disabled={busy}
      data-testid="import-btn"
    >
      {busy ? "导入中…" : "导入规则"}
    </button>
  </div>

  {#if importError}
    <div class="import-error" data-testid="import-error" role="alert">{importError}</div>
  {/if}

  <ul class="source-list__items">
    {#each rules as rule (rule.id)}
      <li class="source-row">
        <button
          type="button"
          class="source-item"
          class:active={rule.id === activeRuleId}
          class:invalid={rule.status === "invalid"}
          disabled={rule.status === "invalid" || $sourceSwitchState.switching}
          title={rule.status === "invalid" ? rule.error?.message : rule.manifest.baseUrl}
          onclick={() => onSelect(rule.id)}
          data-testid="source-item"
          data-status={rule.status}
          data-origin={rule.origin}
        >
          <span class="source-name">{rule.manifest.name}</span>
          {#if rule.origin === "custom"}
            <span class="badge" data-testid="custom-badge">自定义</span>
          {/if}
          {#if rule.status === "invalid"}
            <span class="invalid-mark">无效</span>
          {/if}
        </button>
        {#if rule.origin === "custom"}
          <button
            type="button"
            class="remove-btn"
            aria-label="删除 {rule.manifest.name}"
            onclick={() => onRemove(rule.id)}
            data-testid="remove-btn"
          >
            删除
          </button>
        {/if}
      </li>
    {/each}
  </ul>

  {#if $sourceSwitchState.switching}
    <div class="switching-overlay" data-testid="switching-overlay" aria-live="polite">
      正在切换源…
    </div>
  {/if}
</div>

<style>
  .source-list {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 18rem;
  }
  .source-list__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .source-list__header h3 {
    margin: 0;
    font-size: 1rem;
  }
  .import-btn {
    padding: 0.35rem 0.75rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 0.5rem;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .import-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .import-error {
    padding: 0.5rem 0.75rem;
    border-radius: 0.5rem;
    background: rgba(248, 113, 113, 0.12);
    color: #f87171;
    font-size: 0.85rem;
  }
  .source-list__items {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }
  .source-row {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }
  .source-item {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 0.5rem;
    background: rgba(255, 255, 255, 0.03);
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .source-item.active {
    border-color: rgba(232, 85, 127, 0.55);
    background: rgba(232, 85, 127, 0.1);
  }
  .source-item.invalid {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .source-item:disabled {
    cursor: not-allowed;
  }
  .source-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    font-size: 0.7rem;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: rgba(96, 165, 250, 0.2);
    color: #60a5fa;
    flex-shrink: 0;
  }
  .invalid-mark {
    font-size: 0.7rem;
    color: #f87171;
    flex-shrink: 0;
  }
  .remove-btn {
    padding: 0.35rem 0.5rem;
    border: none;
    border-radius: 0.4rem;
    background: rgba(248, 113, 113, 0.15);
    color: #f87171;
    font-size: 0.75rem;
    cursor: pointer;
    flex-shrink: 0;
  }
  .switching-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.45);
    border-radius: 0.5rem;
    color: #fff;
    font-size: 0.9rem;
    z-index: 2;
  }
</style>
