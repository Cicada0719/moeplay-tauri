<script lang="ts">
  import type { ActivityEventPatch, ActivityEventType, ActivityEventView, ActivityFilters, ActivityResourceKind, ActivityStoreState, ContinueCandidate } from "../../features/activity";
  import { AsyncSection, ContentGrid, MediaRow } from "../ui-v2";
  import StatBlock from "./StatBlock.svelte";

  let { state: activityState, loaded, unavailable, loadError = null, mutationError = null, exportStatus = null, exportPath, exportFormat, exactSeconds = 0, estimatedSeconds = 0, progressOnlyEvents = 0, onFiltersChange, onClearFilters, onContinue, onLoadMore, onEdit, onDelete, onExport, onRetry }: {
    state: ActivityStoreState; loaded: boolean; unavailable: boolean; loadError?: string | null; mutationError?: string | null; exportStatus?: string | null; exportPath: string; exportFormat: "json" | "csv"; exactSeconds?: number; estimatedSeconds?: number; progressOnlyEvents?: number; onFiltersChange: (filters: ActivityFilters) => void; onClearFilters: () => void; onContinue: (candidate: ContinueCandidate) => void; onLoadMore: () => void; onEdit: (event: ActivityEventView) => void; onDelete: (event: ActivityEventView) => void; onExport: (path: string, format: "json" | "csv") => void; onRetry: () => void;
  } = $props();

  const sectionState = $derived(unavailable ? "offline" : loaded ? "ready" : "loading");
  const activeFilterCount = $derived(Object.values(activityState.filters).filter((value) => value !== null && value !== undefined && value !== "").length);

  function updateFilter<K extends keyof ActivityFilters>(key: K, value: ActivityFilters[K] | "") {
    onFiltersChange({ ...activityState.filters, [key]: value === "" ? null : value });
  }
  function formatSeconds(seconds: number | null): string {
    if (!seconds || seconds <= 0) return "0m";
    const hours = seconds / 3600;
    return hours >= 1 ? `${hours < 10 ? hours.toFixed(1) : Math.round(hours)}h` : `${Math.max(1, Math.round(seconds / 60))}m`;
  }
  function dateTime(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }).format(date);
  }
  function kindLabel(kind: ActivityResourceKind): string { return kind === "game" ? "游戏" : kind === "anime" ? "番剧" : "漫画"; }
  function eventLabel(type: ActivityEventType): string {
    return ({ started: "开始", progressed: "推进", completed: "完成", rated: "评分", favorited: "收藏", imported: "导入", failed: "失败" })[type];
  }
  function eventPayload(event: ActivityEventView): Record<string, unknown> {
    return event.payload && typeof event.payload === "object" && !Array.isArray(event.payload) ? event.payload as Record<string, unknown> : {};
  }
  function payloadText(payload: Record<string, unknown>, ...keys: string[]): string {
    for (const key of keys) { const value = payload[key]; if (typeof value === "string" && value.trim()) return value.trim(); }
    return "";
  }
  function activityTitle(event: ActivityEventView): string {
    const payload = eventPayload(event);
    return payloadText(payload, "title", "name", "resource_title", "game_name", "anime_name", "comic_title") || event.resourceId;
  }
  function activityDescription(event: ActivityEventView): string | undefined {
    const payload = eventPayload(event);
    const rating = Number(payload.rating ?? payload.score ?? payload.user_rating);
    if (event.eventType === "rated" && Number.isFinite(rating)) return `我的评分 ${rating.toFixed(1)} / 10`;
    const episode = payloadText(payload, "episode_name", "episode", "position_label");
    if (event.resourceKind === "anime" && episode) return `观看至 ${episode}`;
    const chapter = payloadText(payload, "chapter_title", "chapter", "position_label");
    if (event.resourceKind === "comic" && chapter) return `阅读至 ${chapter}`;
    const note = payloadText(payload, "description", "summary", "note", "message");
    if (note) return note;
    if (event.eventType === "completed") return "这部作品已进入完成档案。";
    if (event.eventType === "favorited") return "已加入私人收藏。";
    return event.sourceLegacyId ? `由旧版记录迁移：${event.sourceLegacyId}` : undefined;
  }</script>

<AsyncSection
  title="Activity v2 活动记录"
  description={unavailable ? `Activity v2 暂不可用${loadError ? `：${loadError}` : ""}。下方继续显示旧版聚合记录。` : "精确、估算和仅进度事件分开统计；不确定时长不会混入精确总时长。"}
  state={sectionState}
  loadingRows={4}
  details={unavailable && loadError ? loadError : undefined}
  primaryAction={unavailable ? { label: "重试 Activity v2", onSelect: onRetry } : undefined}
  ariaLive="polite"
  class="activity-v2-section"
