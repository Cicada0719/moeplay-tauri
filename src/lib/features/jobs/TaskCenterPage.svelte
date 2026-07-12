<script lang="ts">
  import Card from "../../components/ui/Card.svelte";
  import Icon from "../../components/Icon.svelte";
  import JobsPanel from "./JobsPanel.svelte";
</script>

<section class="task-center-page" data-testid="task-center-page">
  <Card class="task-center-head" padding="md">
    <div class="head-copy">
      <span class="eyebrow">QUEUE</span>
      <h1>任务中心</h1>
      <p>查看下载、导入、刮削、验证、备份与更新。</p>
    </div>
    <div class="privacy-note" aria-label="隐私说明">
      <Icon name="check" size={16} />
      <span>仅展示脱敏摘要，不显示凭据。</span>
    </div>
  </Card>

  <div class="task-center-grid">
    <Card class="task-center-body" padding="md"><JobsPanel /></Card>
    <aside class="task-brief" aria-label="任务运行说明">
      <span class="brief-register">QUEUE / ORIGINS</span>
      <h2>任务来源</h2>
      <div class="brief-list">
        <div><b>01</b><strong>下载与更新</strong><small>资源与应用更新</small></div>
        <div><b>02</b><strong>导入与刮削</strong><small>库扫描与元数据</small></div>
        <div><b>03</b><strong>来源验证</strong><small>媒体来源连通性</small></div>
        <div><b>04</b><strong>备份与恢复</strong><small>存档与配置迁移</small></div>
      </div>
      <p>可恢复任务会在下次启动重新入队；错误信息保持脱敏。</p>
    </aside>
  </div>
</section>

<style>
  .task-center-page { min-width: 0; min-height: 100%; padding: clamp(16px,2.4vw,32px); overflow: auto; display: grid; align-content: start; gap: 16px; background: var(--bg-void, var(--bg-base)); }
  :global(.task-center-head) { min-width: 0; display: flex; align-items: center; justify-content: space-between; gap: 20px; }
  .head-copy { min-width: 0; }
  .eyebrow { display: block; margin-bottom: 7px; color: var(--accent); font: 700 10px/1 var(--font-mono); letter-spacing: .14em; text-transform: uppercase; }
  h1 { margin: 0; color: var(--text-primary); font: 750 clamp(24px, 3vw, 34px)/1.1 var(--font-display, var(--font-ui)); }
  p { margin: 8px 0 0; color: var(--text-muted); font-size: 13px; line-height: 1.6; }
  .privacy-note { max-width: 330px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; display: flex; align-items: center; gap: 8px; color: var(--text-secondary); background: var(--bg-elevated); font-size: 11px; line-height: 1.5; }
  .privacy-note :global(.icon) { color: var(--color-success, #4ade80); flex: 0 0 auto; }
  :global(.task-center-body) { min-width: 0; }
  @media (max-width: 760px) { .task-center-page { padding: 14px; } :global(.task-center-head) { align-items: flex-start; flex-direction: column; } .privacy-note { max-width: none; } }

  .task-center-grid { min-width:0; display:grid; grid-template-columns:minmax(0,1fr) minmax(240px,310px); gap:16px; align-items:start; }
  .task-brief { position:sticky; top:0; padding:18px; border:1px solid var(--border); background:linear-gradient(145deg,color-mix(in srgb,var(--bg-card) 92%,transparent),color-mix(in srgb,var(--accent) 5%,transparent)); }
  .brief-register { color:var(--accent); font:700 8px/1 var(--font-mono); letter-spacing:.16em; }
  .task-brief h2 { margin:14px 0 18px; font:720 clamp(1.35rem,2vw,2rem)/1 var(--font-display); letter-spacing:-.04em; }
  .brief-list { display:grid; border-top:1px solid var(--border); }
  .brief-list div { display:grid; grid-template-columns:30px 1fr; gap:5px 9px; padding:12px 0; border-bottom:1px solid var(--border); }
  .brief-list b { grid-row:1/3; color:var(--text-dim); font:700 8px/1.3 var(--font-mono); }
  .brief-list strong { color:var(--text-primary); font-size:12px; }
  .brief-list small { color:var(--text-muted); font-size:10px; line-height:1.45; }
  .task-brief>p { margin:16px 0 0; color:var(--text-muted); font-size:10px; line-height:1.65; }
  @media(max-width:900px){.task-center-grid{grid-template-columns:1fr}.task-brief{position:static}.brief-list{grid-template-columns:repeat(2,minmax(0,1fr))}.brief-list div:nth-child(odd){border-right:1px solid var(--border);padding-right:12px}.brief-list div:nth-child(even){padding-left:12px}}
  @media(max-width:560px){.brief-list{grid-template-columns:1fr}.brief-list div:nth-child(odd){border-right:0;padding-right:0}.brief-list div:nth-child(even){padding-left:0}}
</style>




