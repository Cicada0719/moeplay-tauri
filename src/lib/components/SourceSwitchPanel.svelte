<script lang="ts">
  // 萌游 MoeGame · 源切换最小接线面板（对应 spec §4 Step 10）
  //
  // 这是 `switchSource` 在播放页的**调用点接入**（自包含组件，可整体挂到播放页/
  // 播放器容器的任意位置）：
  // - 父组件传入 `rules`（rules_load_all 结果）与 `context`（当前条目/集数/进度）；
  // - 点击某个规则源 → 调用 `switchSource(ruleId, context)`（FR-02 竞态取消 + 状态保持）；
  // - 结果通过 `onResult` 回调 + `sourceSwitchState.lastResult` 双重透传给播放器容器。
  //
  // 跨任务边界：播放器内核（AnimePlayer.svelte 等）属任务 3 禁止修改清单，
  // 本任务交付此调用点组件 + store 契约；任务 3 在源点击处挂载并消费结果。
  import type { LoadedRule } from "../api/rules";
  import {
    sourceSwitchState,
    switchSource,
    type SwitchContext,
    type SwitchResult,
  } from "../stores/sourceSwitch";

  let {
    rules = [],
    context,
    onResult,
  }: {
    rules?: LoadedRule[];
    context: SwitchContext;
    /** 切换结果透传（ok/fallback/failed），播放器容器据此设源 + seek / 展示错误。 */
    onResult?: (payload: { ruleId: string; result: SwitchResult }) => void;
  } = $props();

  function onSelect(ruleId: string) {
    if ($sourceSwitchState.switching) return;
    void switchSource(ruleId, context).then((result) => {
      onResult?.({ ruleId, result });
    });
  }
</script>

<div class="source-switch-panel" data-testid="source-switch-panel">
  <div class="panel-header">
    <h4>切换播放源</h4>
    <span class="panel-sub">保持当前集数与进度</span>
  </div>

  <ul class="panel-list">
    {#each rules as rule (rule.id)}
      <li>
        <button
          type="button"
          class="panel-item"
          class:invalid={rule.status === "invalid"}
          disabled={rule.status === "invalid" || $sourceSwitchState.switching}
          title={rule.status === "invalid" ? rule.error?.message : rule.manifest.baseUrl}
          onclick={() => onSelect(rule.id)}
          data-testid="switch-source-item"
          data-rule-id={rule.id}
          data-status={rule.status}
        >
          <span class="panel-name">{rule.manifest.name}</span>
          {#if rule.status === "invalid"}
            <span class="badge">无效</span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>

  {#if $sourceSwitchState.switching}
    <div class="switching" data-testid="panel-switching" aria-live="polite">
      正在切换源…
    </div>
  {/if}
  {#if $sourceSwitchState.lastError}
    <div class="error" data-testid="panel-error" role="alert">
      {$sourceSwitchState.lastError}
    </div>
  {/if}
</div>

<style>
  .source-switch-panel {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    min-width: 16rem;
    font-size: 0.9rem;
  }
  .panel-header {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .panel-header h4 {
    margin: 0;
    font-size: 1rem;
  }
  .panel-sub {
    color: var(--text-muted, rgba(255, 255, 255, 0.55));
    font-size: 0.78rem;
  }
  .panel-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    max-height: 18rem;
    overflow-y: auto;
  }
  .panel-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 0.7rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 0.5rem;
    background: rgba(255, 255, 255, 0.03);
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .panel-item:disabled {
    cursor: not-allowed;
  }
  .panel-item.invalid {
    opacity: 0.5;
  }
  .panel-item:not(:disabled):hover {
    border-color: rgba(232, 85, 127, 0.5);
    background: rgba(232, 85, 127, 0.1);
  }
  .panel-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    font-size: 0.68rem;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: rgba(248, 113, 113, 0.18);
    color: #f87171;
    flex-shrink: 0;
  }
  .switching {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.45);
    border-radius: 0.5rem;
    color: #fff;
    font-size: 0.85rem;
    z-index: 2;
  }
  .error {
    padding: 0.5rem 0.7rem;
    border-radius: 0.5rem;
    background: rgba(248, 113, 113, 0.12);
    color: #f87171;
    font-size: 0.82rem;
  }
</style>
