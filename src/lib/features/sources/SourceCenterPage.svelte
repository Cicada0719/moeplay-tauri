<script lang="ts">
  import Card from "../../components/ui/Card.svelte";
  import Icon from "../../components/Icon.svelte";
  import SourceCenterPanel from "./SourceCenterPanel.svelte";
  import { createSourceCenterStore, type SourceCenterStore } from ".";
  export let store: SourceCenterStore = createSourceCenterStore();
</script>

<section class="source-center-page" data-testid="source-center-page">
  <Card class="head" padding="md">
    <div><span class="eyebrow">Provider Control Plane</span><h1>来源中心</h1><p>统一管理动画、漫画和外部运行时来源，并显示自动选择所依据的健康度。</p></div>
    <div class="safety" aria-label="来源中心安全说明"><Icon name="check" size={16} /><span>扩展目录只读取元数据，不下载或执行第三方扩展代码。</span></div>
  </Card>
  <Card padding="md"><SourceCenterPanel {store} /></Card>
</section>

<style>
  .source-center-page { min-width: 0; height: 100%; padding: 24px; overflow: auto; display: grid; align-content: start; gap: 16px; background: var(--bg-void, var(--bg-base)); } :global(.head) { display: flex; justify-content: space-between; align-items: center; gap: 20px; } .eyebrow { display: block; margin-bottom: 7px; color: var(--accent); font: 700 10px/1 var(--font-mono); letter-spacing: .14em; text-transform: uppercase; } h1 { margin: 0; color: var(--text-primary); font: 750 clamp(24px, 3vw, 34px)/1.1 var(--font-display, var(--font-ui)); } p { margin: 8px 0 0; color: var(--text-muted); font-size: 13px; line-height: 1.6; } .safety { max-width: 330px; display: flex; gap: 8px; align-items: center; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; color: var(--text-secondary); background: var(--bg-elevated); font-size: 11px; line-height: 1.5; } .safety :global(.icon) { color: var(--color-success, #4ade80); flex: 0 0 auto; } @media (max-width: 760px) { .source-center-page { padding: 14px; } :global(.head) { align-items: flex-start; flex-direction: column; } .safety { max-width: none; } }
</style>
