<script lang="ts">
  import Icon from "../Icon.svelte";

  let {
    localSignals = [],
    aiExplanation = null,
    aiState = "idle",
  }: {
    localSignals?: string[];
    aiExplanation?: string | null;
    aiState?: "idle" | "loading" | "ready" | "offline" | "error" | "cancelled";
  } = $props();
</script>

<div class="explanation" aria-label="推荐依据">
  <section class="signal-block local">
    <div class="block-head"><Icon name="layers" size={14} /><strong>本地依据</strong><span>规则引擎</span></div>
    <ul>
      {#each localSignals as signal}<li>{signal}</li>{:else}<li>暂无可展示的本地信号</li>{/each}
    </ul>
  </section>
  <section class="signal-block ai">
    <div class="block-head"><Icon name="lightbulb" size={14} /><strong>AI 补充解释</strong><span>可选增益</span></div>
    {#if aiState === "loading"}
      <p>正在生成解释，不影响本地推荐结果。</p>
    {:else if aiState === "offline"}
      <p>AI 当前离线；此推荐完全来自本地信号。</p>
    {:else if aiState === "error"}
      <p>AI 解释校验失败，已丢弃；本地推荐仍有效。</p>
    {:else if aiState === "cancelled"}
      <p>AI 解释已取消，迟到结果不会显示。</p>
    {:else if aiExplanation}
      <p>{aiExplanation}</p>
    {:else}
      <p>AI 未参与此条推荐。</p>
    {/if}
  </section>
</div>

<style>
  .explanation { display: grid; gap: 8px; }
  .signal-block { padding: 10px; border: 1px solid var(--border); border-radius: 7px; display: grid; gap: 7px; background: var(--bg-deep); }
  .block-head { display: flex; align-items: center; gap: 6px; color: var(--text-secondary); }
  .block-head strong { color: var(--text-primary); font-size: 10px; }
  .block-head span { margin-left: auto; color: var(--text-muted); font-family: var(--font-mono); font-size: 8px; letter-spacing: .06em; text-transform: uppercase; }
  ul { margin: 0; padding-left: 16px; display: grid; gap: 4px; }
  li, p { margin: 0; color: var(--text-secondary); font-size: 10px; line-height: 1.45; }
  .local { border-left: 2px solid var(--accent); }
  .ai { border-left: 2px solid var(--border-hover, var(--border)); }
</style>
