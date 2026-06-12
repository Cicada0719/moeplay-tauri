<script lang="ts">
  import type { Game } from "../stores/games.svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { openPath, pickDirectory } from "../api";
  import Icon from "./Icon.svelte";
  import Button from "./Button.svelte";
  import RatingRing from "./RatingRing.svelte";
  import {
    formatPlayTime,
    completionStatusLabel,
  } from "../api";
  import { fileSrc } from "../utils";
  import {
    coverOf,
    developerOf,
    gameLastPlayed,
    gameRating,
    gameTotalSeconds,
    originalNameOf,
    platformOf,
    publisherOf,
    releaseYearOf,
    tagsOf,
  } from "../utils/game";

  let { game, onClose }: { game: Game; onClose: () => void } = $props();

  let busy = $state<string | null>(null);
  let confirmRemove = $state(false);

  const rating = $derived(gameRating(game));
  const status = $derived(game.play_tracker?.completion_status);
  const lastPlayed = $derived(gameLastPlayed(game));
  const playTime = $derived(formatPlayTime(gameTotalSeconds(game)));
  const saveDir = $derived(game.save_data?.save_dir ?? null);
  const installDir = $derived(game.install_dir ?? null);
  const coverSource = $derived(coverOf(game));
  const originalName = $derived(originalNameOf(game));
  const developer = $derived(developerOf(game));
  const publisher = $derived(publisherOf(game));
  const releaseYear = $derived(releaseYearOf(game)?.toString() ?? "鈥?");
  const platform = $derived(platformOf(game));
  const uniqueTags = $derived(tagsOf(game).slice(0, 12));

  const statusLabel = $derived(
    status ? completionStatusLabel(status) : "未开始"
  );

  async function withBusy(key: string, fn: () => Promise<void> | void) {
    if (busy) return;
    busy = key;
    try { await fn(); } finally { busy = null; }
  }

  async function handleLaunch() {
    await withBusy("launch", () => gameStore.launch(game.id));
  }
  async function handleFavorite() {
    await withBusy("fav", () => gameStore.toggleFavorite(game.id));
  }
  async function handleRevealSave() {
    if (!saveDir) { uiStore.toast("尚未设置存档目录", "info"); return; }
    await withBusy("save", async () => {
      try { await openPath(saveDir); } catch (e) { uiStore.toast(`打开失败: ${e}`, "error"); }
    });
  }
  async function handleRevealInstall() {
    if (!installDir) { uiStore.toast("尚未设置安装目录", "info"); return; }
    await withBusy("install", async () => {
      try { await openPath(installDir); } catch (e) { uiStore.toast(`打开失败: ${e}`, "error"); }
    });
  }
  async function handleSetSaveDir() {
    let path: string | null = null;
    try { path = await pickDirectory(); } catch { path = null; }
    if (!path) return;
    await withBusy("setSave", async () => {
      const ok = await gameStore.updateSaveDir(game.id, path);
      if (ok) uiStore.toast("已更新存档目录", "success");
      else uiStore.toast("更新失败", "error");
    });
  }
  async function handleCopyPath() {
    if (!installDir) return;
    try { await navigator.clipboard.writeText(installDir); uiStore.toast("已复制安装路径", "success"); }
    catch { uiStore.toast("复制失败", "error"); }
  }
  async function handleRemove() {
    if (!confirmRemove) { confirmRemove = true; setTimeout(() => confirmRemove = false, 3000); return; }
    await withBusy("remove", async () => {
      try { await gameStore.remove(game.id); uiStore.toast("已从库中移除", "success"); onClose(); }
      catch (e) { uiStore.toast(`移除失败: ${e}`, "error"); }
    });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); onClose(); }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<aside class="bp-detail glass-card" aria-label="游戏详情" tabindex="-1">
  <header class="d-head">
    <button class="d-close" onclick={onClose} aria-label="关闭">
      <Icon name="x" size={18} />
    </button>
    <div class="d-head-meta">
      <span class="d-pos">详情</span>
      <span class="d-name-line">{game.name}</span>
    </div>
  </header>

  <div class="d-body">
    <!-- 封面 + 标题 -->
    <div class="d-cover">
      {#if fileSrc(coverSource)}
        <img src={fileSrc(coverSource)!} alt={game.name} />
      {:else}
        <div class="d-cover-empty">{(game.name?.[0] ?? "?").toUpperCase()}</div>
      {/if}
    </div>

    <div class="d-title">
      {#if originalName}
        <p class="d-jp">{originalName}</p>
      {/if}
      <h2>{game.name}</h2>
      <p class="d-dev">
        {developer}
        {#if publisher && publisher !== developer} / <span>{publisher}</span>{/if}
      </p>
    </div>

    <!-- 评分 / 状态 -->
    <div class="d-stats">
      <div class="d-rating">
        <RatingRing value={rating ?? 0} max={10} size={56} />
        <div class="d-rating-text">
          <span class="d-rating-label">综合评分</span>
          <span class="d-rating-value">{rating ? rating.toFixed(1) : "—"}</span>
        </div>
      </div>
      <div class="d-info-grid">
        <div class="d-info-cell"><span>状态</span><b>{statusLabel}</b></div>
        <div class="d-info-cell"><span>时长</span><b>{playTime}</b></div>
        <div class="d-info-cell"><span>发行年</span><b>{releaseYear}</b></div>
        <div class="d-info-cell"><span>平台</span><b>{platform}</b></div>
      </div>
    </div>

    <!-- 标签 -->
    {#if uniqueTags.length}
      <div class="d-section">
        <h3>标签</h3>
        <div class="d-tags">
          {#each uniqueTags as t}<span class="d-tag">{t}</span>{/each}
        </div>
      </div>
    {/if}

    <!-- 简介 -->
    <div class="d-section">
      <h3>简介</h3>
      <p class="d-desc">{
        game.description?.trim()
        || (game.metadata?.genres?.length ? game.metadata.genres.join(" / ") : "")
        || "暂无简介"
      }</p>
    </div>

    <!-- 路径信息 -->
    <div class="d-section">
      <h3>路径</h3>
      <div class="d-path-list">
        <div class="d-path-row">
          <span class="d-path-k">安装目录</span>
          <span class="d-path-v" title={installDir ?? ""}>{installDir ?? "—"}</span>
        </div>
        <div class="d-path-row">
          <span class="d-path-k">存档目录</span>
          <span class="d-path-v" title={saveDir ?? ""}>{saveDir ?? "—"}</span>
        </div>
        {#if lastPlayed}
          <div class="d-path-row">
            <span class="d-path-k">上次游玩</span>
            <span class="d-path-v">{new Date(lastPlayed).toLocaleString()}</span>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <footer class="d-foot">
    <div class="d-actions-main">
      <Button onclick={handleLaunch} disabled={busy !== null}>
        <Icon name="play" size={14} /> 启动
      </Button>
      <Button variant="secondary" onclick={handleRevealInstall} disabled={!installDir || busy !== null}>
        <Icon name="folder" size={14} /> 设置（安装目录）
      </Button>
      <Button variant="secondary" onclick={handleRevealSave} disabled={!saveDir || busy !== null}>
        <Icon name="save" size={14} /> 存档位置
      </Button>
      <Button variant="secondary" onclick={handleSetSaveDir} disabled={busy !== null}>
        <Icon name="tag" size={14} /> 改存档目录
      </Button>
    </div>
    <div class="d-actions-sub">
      <Button variant="ghost" onclick={handleFavorite} disabled={busy !== null}>
        <Icon name={game.favorite ? "heartFill" : "heart"} size={14} />
        {game.favorite ? "已收藏" : "收藏"}
      </Button>
      <Button variant="ghost" onclick={handleCopyPath} disabled={!installDir}>
        <Icon name="paperclip" size={14} /> 复制路径
      </Button>
      <Button variant="ghost" onclick={handleRemove} disabled={busy !== null}>
        <Icon name="trash" size={14} />
        {confirmRemove ? "确认移除？" : "从库移除"}
      </Button>
    </div>
  </footer>
</aside>

<style>
  .bp-detail {
    position: absolute; right: 0; top: 0; bottom: 0;
    width: min(440px, 42vw);
    display: flex; flex-direction: column;
    z-index: 5;
    border-radius: 0;
    border-left: 1px solid var(--border);
    border-top: 0; border-bottom: 0; border-right: 0;
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    box-shadow: -20px 0 60px -20px rgba(0, 0, 0, .6);
    animation: slideInRight 0.32s cubic-bezier(.22,1,.36,1);
  }
  @keyframes slideInRight {
    from { transform: translateX(40px); opacity: 0; }
    to   { transform: translateX(0); opacity: 1; }
  }

  .d-head {
    display: flex; align-items: center; gap: 14px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }
  .d-close {
    border: 1px solid var(--border);
    background: var(--bg-hover);
    color: var(--text-secondary);
    width: 32px; height: 32px;
    border-radius: 999px;
    display: grid; place-items: center;
    cursor: pointer;
    transition: color .18s ease, border-color .18s ease;
  }
  .d-close:hover { color: var(--accent); border-color: var(--accent-ring); }
  .d-head-meta { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .d-pos { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 1px; }
  .d-name-line {
    font-size: 14px; font-weight: 700; color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  .d-body {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 20px;
    display: flex; flex-direction: column; gap: 18px;
  }

  .d-cover {
    width: 100%;
    aspect-ratio: 3 / 4;
    max-height: 240px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--bg-elev);
    border: 1px solid var(--border);
  }
  .d-cover img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .d-cover-empty {
    width: 100%; height: 100%;
    display: grid; place-items: center;
    font-size: 72px; color: var(--text-muted); font-weight: 800;
  }

  .d-title h2 {
    margin: 0; font-size: 22px; font-weight: 820; line-height: 1.2;
    color: var(--text-primary);
  }
  .d-jp {
    font-family: var(--font-jp);
    color: var(--text-muted);
    font-size: 13px; margin-bottom: 4px;
  }
  .d-dev { color: var(--text-muted); font-size: 13px; margin-top: 4px; }

  .d-stats {
    display: flex; flex-direction: column; gap: 12px;
    padding: 14px; border-radius: var(--radius-md);
    background: var(--bg-hover);
    border: 1px solid var(--border);
  }
  .d-rating { display: flex; align-items: center; gap: 14px; }
  .d-rating-text { display: flex; flex-direction: column; }
  .d-rating-label { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 1px; }
  .d-rating-value { font-size: 22px; font-weight: 820; color: var(--accent); font-family: var(--font-mono); }

  .d-info-grid {
    display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px 12px;
  }
  .d-info-cell {
    display: flex; flex-direction: column; gap: 2px;
    font-size: 12px;
  }
  .d-info-cell span { color: var(--text-muted); }
  .d-info-cell b { color: var(--text-primary); font-weight: 700; }

  .d-section h3 {
    font-size: 11px; text-transform: uppercase; letter-spacing: 1.2px;
    color: var(--text-muted); margin: 0 0 8px;
    font-weight: 700;
  }
  .d-tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .d-tag {
    border: 1px solid var(--border);
    background: var(--bg-hover);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 3px 10px; font-size: 11px; font-weight: 600;
  }
  .d-desc {
    color: var(--text-secondary);
    font-size: 13px; line-height: 1.6;
    margin: 0;
    display: -webkit-box; -webkit-line-clamp: 6; line-clamp: 6;
    -webkit-box-orient: vertical; overflow: hidden;
  }

  .d-path-list { display: flex; flex-direction: column; gap: 6px; }
  .d-path-row {
    display: flex; gap: 12px; align-items: baseline;
    font-size: 12px;
  }
  .d-path-k { color: var(--text-muted); min-width: 64px; flex-shrink: 0; }
  .d-path-v {
    color: var(--text-primary); font-family: var(--font-mono);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    flex: 1; min-width: 0;
  }

  .d-foot {
    padding: 14px 20px 18px;
    border-top: 1px solid var(--border);
    display: flex; flex-direction: column; gap: 10px;
    background: rgba(11, 14, 21, .55);
  }
  .d-actions-main, .d-actions-sub {
    display: flex; flex-wrap: wrap; gap: 8px;
  }
  .d-actions-main :global(.btn) { flex: 1 1 0; min-width: 0; padding: 10px 12px; font-size: 13px; }
  .d-actions-sub :global(.btn) { flex: 1 1 0; min-width: 0; padding: 8px 10px; font-size: 12px; }
</style>
