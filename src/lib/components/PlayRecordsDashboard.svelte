<script lang="ts">
  import { onMount } from "svelte";
  import { animeStore, type AnimeHistory } from "../stores/anime.svelte";
  import { comicStore, type ReadRecord } from "../stores/comic.svelte";
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
  const mediaActivities = $derived(buildMediaActivities(activeSummary.recent_sessions, animeStore.history, comicStore.readHistory));
  const recentActivityCount = $derived(countRecentActivities(mediaActivities, 14));
  const hasRecords = $derived(activeSummary.session_count > 0 || activeSummary.total_seconds > 0 || animeStore.history.length > 0 || comicStore.readHistory.length > 0);
  const topGames = $derived(activeSummary.top_games.slice(0, 5));
  const recentSessions = $derived(activeSummary.recent_sessions.slice(0, 7));
  const dailyBars = $derived(fillDailyBars(activeSummary.daily, 14));
  const activityBars = $derived(fillActivityBars(mediaActivities, 14));
  const monthlyBars = $derived(activeSummary.monthly.slice(-8));
  const dailyMax = $derived(Math.max(1, ...dailyBars.map(d => d.seconds)));
  const activityMax = $derived(Math.max(1, ...activityBars.map(d => d.count)));
  const monthlyMax = $derived(Math.max(1, ...monthlyBars.map(m => m.seconds)));
  const mediaCounts = $derived(countMediaKinds(mediaActivities));
  const mediaTotal = $derived(Math.max(1, mediaCounts.game + mediaCounts.anime + mediaCounts.comic));
  const continueItems = $derived(mediaActivities.slice(0, 5));
  const animeRecent = $derived(animeStore.history.slice(0, 4));
  const comicRecent = $derived(comicStore.readHistory.slice(0, 4));
  const lastPlayedGame = $derived(findGame(topGames[0]?.game_id));
  const latestActivity = $derived(mediaActivities[0] ?? null);

  type MediaActivity = {
    id: string;
    kind: "game" | "anime" | "comic";
    title: string;
    subtitle: string;
    timeLabel: string;
    timestamp: number;
    payload?: PlaySessionEntry | AnimeHistory | ReadRecord;
  };

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

  function buildMediaActivities(
    sessions: PlaySessionEntry[],
    animeHistory: AnimeHistory[],
    comicHistory: ReadRecord[],
  ): MediaActivity[] {
    const gameItems = sessions.map((entry) => ({
      id: `game:${entry.game_id}:${entry.session.id}`,
      kind: "game" as const,
      title: entry.game_name,
      subtitle: `游玩 ${formatCompactSeconds(entry.session.duration_seconds)}`,
      timeLabel: formatDateTime(entry.session.start_time),
      timestamp: toTs(entry.session.start_time),
      payload: entry,
    }));
    const animeItems = animeHistory.map((entry) => ({
      id: `anime:${entry.key}`,
      kind: "anime" as const,
      title: entry.name,
      subtitle: `看到 ${entry.lastEpisodeName || `第 ${entry.lastEpisode + 1} 集`}`,
      timeLabel: formatDateTime(entry.updatedAt),
      timestamp: toTs(entry.updatedAt),
      payload: entry,
    }));
    const comicItems = comicHistory.map((entry) => ({
      id: `comic:${entry.id}`,
      kind: "comic" as const,
      title: entry.title,
      subtitle: `读到 ${entry.last_title || `第 ${entry.last_order} 话`}`,
      timeLabel: formatDateTime(new Date(entry.ts).toISOString()),
      timestamp: entry.ts || 0,
      payload: entry,
    }));

    return [...gameItems, ...animeItems, ...comicItems]
      .filter(item => item.timestamp > 0)
      .sort((a, b) => b.timestamp - a.timestamp);
  }

  function fillActivityBars(items: MediaActivity[], count: number) {
    const byDate = new Map<string, number>();
    for (const item of items) {
      const key = dateKey(new Date(item.timestamp).toISOString());
      byDate.set(key, (byDate.get(key) ?? 0) + 1);
    }
    const list: { date: string; count: number }[] = [];
    const now = new Date();
    for (let i = count - 1; i >= 0; i--) {
      const d = new Date(now);
      d.setDate(now.getDate() - i);
      const key = dateKey(d.toISOString());
      list.push({ date: key, count: byDate.get(key) ?? 0 });
    }
    return list;
  }

  function countRecentActivities(items: MediaActivity[], days: number): number {
    const since = Date.now() - days * 24 * 60 * 60 * 1000;
    return items.filter(item => item.timestamp >= since).length;
  }

  function countMediaKinds(items: MediaActivity[]) {
    return items.reduce(
      (acc, item) => {
        acc[item.kind] += 1;
        return acc;
      },
      { game: 0, anime: 0, comic: 0 },
    );
  }

  function toTs(value: string | undefined): number {
    if (!value) return 0;
    const ts = new Date(value).getTime();
    return Number.isNaN(ts) ? 0 : ts;
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

  function activityCover(item: MediaActivity): string | null {
    if (item.kind === "game") return coverFor((item.payload as PlaySessionEntry | undefined)?.game_id);
    if (item.kind === "anime") return (item.payload as AnimeHistory | undefined)?.image ?? null;
    return (item.payload as ReadRecord | undefined)?.thumb_url ?? null;
  }

  function kindLabel(kind: MediaActivity["kind"]): string {
    if (kind === "game") return "游戏";
    if (kind === "anime") return "番剧";
    return "漫画";
  }

  function kindIcon(kind: MediaActivity["kind"]): "gamepad" | "film" | "book" {
    return kind === "game" ? "gamepad" : kind === "anime" ? "film" : "book";
  }

  function animeActivity(entry: AnimeHistory): MediaActivity {
    return {
      id: `anime:${entry.key}`,
      kind: "anime",
      title: entry.name,
      subtitle: `看到 ${entry.lastEpisodeName || `第 ${entry.lastEpisode + 1} 集`}`,
      timeLabel: formatDateTime(entry.updatedAt),
      timestamp: toTs(entry.updatedAt),
      payload: entry,
    };
  }

  function comicActivity(entry: ReadRecord): MediaActivity {
    return {
      id: `comic:${entry.id}`,
      kind: "comic",
      title: entry.title,
      subtitle: `读到 ${entry.last_title || `第 ${entry.last_order} 话`}`,
      timeLabel: formatDateTime(new Date(entry.ts).toISOString()),
      timestamp: entry.ts || 0,
      payload: entry,
    };
  }

  function percent(value: number, total: number): number {
    return Math.max(4, Math.round((value / Math.max(1, total)) * 100));
  }

  function openGame(gameId: string | undefined) {
    const game = findGame(gameId);
    if (!game) return;
    gameStore.selectGame(game.id);
    uiStore.currentView = "game-detail";
  }

  async function openActivity(item: MediaActivity) {
    if (item.kind === "game") {
      openGame((item.payload as PlaySessionEntry | undefined)?.game_id);
      return;
    }
    if (item.kind === "anime") {
      uiStore.currentView = "anime";
      await animeStore.resumeHistory(item.payload as AnimeHistory);
      return;
    }
    uiStore.currentView = "comic";
    await comicStore.resumeHistory(item.payload as ReadRecord);
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
      <p>把最近玩了什么、看了什么、读到哪里，整理成一个安静的综合记录仪表盘。</p>
      <div class="hero-meta" aria-label="记录概览">
        <span><Icon name="gamepad" size={14} /> {activeSummary.session_count} 次游戏会话</span>
        <span><Icon name="film" size={14} /> {animeStore.history.length} 条番剧历史</span>
        <span><Icon name="book" size={14} /> {comicStore.readHistory.length} 条漫画历史</span>
      </div>
      <div class="hero-actions">
        {#if latestActivity}
          <Button variant="primary" size="sm" press={() => openActivity(latestActivity)}>
            <Icon name={latestActivity.kind === "game" ? "play" : latestActivity.kind === "anime" ? "film" : "book"} size={16} />
            <span>继续 {latestActivity.title}</span>
          </Button>
        {:else if lastPlayedGame}
          <Button variant="primary" size="sm" press={() => launchGame(lastPlayedGame?.id)}>
            <Icon name="play" size={16} />
            <span>继续 {lastPlayedGame.name}</span>
          </Button>
        {/if}
        {#if lastPlayedGame}
          <Button variant="secondary" size="sm" press={() => openGame(lastPlayedGame?.id)}>
            <Icon name="info" size={16} />
            <span>查看详情</span>
          </Button>
        {:else if !latestActivity}
          <Button variant="primary" size="sm" press={() => (uiStore.currentView = "steam-import")}>
            <Icon name="database" size={16} />
            <span>导入游戏</span>
          </Button>
        {/if}
      </div>
    </div>

    <div class="hero-console" aria-label="最近继续">
      <div class="console-head">
        <span>最近继续</span>
        {#if latestActivity}<b>{kindLabel(latestActivity.kind)}</b>{/if}
      </div>
      {#if continueItems.length > 0}
        <div class="continue-stack">
          {#each continueItems.slice(0, 3) as item (item.id)}
            <button class="continue-card kind-{item.kind}" type="button" onclick={() => openActivity(item)}>
              <span class="continue-cover">
                {#if activityCover(item)}
                  <img src={activityCover(item)!} alt="" loading="lazy" />
                {:else}
                  <Icon name={kindIcon(item.kind)} size={18} />
                {/if}
              </span>
              <span class="continue-main">
                <small>{kindLabel(item.kind)} · {item.timeLabel}</small>
                <b>{item.title}</b>
                <em>{item.subtitle}</em>
              </span>
            </button>
          {/each}
        </div>
      {:else}
        <div class="console-empty">
          <Icon name="chart" size={22} />
          <span>开始游玩、追番或阅读后会出现在这里</span>
        </div>
      {/if}
      <div class="media-mix" aria-label="媒体占比">
        <span class="mix-game" style={`width: ${percent(mediaCounts.game, mediaTotal)}%`}></span>
        <span class="mix-anime" style={`width: ${percent(mediaCounts.anime, mediaTotal)}%`}></span>
        <span class="mix-comic" style={`width: ${percent(mediaCounts.comic, mediaTotal)}%`}></span>
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
        <span class="metric-label">番剧历史</span>
        <strong>{animeStore.history.length}</strong>
        <small>{animeStore.history[0]?.lastEpisodeName ?? "开始观看后自动记录"}</small>
      </Card>
      <Card class="record-metric">
        <span class="metric-label">漫画历史</span>
        <strong>{comicStore.readHistory.length}</strong>
        <small>{comicStore.readHistory[0]?.last_title ?? "开始阅读后自动记录"}</small>
      </Card>
      <Card class="record-metric">
        <span class="metric-label">近 14 天活动</span>
        <strong>{recentActivityCount}</strong>
        <small>游戏 / 番剧 / 漫画合计</small>
      </Card>
    </section>

    {#if continueItems.length > 0}
      <section class="quick-continue" aria-label="继续项目">
        {#each continueItems as item (item.id)}
          <button class="quick-item kind-{item.kind}" type="button" onclick={() => openActivity(item)}>
            <span class="quick-icon"><Icon name={kindIcon(item.kind)} size={15} /></span>
            <span class="quick-text">
              <b>{item.title}</b>
              <small>{item.subtitle}</small>
            </span>
            <span class="quick-time">{item.timeLabel}</span>
          </button>
        {/each}
      </section>
    {/if}

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

        <Card class="panel media-panel">
          <div class="panel-head">
            <div>
              <span class="panel-kicker">Media Mix</span>
              <h2>媒体记录</h2>
            </div>
            <Tag variant="muted">{mediaActivities.length} 条</Tag>
          </div>
          <div class="media-stats">
            <div class="media-stat">
              <span class="kind-dot game"></span>
              <b>{mediaCounts.game}</b>
              <small>游戏</small>
            </div>
            <div class="media-stat">
              <span class="kind-dot anime"></span>
              <b>{mediaCounts.anime}</b>
              <small>番剧</small>
            </div>
            <div class="media-stat">
              <span class="kind-dot comic"></span>
              <b>{mediaCounts.comic}</b>
              <small>漫画</small>
            </div>
          </div>
          <div class="media-split">
            <div>
              <span class="split-title">最近番剧</span>
              {#if animeRecent.length > 0}
                {#each animeRecent as item (item.key)}
                  <button class="mini-media-row" type="button" onclick={() => openActivity(animeActivity(item))}>
                    <span>{item.name}</span>
                    <small>{item.lastEpisodeName || `第 ${item.lastEpisode + 1} 集`}</small>
                  </button>
                {/each}
              {:else}
                <p>暂无番剧观看记录</p>
              {/if}
            </div>
            <div>
              <span class="split-title">最近漫画</span>
              {#if comicRecent.length > 0}
                {#each comicRecent as item (item.id)}
                  <button class="mini-media-row" type="button" onclick={() => openActivity(comicActivity(item))}>
                    <span>{item.title}</span>
                    <small>{item.last_title || `第 ${item.last_order} 话`}</small>
                  </button>
                {/each}
              {:else}
                <p>暂无漫画阅读记录</p>
              {/if}
            </div>
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

        <Card class="panel activity-panel">
          <div class="panel-head">
            <div>
              <span class="panel-kicker">All Media</span>
              <h2>综合时间线</h2>
            </div>
            <Tag variant="muted">{mediaActivities.length} 条</Tag>
          </div>
          <div class="activity-list">
            {#each mediaActivities.slice(0, 8) as item (item.id)}
              <button class="activity-row" type="button" onclick={() => openActivity(item)}>
                <span class="activity-kind kind-{item.kind}">
                  <Icon name={item.kind === "game" ? "gamepad" : item.kind === "anime" ? "film" : "book"} size={15} />
                </span>
                <span class="activity-main">
                  <b>{item.title}</b>
                  <small>{item.subtitle}</small>
                </span>
                <span class="activity-time">{item.timeLabel}</span>
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

        <Card class="panel rhythm-panel">
          <div class="panel-head">
            <div>
              <span class="panel-kicker">Activity Rhythm</span>
              <h2>综合活跃度</h2>
            </div>
          </div>
          <div class="activity-bars" aria-label="最近两周综合活动">
            {#each activityBars as day}
              <div class="activity-bar" title={`${dayLabel(day.date)} · ${day.count} 次活动`}>
                <span style={`height: ${Math.max(day.count ? 14 : 4, Math.round(day.count / activityMax * 100))}%`}></span>
                <small>{dayLabel(day.date)}</small>
              </div>
            {/each}
          </div>
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
    --records-max: min(1680px, calc(100vw - 48px));
    min-width: 0;
    height: 100%;
    padding: clamp(16px, 2vw, 28px);
    overflow: auto;
    display: grid;
    justify-items: center;
    align-content: start;
    gap: clamp(12px, 1.4vw, 18px);
    background:
      repeating-linear-gradient(90deg, rgba(116, 255, 184, 0.035) 0 1px, transparent 1px 76px),
      linear-gradient(160deg, #050806 0%, #07100b 42%, #020403 100%);
  }

  .records-hero {
    position: relative;
    min-height: clamp(176px, 20dvh, 238px);
    width: var(--records-max);
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 34%);
    gap: clamp(16px, 2vw, 28px);
    padding: clamp(18px, 2vw, 28px);
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
    gap: 10px;
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
    font-size: clamp(30px, 3.8vw, 56px);
    font-weight: 820;
  }

  .hero-copy p {
    max-width: 620px;
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

  .hero-meta {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .hero-meta span {
    height: 28px;
    padding: 0 10px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid rgba(139, 255, 192, 0.1);
    border-radius: 999px;
    background: rgba(255,255,255,0.035);
    color: rgba(214, 236, 222, 0.82);
    font-size: 12px;
    white-space: nowrap;
  }

  .hero-actions :global(.ui-button__content) {
    min-width: 0;
  }

  .hero-actions :global(.ui-button span:last-child) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hero-console {
    min-width: 0;
    min-height: 166px;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid rgba(139, 255, 192, 0.12);
    background:
      linear-gradient(145deg, rgba(8, 18, 13, 0.92), rgba(5, 10, 8, 0.72)),
      radial-gradient(circle at 84% 16%, rgba(88, 255, 174, 0.16), transparent 34%);
    box-shadow: inset 0 1px rgba(255,255,255,0.05);
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 10px;
    padding: 12px;
  }

  .console-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    color: rgba(214, 236, 222, 0.8);
    font-size: 12px;
  }

  .console-head b {
    color: rgba(133, 242, 186, 0.92);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .continue-stack {
    min-width: 0;
    display: grid;
    gap: 8px;
  }

  .continue-card {
    min-width: 0;
    width: 100%;
    border: 1px solid rgba(255,255,255,0.06);
    border-radius: 8px;
    background: rgba(255,255,255,0.035);
    color: var(--text-secondary);
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    padding: 8px;
    text-align: left;
    cursor: pointer;
    transition: transform 0.16s ease, border-color 0.16s ease, background 0.16s ease;
  }

  .continue-card:hover {
    transform: translateY(-1px);
    border-color: rgba(117, 255, 186, 0.2);
    background: rgba(117, 255, 186, 0.07);
  }

  .continue-card:active,
  .quick-item:active,
  .mini-media-row:active {
    transform: scale(0.985);
  }

  .continue-cover {
    width: 44px;
    height: 54px;
    border-radius: 7px;
    overflow: hidden;
    display: grid;
    place-items: center;
    background: rgba(255,255,255,0.06);
    color: rgba(133, 242, 186, 0.9);
  }

  .continue-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .continue-main {
    min-width: 0;
    display: grid;
    gap: 3px;
  }

  .continue-main small,
  .continue-main em {
    min-width: 0;
    color: var(--text-muted);
    font-size: 11px;
    font-style: normal;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .continue-main b {
    min-width: 0;
    color: var(--text-primary);
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .console-empty {
    min-height: 96px;
    display: grid;
    place-items: center;
    gap: 8px;
    color: var(--text-muted);
    text-align: center;
    font-size: 12px;
  }

  .media-mix {
    height: 8px;
    display: flex;
    gap: 3px;
  }

  .media-mix span {
    min-width: 4px;
    border-radius: 999px;
  }

  .mix-game { background: rgba(104, 255, 178, 0.86); }
  .mix-anime { background: rgba(139, 184, 255, 0.78); }
  .mix-comic { background: rgba(245, 188, 119, 0.82); }

  .soft-warning {
    width: var(--records-max);
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
  .quick-continue,
  .records-layout,
  :global(.ui-card.empty-records) {
    width: var(--records-max);
  }

  .records-loading {
    display: grid;
    gap: 14px;
  }

  .metric-grid {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 12px;
  }

  :global(.ui-card.record-metric) {
    min-width: 0;
    min-height: clamp(96px, 11dvh, 124px);
    display: grid;
    align-content: center;
    gap: 8px;
    background: rgba(8, 17, 12, 0.78);
    border-color: rgba(139, 255, 192, 0.12);
  }

  :global(.ui-card.record-metric) {
    grid-column: span 3;
  }

  :global(.ui-card.record-metric.total) {
    grid-column: span 3;
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

  .quick-continue {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 8px;
  }

  .quick-item {
    min-width: 0;
    min-height: 64px;
    border: 1px solid rgba(139, 255, 192, 0.1);
    border-radius: 8px;
    background: rgba(8, 17, 12, 0.64);
    color: var(--text-secondary);
    display: grid;
    grid-template-columns: 32px minmax(0, 1fr) auto;
    gap: 9px;
    align-items: center;
    padding: 9px 10px;
    text-align: left;
    cursor: pointer;
    transition: transform 0.16s ease, border-color 0.16s ease, background 0.16s ease;
  }

  .quick-item:hover {
    transform: translateY(-1px);
    border-color: rgba(117, 255, 186, 0.22);
    background: rgba(117, 255, 186, 0.07);
  }

  .quick-icon {
    width: 32px;
    height: 32px;
    border: 1px solid rgba(255,255,255,0.06);
    border-radius: 8px;
    display: grid;
    place-items: center;
    background: rgba(255,255,255,0.04);
  }

  .quick-item.kind-game .quick-icon { color: rgba(133, 242, 186, 0.92); }
  .quick-item.kind-anime .quick-icon { color: rgba(139, 184, 255, 0.92); }
  .quick-item.kind-comic .quick-icon { color: rgba(245, 188, 119, 0.92); }

  .quick-text {
    min-width: 0;
    display: grid;
    gap: 3px;
  }

  .quick-text b,
  .quick-text small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quick-text b {
    color: var(--text-primary);
    font-size: 13px;
  }

  .quick-text small,
  .quick-time {
    color: var(--text-muted);
    font-size: 11px;
  }

  .quick-time {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .records-layout {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    grid-auto-flow: dense;
    align-items: stretch;
    gap: 12px;
  }

  :global(.ui-card.panel) {
    min-width: 0;
    min-height: 0;
    display: grid;
    gap: 14px;
    background: rgba(8, 17, 12, 0.78);
    border-color: rgba(139, 255, 192, 0.12);
  }

  :global(.ui-card.daily-panel),
  :global(.ui-card.activity-panel),
  :global(.ui-card.sessions-panel) {
    grid-column: span 7;
  }

  :global(.ui-card.top-panel),
  :global(.ui-card.rhythm-panel),
  :global(.ui-card.month-panel),
  :global(.ui-card.media-panel) {
    grid-column: span 5;
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
    height: clamp(132px, 18dvh, 210px);
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
  .activity-list,
  .session-list,
  .month-bars {
    min-width: 0;
    display: grid;
    gap: 8px;
  }

  .top-row,
  .activity-row,
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
  .activity-row:hover,
  .session-row:hover {
    border-color: rgba(117, 255, 186, 0.22);
    background: rgba(117, 255, 186, 0.07);
    transform: translateY(-1px);
  }

  .top-row {
    display: grid;
    grid-template-columns: 24px 40px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 7px 8px;
    text-align: left;
  }

  .rank {
    color: rgba(133, 242, 186, 0.92);
    font-family: var(--font-mono);
    font-weight: 760;
    text-align: center;
  }

  .cover {
    width: 40px;
    height: 52px;
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
    padding: 11px 12px;
    text-align: left;
  }

  .activity-row {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 10px 12px;
    text-align: left;
  }

  .activity-kind {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    border: 1px solid rgba(255,255,255,0.06);
    background: rgba(255,255,255,0.04);
  }

  .activity-kind.kind-game {
    color: rgba(133, 242, 186, 0.92);
  }

  .activity-kind.kind-anime {
    color: rgba(139, 184, 255, 0.92);
  }

  .activity-kind.kind-comic {
    color: rgba(245, 188, 119, 0.92);
  }

  .activity-main {
    min-width: 0;
    display: grid;
    gap: 3px;
  }

  .activity-main b {
    min-width: 0;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-main small,
  .activity-time {
    color: var(--text-muted);
    font-size: 12px;
  }

  .activity-time {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
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

  .activity-bars {
    height: clamp(112px, 15dvh, 168px);
    display: grid;
    grid-template-columns: repeat(14, minmax(0, 1fr));
    gap: 7px;
    align-items: end;
  }

  .activity-bar {
    min-width: 0;
    height: 100%;
    display: grid;
    grid-template-rows: 1fr auto;
    gap: 8px;
    align-items: end;
  }

  .activity-bar span {
    width: 100%;
    min-height: 4px;
    border-radius: 999px 999px 3px 3px;
    background: linear-gradient(180deg, rgba(160, 211, 255, 0.86), rgba(80, 255, 162, 0.52));
  }

  .activity-bar small {
    color: var(--text-muted);
    font-size: 10px;
    text-align: center;
  }

  .media-stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .media-stat {
    min-width: 0;
    border: 1px solid rgba(255,255,255,0.06);
    border-radius: 8px;
    background: rgba(255,255,255,0.03);
    padding: 10px;
    display: grid;
    gap: 4px;
  }

  .media-stat b {
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 22px;
    line-height: 1;
  }

  .media-stat small {
    color: var(--text-muted);
    font-size: 11px;
  }

  .kind-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
  }

  .kind-dot.game { background: rgba(104, 255, 178, 0.88); }
  .kind-dot.anime { background: rgba(139, 184, 255, 0.86); }
  .kind-dot.comic { background: rgba(245, 188, 119, 0.86); }

  .media-split {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .media-split > div {
    min-width: 0;
    display: grid;
    align-content: start;
    gap: 6px;
  }

  .split-title {
    color: rgba(214, 236, 222, 0.76);
    font-size: 12px;
    font-weight: 720;
  }

  .mini-media-row {
    min-width: 0;
    width: 100%;
    border: 1px solid transparent;
    border-radius: 7px;
    background: rgba(255,255,255,0.025);
    color: var(--text-secondary);
    display: grid;
    gap: 2px;
    padding: 7px 8px;
    text-align: left;
    cursor: pointer;
    transition: transform 0.16s ease, border-color 0.16s ease, background 0.16s ease;
  }

  .mini-media-row:hover {
    transform: translateY(-1px);
    border-color: rgba(117, 255, 186, 0.18);
    background: rgba(117, 255, 186, 0.06);
  }

  .mini-media-row span,
  .mini-media-row small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mini-media-row span {
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 650;
  }

  .mini-media-row small,
  .media-split p {
    color: var(--text-muted);
    font-size: 11px;
    margin: 0;
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
    .records-page {
      --records-max: calc(100vw - 32px);
    }

    .records-hero,
    .records-layout {
      grid-template-columns: 1fr;
    }

    :global(.ui-card.daily-panel),
    :global(.ui-card.activity-panel),
    :global(.ui-card.sessions-panel),
    :global(.ui-card.top-panel),
    :global(.ui-card.rhythm-panel),
    :global(.ui-card.month-panel),
    :global(.ui-card.media-panel) {
      grid-column: 1;
    }

    .metric-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .quick-continue {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    :global(.ui-card.record-metric),
    :global(.ui-card.record-metric.total) {
      grid-column: auto;
    }
  }

  @media (max-width: 620px) {
    .records-page {
      --records-max: calc(100vw - 32px);
      padding: 16px;
    }

    .records-hero {
      padding: 18px;
    }

    .metric-grid {
      grid-template-columns: 1fr;
    }

    .quick-continue,
    .media-split {
      grid-template-columns: 1fr;
    }

    .quick-item {
      grid-template-columns: 32px minmax(0, 1fr);
    }

    .quick-time {
      grid-column: 2;
    }

    .daily-bars {
      gap: 5px;
    }

    .activity-bars {
      gap: 5px;
    }

    .day-bar small,
    .activity-bar small {
      display: none;
    }

    .top-row {
      grid-template-columns: 20px 36px minmax(0, 1fr);
    }

    .top-row .time {
      grid-column: 3;
    }
  }

  @media (min-width: 1440px) {
    .records-page {
      --records-max: min(1760px, calc(100vw - 64px));
    }

    .records-layout {
      grid-template-columns: repeat(16, minmax(0, 1fr));
    }

    :global(.ui-card.daily-panel) {
      grid-column: span 10;
    }

    :global(.ui-card.activity-panel) {
      grid-column: span 9;
    }

    :global(.ui-card.top-panel) {
      grid-column: span 6;
    }

    :global(.ui-card.sessions-panel) {
      grid-column: span 9;
    }

    :global(.ui-card.rhythm-panel) {
      grid-column: span 7;
    }

    :global(.ui-card.month-panel) {
      grid-column: span 7;
    }

    :global(.ui-card.media-panel) {
      grid-column: span 6;
    }

    .top-list,
    .session-list {
      gap: 7px;
    }
  }

  @media (min-width: 1280px) and (max-height: 820px) {
    .records-page {
      gap: 12px;
      padding-block: 16px;
    }

    .records-hero {
      min-height: 154px;
    }

    .hero-console {
      min-height: 124px;
    }

    :global(.ui-card.record-metric) {
      min-height: 88px;
    }

    .daily-bars {
      height: 126px;
    }

    .activity-bars {
      height: 106px;
    }

    .records-layout {
      gap: 10px;
    }
  }
</style>
