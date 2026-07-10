<script lang="ts">
  import { onMount } from "svelte";
  import { animeStore, type AnimeHistory } from "../stores/anime.svelte";
  import { comicStore, type ReadRecord } from "../stores/comic.svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { formatPlayTime, getPlaytimeSummary, type Game, type PlaySessionEntry, type PlaytimeSummary } from "../api";
  import { createActivityStore, tauriActivityApi, type ActivityEventPatch, type ActivityEventView, type ActivityFilters, type ContinueCandidate } from "../features/activity";
  import { backfillLegacyGameActivityOnce, shouldFallbackActivityV2 } from "./activity/backfill";
  import { splitActivityDurations } from "./activity/metrics";
  import ActivityEditorDialog from "./activity/ActivityEditorDialog.svelte";
  import ActivityV2Section from "./activity/ActivityV2Section.svelte";
  import LegacyInsightsSection from "./activity/LegacyInsightsSection.svelte";
  import LegacyOverviewSection from "./activity/LegacyOverviewSection.svelte";
  import type { DashboardMediaActivity, DashboardStat } from "./activity/dashboard-model";
  import {
    activityChartPoints,
    buildLocalSummary,
    buildMediaActivities,
    countMediaKinds,
    countRecentActivities,
    dailyChartPoints,
    fillActivityBars,
    fillDailyBars,
    formatCompactSeconds,
    monthlyChartPoints,
    summarizeChart,
    toDashboardSessions,
    toDashboardTopGames,
  } from "./activity/dashboard-data";
  import { PageHeader, PageShell } from "./ui-v2";

  const activityStore = createActivityStore(tauriActivityApi);
  let summary = $state<PlaytimeSummary | null>(null);
  let loading = $state(true);
  let summaryWarning = $state<string | null>(null);
  let activityV2State = $state(activityStore.getSnapshot());
  let activityV2Loaded = $state(false);
  let activityV2Unavailable = $state(false);
  let activityV2LoadError = $state<string | null>(null);
  let activityExportPath = $state("activity-export.json");
  let activityExportFormat = $state<"json" | "csv">("json");
  let activityExportStatus = $state<string | null>(null);
  let editActivity = $state<ActivityEventView | null>(null);
  let editing = $state(false);
  let deletingActivityId = $state<string | null>(null);

  const localSummary = $derived(buildLocalSummary(gameStore.allGames));
  const activeSummary = $derived(summary ?? localSummary);
  const mediaActivities = $derived(buildMediaActivities(activeSummary.recent_sessions, animeStore.history, comicStore.readHistory, gameStore.allGames));
  const continueItems = $derived(mediaActivities.slice(0, 5));
  const hasRecords = $derived(activeSummary.session_count > 0 || activeSummary.total_seconds > 0 || animeStore.history.length > 0 || comicStore.readHistory.length > 0);
  const dailyBars = $derived(fillDailyBars(activeSummary.daily, 14));
  const monthlyBars = $derived(activeSummary.monthly.slice(-8));
  const activityBars = $derived(fillActivityBars(mediaActivities, 14));
  const dailyPoints = $derived(dailyChartPoints(dailyBars));
  const monthlyPoints = $derived(monthlyChartPoints(monthlyBars));
  const combinedActivityPoints = $derived(activityChartPoints(activityBars));
  const dailySummary = $derived(summarizeChart(dailyPoints, "游玩时长"));
  const monthlySummary = $derived(summarizeChart(monthlyPoints, "游玩时长"));
  const activitySummaryText = $derived(summarizeChart(combinedActivityPoints, "活动"));
  const mediaCounts = $derived(countMediaKinds(mediaActivities));
  const recentActivityCount = $derived(countRecentActivities(mediaActivities, 14));
  const topGames = $derived(toDashboardTopGames(activeSummary, gameStore.allGames));
  const recentSessions = $derived(toDashboardSessions(activeSummary, gameStore.allGames));
  const lastPlayedGame = $derived(findGame(activeSummary.top_games[0]?.game_id));
  const latestActivity = $derived(mediaActivities[0] ?? null);
  const activityV2Summary = $derived(activityV2State.summary);
  const activityV2Metrics = $derived(splitActivityDurations(activityV2Summary));
  const activityV2MutationError = $derived(activityV2State.error && !activityV2State.error.cancelled ? activityV2State.error.message : null);
  const dashboardStats = $derived<DashboardStat[]>([
    { id: "playtime", label: "总游玩时长", value: formatPlayTime(activeSummary.total_seconds), detail: `${activeSummary.play_days} 个活跃日`, tone: "accent" },
    { id: "anime", label: "番剧历史", value: animeStore.history.length, detail: animeStore.history[0]?.lastEpisodeName ?? "开始观看后自动记录" },
    { id: "comic", label: "漫画历史", value: comicStore.readHistory.length, detail: comicStore.readHistory[0]?.last_title ?? "开始阅读后自动记录" },
    { id: "recent", label: "近 14 天活动", value: recentActivityCount, detail: "游戏 / 番剧 / 漫画合计", tone: "success" },
  ]);

  onMount(() => {
    const unsubscribe = activityStore.subscribe((next) => { activityV2State = next; });
    void loadSummary();
    void loadActivityV2();
    return () => { unsubscribe(); activityStore.cancelTimeline(); activityStore.cancelContinue(); };
  });

  async function loadSummary() {
    loading = true; summaryWarning = null;
    try { summary = await getPlaytimeSummary(30, 12, 10); }
    catch (error) { summary = null; summaryWarning = "当前环境未连接原生统计服务，已使用本地游戏库数据预览。"; console.debug("[records] playtime summary fallback:", error); }
    finally { loading = false; }
  }

  async function loadActivityV2() {
    activityV2Loaded = false; activityV2Unavailable = false; activityV2LoadError = null;
    try {
      await backfillLegacyGameActivityOnce();
      await activityStore.load({});
      const timelineError = activityStore.getSnapshot().error;
      if (timelineError && shouldFallbackActivityV2(timelineError.operation)) throw new Error(timelineError.message);
      await activityStore.loadContinue({ limit: 12 });
      const continueError = activityStore.getSnapshot().error;
      if (continueError && shouldFallbackActivityV2(continueError.operation)) throw new Error(continueError.message);
      activityV2Loaded = true;
    } catch (error) {
      activityV2Unavailable = true;
      activityV2LoadError = error instanceof Error ? error.message : "Activity v2 unavailable";
      console.debug("[records] activity v2 fallback:", error);
    }
  }

  async function applyActivityFilters(filters: ActivityFilters) {
    if (!activityV2Loaded || activityV2Unavailable) return;
    await activityStore.load(filters);
    const nextError = activityStore.getSnapshot().error;
    if (nextError && shouldFallbackActivityV2(nextError.operation)) { activityV2Unavailable = true; activityV2LoadError = nextError.message; }
  }

  function findGame(id: string | undefined): Game | null { return id ? gameStore.allGames.find((game) => game.id === id) ?? null : null; }
  function openGame(gameId: string | undefined) { const game = findGame(gameId); if (!game) return; gameStore.selectGame(game.id); uiStore.currentView = "game-detail"; }
  async function launchGame(gameId: string | undefined) { if (gameId) await gameStore.launch(gameId); }

  async function openActivity(item: DashboardMediaActivity) {
    if (item.kind === "game") { openGame((item.payload as PlaySessionEntry).game_id); return; }
    if (item.kind === "anime") { uiStore.currentView = "anime"; await animeStore.resumeHistory(item.payload as AnimeHistory); return; }
    uiStore.currentView = "comic"; await comicStore.resumeHistory(item.payload as ReadRecord);
  }

  async function openContinueCandidate(candidate: ContinueCandidate) {
    if (candidate.resourceKind === "game") { openGame(candidate.resourceId); return; }
    if (candidate.resourceKind === "anime") { uiStore.currentView = "anime"; const history = animeStore.history.find((item) => item.key === candidate.resourceId); if (history) await animeStore.resumeHistory(history); return; }
    uiStore.currentView = "comic"; const history = comicStore.readHistory.find((item) => item.id === candidate.resourceId); if (history) await comicStore.resumeHistory(history);
  }

  function editActivityEvent(event: ActivityEventView) { editActivity = event; activityExportStatus = null; }
  function closeActivityEditor() { if (!editing) editActivity = null; }
  async function saveActivityEdit(patch: ActivityEventPatch) {
    if (!editActivity || editing) return;
    editing = true;
    await activityStore.edit(editActivity.id, patch);
    editing = false;
    if (activityStore.getSnapshot().error?.operation !== "edit") editActivity = null;
  }
  async function deleteActivityEvent(event: ActivityEventView) {
    if (deletingActivityId) return;
    if (typeof window !== "undefined" && !window.confirm("删除这条活动记录？此操作不可撤销。")) return;
    deletingActivityId = event.id; await activityStore.remove(event.id); deletingActivityId = null;
  }
  async function exportActivity(path: string, format: "json" | "csv") {
    activityExportPath = path; activityExportFormat = format; activityExportStatus = null;
    if (!path.trim()) { activityExportStatus = "请输入导出路径。"; return; }
    const exported = await activityStore.export(format, path.trim());
    activityExportStatus = exported ? `已导出：${exported}` : null;
  }
