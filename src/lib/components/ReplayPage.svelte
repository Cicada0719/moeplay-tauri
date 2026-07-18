<script lang="ts">
  import { formatPlayTime } from "../api";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { i18n } from "../stores/i18n.svelte";
  import { coverOf } from "../utils/game";
  import {
    filterYearSessions,
    monthlyHeat,
    newGamesTimeline,
    summarizeYear,
    topAchievements,
    topCompletions,
    topPlayed,
  } from "../features/replay/aggregate";
  import CachedImage from "./CachedImage.svelte";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, StatBlock } from "./ui";
  import { PageHeader, PageShell } from "./ui-v2";

  const currentYear = new Date().getFullYear();
  let year = $state(currentYear);

  // 数据流：gameStore.games → 年度会话过滤 → 各聚合派生；全部纯前端，无 Rust 调用。
  const games = $derived(gameStore.games);
  const entries = $derived(filterYearSessions(games, year));
  const summary = $derived(summarizeYear(entries));
  const top5 = $derived(
    topPlayed(entries, 5).map((entry) => ({ ...entry, cover: coverOf(entry.game) })),
  );
  const heat = $derived(monthlyHeat(entries));
  const achievementTop = $derived(topAchievements(games, 3));
  const completionTop = $derived(topCompletions(games, 3));
  const timeline = $derived(newGamesTimeline(games, year));
  const newGamesTotal = $derived(timeline.reduce((sum, m) => sum + m.games.length, 0));
  const hasData = $derived(entries.length > 0);

  const monthLabels = $derived(
    Array.from({ length: 12 }, (_, m) =>
      new Date(2024, m, 1).toLocaleString(i18n.locale, { month: "short" })),
  );

  const oneLiner = $derived(
    i18n.t("replay.oneliner", { games: summary.gameCount, hours: summary.hours }),
  );

  function prevYear() {
    year -= 1;
  }

  function nextYear() {
    if (year < currentYear) year += 1;
  }

  function backToThisYear() {
    year = currentYear;
  }

  function goHome() {
    uiStore.currentView = "home";
  }
</script>

