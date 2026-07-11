<script lang="ts">
  import Card from "../../components/ui/Card.svelte";
  import Icon from "../../components/Icon.svelte";
  import JobsPanel from "./JobsPanel.svelte";
</script>

<section class="task-center-page" data-testid="task-center-page">
  <Card class="task-center-head" padding="md">
    <div class="head-copy">
      <span class="eyebrow">Background Control Plane</span>
      <h1>任务中心</h1>
      <p>统一查看下载、导入、刮削、来源验证、AI、备份与更新任务。</p>
    </div>
    <div class="privacy-note" aria-label="任务中心隐私说明">
      <Icon name="check" size={16} />
      <span>仅展示脱敏摘要，不显示凭据、请求头或 AI 原文。</span>
    </div>
  </Card>

  <Card class="task-center-body" padding="md">
    <JobsPanel />
  </Card>
</section>

<style>
  .task-center-page { min-width: 0; height: 100%; padding: 24px; overflow: auto; display: grid; align-content: start; gap: 16px; background: var(--bg-void, var(--bg-base)); }
  :global(.task-center-head) { min-width: 0; display: flex; align-items: center; justify-content: space-between; gap: 20px; }
  .head-copy { min-width: 0; }
  .eyebrow { display: block; margin-bottom: 7px; color: var(--accent); font: 700 10px/1 var(--font-mono); letter-spacing: .14em; text-transform: uppercase; }
  h1 { margin: 0; color: var(--text-primary); font: 750 clamp(24px, 3vw, 34px)/1.1 var(--font-display, var(--font-ui)); }
  p { margin: 8px 0 0; color: var(--text-muted); font-size: 13px; line-height: 1.6; }
  .privacy-note { max-width: 330px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; display: flex; align-items: center; gap: 8px; color: var(--text-secondary); background: var(--bg-elevated); font-size: 11px; line-height: 1.5; }
  .privacy-note :global(.icon) { color: var(--color-success, #4ade80); flex: 0 0 auto; }
  :global(.task-center-body) { min-width: 0; }
  @media (max-width: 760px) { .task-center-page { padding: 14px; } :global(.task-center-head) { align-items: flex-start; flex-direction: column; } .privacy-note { max-width: none; } }
</style>