</script>

<PageShell as="div" ariaLabel="游玩记录" width="full" class="records-v2-shell">
  <PageHeader title="游玩记录" eyebrow="Play Records" description="统一查看游戏会话、番剧观看和漫画阅读进度；Activity v2 不可用时自动保留旧版聚合视图。" id="records-page-title">
    {#snippet actions()}
      <div class="records-header-actions">
        {#if latestActivity}<button class="primary" type="button" onclick={() => openActivity(latestActivity)}>继续 {latestActivity.title}</button>
        {:else if lastPlayedGame}<button class="primary" type="button" onclick={() => launchGame(lastPlayedGame?.id)}>继续 {lastPlayedGame.name}</button>
        {:else}<button class="primary" type="button" onclick={() => (uiStore.currentView = "steam-import")}>导入游戏</button>{/if}
        {#if lastPlayedGame}<button type="button" onclick={() => openGame(lastPlayedGame?.id)}>查看最近游戏</button>{/if}
      </div>
    {/snippet}
  </PageHeader>

  <ActivityV2Section
    state={activityV2State}
    loaded={activityV2Loaded}
    unavailable={activityV2Unavailable}
    loadError={activityV2LoadError}
    mutationError={activityV2MutationError}
    exportStatus={activityExportStatus}
    exportPath={activityExportPath}
    exportFormat={activityExportFormat}
    exactSeconds={activityV2Metrics.exactSeconds}
    estimatedSeconds={activityV2Metrics.estimatedSeconds}
    progressOnlyEvents={activityV2Metrics.progressOnlyEvents}
    onFiltersChange={(filters) => { void applyActivityFilters(filters); }}
    onClearFilters={() => { void applyActivityFilters({}); }}
    onContinue={(candidate) => { void openContinueCandidate(candidate); }}
    onLoadMore={() => { void activityStore.loadMore(); }}
    onEdit={editActivityEvent}
    onDelete={(event) => { void deleteActivityEvent(event); }}
    onExport={(path, format) => { void exportActivity(path, format); }}
    onRetry={() => { void loadActivityV2(); }}
  />

  <LegacyOverviewSection loading={loading} {hasRecords} stats={dashboardStats} continueItems={continueItems} warning={summaryWarning} onOpenActivity={(item) => { void openActivity(item); }} onImport={() => (uiStore.currentView = "steam-import")} onHome={() => (uiStore.currentView = "home")} />

  <LegacyInsightsSection {hasRecords} {dailyPoints} {dailySummary} {monthlyPoints} {monthlySummary} activityPoints={combinedActivityPoints} activitySummary={activitySummaryText} {mediaCounts} {topGames} mediaActivities={mediaActivities} {recentSessions} onOpenGame={openGame} onOpenActivity={(item) => { void openActivity(item); }} />
</PageShell>

{#if editActivity}
  <ActivityEditorDialog event={editActivity} saving={editing} error={activityV2State.error?.operation === "edit" ? activityV2State.error.message : null} onCancel={closeActivityEditor} onSave={saveActivityEdit} />
{/if}

<style>
  :global(.records-v2-shell .v2-page-shell__inner) { display: grid; align-content: start; gap: var(--v2-space-6); max-width: min(106rem, 100%); }
  .records-header-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: var(--v2-space-2); }
  .records-header-actions button { min-height: 2.75rem; padding: .55rem .85rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: var(--v2-color-surface); color: var(--v2-color-text); font: inherit; font-weight: 800; cursor: pointer; }
  .records-header-actions button.primary { border-color: var(--v2-color-accent); background: var(--v2-color-accent); color: var(--v2-color-on-accent, #fff); }
  .records-header-actions button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  @media (max-width: 42rem) { .records-header-actions { width: 100%; justify-content: stretch; } .records-header-actions button { flex: 1 1 auto; } }
</style>
