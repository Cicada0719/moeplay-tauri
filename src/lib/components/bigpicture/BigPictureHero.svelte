<script lang="ts">
  import Icon from "../Icon.svelte";
  import RatingRing from "../RatingRing.svelte";
  import { formatPlayTime } from "../../api";
  import {
    developerOf,
    gameCompletionStatus,
    gameLastPlayed,
    gameRating,
    gameTotalSeconds,
    releaseYearOf,
    tagsOf as gameTagsOf,
  } from "../../utils/game";
  import type { Game } from "../../stores/games.svelte";

  let {
    game,
    weekHours,
    onLaunch,
    onFavorite,
    onDetail,
  }: {
    game: Game;
    weekHours: string;
    onLaunch: () => void;
    onFavorite: () => void;
    onDetail: () => void;
  } = $props();

  const STATUS: Record<string, string> = {
    not_started: "未开始", playing: "游玩中", completed: "已通关",
    on_hold: "搁置", dropped: "已弃坑", plan_to_play: "计划中", replaying: "重温中",
  };

  const scoreValue = $derived(Math.round(Math.min(10, Math.max(0, gameRating(game))) * 10));
  const allTags = $derived(gameTagsOf(game));
  const achTotal = $derived(game.play_tracker?.achievements_total ?? 0);
  const achDone = $derived(game.play_tracker?.achievements_unlocked ?? 0);

  function metaLine(g: Game): string {
    const parts: string[] = [];
    const dev = developerOf(g);
    if (dev) parts.push(dev);
    const year = releaseYearOf(g);
    if (year) parts.push(String(year));
    const st = STATUS[gameCompletionStatus(g)];
    if (st) parts.push(st);
    const secs = gameTotalSeconds(g);
    if (secs > 0) parts.push(formatPlayTime(secs));
    return parts.join("  ·  ");
  }

  function timeAgo(v: string | null | undefined): string {
    if (!v) return "尚未游玩";
    const days = Math.floor((Date.now() - new Date(v).getTime()) / 86400000);
    if (days <= 0) return "今天";
    if (days === 1) return "昨天";
    if (days < 7) return `${days} 天前`;
    if (days < 30) return `${Math.floor(days / 7)} 周前`;
    return `${Math.floor(days / 30)} 个月前`;
  }

  const desc = $derived(
    game.description?.trim() || allTags.slice(0, 6).join(" / ") || "暂无简介"
  );
  const trimmedDesc = $derived(desc.length > 180 ? desc.slice(0, 180) + "…" : desc);
</script>

<div class="bp-hero">
  {#if game.metadata?.original_name}
    <p class="bp-jp">{game.metadata.original_name}</p>
  {/if}
  <h1 class="bp-title">{game.name}</h1>
  <p class="bp-meta">{metaLine(game)}</p>

  <div class="bp-actions">
    <button class="bp-play" onclick={onLaunch}>
      <Icon name="play" size={22} /><span>开始游戏</span>
    </button>
    <button class="bp-secondary" class:active={game.favorite} onclick={onFavorite}>
      <Icon name={game.favorite ? "heartFill" : "heart"} size={18} />
      <span>{game.favorite ? "已收藏" : "收藏"}</span>
    </button>
    <button class="bp-secondary" onclick={onDetail}>
      <Icon name="database" size={18} /><span>详情</span>
    </button>
  </div>

  <div class="bp-tags">
    {#each allTags.slice(0, 7) as t}
      <span class="bp-tag">{t}</span>
    {/each}
  </div>
  <p class="bp-desc">{trimmedDesc}</p>

  <div class="bp-stats-row">
    <div class="bp-stat">
      <RatingRing value={scoreValue} max={100} size={52} />
    </div>
    <div class="bp-stat">
      <strong>{achTotal > 0 ? `${achDone}/${achTotal}` : "—"}</strong>
      <span>成就</span>
    </div>
    <div class="bp-stat">
      <strong>{formatPlayTime(gameTotalSeconds(game))}</strong>
      <span>时长</span>
    </div>
    <div class="bp-stat">
      <strong>{timeAgo(gameLastPlayed(game))}</strong>
      <span>最后</span>
    </div>
    <div class="bp-stat">
      <strong>{weekHours}h</strong>
      <span>本周</span>
    </div>
  </div>
</div>

<style>
  .bp-hero {
    max-width: 64%;
    max-height: 100%;
    text-align: left;
    margin-top: auto;
    padding-bottom: 4px;
  }
  .bp-jp { color: var(--text-muted); font-size: 13px; margin: 0 0 3px; }
  .bp-title {
    font-family: var(--font-display);
    font-size: clamp(26px, 3.4vw, 44px);
    font-weight: 800; line-height: 1.08; margin: 0 0 6px;
    text-shadow: 0 2px 24px rgba(0,0,0,0.5);
  }
  .bp-meta { color: var(--text-secondary); font-size: 13px; margin: 0 0 10px; }

  .bp-actions { display: flex; gap: 10px; margin-bottom: 10px; flex-wrap: wrap; }
  .bp-play {
    display: inline-flex; align-items: center; gap: 10px;
    border: none; cursor: pointer;
    background: var(--accent); color: #fff;
    font-size: 14px; font-weight: 700;
    padding: 10px 22px; border-radius: var(--radius-full);
    transition: transform 0.15s ease, background 0.18s ease;
  }
  .bp-play:hover { background: var(--accent-hi); transform: translateY(-1px); }
  .bp-secondary {
    display: inline-flex; align-items: center; gap: 8px;
    border: 1px solid var(--border-hover); cursor: pointer;
    background: rgba(7,9,15,0.45); color: var(--text-secondary);
    font-size: 13px; font-weight: 600;
    padding: 10px 16px; border-radius: var(--radius-full);
    backdrop-filter: blur(6px);
    transition: color 0.18s ease, border-color 0.18s ease;
  }
  .bp-secondary:hover { color: var(--text-primary); border-color: var(--text-muted); }
  .bp-secondary.active { color: var(--accent); }

  .bp-tags { display: flex; gap: 7px; flex-wrap: wrap; margin-bottom: 8px; max-width: 580px; }
  .bp-tag {
    font-size: 11px; padding: 3px 10px; border-radius: var(--radius-full);
    background: rgba(255,255,255,0.08); color: var(--text-secondary);
  }
  .bp-desc {
    max-width: 560px; color: var(--text-secondary);
    font-size: 12.5px; line-height: 1.55; margin: 0 0 10px;
  }

  .bp-stats-row {
    display: flex; gap: 14px; align-items: center;
    padding: 8px 14px;
    background: rgba(15,19,28,0.55);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    backdrop-filter: blur(10px);
    max-width: fit-content;
    flex-wrap: wrap;
  }
  .bp-stat {
    display: flex; flex-direction: column; align-items: center; gap: 1px;
    min-width: 48px;
  }
  .bp-stat strong { font-size: 14px; font-weight: 700; color: var(--text-primary); }
  .bp-stat span { font-size: 10.5px; color: var(--text-muted); }
</style>
