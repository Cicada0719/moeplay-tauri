<script lang="ts">
  import { AsyncSection, ContentGrid, MediaRow } from "../ui-v2";
  import StatBlock from "./StatBlock.svelte";
  import type { DashboardMediaActivity, DashboardStat } from "./dashboard-model";

  let { loading = false, hasRecords = false, stats = [], continueItems = [], warning = null, onOpenActivity, onImport, onHome }: { loading?: boolean; hasRecords?: boolean; stats?: DashboardStat[]; continueItems?: DashboardMediaActivity[]; warning?: string | null; onOpenActivity: (item: DashboardMediaActivity) => void; onImport: () => void; onHome: () => void; } = $props();
  const state = $derived(loading ? "loading" : hasRecords ? "ready" : "empty");
</script>

<AsyncSection title="记录概览" description="Activity v2 不可用时仍保留旧版游戏时长、番剧历史和漫画进度聚合。" {state} loadingRows={4} primaryAction={!hasRecords && !loading ? { label: "导入游戏", onSelect: onImport } : undefined} secondaryAction={!hasRecords && !loading ? { label: "返回游戏库", onSelect: onHome } : undefined} class="legacy-overview">
  {#if warning}<p class="legacy-warning" role="status">{warning}</p>{/if}
  <ContentGrid label="记录统计" minItemWidth="11rem" gap="sm">
    {#each stats as stat (stat.id)}<StatBlock label={stat.label} value={stat.value} detail={stat.detail} tone={stat.tone} />{/each}
  </ContentGrid>
  {#if continueItems.length > 0}
    <section class="legacy-continue" aria-labelledby="legacy-continue-title"><header><h3 id="legacy-continue-title">最近继续</h3><span>{continueItems.length} 项</span></header><ContentGrid label="旧版最近继续" minItemWidth="18rem" gap="sm">
      {#each continueItems as item (item.id)}
        {#snippet meta()}<span>{item.subtitle} · {item.timeLabel}</span>{/snippet}
        {#snippet badge()}<span class="legacy-badge">{item.kind === "game" ? "游戏" : item.kind === "anime" ? "番剧" : item.kind === "novel" ? "小说" : "漫画"}</span>{/snippet}
        <MediaRow title={item.title} imageSrc={item.imageSrc ?? undefined} ariaLabel={`继续 ${item.title}`} onActivate={() => onOpenActivity(item)} meta={meta} badge={badge} focusKey={`legacy-continue-${item.id}`} />
      {/each}
    </ContentGrid></section>
  {/if}
</AsyncSection>

<style>
  :global(.legacy-overview) { padding: var(--v2-space-5); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-xl); background: var(--v2-color-surface); } .legacy-warning { margin: 0 0 var(--v2-space-4); padding: var(--v2-space-3); border: 1px solid color-mix(in srgb, #f0bf70 50%, var(--v2-color-border)); border-radius: var(--v2-radius-md); color: #f4d59d; background: color-mix(in srgb, #4a3515 45%, transparent); } .legacy-continue { display: grid; gap: var(--v2-space-3); margin-top: var(--v2-space-6); } .legacy-continue header { display: flex; justify-content: space-between; gap: var(--v2-space-3); } h3 { margin: 0; font-size: var(--v2-text-md); } header span { color: var(--v2-color-text-secondary); font-family: var(--v2-font-mono); font-size: var(--v2-text-xs); } .legacy-badge { display: inline-flex; padding: .2rem .45rem; border-radius: 999px; background: color-mix(in srgb, var(--v2-color-accent) 18%, transparent); color: var(--v2-color-accent); font-size: .68rem; font-weight: 800; }
</style>