>
  {#snippet actions()}
    <form class="activity-export" aria-label="导出活动记录" onsubmit={(event) => { event.preventDefault(); const data = new FormData(event.currentTarget); onExport(String(data.get("path") ?? ""), data.get("format") === "csv" ? "csv" : "json"); }}>
      <label>格式<select name="format" value={exportFormat}><option value="json">JSON</option><option value="csv">CSV</option></select></label>
      <label>路径<input name="path" value={exportPath} aria-label="导出路径" /></label>
      <button type="submit">导出</button>
    </form>
  {/snippet}

  {#if mutationError}<p class="activity-message activity-message--error" role="alert">{mutationError}</p>{/if}
  {#if exportStatus}<p class="activity-message" role="status">{exportStatus}</p>{/if}

  {#if activityState.summary}
    <ContentGrid label="Activity v2 时长质量概览" minItemWidth="11rem" gap="sm">
      <StatBlock label="精确时长" value={formatSeconds(exactSeconds)} detail={`${activityState.summary.eventCount - activityState.summary.progressOnlyCount} 条含时长记录`} tone="accent" />
      <StatBlock label="估算时长" value={formatSeconds(estimatedSeconds)} detail="不计入精确总时长" tone="warning" />
      <StatBlock label="仅进度" value={progressOnlyEvents} detail="不含可加总时长" />
      <StatBlock label="事件总数" value={activityState.summary.eventCount} detail="事件条数不等于时长" tone="success" />
    </ContentGrid>
  {/if}

  <section class="activity-subsection" aria-labelledby="activity-continue-title">
    <header><div><h3 id="activity-continue-title">继续项目</h3><p>来自 Activity v2 的跨媒体进度。</p></div><span>{activityState.continueCandidates.length} 项</span></header>
    {#if activityState.isLoadingContinue}<p class="activity-message" role="status">正在加载继续项目…</p>
    {:else if activityState.continueCandidates.length === 0}<p class="activity-empty">还没有可继续的游戏、番剧或漫画。</p>
    {:else}
      <ContentGrid label="Activity v2 继续项目" minItemWidth="18rem" gap="sm">
        {#each activityState.continueCandidates as candidate (candidate.resourceKind + candidate.resourceId + (candidate.providerId ?? ""))}
          {#snippet candidateMeta()}<span>{candidate.durationQuality === "exact" ? "精确" : candidate.durationQuality === "estimated" ? "估算" : "仅进度"} · {dateTime(candidate.updatedAt)}{#if candidate.exactDurationSeconds || candidate.estimatedDurationSeconds} · {formatSeconds(candidate.exactDurationSeconds ?? candidate.estimatedDurationSeconds)}{/if}</span>{/snippet}
          {#snippet candidateBadge()}<span class="activity-badge">{kindLabel(candidate.resourceKind)}</span>{/snippet}
          <MediaRow title={candidate.title} subtitle={candidate.providerId ?? "本地记录"} imageSrc={candidate.artworkUrl ?? undefined} ariaLabel={`继续 ${candidate.title}`} focusKey={`activity-continue-${candidate.resourceKind}-${candidate.resourceId}`} onActivate={() => onContinue(candidate)} meta={candidateMeta} badge={candidateBadge} />
        {/each}
      </ContentGrid>
    {/if}
  </section>

  <section class="activity-subsection" aria-labelledby="activity-timeline-title">
    <header><div><h3 id="activity-timeline-title">活动时间线</h3><p>可筛选、编辑、删除并继续加载历史事件。</p></div><span>{activityState.events.length} 条</span></header>
    <div class="activity-filters" aria-label="活动筛选" aria-busy={activityState.isLoadingTimeline}>
      <label>媒体<select value={activityState.filters.resourceKind ?? ""} onchange={(event) => updateFilter("resourceKind", event.currentTarget.value as ActivityResourceKind | "")}><option value="">全部</option><option value="game">游戏</option><option value="anime">番剧</option><option value="comic">漫画</option></select></label>
      <label>事件<select value={activityState.filters.eventType ?? ""} onchange={(event) => updateFilter("eventType", event.currentTarget.value as ActivityEventType | "")}><option value="">全部</option><option value="started">开始</option><option value="progressed">推进</option><option value="completed">完成</option><option value="rated">评分</option><option value="favorited">收藏</option><option value="imported">导入</option><option value="failed">失败</option></select></label>
      <label>从<input type="datetime-local" value={activityState.filters.startedAtFrom ?? ""} onchange={(event) => updateFilter("startedAtFrom", event.currentTarget.value)} /></label>
      <label>到<input type="datetime-local" value={activityState.filters.startedAtTo ?? ""} onchange={(event) => updateFilter("startedAtTo", event.currentTarget.value)} /></label>
      <button type="button" onclick={onClearFilters} disabled={activityState.isLoadingTimeline || activeFilterCount === 0}>清除筛选{activeFilterCount ? ` (${activeFilterCount})` : ""}</button>
    </div>
    {#if activityState.isLoadingTimeline}<p class="activity-message" role="status">正在加载活动时间线…</p>
    {:else if activityState.events.length === 0}<p class="activity-empty">当前筛选条件下没有活动记录。</p>
    {:else}
      <div class="activity-timeline" role="list" aria-label="活动时间线事件">
        {#each activityState.events as event (event.id)}
          {#snippet eventMeta()}<span>{dateTime(event.startedAt)} · {event.durationQuality === "exact" ? "精确" : event.durationQuality === "estimated" ? "估算" : "仅进度"}{#if event.durationSeconds !== null} · {formatSeconds(event.durationSeconds)}{/if}</span>{/snippet}
          {#snippet eventBadge()}<span class="activity-badge">{kindLabel(event.resourceKind)}</span>{/snippet}
          {#snippet eventActions()}<button type="button" aria-label={`编辑 ${event.resourceId} 活动`} onclick={() => onEdit(event)}>编辑</button><button type="button" class="danger" aria-label={`删除 ${event.resourceId} 活动`} onclick={() => onDelete(event)}>删除</button>{/snippet}
          <MediaRow title={`${eventLabel(event.eventType)} · ${activityTitle(event)}`} subtitle={`${kindLabel(event.resourceKind)}档案 · ${event.providerId ?? "本地记录"}`} description={activityDescription(event)} meta={eventMeta} badge={eventBadge} actions={eventActions} focusKey={`activity-event-${event.id}`} />
        {/each}
      </div>
      {#if activityState.nextCursor}<button class="load-more" type="button" onclick={onLoadMore} disabled={activityState.isLoadingMore}>{activityState.isLoadingMore ? "加载中…" : "加载更多"}</button>{/if}
    {/if}
  </section>
</AsyncSection>

<style>
  :global(.activity-v2-section) { padding: var(--v2-space-5); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-xl); background: color-mix(in srgb, var(--v2-color-surface) 88%, transparent); }
  .activity-export, .activity-filters { display: flex; align-items: end; flex-wrap: wrap; gap: var(--v2-space-2); } .activity-export label, .activity-filters label { display: grid; gap: var(--v2-space-1); color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); font-weight: 700; }
  input, select, button { min-height: 2.5rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: var(--v2-color-surface-subtle); color: var(--v2-color-text); font: inherit; } input, select { padding: .45rem .6rem; } button { padding: .45rem .75rem; cursor: pointer; } button:focus-visible, input:focus-visible, select:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); } button:disabled { cursor: not-allowed; opacity: .55; }
  .activity-export input { width: min(16rem, 50vw); } .activity-subsection { display: grid; gap: var(--v2-space-3); margin-top: var(--v2-space-6); } .activity-subsection > header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--v2-space-3); } .activity-subsection h3 { margin: 0; font-size: var(--v2-text-md); } .activity-subsection p { margin: var(--v2-space-1) 0 0; color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); } .activity-subsection > header > span { color: var(--v2-color-text-secondary); font-family: var(--v2-font-mono); font-size: var(--v2-text-xs); }
  .activity-filters { padding: var(--v2-space-3); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-lg); background: var(--v2-color-surface-subtle); } .activity-timeline { display: grid; gap: var(--v2-space-2); } .activity-timeline :global(.v2-media-row__actions button) { min-height: 2.25rem; } .activity-timeline :global(.v2-media-row__actions .danger) { border-color: color-mix(in srgb, #ef6a7d 55%, var(--v2-color-border)); color: #ffb5c0; } .activity-badge { display: inline-flex; padding: .2rem .45rem; border-radius: 999px; background: color-mix(in srgb, var(--v2-color-accent) 18%, transparent); color: var(--v2-color-accent); font-size: .68rem; font-weight: 800; } .activity-message, .activity-empty { margin: 0; padding: var(--v2-space-3); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); color: var(--v2-color-text-secondary); background: var(--v2-color-surface-subtle); } .activity-message--error { border-color: color-mix(in srgb, #ef6a7d 55%, var(--v2-color-border)); color: #ffb5c0; } .load-more { justify-self: center; margin-top: var(--v2-space-3); }
  @media (max-width: 42rem) { .activity-export { width: 100%; } .activity-export label, .activity-export input { width: 100%; } .activity-subsection > header { flex-direction: column; } }
</style>
