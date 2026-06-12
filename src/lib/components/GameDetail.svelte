<script lang="ts">
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { formatPlayTime } from "../api";
  import type { Game } from "../stores/games.svelte";
  import { fileSrc } from "../utils";
  import Button from "./Button.svelte";
  import TagPill from "./TagPill.svelte";
  import RatingRing from "./RatingRing.svelte";
  import StatusBadge from "./StatusBadge.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Icon from "./Icon.svelte";
  import {
    coverOf as gameCoverOf,
    developerOf as gameDeveloperOf,
    gameCompletionStatus,
    gameRating,
    gameTotalSeconds,
    originalNameOf,
    platformOf as gamePlatformOf,
    publisherOf as gamePublisherOf,
    releaseYearOf,
    tagsOf as gameTagsOf,
  } from "../utils/game";

  let game = $derived(gameStore.selectedGame);

  const developerOf = (g: Game) => gameDeveloperOf(g);
  const publisherOf = (g: Game) => gamePublisherOf(g);
  const ratingOf = (g: Game) => gameRating(g);
  const yearOf = (g: Game) => releaseYearOf(g)?.toString() ?? "----";
  const playTimeOf = (g: Game) => formatPlayTime(gameTotalSeconds(g));
  const coverSrcOf = (g: Game) => fileSrc(gameCoverOf(g));
  const tagsOf = (g: Game) => gameTagsOf(g);
  const statusText = (g: Game) => {
    const status = gameCompletionStatus(g);
    if (status === "completed") return "Completed";
    if (status === "playing") return "Playing";
    return "PlanToPlay";
  };
  const monogramOf = (g: Game) => (g.name?.trim()?.[0] ?? "?").toUpperCase();

  async function handleLaunch() {
    if (!game) return;
    await gameStore.launch(game.id);
    uiStore.notify(`正在启动 ${game.name}...`, "info");
  }

  function handleScrape() {
    if (!game) return;
    uiStore.openScrapeDialog(game.id);
  }

  async function handleToggleFav() {
    if (!game) return;
    await gameStore.toggleFavorite(game.id);
  }

  function openDetailPage() {
    uiStore.currentView = "game-detail";
  }
</script>

<aside class="detail-panel glass-card" aria-label="游戏详情">
  {#if game}
    <div class="detail-hero">
      {#if coverSrcOf(game)}
        <img src={coverSrcOf(game)!} alt={game.name} />
      {:else}
        <div class="cover-placeholder"><span>{monogramOf(game)}</span></div>
      {/if}
      <button class="favorite" class:active={game.favorite} onclick={handleToggleFav} aria-label="切换收藏">
        <Icon name={game.favorite ? "heartFill" : "heart"} size={17} />
      </button>
      <StatusBadge status={statusText(game)} />
    </div>

    <div class="title-block">
      {#if originalNameOf(game)}
        <p class="jp-name">{originalNameOf(game)}</p>
      {/if}
      <h2>{game.name}</h2>
      <p>{developerOf(game)}</p>
    </div>

    <div class="quick-row">
      <RatingRing value={ratingOf(game)} max={10} size={58} />
      <div class="meta-stack">
        <span><b>{yearOf(game)}</b><small>发行</small></span>
        <span><b>{playTimeOf(game)}</b><small>时长</small></span>
      </div>
    </div>

    {#if tagsOf(game).length}
      <div class="tags">
        {#each tagsOf(game).slice(0, 5) as tag, index}
          <TagPill label={tag} active={index === 0} />
        {/each}
      </div>
    {/if}

    <p class="description">{game.description || "暂无简介。可使用 AI 刮削补全作品介绍、开发商、标签与截图。"}</p>

    <div class="info-list">
      <div><span>开发商</span><b>{developerOf(game)}</b></div>
      <div><span>发行商</span><b>{publisherOf(game)}</b></div>
      <div><span>平台</span><b>{gamePlatformOf(game)}</b></div>
      <div><span>路径</span><b title={game.exe_path}>{game.exe_path || "未记录"}</b></div>
    </div>

    <div class="actions">
      <Button onclick={handleLaunch}>开始游戏</Button>
      <Button variant="secondary" onclick={openDetailPage}>详情页</Button>
      <Button variant="ghost" onclick={handleScrape}>AI 刮削</Button>
    </div>
  {:else}
    <EmptyState
      title="选择一款游戏"
      description="单击封面卡片后，这里会显示封面、评分、标签与启动操作。"
    />
  {/if}
</aside>

<style>
  .detail-panel {
    min-height: 0;
    overflow: auto;
    border-radius: 0;
    border-width: 0 0 0 1px;
    padding: 18px;
    padding-top: 66px;  /* offset to clear hero area, matching WPF margin-top:48px */
    background: rgba(16, 19, 26, 0.70);
  }

  .detail-hero {
    position: relative;
    aspect-ratio: 16 / 10;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    margin-bottom: 16px;
  }

  .detail-hero img { width: 100%; height: 100%; object-fit: cover; }
  .detail-hero::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, transparent 45%, rgba(8, 11, 18, 0.74));
    pointer-events: none;
  }

  .cover-placeholder {
    width: 100%; height: 100%; display: grid; place-items: center;
    background: linear-gradient(135deg, var(--bg-elev), var(--bg-secondary));
  }
  .cover-placeholder span { font-family: var(--font-display); font-size: 44px; color: var(--text-muted); }

  .favorite {
    position: absolute; top: 10px; right: 10px; z-index: 1;
    width: 32px; height: 32px; border-radius: var(--radius-full);
    border: 1px solid var(--border); background: rgba(10, 13, 20, 0.55);
    color: var(--text-secondary); display: grid; place-items: center; cursor: pointer;
    backdrop-filter: blur(8px);
  }
  .favorite.active { color: var(--accent-pink); }
  :global(.detail-hero .badge) { position: absolute; left: 10px; bottom: 10px; z-index: 1; }

  .title-block { margin-bottom: 14px; }
  .jp-name { font-family: var(--font-jp); color: var(--text-muted); font-size: 13px; margin-bottom: 4px; }
  h2 { margin: 0; font-size: 22px; line-height: 1.18; font-weight: 720; color: var(--text-primary); }
  .title-block p:last-child { margin-top: 6px; color: var(--text-muted); font-size: 12px; }

  .quick-row { display: flex; align-items: center; gap: 16px; margin: 12px 0; }
  .meta-stack { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; flex: 1; }
  .meta-stack span { border: 1px solid var(--border); border-radius: var(--radius-md); padding: 9px 10px; background: rgba(255,255,255,.03); }
  .meta-stack b { display: block; font-family: var(--font-mono); font-variant-numeric: tabular-nums; font-size: 13px; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta-stack small { display: block; margin-top: 2px; color: var(--text-dim, var(--text-muted)); font-size: 10px; }

  .tags { display: flex; flex-wrap: wrap; gap: 7px; margin: 12px 0 14px; }
  .description { color: var(--text-secondary); line-height: 1.65; font-size: 13px; display: -webkit-box; -webkit-line-clamp: 5; line-clamp: 5; -webkit-box-orient: vertical; overflow: hidden; margin-bottom: 14px; }

  .info-list { border-top: 1px solid var(--border); margin-bottom: 16px; }
  .info-list div { display: grid; grid-template-columns: 70px minmax(0, 1fr); gap: 10px; padding: 9px 0; border-bottom: 1px solid var(--border); font-size: 12px; }
  .info-list span { color: var(--text-muted); }
  .info-list b { color: var(--text-secondary); font-weight: 520; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .actions { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .actions :global(.btn:first-child) { grid-column: 1 / -1; }
</style>
