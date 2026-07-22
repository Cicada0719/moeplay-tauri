<script lang="ts">
  import type { PlaytimeSummary } from "../../api";
  import { AsyncSection, ContentGrid, MediaRow } from "../ui-v2";
  import BarChart from "./BarChart.svelte";
  import StatBlock from "./StatBlock.svelte";
  import type { DashboardChartPoint, DashboardMediaActivity, DashboardSession, DashboardTopGame } from "./dashboard-model";

  let { hasRecords, dailyPoints = [], dailySummary, monthlyPoints = [], monthlySummary, activityPoints = [], activitySummary, mediaCounts, topGames = [], mediaActivities = [], recentSessions = [], onOpenGame, onOpenActivity }: { hasRecords: boolean; dailyPoints?: DashboardChartPoint[]; dailySummary: string; monthlyPoints?: DashboardChartPoint[]; monthlySummary: string; activityPoints?: DashboardChartPoint[]; activitySummary: string; mediaCounts: { game: number; anime: number; comic: number; novel: number }; topGames?: DashboardTopGame[]; mediaActivities?: DashboardMediaActivity[]; recentSessions?: DashboardSession[]; onOpenGame: (id: string) => void; onOpenActivity: (item: DashboardMediaActivity) => void; } = $props();
</script>

<AsyncSection title="趋势与历史" description="图表同时提供文本摘要，列表统一使用可键盘激活的 MediaRow。" state={hasRecords ? "ready" : "empty"} class="legacy-insights">
  <ContentGrid label="记录趋势图表" minItemWidth="22rem" gap="md">
    <BarChart label="最近两周每日游玩时长" points={dailyPoints} summary={dailySummary} />
    <BarChart label="月度游玩节奏" points={monthlyPoints} summary={monthlySummary} orientation="horizontal" />
    <BarChart label="最近两周综合活跃度" points={activityPoints} summary={activitySummary} />
    <section class="media-mix" aria-labelledby="media-mix-title"><header><h3 id="media-mix-title">媒体记录分布</h3><p>按活动条数统计，不代表实际时长。</p></header><ContentGrid label="媒体记录计数" minItemWidth="8rem" gap="sm"><StatBlock label="游戏" value={mediaCounts.game} /><StatBlock label="番剧" value={mediaCounts.anime} /><StatBlock label="漫画" value={mediaCounts.comic} /><StatBlock label="小说" value={mediaCounts.novel} /></ContentGrid></section>
  </ContentGrid>

  <div class="legacy-lists"><ContentGrid label="记录明细" minItemWidth="24rem" gap="md">
    <section class="legacy-list" aria-labelledby="top-games-title"><header><h3 id="top-games-title">时长排行</h3><span>{topGames.length} 项</span></header><div role="list">{#each topGames as item, index (item.game_id)}
      {#snippet topMeta()}<span>第 {index + 1} 名 · {item.last_played ? new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit" }).format(new Date(item.last_played)) : "暂无最近时间"}</span>{/snippet}
      {#snippet topBadge()}<span class="rank">#{index + 1}</span>{/snippet}
      <MediaRow title={item.game_name} subtitle={`${item.sessions} 次会话`} description={item.total_seconds > 0 ? `${(item.total_seconds / 3600).toFixed(item.total_seconds >= 36000 ? 0 : 1)} 小时` : "暂无时长"} imageSrc={item.cover ?? undefined} ariaLabel={`打开 ${item.game_name} 详情`} onActivate={() => onOpenGame(item.game_id)} meta={topMeta} badge={topBadge} />
    {/each}</div></section>

    <section class="legacy-list" aria-labelledby="media-timeline-title"><header><h3 id="media-timeline-title">综合时间线</h3><span>{mediaActivities.length} 条</span></header><div role="list">{#each mediaActivities.slice(0, 10) as item (item.id)}
      {#snippet mediaMeta()}<span>{item.subtitle} · {item.timeLabel}</span>{/snippet}
      {#snippet mediaBadge()}<span class="kind">{item.kind === "game" ? "游戏" : item.kind === "anime" ? "番剧" : item.kind === "novel" ? "小说" : "漫画"}</span>{/snippet}
      <MediaRow title={item.title} imageSrc={item.imageSrc ?? undefined} ariaLabel={`打开 ${item.title} 记录`} onActivate={() => onOpenActivity(item)} meta={mediaMeta} badge={mediaBadge} />
    {/each}</div></section>

    <section class="legacy-list" aria-labelledby="sessions-title"><header><h3 id="sessions-title">最近游戏会话</h3><span>{recentSessions.length} 条</span></header><div role="list">{#each recentSessions as entry (`${entry.game_id}-${entry.session.id}`)}
      {#snippet sessionMeta()}<span>{entry.formattedTime} · {entry.formattedDuration}</span>{/snippet}
      <MediaRow title={entry.game_name} imageSrc={entry.imageSrc ?? undefined} ariaLabel={`打开 ${entry.game_name} 会话`} onActivate={() => onOpenGame(entry.game_id)} meta={sessionMeta} />
    {/each}</div></section>
  </ContentGrid></div>
</AsyncSection>

<style>
  :global(.legacy-insights) { padding: var(--v2-space-5); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-xl); background: var(--v2-color-surface); } .media-mix, .legacy-list { display: grid; gap: var(--v2-space-3); min-width: 0; padding: var(--v2-space-4); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-lg); background: var(--v2-color-surface); } .media-mix header p { margin: var(--v2-space-1) 0 0; color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); } .legacy-lists { margin-top: var(--v2-space-6); } .legacy-list > header { display: flex; align-items: baseline; justify-content: space-between; gap: var(--v2-space-3); } h3 { margin: 0; font-size: var(--v2-text-md); } .legacy-list > header span { color: var(--v2-color-text-secondary); font-family: var(--v2-font-mono); font-size: var(--v2-text-xs); } .legacy-list > div { display: grid; gap: var(--v2-space-2); } .rank, .kind { display: inline-flex; padding: .2rem .45rem; border-radius: 999px; background: color-mix(in srgb, var(--v2-color-accent) 18%, transparent); color: var(--v2-color-accent); font-size: .68rem; font-weight: 800; }
</style>