<PageShell as="div" width="full" scrollable={false} class="replay-v2-shell" labelledBy="replay-page-title" ariaLabel={i18n.t("replay.title")}>
  <div class="rp">
    <div class="v2-grain rp-grain" aria-hidden="true"></div>

    <PageHeader
      id="replay-page-title"
      class="rp-header"
      eyebrow="回想 / REPLAY"
      title={i18n.t("replay.title")}
      description={i18n.t("replay.subtitle")}
    >
      {#snippet actions()}
        <div class="rp-yearbar">
          <Button variant="secondary" size="sm" press={prevYear} ariaLabel={i18n.t("replay.prev_year")} title={i18n.t("replay.prev_year")}>
            <Icon name="chevronLeft" size={14} />
          </Button>
          <span class="rp-year mono" aria-live="polite">{year}</span>
          <Button variant="secondary" size="sm" press={nextYear} disabled={year >= currentYear} ariaLabel={i18n.t("replay.next_year")} title={i18n.t("replay.next_year")}>
            <Icon name="chevronRight" size={14} />
          </Button>
          {#if year !== currentYear}
            <Button variant="ghost" size="sm" press={backToThisYear}>{i18n.t("replay.this_year")}</Button>
          {/if}
        </div>
      {/snippet}
    </PageHeader>

    <main class="rp-content">
      {#if !hasData}
        <div class="rp-empty">
          <EmptyState
            icon="calendar"
            title={i18n.t("replay.empty_title")}
            description={i18n.t("replay.empty_desc")}
            action={{ label: i18n.t("replay.empty_action"), onclick: goHome }}
          />
        </div>
      {:else}
        <div class="bento">
          <Card class="rp-card rp-hero">
            <span class="rp-hero-year mono" aria-hidden="true">{year}</span>
            <p class="rp-hero-line">{oneLiner}</p>
          </Card>

          <StatBlock label={i18n.t("replay.stat_total_time")} value={summary.hours} unit={i18n.t("replay.unit_hours")} class="stat-cell" />
          <StatBlock label={i18n.t("replay.stat_play_days")} value={summary.playDays} unit={i18n.t("replay.unit_days")} class="stat-cell" />
          <StatBlock label={i18n.t("replay.stat_sessions")} value={summary.sessionCount} unit={i18n.t("replay.unit_sessions")} class="stat-cell" />

          <Card class="rp-card rp-top">
            <span class="label">{i18n.t("replay.top_played")}</span>
            <div class="top-list">
              {#each top5 as entry, index (entry.game.id)}
                <div class="top-row">
                  <span class="rank mono">#{index + 1}</span>
                  <span class="cover-frame">
                    {#if entry.cover}
                      <CachedImage source={entry.cover} cacheKey="replay-{entry.game.id}" alt={entry.game.name} loading="lazy" />
                    {:else}
                      <span class="cover-fallback" aria-hidden="true">{entry.game.name.slice(0, 1)}</span>
                    {/if}
                  </span>
                  <span class="game-name">{entry.game.name}</span>
                  <span class="mono muted">{formatPlayTime(entry.seconds)}</span>
                </div>
              {/each}
            </div>
          </Card>

          <Card class="rp-card">
            <span class="label">{i18n.t("replay.achievements")}</span>
            {#if achievementTop.length > 0}
              <div class="rank-list">
                {#each achievementTop as entry (entry.game.id)}
                  <div class="rank-row">
                    <div class="rank-row-head">
                      <span class="game-name">{entry.game.name}</span>
                      <span class="mono muted">{entry.unlocked}/{entry.total} · {Math.round(entry.ratio * 100)}%</span>
                    </div>
                    <div class="bar-track">
                      <div class="bar-fill" style="--bar: {Math.round(entry.ratio * 100)}%"></div>
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <EmptyState title={i18n.t("replay.achievements_empty")} />
            {/if}
          </Card>

          <Card class="rp-card rp-heat">
            <span class="label">{i18n.t("replay.monthly_heat")}</span>
            <div class="heat-grid">
              {#each heat as cell (cell.month)}
                <div class="heat-cell">
                  <div
                    class="heat-bar"
                    style="--heat: {Math.round(cell.ratio * 100)}%"
                    title="{monthLabels[cell.month]} · {formatPlayTime(cell.seconds)}"
                  ></div>
                  <span class="heat-label mono">{monthLabels[cell.month]}</span>
                </div>
              {/each}
            </div>
          </Card>

          <Card class="rp-card">
            <span class="label">{i18n.t("replay.completions")}</span>
            {#if completionTop.length > 0}
              <div class="rank-list">
                {#each completionTop as entry, index (entry.game.id)}
                  <div class="flat-row">
                    <span class="rank mono">#{index + 1}</span>
                    <span class="game-name">{entry.game.name}</span>
                    <span class="mono">×{entry.count}</span>
                  </div>
                {/each}
              </div>
            {:else}
              <EmptyState title={i18n.t("replay.completions_empty")} />
            {/if}
          </Card>

          <Card class="rp-card rp-timeline">
            <span class="label">{i18n.t("replay.new_games")}</span>
            {#if newGamesTotal > 0}
              <span class="hint">{i18n.t("replay.new_games_count", { count: newGamesTotal })}</span>
              <div class="timeline-grid">
                {#each timeline as bucket (bucket.month)}
                  <div
                    class="timeline-cell"
                    class:is-empty={bucket.games.length === 0}
                    title={bucket.games.map((g) => g.name).join(" / ")}
                  >
                    <span class="heat-label mono">{monthLabels[bucket.month]}</span>
                    <span class="timeline-count mono">{bucket.games.length > 0 ? bucket.games.length : "—"}</span>
                  </div>
                {/each}
              </div>
            {:else}
              <EmptyState title={i18n.t("replay.new_games_empty")} />
            {/if}
          </Card>
        </div>
      {/if}
    </main>
  </div>
</PageShell>

<style>
  :global(.replay-v2-shell) { height: 100%; }
  :global(.replay-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .rp {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  .rp-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.rp-header) {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: 26px 28px 14px;
    flex-shrink: 0;
  }

  .rp-yearbar {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .rp-year {
    min-width: 64px;
    text-align: center;
    color: var(--text-primary);
    font-size: 18px;
    font-weight: 760;
  }

  .rp-content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: 0 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    scroll-behavior: smooth;
  }

  .rp-empty {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
  }

  .bento {
    min-width: 0;
    width: 100%;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-auto-flow: dense;
    gap: 14px;
  }

  :global(.ui-card.rp-card),
  :global(.ui-stat.stat-cell) {
    min-width: 0;
  }

  :global(.ui-card.rp-card) {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 132px;
  }

  :global(.ui-stat.stat-cell) {
    min-height: 132px;
  }

  :global(.ui-card.rp-card),
  :global(.ui-stat.stat-cell) {
    animation: rp-rise 0.45s ease both;
  }

  @keyframes rp-rise {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: none; }
  }

  :global(.ui-card.rp-hero) {
    grid-column: span 3;
    position: relative;
    overflow: hidden;
    justify-content: center;
    min-height: 128px;
  }

  .rp-hero-year {
    position: absolute;
    right: 14px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--accent);
    opacity: 0.12;
    font-size: clamp(72px, 12vw, 132px);
    font-weight: 800;
    line-height: 1;
    pointer-events: none;
    user-select: none;
  }

  .rp-hero-line {
    position: relative;
    margin: 0;
    max-width: 26ch;
    color: var(--text-primary);
    font-size: clamp(22px, 3.2vw, 34px);
    font-weight: 760;
    line-height: 1.25;
    letter-spacing: -0.01em;
    overflow-wrap: anywhere;
  }

  :global(.ui-card.rp-top),
  :global(.ui-card.rp-heat) {
    grid-column: span 2;
  }

  :global(.ui-card.rp-timeline) {
    grid-column: span 3;
  }

  .label {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.2;
  }

  .hint,
  .muted {
    color: var(--text-muted);
  }

  .hint {
    font-size: 13px;
    line-height: 1.4;
  }

  .mono {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .top-list,
  .rank-list {
    min-width: 0;
    display: grid;
    gap: 8px;
  }

  .top-row {
    min-width: 0;
    display: grid;
    grid-template-columns: 34px 40px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .rank {
    color: var(--accent);
    font-size: 13px;
    font-weight: 760;
  }

  .cover-frame {
    width: 40px;
    height: 54px;
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg-hover);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .cover-frame :global(.cached-image) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .cover-fallback {
    color: var(--text-muted);
    font-size: 16px;
    font-weight: 700;
  }

  .game-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rank-row {
    min-width: 0;
    display: grid;
    gap: 6px;
  }

  .rank-row-head {
    min-width: 0;
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .bar-track {
    height: 6px;
    border-radius: 999px;
    background: var(--bg-hover);
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    width: var(--bar, 0%);
    border-radius: 999px;
    background: var(--accent);
  }

  .heat-grid {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 6px;
    align-items: end;
    flex: 1;
  }

  .heat-cell {
    min-width: 0;
    display: grid;
    gap: 6px;
    justify-items: center;
  }

  .heat-bar {
    width: 100%;
    height: 72px;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--accent) var(--heat, 0%), var(--bg-hover));
    transition: transform 0.15s ease;
  }

  .heat-bar:hover {
    transform: translateY(-2px);
  }

  .heat-label {
    color: var(--text-muted);
    font-size: 10px;
    line-height: 1;
  }

  .flat-row {
    min-width: 0;
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .timeline-grid {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 6px;
  }

  .timeline-cell {
    min-width: 0;
    display: grid;
    gap: 6px;
    justify-items: center;
    padding: 10px 4px;
    border-radius: var(--radius-md);
    background: var(--bg-hover);
  }

  .timeline-cell.is-empty {
    opacity: 0.45;
  }

  .timeline-count {
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 760;
    line-height: 1;
  }

  .timeline-cell.is-empty .timeline-count {
    color: var(--text-muted);
    font-weight: 500;
  }

  /* ── Responsive ── */
  @media (max-width: 900px) {
    .bento {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    :global(.ui-card.rp-hero),
    :global(.ui-card.rp-top),
    :global(.ui-card.rp-heat),
    :global(.ui-card.rp-timeline) {
      grid-column: span 2;
    }
  }

  @media (max-width: 560px) {
    .rp-content { padding: 0 16px 36px; }
    :global(.rp-header) { padding: 20px 16px 12px; }

    .bento {
      grid-template-columns: 1fr;
    }

    :global(.ui-card.rp-card),
    :global(.ui-card.rp-hero),
    :global(.ui-card.rp-top),
    :global(.ui-card.rp-heat),
    :global(.ui-card.rp-timeline),
    :global(.ui-stat.stat-cell) {
      grid-column: 1;
    }

    .heat-grid,
    .timeline-grid {
      grid-template-columns: repeat(6, minmax(0, 1fr));
    }
  }

  /* ── Reduced motion：OS prefers-reduced-motion + 应用内 data-motion 双信号 ── */
  @media (prefers-reduced-motion: reduce) {
    .rp-content { scroll-behavior: auto; }
    :global(.ui-card.rp-card),
    :global(.ui-stat.stat-cell) { animation: none; }
    .heat-bar { transition: none; }
    .heat-bar:hover { transform: none; }
  }
  :global([data-motion="reduce"]) .rp-content { scroll-behavior: auto; }
  :global([data-motion="reduce"]) :global(.ui-card.rp-card),
  :global([data-motion="reduce"]) :global(.ui-stat.stat-cell) { animation: none; }
  :global([data-motion="reduce"]) .heat-bar { transition: none; }
  :global([data-motion="reduce"]) .heat-bar:hover { transform: none; }
</style>
