<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { fileSrc } from "../utils";
  import { coverOf, gameLastPlayed, gameTotalSeconds } from "../utils/game";
  import { formatPlayTime, getPlaytimeSummary, type Game, type PlaySessionEntry, type PlaytimeSummary } from "../api";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, Skeleton, Tag } from "./ui";

  let summary = $state<PlaytimeSummary | null>(null);
  let loading = $state(true);
  let summaryWarning = $state<string | null>(null);

  const localSummary = $derived(buildLocalSummary(gameStore.allGames));
  const activeSummary = $derived(summary ?? localSummary);
  const hasRecords = $derived(activeSummary.session_count > 0 || activeSummary.total_seconds > 0);
  const topGames = $derived(activeSummary.top_games.slice(0, 5));
  const recentSessions = $derived(activeSummary.recent_sessions.slice(0, 7));
  const dailyBars = $derived(fillDailyBars(activeSummary.daily, 14));
  const monthlyBars = $derived(activeSummary.monthly.slice(-8));
  const dailyMax = $derived(Math.max(1, ...dailyBars.map(d => d.seconds)));
  const monthlyMax = $derived(Math.max(1, ...monthlyBars.map(m => m.seconds)));
  const lastPlayedGame = $derived(findGame(topGames[0]?.game_id));

  onMount(() => {
    void loadSummary();
  });

  async function loadSummary() {
    loading = true;
    summaryWarning = null;
    try {
      summary = await getPlaytimeSummary(30, 12, 10);
    } catch (e) {
      summary = null;
      summaryWarning = "当前环境未连接原生统计服务，已使用本地游戏库数据预览。";
      console.debug("[records] playtime summary fallback:", e);
    } finally {
      loading = false;
    }
  }

  function buildLocalSummary(games: Game[]): PlaytimeSummary {
    const sessions: PlaySessionEntry[] = [];
    const daily = new Map<string, { seconds: number; sessions: number }>();
    const monthly = new Map<string, { seconds: number; sessions: number }>();
    let totalSeconds = 0;

    for (const game of games) {
      const gameSessions = game.play_tracker?.sessions ?? [];
      totalSeconds += gameTotalSeconds(game);
      for (const session of gameSessions) {
        const seconds = Number(session.duration_seconds ?? 0);
        if (!seconds || seconds <= 0) continue;
        sessions.push({ game_id: game.id, game_name: game.name, session });
        const day = dateKey(session.start_time);
        const month = day.slice(0, 7);
        const d = daily.get(day) ?? { seconds: 0, sessions: 0 };
        d.seconds += seconds;
        d.sessions += 1;
        daily.set(day, d);
        const m = monthly.get(month) ?? { seconds: 0, sessions: 0 };
        m.seconds += seconds;
        m.sessions += 1;
        monthly.set(month, m);
      }
    }

    sessions.sort((a, b) => new Date(b.session.start_time).getTime() - new Date(a.session.start_time).getTime());

    const top = games
      .map(game => ({
        game_id: game.id,
        game_name: game.name,
        total_seconds: gameTotalSeconds(game),
        sessions: game.play_tracker?.sessions?.length ?? 0,
        last_played: gameLastPlayed(game) ?? undefined,
      }))
      .filter(item => item.total_seconds > 0)
      .sort((a, b) => b.total_seconds - a.total_seconds);

    const playDays = [...daily.values()].filter(d => d.seconds > 0).length;
    const sessionCount = sessions.length;
    return {
      total_seconds: totalSeconds,
      session_count: sessionCount,
      play_days: playDays,
      average_session_seconds: sessionCount ? Math.round(sessions.reduce((sum, i) => sum + i.session.duration_seconds, 0) / sessionCount) : 0,
      daily: [...daily.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([date, value]) => ({ date, ...value })),
      monthly: [...monthly.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([month, value]) => ({ month, ...value })),
      recent_sessions: sessions,
      top_games: top,
    };
  }

  function fillDailyBars(days: PlaytimeSummary["daily"], count: number) {
    const byDate = new Map(days.map(day => [day.date, day]));
    const list: PlaytimeSummary["daily"] = [];
    const now = new Date();
    for (let i = count - 1; i >= 0; i--) {
      const d = new Date(now);
      d.setDate(now.getDate() - i);
      const key = dateKey(d.toISOString());
      list.push(byDate.get(key) ?? { date: key, seconds: 0, sessions: 0 });
    }
    return list;
  }

  function dateKey(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return new Date().toISOString().slice(0, 10);
    return date.toISOString().slice(0, 10);
  }

  function monthLabel(value: string): string {
    const [, month] = value.split("-");
    return `${Number(month || 0)}月`;
  }

  function dayLabel(value: string): string {
    const date = new Date(`${value}T00:00:00`);
    return Number.isNaN(date.getTime()) ? value.slice(5) : `${date.getMonth() + 1}/${date.getDate()}`;
  }

  function formatDateTime(value: string | undefined): string {
    if (!value) return "暂无记录";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return new Intl.DateTimeFormat("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(date);
  }

  function formatCompactSeconds(seconds: number): string {
    if (!seconds || seconds <= 0) return "0m";
    const hours = seconds / 3600;
    if (hours >= 10) return `${Math.round(hours)}h`;
    if (hours >= 1) return `${hours.toFixed(1)}h`;
    return `${Math.max(1, Math.round(seconds / 60))}m`;
  }

  function findGame(id: string | undefined): Game | null {
    if (!id) return null;
    return gameStore.allGames.find(game => game.id === id) ?? null;
  }

  function coverFor(gameId: string | undefined): string | null {
    const game = findGame(gameId);
    return fileSrc(coverOf(game));
  }

  function openGame(gameId: string | undefined) {
    const game = findGame(gameId);
    if (!game) return;
    gameStore.selectGame(game.id);
    uiStore.currentView = "game-detail";
  }

  async function launchGame(gameId: string | undefined) {
    if (!gameId) return;
    await gameStore.launch(gameId);
  }
</script>

<section class="records-page">
  <header class="records-hero">
    <div class="hero-copy">
      <span class="eyebrow">Play Records</span>
      <h1>游玩记录</h1>
      <p>把最近玩了什么、玩了多久、什么时候继续，整理成一个安静的仪表盘。</p>
      <div class="hero-actions">
        {#if lastPlayedGame}
          <Button variant="primary" size="sm" press={() => launchGame(lastPlayedGame?.id)}>
            <Icon name="play" size={16} />
            <span>继续 {lastPlayedGame.name}</span>
          </Button>
          <Button variant="secondary" size="sm" press={() => openGame(lastPlayedGame?.id)}>
            <Icon name="info" size={16} />
            <span>查看详情</span>
          </Button>
        {:else}
          <Button variant="primary" size="sm" press={() => (uiStore.currentView = "steam-import")}>
            <Icon name="database" size={16} />
            <span>导入游戏</span>
          </Button>
        {/if}
      </div>
    </div>

    <div class="hero-template" aria-hidden="true">
      <div class="template-glow"></div>
      <div class="template-card template-card-main">
        <div class="template-line wide"></div>
        <div class="template-bars">
          <span style="height: 38%"></span>
          <span style="height: 62%"></span>
          <span style="height: 48%"></span>
          <span style="height: 76%"></span>
          <span style="height: 58%"></span>
          <span style="height: 86%"></span>
          <span style="height: 44%"></span>
        </div>
      </div>
      <div class="template-card template-card-side">
        <div class="template-ring"></div>
        <div class="template-line"></div>
        <div class="template-line short"></div>
      </div>
      <div class="template-pad">
        <span></span>
        <i></i>
        <b></b>
      </div>
    </div>
  </header>

  {#if summaryWarning}
    <div class="soft-warning">
      <Icon name="info" size={15} />
      <span>{summaryWarning}</span>
    </div>
  {/if}

  {#if loading}
    <div class="records-loading">
      <Skeleton variant="stat" count={4} />
      <Skeleton variant="block" count={2} />
    </div>
  {:else}
    <section class="metric-grid" aria-label="游玩统计">
      <Card class="record-metric total">
        <span class="metric-label">总游玩时长</span>
        <strong>{formatPlayTime(activeSummary.total_seconds)}</strong>
        <small>{activeSummary.play_days} 个活跃日</small>
      </Card>
      <Card class="record-metric">
        <span class="metric-label">近期开局</span>
        <strong>{activeSummary.session_count}</strong>
        <small>记录到的游玩会话</small>
      </Card>
      <Card class="record-metric">
        <span class="metric-label">平均一局</span>
        <strong>{formatCompactSeconds(activeSummary.average_session_seconds)}</strong>
        <small>按会话时长计算</small>
      </Card>
      <Card class="record-metric">
        <span class="metric-label">最常打开</span>
        <strong>{topGames[0]?.game_name ?? "暂无"}</strong>
        <small>{topGames[0] ? formatCompactSeconds(topGames[0].total_seconds) : "导入并游玩后出现"}</small>
      </Card>
    </section>

    {#if hasRecords}
      <main class="records-layout">
        <Card class="panel daily-panel">
          <div class="panel-head">
            <div>
              <span class="panel-kicker">Last 14 Days</span>
              <h2>最近两周</h2>
            </div>
            <Tag variant="accent">{formatCompactSeconds(dailyBars.reduce((sum, day) => sum + day.seconds, 0))}</Tag>
          </div>
          <div class="daily-bars" aria-label="最近两周每日游玩时长">
            {#each dailyBars as day}
              <div class="day-bar" title={`${dayLabel(day.date)} · ${formatPlayTime(day.seconds)}`}>
                <span style={`height: ${Math.max(8, Math.round(day.seconds / dailyMax * 100))}%`}></span>
                <small>{dayLabel(day.date)}</small>
              </div>
            {/each}
          </div>
        </Card>

        <Card class="panel top-panel">
          <div class="panel-head">
            <div>
              <span class="panel-kicker">Top Games</span>
              <h2>时长排行</h2>
            </div>
          </div>
          <div class="top-list">
            {#each topGames as item, index (item.game_id)}
              <button class="top-row" type="button" onclick={() => openGame(item.game_id)}>
                <span class="rank">{index + 1}</span>
                <span class="cover">
                  {#if coverFor(item.game_id)}
                    <img src={coverFor(item.game_id)!} alt="" loading="lazy" />
                  {:else}
                    <Icon name="gamepad" size={18} />
                  {/if}
                </span>
                <span class="top-info">
                  <b>{item.game_name}</b>
                  <small>{formatDateTime(item.last_played)}</small>
                </span>
                <span class="time">{formatCompactSeconds(item.total_seconds)}</span>
              </button>
            {/each}
          </div>
        </Card>

        <Card class="panel sessions-panel">
          <div class="panel-head">
            <div>
              <span class="panel-kicker">Timeline</span>
              <h2>最近会话</h2>
            </div>
          </div>
          <div class="session-list">
            {#each recentSessions as entry (`${entry.game_id}-${entry.session.id}`)}
              <button class="session-row" type="button" onclick={() => openGame(entry.game_id)}>
                <span class="session-dot"></span>
                <span class="session-main">
                  <b>{entry.game_name}</b>
                  <small>{formatDateTime(entry.session.start_time)}</small>
                </span>
                <span class="time">{formatCompactSeconds(entry.session.duration_seconds)}</span>
              </button>
            {/each}
          </div>
        </Card>

        <Card class="panel month-panel">
          <div class="panel-head">
            <div>
              <span class="panel-kicker">Monthly Rhythm</span>
              <h2>月度节奏</h2>
            </div>
          </div>
          {#if monthlyBars.length > 0}
            <div class="month-bars">
              {#each monthlyBars as month}
                <div class="month-bar">
                  <span style={`width: ${Math.max(6, Math.round(month.seconds / monthlyMax * 100))}%`}></span>
                  <b>{monthLabel(month.month)}</b>
                  <small>{formatCompactSeconds(month.seconds)}</small>
                </div>
              {/each}
            </div>
          {:else}
            <EmptyState title="暂无月度记录" description="启动游戏后，这里会自动沉淀游玩节奏。" />
          {/if}
        </Card>
      </main>
    {:else}
      <Card class="empty-records">
        <EmptyState
          icon="chart"
          title="还没有游玩记录"
          description="导入游戏并从 MoePlay 启动后，这里会自动记录会话、总时长和最近趋势。"
        />
        <div class="empty-actions">
          <Button variant="primary" size="sm" press={() => (uiStore.currentView = "steam-import")}>导入游戏</Button>
          <Button variant="secondary" size="sm" press={() => (uiStore.currentView = "home")}>返回游戏库</Button>
        </div>
      </Card>
    {/if}
  {/if}
</section>

<style>
  .records-page {
    min-width: 0;
    height: 100%;
    padding: 24px;
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 16px;
    background:
      radial-gradient(circle at 15% 0%, rgba(31, 185, 120, 0.14), transparent 34%),
      linear-gradient(155deg, #050806 0%, #08120d 44%, #030504 100%);
  }

  .records-hero {
    position: relative;
    min-height: 224px;
    width: min(1180px, 100%);
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 440px);
    gap: 22px;
    padding: 26px;
    overflow: hidden;
    border: 1px solid rgba(97, 255, 180, 0.12);
    border-radius: 8px;
    background:
      linear-gradient(135deg, rgba(12, 24, 18, 0.96), rgba(8, 12, 10, 0.9)),
      repeating-linear-gradient(90deg, rgba(255,255,255,0.03) 0 1px, transparent 1px 46px);
    box-shadow: 0 24px 80px rgba(0,0,0,0.36);
  }

  .hero-copy {
    min-width: 0;
    display: grid;
    align-content: center;
    gap: 12px;
    z-index: 1;
  }

  .eyebrow,
  .panel-kicker,
  .metric-label {
    color: rgba(133, 242, 186, 0.82);
    font-size: 12px;
    font-weight: 720;
    line-height: 1.2;
  }

  .hero-copy h1,
  .panel-head h2 {
    margin: 0;
    color: var(--text-primary);
    line-height: 1.08;
  }

  .hero-copy h1 {
    font-size: clamp(32px, 5vw, 58px);
    font-weight: 820;
  }

  .hero-copy p {
    max-width: 520px;
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.7;
  }

  .hero-actions {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .hero-actions :global(.ui-button__content) {
    min-width: 0;
  }

  .hero-actions :global(.ui-button span:last-child) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hero-template {
    position: relative;
    min-height: 176px;
    border-radius: 8px;
    overflow: hidden;
    background:
      linear-gradient(135deg, rgba(6, 18, 12, 0.68), rgba(9, 15, 12, 0.38)),
      radial-gradient(circle at 72% 20%, rgba(83, 255, 169, 0.24), transparent 30%);
    border: 1px solid rgba(139, 255, 192, 0.1);
  }

  .template-glow {
    position: absolute;
    inset: -30%;
    background:
      radial-gradient(circle at 62% 42%, rgba(88, 255, 174, 0.25), transparent 20%),
      radial-gradient(circle at 30% 72%, rgba(46, 139, 95, 0.22), transparent 26%);
    filter: blur(8px);
  }

  .template-card {
    position: absolute;
    border: 1px solid rgba(177, 255, 211, 0.16);
    border-radius: 8px;
    background: rgba(5, 13, 9, 0.68);
    box-shadow: inset 0 1px rgba(255,255,255,0.05);
  }

  .template-card-main {
    left: 28px;
    bottom: 24px;
    width: 56%;
    height: 62%;
    padding: 18px;
    display: grid;
    gap: 16px;
  }

  .template-card-side {
    right: 28px;
    top: 24px;
    width: 30%;
    height: 54%;
    padding: 16px;
    display: grid;
    align-content: center;
    justify-items: center;
    gap: 10px;
  }

  .template-line {
    height: 7px;
    width: 58%;
    border-radius: 999px;
    background: rgba(142, 255, 195, 0.18);
  }

  .template-line.wide {
    width: 72%;
  }

  .template-line.short {
    width: 38%;
  }

  .template-bars {
    height: 78px;
    display: flex;
    align-items: end;
    gap: 9px;
  }

  .template-bars span {
    flex: 1;
    min-width: 8px;
    border-radius: 999px 999px 3px 3px;
    background: linear-gradient(180deg, rgba(103, 255, 174, 0.9), rgba(38, 112, 76, 0.55));
  }

  .template-ring {
    width: 58px;
    height: 58px;
    border-radius: 50%;
    border: 10px solid rgba(112, 255, 184, 0.2);
    border-top-color: rgba(112, 255, 184, 0.78);
  }

  .template-pad {
    position: absolute;
    right: 76px;
    bottom: 20px;
    width: 92px;
    height: 52px;
    border-radius: 24px;
    border: 1px solid rgba(177, 255, 211, 0.18);
    background: rgba(4, 11, 8, 0.72);
  }

  .template-pad span,
  .template-pad i,
  .template-pad b {
    position: absolute;
    display: block;
    background: rgba(139, 255, 192, 0.62);
  }

  .template-pad span {
    left: 18px;
    top: 24px;
    width: 22px;
    height: 4px;
    box-shadow: 9px -9px 0 -1px rgba(139, 255, 192, 0.62), 9px 9px 0 -1px rgba(139, 255, 192, 0.62);
  }

  .template-pad i,
  .template-pad b {
    right: 20px;
    top: 18px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .template-pad b {
    right: 34px;
    top: 30px;
  }

  .soft-warning {
    width: min(1180px, 100%);
    min-height: 40px;
    padding: 10px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    color: rgba(214, 236, 222, 0.86);
    border: 1px solid rgba(139, 255, 192, 0.12);
    border-radius: 8px;
    background: rgba(11, 22, 16, 0.72);
  }

  .records-loading,
  .metric-grid,
  .records-layout,
  :global(.ui-card.empty-records) {
    width: min(1180px, 100%);
  }

  .records-loading {
    display: grid;
    gap: 14px;
  }

  .metric-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }

  :global(.ui-card.record-metric) {
    min-width: 0;
    min-height: 118px;
    display: grid;
    align-content: center;
    gap: 8px;
    background: rgba(8, 17, 12, 0.78);
    border-color: rgba(139, 255, 192, 0.12);
  }

  :global(.ui-card.record-metric.total) {
    background: linear-gradient(135deg, rgba(33, 114, 76, 0.22), rgba(8, 17, 12, 0.8));
  }

  :global(.ui-card.record-metric strong) {
    min-width: 0;
    color: var(--text-primary);
    font-size: clamp(22px, 3vw, 34px);
    font-weight: 780;
    line-height: 1.05;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.ui-card.record-metric small) {
    color: var(--text-muted);
  }

  .records-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.45fr) minmax(320px, 0.9fr);
    gap: 12px;
  }

  :global(.ui-card.panel) {
    min-width: 0;
    display: grid;
    gap: 14px;
    background: rgba(8, 17, 12, 0.78);
    border-color: rgba(139, 255, 192, 0.12);
  }

  :global(.ui-card.daily-panel),
  :global(.ui-card.sessions-panel) {
    grid-column: 1;
  }

  :global(.ui-card.top-panel),
  :global(.ui-card.month-panel) {
    grid-column: 2;
  }

  .panel-head {
    min-width: 0;
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 12px;
  }

  .panel-head h2 {
    margin-top: 3px;
    font-size: 18px;
    font-weight: 760;
  }

  .daily-bars {
    height: 184px;
    display: grid;
    grid-template-columns: repeat(14, minmax(0, 1fr));
    gap: 8px;
    align-items: end;
  }

  .day-bar {
    min-width: 0;
    height: 100%;
    display: grid;
    grid-template-rows: 1fr auto;
    gap: 8px;
    align-items: end;
  }

  .day-bar span {
    width: 100%;
    min-height: 8px;
    border-radius: 999px 999px 3px 3px;
    background: linear-gradient(180deg, rgba(104, 255, 178, 0.92), rgba(43, 128, 87, 0.55));
    box-shadow: 0 0 18px rgba(80, 255, 162, 0.12);
  }

  .day-bar small {
    color: var(--text-muted);
    font-size: 10px;
    text-align: center;
  }

  .top-list,
  .session-list,
  .month-bars {
    min-width: 0;
    display: grid;
    gap: 8px;
  }

  .top-row,
  .session-row {
    min-width: 0;
    width: 100%;
    border: 1px solid rgba(255,255,255,0.06);
    border-radius: 8px;
    background: rgba(255,255,255,0.03);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.16s ease, border-color 0.16s ease, transform 0.16s ease;
  }

  .top-row:hover,
  .session-row:hover {
    border-color: rgba(117, 255, 186, 0.22);
    background: rgba(117, 255, 186, 0.07);
    transform: translateY(-1px);
  }

  .top-row {
    display: grid;
    grid-template-columns: 24px 42px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 8px;
    text-align: left;
  }

  .rank {
    color: rgba(133, 242, 186, 0.92);
    font-family: var(--font-mono);
    font-weight: 760;
    text-align: center;
  }

  .cover {
    width: 42px;
    height: 56px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 7px;
    background: rgba(255,255,255,0.06);
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .top-info,
  .session-main {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .top-info b,
  .session-main b {
    min-width: 0;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .top-info small,
  .session-main small,
  .time {
    color: var(--text-muted);
    font-size: 12px;
  }

  .time {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .session-row {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 12px;
    text-align: left;
  }

  .session-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: rgba(109, 255, 181, 0.88);
    box-shadow: 0 0 12px rgba(109, 255, 181, 0.38);
  }

  .month-bar {
    min-width: 0;
    display: grid;
    grid-template-columns: 42px minmax(0, 1fr) 46px;
    gap: 10px;
    align-items: center;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .month-bar span {
    height: 9px;
    min-width: 6px;
    border-radius: 999px;
    background: linear-gradient(90deg, rgba(104, 255, 178, 0.95), rgba(57, 159, 107, 0.48));
  }

  .month-bar b {
    color: var(--text-primary);
    font-weight: 700;
  }

  :global(.ui-card.empty-records) {
    min-height: 300px;
    display: grid;
    place-items: center;
    gap: 14px;
    background: rgba(8, 17, 12, 0.78);
    border-color: rgba(139, 255, 192, 0.12);
  }

  .empty-actions {
    display: flex;
    justify-content: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  @media (max-width: 940px) {
    .records-hero,
    .records-layout {
      grid-template-columns: 1fr;
    }

    :global(.ui-card.daily-panel),
    :global(.ui-card.sessions-panel),
    :global(.ui-card.top-panel),
    :global(.ui-card.month-panel) {
      grid-column: 1;
    }

    .metric-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 620px) {
    .records-page {
      padding: 16px;
    }

    .records-hero {
      padding: 18px;
    }

    .metric-grid {
      grid-template-columns: 1fr;
    }

    .daily-bars {
      gap: 5px;
    }

    .day-bar small {
      display: none;
    }

    .top-row {
      grid-template-columns: 20px 36px minmax(0, 1fr);
    }

    .top-row .time {
      grid-column: 3;
    }
  }
</style>
