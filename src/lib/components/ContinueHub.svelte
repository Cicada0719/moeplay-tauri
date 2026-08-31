<script lang="ts">
  import { continueStore, type ContinueItem, type ContinueStats } from "../stores/continue.svelte";
  import { animeStore } from "../stores/anime.svelte";
  import { comicStore } from "../stores/comic.svelte";
  import { novelStore } from "../features/novel/store.svelte";
  import { openUnifiedMediaHistory } from "../features/media-history/open";
  import UnifiedMediaHistory from "../features/media-history/UnifiedMediaHistory.svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import ContinueCard from "./ContinueCard.svelte";
  import StatBlock from "./activity/StatBlock.svelte";
  import Icon from "./Icon.svelte";
  import { AsyncSection, ContentGrid, PageHeader, PageShell } from "./ui-v2";

  type ContinueFilter = "all" | "game" | "anime" | "comic";
  let { items: suppliedItems, stats: suppliedStats, topItem: suppliedTopItem = undefined, onSelect }: { items?: ContinueItem[]; stats?: ContinueStats; topItem?: ContinueItem | null; onSelect?: (item: ContinueItem) => void } = $props();
  let filter = $state<ContinueFilter>("all");

  const items = $derived(suppliedItems ?? continueStore.items);
  const stats = $derived(suppliedStats ?? continueStore.stats);
  const topItem = $derived(suppliedTopItem === undefined ? continueStore.topItem : suppliedTopItem);
  const filteredItems = $derived(filter === "all" ? items : items.filter((item) => item.type === filter));
  const gameItems = $derived(items.filter((item) => item.type === "game"));
  const animeItems = $derived(items.filter((item) => item.type === "anime"));
  const comicItems = $derived(items.filter((item) => item.type === "comic"));
  const filterOptions = $derived([
    { value: "all" as const, label: "全部", count: stats.totalCount },
    { value: "game" as const, label: "游戏", count: stats.gameCount },
    { value: "anime" as const, label: "番剧", count: stats.animeCount },
    { value: "comic" as const, label: "漫画", count: stats.comicCount },
  ]);

  async function handleSelect(item: ContinueItem) {
    if (onSelect) { onSelect(item); return; }
    if (item.type === "game") { gameStore.selectGame(item.id.replace("game-", "")); uiStore.currentView = "game-detail"; }
    else if (item.type === "anime") {
      const entry = animeStore.history.find((history) => history.key === item.id.replace("anime-", ""));
      if (entry) await openUnifiedMediaHistory({ kind: "anime", payload: entry });
      else uiStore.currentView = "anime";
    } else {
      const entry = comicStore.readHistory.find((history) => history.id === item.id.replace("comic-", ""));
      if (entry) await openUnifiedMediaHistory({ kind: "comic", payload: entry });
      else uiStore.currentView = "comic";
    }
  }
  function goTo(view: string) { uiStore.currentView = view; }
  function titleFor(type: ContinueFilter): string { return type === "game" ? "最近在玩" : type === "anime" ? "最近在看" : type === "comic" ? "最近在读" : "全部继续项目"; }
  const todayWeekday = $derived(new Date().getDay() || 7);
  const todaySchedule = $derived((animeStore.calendar ?? []).find((day) => day.weekday === todayWeekday)?.items ?? []);
  const calendarLoading = $derived(animeStore.calendarLoading && (animeStore.calendar ?? []).length === 0);
  $effect(() => { void animeStore.loadCalendar(); });
  function openScheduleTab() {
    animeStore.calendarDay = todayWeekday;
    animeStore.setTab("calendar");
    uiStore.currentView = "anime";
  }
</script>

