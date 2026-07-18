<script lang="ts">
  import Card from "../../components/ui/Card.svelte";
  import Icon from "../../components/Icon.svelte";
  import SourceCenterPanel from "./SourceCenterPanel.svelte";
  import { createSourceCenterStore, type SourceCenterStore } from ".";
  import {
    CAPABILITY_LABELS,
    SOURCE_ADAPTER_MANIFESTS,
    SOURCE_ECOSYSTEM_LABELS,
  } from "../../sources/sourceRegistry";
  export let store: SourceCenterStore = createSourceCenterStore();
  const novelSources = SOURCE_ADAPTER_MANIFESTS.filter((source) => source.mediaType === "novel" && source.lifecycle === "active");
  const gameMetadataSources = SOURCE_ADAPTER_MANIFESTS.filter(
    (source) => source.mediaType === "game" && source.id !== "local-game-library",
  );

  const lifecycleLabel = {
    active: "可接入",
    planned: "待配置",
    reference: "资料参考",
  } as const;
</script>

<section class="source-center-page" data-testid="source-center-page">
  <Card class="head" padding="md">
    <div><span class="eyebrow">Provider Control Plane</span><h1>来源中心</h1><p>统一查看动画、漫画、小说和外部运行时来源，并显示自动选择所依据的健康度。</p></div>
    <div class="safety" aria-label="来源中心安全说明"><Icon name="check" size={16} /><span>扩展目录只读取元数据，不下载或执行第三方扩展代码。</span></div>
  </Card>
  <Card class="novel-sources" padding="md">
    <div class="novel-source-title"><span class="eyebrow">OPEN TEXT SOURCES</span><h2>内置小说来源</h2><p>只接入公版或自由许可目录；下载能力仅对上游明确提供的合法文件开放。</p></div>
    <div class="novel-source-grid">
      {#each novelSources as source (source.id)}
        <article><Icon name="book" size={20} /><div><strong>{source.name}</strong><span>{source.capabilities.map((capability) => capability === "text" ? "正文" : capability === "download" ? "下载" : capability === "search" ? "搜索" : capability === "detail" ? "详情" : capability === "chapters" ? "目录" : "元数据").join(" · ")}</span><small>{source.note}</small></div><b>已内置</b></article>
      {/each}
    </div>
  </Card>
  <Card class="game-sources" padding="md">
    <div class="novel-source-title">
      <span class="eyebrow">VISUAL NOVEL METADATA</span>
      <h2>视觉小说与游戏资料源</h2>
      <p>Windows 与 Android 共用同一份元数据目录；这里只提供检索、详情和正版商店入口，不包含受版权保护内容的下载。</p>
    </div>
    <div class="game-source-grid">
      {#each gameMetadataSources as source (source.id)}
        <article>
          <Icon name="gamepad" size={20} />
          <div>
            <strong>{source.name}</strong>
            <span>{SOURCE_ECOSYSTEM_LABELS[source.ecosystem]} · {source.capabilities.map((capability) => CAPABILITY_LABELS[capability]).join(" · ")}</span>
            <small>{source.note}</small>
          </div>
          <b class:planned={source.lifecycle === "planned"} class:reference={source.lifecycle === "reference"}>{lifecycleLabel[source.lifecycle]}</b>
        </article>
      {/each}
    </div>
  </Card>
  <Card padding="md"><SourceCenterPanel {store} /></Card>
</section>

<style>
  .source-center-page { min-width: 0; height: 100%; padding: 24px; overflow: auto; display: grid; align-content: start; gap: 16px; background: var(--bg-void, var(--bg-base)); } :global(.head) { display: flex; justify-content: space-between; align-items: center; gap: 20px; } .eyebrow { display: block; margin-bottom: 7px; color: var(--accent); font: 700 10px/1 var(--font-mono); letter-spacing: .14em; text-transform: uppercase; } h1, h2 { margin: 0; color: var(--text-primary); font: 750 clamp(24px, 3vw, 34px)/1.1 var(--font-display, var(--font-ui)); } h2 { font-size: 20px; } p { margin: 8px 0 0; color: var(--text-muted); font-size: 13px; line-height: 1.6; } .safety { max-width: 330px; display: flex; gap: 8px; align-items: center; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; color: var(--text-secondary); background: var(--bg-elevated); font-size: 11px; line-height: 1.5; } .safety :global(.icon) { color: var(--color-success, #4ade80); flex: 0 0 auto; } :global(.novel-sources), :global(.game-sources) { display: grid; gap: 14px; } .novel-source-grid, .game-source-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; } .novel-source-grid article, .game-source-grid article { min-width: 0; display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: start; gap: 10px; padding: 13px; border: 1px solid var(--border); background: var(--bg-elevated); } .novel-source-grid article > :global(.icon), .game-source-grid article > :global(.icon) { color: var(--accent); } .novel-source-grid article div, .game-source-grid article div { min-width: 0; display: grid; gap: 5px; } .novel-source-grid strong, .game-source-grid strong { color: var(--text-primary); font-size: 13px; } .novel-source-grid span, .novel-source-grid small, .game-source-grid span, .game-source-grid small { color: var(--text-muted); font-size: 10px; line-height: 1.5; } .novel-source-grid b, .game-source-grid b { padding: 4px 6px; border: 1px solid var(--color-success, #4ade80); color: var(--color-success, #4ade80); font: 700 9px/1 var(--font-mono); white-space: nowrap; } .game-source-grid b.planned { border-color: var(--accent); color: var(--accent); } .game-source-grid b.reference { border-color: var(--text-muted); color: var(--text-muted); } @media (max-width: 760px) { .source-center-page { padding: 14px; } :global(.head) { align-items: flex-start; flex-direction: column; } .safety { max-width: none; } .novel-source-grid, .game-source-grid { grid-template-columns: 1fr; } }
</style>