<PageShell as="div" ariaLabel="今日继续" width="content" class="continue-hub-shell">
  <PageHeader title="今日继续" eyebrow="Continue" description="游戏、番剧、漫画和小说共用一条本地历史时间线，选择后直接恢复到上次的位置。" id="continue-page-title">
    {#snippet actions()}<span class="continue-count" role="status">{stats.totalCount} 项继续 · {animeStore.history.length + comicStore.readHistory.length + novelStore.history.length} 条媒体历史</span>{/snippet}
  </PageHeader>

  <AsyncSection title="今日概览" description="根据本地游玩会话和媒体进度计算。" state="ready" class="continue-section">
    <ContentGrid label="今日继续统计" minItemWidth="9rem" gap="sm">
      <StatBlock label="今日活跃" value={stats.todayMinutes} detail="分钟" tone="accent" />
      <StatBlock label="本周活跃" value={stats.weekMinutes} detail="分钟" />
      <StatBlock label="连续活跃" value={stats.streakDays} detail="天" tone="success" />
      <StatBlock label="游戏" value={stats.gameCount} />
      <StatBlock label="番剧" value={stats.animeCount} />
      <StatBlock label="漫画" value={stats.comicCount} />
    </ContentGrid>
  </AsyncSection>

  {#if todaySchedule.length > 0}
    <AsyncSection title="今日放送" description="今日在播番剧来自 Bangumi 放送表；点击进入完整时间表。" state="ready" class="continue-section">
      <div class="today-schedule">
        <div class="today-schedule-summary">
          <StatBlock label="今日在播" value={todaySchedule.length} detail="部" tone="accent" />
          <button class="today-schedule-link" type="button" onclick={openScheduleTab}>查看完整时间表 &#8594;</button>
        </div>
        <div class="today-schedule-covers">
          {#each todaySchedule.slice(0, 3) as sub (sub.id)}
            <button class="today-schedule-card" type="button" onclick={openScheduleTab}>
              {#if animeStore.getImg(sub.image)}
                <img src={animeStore.getImg(sub.image)} alt={sub.name_cn || sub.name} loading="lazy" decoding="async" />
              {:else}
                <div class="today-schedule-fallback"><Icon name="film" /></div>
              {/if}
              <span>{sub.name_cn || sub.name}</span>
            </button>
          {/each}
        </div>
        {#if calendarLoading}
          <p class="today-schedule-loading" role="status">正在加载放送表…</p>
        {/if}
      </div>
    </AsyncSection>
  {/if}

  <AsyncSection title="统一媒体历史" description="番剧、漫画、小说的最近观看与阅读记录；打开后会自动恢复集数、章节和阅读进度。" state="ready" class="continue-section">
    <UnifiedMediaHistory />
  </AsyncSection>

  {#if topItem}
    <AsyncSection title="优先继续" description="综合最近活动、媒体类型和完成进度推荐。" state="ready" class="continue-section">
      <ContinueCard item={topItem} onclick={() => handleSelect(topItem)} emphasized />
    </AsyncSection>
  {/if}

  <AsyncSection title="继续列表" description="筛选不会改变原始排序。" state={filteredItems.length > 0 ? "ready" : "empty"} primaryAction={filteredItems.length === 0 ? { label: "查看游戏库", onSelect: () => goTo("home") } : undefined} secondaryAction={filteredItems.length === 0 ? { label: "查看番剧", onSelect: () => goTo("anime") } : undefined} class="continue-section">
    {#snippet actions()}
      <div class="continue-filters" role="group" aria-label="继续项目类型筛选">{#each filterOptions as option (option.value)}<button type="button" aria-pressed={filter === option.value} onclick={() => (filter = option.value)}>{option.label}<span>{option.count}</span></button>{/each}</div>
    {/snippet}

    {#if filter === "all"}
      {#each [{ type: "game" as const, items: gameItems }, { type: "anime" as const, items: animeItems }, { type: "comic" as const, items: comicItems }] as group (group.type)}
        {#if group.items.length > 0}
          <section class="continue-group" aria-labelledby={`continue-${group.type}-title`}><header><h3 id={`continue-${group.type}-title`}>{titleFor(group.type)}</h3><span>{group.items.length} 项</span></header><ContentGrid label={titleFor(group.type)} minItemWidth="20rem" gap="sm">{#each group.items.slice(0, 8) as item (item.id)}<ContinueCard {item} onclick={() => handleSelect(item)} />{/each}</ContentGrid></section>
        {/if}
      {/each}
    {:else}
      <ContentGrid label={titleFor(filter)} minItemWidth="20rem" gap="sm">{#each filteredItems as item (item.id)}<ContinueCard {item} onclick={() => handleSelect(item)} />{/each}</ContentGrid>
    {/if}
  </AsyncSection>
</PageShell>

<style>
  :global(.continue-hub-shell .v2-page-shell__inner) { display: grid; align-content: start; gap: var(--v2-space-6); }
  .continue-count { display: inline-flex; min-height: 2.5rem; align-items: center; padding: .45rem .8rem; border: 1px solid var(--v2-color-border); border-radius: 999px; color: var(--v2-color-text-secondary); background: var(--v2-color-surface); font-family: var(--v2-font-mono); font-size: var(--v2-text-xs); }
  :global(.continue-section) { padding: var(--v2-space-5); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-xl); background: var(--v2-color-surface); }
  .continue-filters { display: flex; flex-wrap: wrap; gap: var(--v2-space-2); }
  .continue-filters button { display: inline-flex; align-items: center; gap: var(--v2-space-2); min-height: 2.5rem; padding: .45rem .75rem; border: 1px solid var(--v2-color-border); border-radius: 999px; background: var(--v2-color-surface-subtle); color: var(--v2-color-text-secondary); font: inherit; font-weight: 700; cursor: pointer; }
  .continue-filters button[aria-pressed="true"] { border-color: var(--v2-color-accent); background: color-mix(in srgb, var(--v2-color-accent) 16%, var(--v2-color-surface)); color: var(--v2-color-text); }
  .continue-filters button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .continue-filters span { min-width: 1.4rem; padding: .1rem .35rem; border-radius: 999px; background: color-mix(in srgb, var(--v2-color-text) 8%, transparent); font-family: var(--v2-font-mono); font-size: .68rem; text-align: center; }
  .today-schedule { display: grid; gap: var(--v2-space-4); }
  .today-schedule-summary { display: flex; align-items: center; justify-content: space-between; gap: var(--v2-space-3); flex-wrap: wrap; }
  .today-schedule-link { min-height: 2.5rem; padding: .45rem .85rem; border: 1px solid var(--v2-color-border); border-radius: 999px; background: var(--v2-color-surface-subtle); color: var(--v2-color-text-secondary); font: inherit; font-weight: 700; cursor: pointer; }
  .today-schedule-link:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .today-schedule-covers { display: grid; grid-template-columns: repeat(auto-fit, minmax(7.5rem, 1fr)); gap: var(--v2-space-3); }
  .today-schedule-card { display: grid; gap: var(--v2-space-2); min-width: 0; padding: 0; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .today-schedule-card img { width: 100%; aspect-ratio: 2 / 3; object-fit: cover; border-radius: var(--v2-radius-lg); background: var(--v2-color-surface-subtle); }
  .today-schedule-fallback { display: grid; place-items: center; width: 100%; aspect-ratio: 2 / 3; border-radius: var(--v2-radius-lg); background: var(--v2-color-surface-subtle); color: var(--v2-color-text-dim); }
  .today-schedule-card span { overflow: hidden; font-size: var(--v2-text-sm); font-weight: 700; text-overflow: ellipsis; white-space: nowrap; }
  .today-schedule-card:focus-visible img, .today-schedule-card:focus-visible .today-schedule-fallback { outline: none; box-shadow: var(--v2-focus-ring); }
  .today-schedule-loading { margin: 0; color: var(--v2-color-text-secondary); font-family: var(--v2-font-mono); font-size: var(--v2-text-xs); }
  .continue-group { display: grid; gap: var(--v2-space-3); margin-top: var(--v2-space-5); } .continue-group:first-child { margin-top: 0; } .continue-group header { display: flex; align-items: baseline; justify-content: space-between; gap: var(--v2-space-3); } .continue-group h3 { margin: 0; font-size: var(--v2-text-md); } .continue-group header > span { color: var(--v2-color-text-secondary); font-family: var(--v2-font-mono); font-size: var(--v2-text-xs); }
  @media (max-width: 42rem) { .continue-filters { width: 100%; } .continue-filters button { flex: 1 1 auto; justify-content: center; } }
</style>
