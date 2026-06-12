<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import {
    createSaveSnapshot,
    detectSaveCandidates,
    formatPlayTime,
    listSaveSnapshots,
    restoreSaveSnapshot,
    updateGame,
    type SaveCandidateDir,
    type SaveSnapshot,
  } from "../api";
  import type { Game } from "../stores/games.svelte";
  import { fileSrc } from "../utils";
  import Button from "./Button.svelte";
  import TagPill from "./TagPill.svelte";
  import RatingRing from "./RatingRing.svelte";
  import Icon from "./Icon.svelte";
  import CachedImage from "./CachedImage.svelte";
  import {
    coverOf,
    developerOf,
    gameRating,
    gameTotalSeconds,
    heroImageOf,
    originalNameOf,
    platformOf,
    publisherOf,
    releaseYearOf,
    screenshotsOf,
    tagsOf,
  } from "../utils/game";

  let game = $derived(gameStore.selectedGame);
  let bgA = $state<HTMLDivElement>();
  let bgB = $state<HTMLDivElement>();
  let activeBg = $state(0);
  let lastBg = "";
  let coverflowEl = $state<HTMLDivElement>();
  let saveCandidates = $state<SaveCandidateDir[]>([]);
  let saveSnapshots = $state<SaveSnapshot[]>([]);
  let savesLoading = $state(false);
  let savesError = $state("");
  let lastSaveGameId = "";

  const currentArt = $derived(fileSrc(heroImageOf(game)) ?? "");
  const coverSource = $derived(coverOf(game));
  const screenshots = $derived.by(() => {
    if (!game) return [];
    const raw = screenshotsOf(game);
    return raw.map(s => fileSrc(s) ?? s);
  });
  const originalName = $derived(originalNameOf(game));
  const developer = $derived(developerOf(game));
  const publisher = $derived(publisherOf(game));
  const rating = $derived(gameRating(game));
  const releaseYear = $derived(releaseYearOf(game)?.toString() ?? "----");
  const playTime = $derived(formatPlayTime(gameTotalSeconds(game)));
  const platform = $derived(platformOf(game));
  const detailTags = $derived(tagsOf(game));
  const saveDirOf = (g: Game) => g.save_data?.save_dir ?? "";
  const achievementTotal = $derived(game?.play_tracker?.achievements_total ?? 0);
  const achievementUnlocked = $derived(game?.play_tracker?.achievements_unlocked ?? 0);
  const achievementPercent = $derived(achievementTotal > 0 ? Math.round((achievementUnlocked / achievementTotal) * 100) : 0);
  const latestSnapshot = $derived(saveSnapshots[0] ?? null);
  const recentSessions = $derived.by(() =>
    [...(game?.play_tracker?.sessions ?? [])]
      .sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime())
      .slice(0, 3)
  );

  function sessionDate(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value || "未记录";
    return date.toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" });
  }

  onMount(() => {
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    const node = coverflowEl;
    if (reduce || !node) return;
    const ctx = gsap.context(() => {
      gsap.from(node.querySelectorAll(".shot"), {
        opacity: 0,
        y: 18,
        rotateY: -10,
        duration: 0.55,
        ease: "power3.out",
        stagger: { each: 0.05, from: "center" },
      });
    }, node);
    return () => ctx.revert();
  });

  $effect(() => {
    if (!currentArt || currentArt === lastBg || !bgA || !bgB) return;
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    const incoming = activeBg === 0 ? bgB : bgA;
    const outgoing = activeBg === 0 ? bgA : bgB;
    incoming.style.backgroundImage = `url("${currentArt}")`;
    if (reduce) {
      incoming.style.opacity = "1";
      outgoing.style.opacity = "0";
    } else {
      const ctx = gsap.context(() => {
        gsap.to(incoming, { opacity: 1, duration: 0.45, ease: "power2.out" });
        gsap.to(outgoing, { opacity: 0, duration: 0.45, ease: "power2.out" });
      });
      setTimeout(() => ctx.revert(), 520);
    }
    activeBg = activeBg === 0 ? 1 : 0;
    lastBg = currentArt;
  });

  $effect(() => {
    const id = game?.id ?? "";
    if (!id || id === lastSaveGameId) return;
    lastSaveGameId = id;
    void refreshSaves(id);
  });

  async function handleLaunch() {
    if (!game) return;
    await gameStore.launch(game.id);
    uiStore.notify(`正在启动 ${game.name}...`, "info");
  }

  async function handleLaunchJP() {
    if (!game) return;
    try {
      await gameStore.launchJP(game.id);
      uiStore.notify(`正在日区启动 ${game.name}...`, "info");
    } catch (e) {
      uiStore.notify(`日区启动失败：${e}`, "error");
    }
  }

  function handleBackup() {
    uiStore.currentView = "backup";
  }

  async function refreshSaves(gameId = game?.id) {
    if (!gameId) return;
    savesLoading = true;
    savesError = "";
    try {
      const [candidates, snapshots] = await Promise.all([
        detectSaveCandidates(gameId),
        listSaveSnapshots(gameId),
      ]);
      saveCandidates = candidates;
      saveSnapshots = snapshots;
    } catch (e) {
      savesError = String(e);
    } finally {
      savesLoading = false;
    }
  }

  async function createSnapshot(path?: string) {
    if (!game) return;
    try {
      await createSaveSnapshot(game.id, path || saveDirOf(game) || null, "详情页快速快照");
      await refreshSaves(game.id);
      uiStore.notify("存档快照已创建", "success");
    } catch (e) {
      savesError = String(e);
      uiStore.notify(`存档快照失败：${e}`, "error");
    }
  }

  async function restoreSnapshot(snapshot: SaveSnapshot) {
    if (!game) return;
    try {
      await restoreSaveSnapshot(game.id, snapshot.file_path, saveDirOf(game) || null);
      uiStore.notify("存档已恢复", "success");
    } catch (e) {
      savesError = String(e);
      uiStore.notify(`恢复失败：${e}`, "error");
    }
  }

  function handleScrape() {
    if (!game) return;
    uiStore.openScrapeDialog(game.id);
  }

  // ── 内联编辑 ──
  let isEditing = $state(false);
  let editName = $state("");
  let editDesc = $state("");
  let editExePath = $state("");
  let isSaving = $state(false);

  function openEdit() {
    if (!game) return;
    editName = game.name ?? "";
    editDesc = game.description ?? "";
    editExePath = game.exe_path ?? "";
    isEditing = true;
  }

  async function saveEdit() {
    if (!game || isSaving) return;
    isSaving = true;
    try {
      await updateGame({ ...game, name: editName, description: editDesc, exe_path: editExePath });
      await gameStore.load();
      isEditing = false;
      uiStore.notify("游戏信息已保存", "success");
    } catch (e) {
      uiStore.notify(`保存失败：${e}`, "error");
    } finally {
      isSaving = false;
    }
  }
</script>

{#if game}
  <section class="detail-page" data-testid="game-detail-page">
    <div class="bg-layer" bind:this={bgA}></div>
    <div class="bg-layer" bind:this={bgB}></div>
    <div class="shade"></div>

    <button class="back" onclick={() => uiStore.currentView = "home"} aria-label="返回游戏库">
      <Icon name="arrowLeft" size={18} /> 返回游戏库
    </button>

    <div class="content">
      <aside class="poster-card" aria-label="游戏封面">
        <div class="poster-frame">
          {#if coverSource}
            <CachedImage source={coverSource} cacheKey={`detail-cover-${game.id}`} alt={`${game.name} 封面`} loading="eager" />
          {:else}
            <span class="poster-mono">{game.name?.trim()?.[0]?.toUpperCase() ?? "?"}</span>
          {/if}
        </div>
        <div class="poster-meta">
          <span>{platform}</span>
          <strong>{releaseYear}</strong>
        </div>
      </aside>

      <article class="info glass-card">
        {#if originalName}<p class="jp-name">{originalName}</p>{/if}
        <h1>{game.name}</h1>
        <p class="cn-name">{developer}</p>

        <div class="tags">
          {#if detailTags.length}
            {#each detailTags.slice(0, 6) as tag, index}
              <TagPill label={tag} active={index === 0} />
            {/each}
          {:else}
            <span class="tag-empty">待补全标签</span>
          {/if}
        </div>

        <div class="score-row">
          <RatingRing value={rating} max={10} size={78} />
          <div class="meta-line">
            <span><b>{releaseYear}</b> 年份</span>
            <span><b>{playTime}</b> 时长</span>
            <span><b>{platform}</b> 平台</span>
            <span><b>{publisher}</b> 发行</span>
          </div>
        </div>

        <p class="description">{game.description || "暂无简介。可使用 AI 刮削补全剧情简介、角色图、标签与截图。"}</p>

        <div class="action-bar">
          <Button onclick={handleLaunch}>启动游戏</Button>
          <Button variant="secondary" onclick={handleLaunchJP}>日区启动</Button>
          <Button variant="secondary" onclick={handleScrape}>AI 刮削</Button>
          <Button variant="ghost" onclick={handleBackup}>存档</Button>
          <Button variant="ghost" onclick={openEdit}>编辑</Button>
        </div>
      </article>

      <div class="coverflow" bind:this={coverflowEl} aria-label="截图画廊">
        {#if screenshots.length}
          {#each screenshots.slice(0, 5) as shot, index}
            <figure class="shot" class:focus={index === Math.min(1, screenshots.length - 1)} style={`--i:${index}; --center:${Math.min(1, screenshots.length - 1)}`}>
              <img src={shot} alt={`${game.name} 截图 ${index + 1}`} />
            </figure>
          {/each}
        {:else}
          <div class="shot-empty glass-card">暂无截图</div>
        {/if}
      </div>
    </div>

    <section class="detail-panels" aria-label="存档与成就">
      <article class="info-panel glass-card">
        <header>
          <span>Save Data</span>
          <strong>存档</strong>
        </header>
        <div class="panel-grid">
          <div>
            <span>目录</span>
            <b title={saveDirOf(game)}>{saveDirOf(game) || saveCandidates[0]?.path || "未配置"}</b>
          </div>
          <div>
            <span>候选</span>
            <b>{savesLoading ? "扫描中" : `${saveCandidates.length} 个`}</b>
          </div>
          <div>
            <span>快照</span>
            <b>{saveSnapshots.length} 份</b>
          </div>
        </div>
        {#if latestSnapshot}
          <p class="panel-note">最近快照：{latestSnapshot.file_name} / {latestSnapshot.created_at}</p>
        {:else if savesError}
          <p class="panel-note error">{savesError}</p>
        {:else}
          <p class="panel-note">可从候选目录创建第一份安全快照。</p>
        {/if}
        <div class="panel-actions">
          <Button variant="secondary" onclick={() => createSnapshot(saveCandidates[0]?.path)}>创建快照</Button>
          <Button variant="ghost" onclick={() => latestSnapshot && restoreSnapshot(latestSnapshot)} disabled={!latestSnapshot}>恢复最近</Button>
          <Button variant="ghost" onclick={handleBackup}>打开存档页</Button>
        </div>
      </article>

      <article class="info-panel glass-card">
        <header>
          <span>Achievements</span>
          <strong>成就</strong>
        </header>
        <div class="achievement-ring" style={`--achievement:${achievementPercent}%`}>
          <strong>{achievementTotal ? `${achievementPercent}%` : "--"}</strong>
          <span>{achievementUnlocked} / {achievementTotal}</span>
        </div>
        <div class="achievement-bar" aria-label="成就进度">
          <i style={`width:${achievementPercent}%`}></i>
        </div>
        <p class="panel-note">
          {achievementTotal ? "来自平台同步的成就数据；重新同步 Steam 可刷新进度。" : "当前没有成就数据，Steam 全库同步后会自动填充。"}
        </p>
      </article>

      <article class="info-panel glass-card">
        <header>
          <span>Recent Play</span>
          <strong>最近会话</strong>
        </header>
        {#if recentSessions.length}
          <div class="session-list">
            {#each recentSessions as session}
              <div class="session-row">
                <span>{sessionDate(session.start_time)}</span>
                <b>{formatPlayTime(session.duration_seconds)}</b>
                <small>{session.notes || (session.end_time ? "已结束" : "进行中")}</small>
              </div>
            {/each}
          </div>
        {:else}
          <p class="panel-note">还没有游玩记录；启动一次游戏后这里会显示最近会话。</p>
        {/if}
      </article>
    </section>

    {#if isEditing}
      <div class="edit-overlay" role="dialog" aria-modal="true" aria-label="编辑游戏">
        <div class="edit-panel glass-card">
          <header class="edit-header">
            <h3>编辑：{game.name}</h3>
            <button class="edit-close" onclick={() => isEditing = false} aria-label="关闭">
              <Icon name="x" size={18} />
            </button>
          </header>
          <div class="edit-body">
            <div class="edit-field">
              <label for="edit-name">游戏名称</label>
              <input id="edit-name" bind:value={editName} />
            </div>
            <div class="edit-field">
              <label for="edit-exe">可执行文件路径</label>
              <input id="edit-exe" class="mono" bind:value={editExePath} />
            </div>
            <div class="edit-field">
              <label for="edit-desc">游戏简介</label>
              <textarea id="edit-desc" bind:value={editDesc} rows={5}></textarea>
            </div>
          </div>
          <footer class="edit-footer">
            <Button onclick={saveEdit} disabled={isSaving}>{isSaving ? "保存中…" : "保存修改"}</Button>
            <Button variant="ghost" onclick={() => isEditing = false}>取消</Button>
          </footer>
        </div>
      </div>
    {/if}
  </section>
{/if}

<style>
  .detail-page { position: relative; flex: 1; min-height: 0; overflow: auto; background: var(--bg-primary); }
  .bg-layer { position: absolute; inset: 0; opacity: 0; background-size: cover; background-position: center right; filter: blur(6px); transform: scale(1.04); }
  .shade { position: absolute; inset: 0; background: linear-gradient(90deg, rgba(9,9,11,.96) 0%, rgba(9,9,11,.76) 43%, rgba(9,9,11,.34) 100%), linear-gradient(180deg, rgba(9,9,11,.18), rgba(9,9,11,.72)); }

  .back { position: absolute; top: 22px; left: 26px; z-index: 2; display: inline-flex; align-items: center; gap: 8px; border: 1px solid var(--border); background: rgba(16,19,26,.62); color: var(--text-secondary); border-radius: var(--radius-full); padding: 8px 13px; cursor: pointer; backdrop-filter: blur(14px); }
  .back:hover { color: var(--accent); border-color: var(--accent-ring); }

  .content { position: relative; z-index: 1; min-height: min(720px, 100%); display: grid; grid-template-columns: minmax(180px, 0.38fr) minmax(340px, 0.78fr) minmax(420px, 1fr); align-items: center; gap: 24px; padding: 78px 44px 24px; }
  .poster-card { display: grid; gap: 12px; justify-items: center; align-self: center; }
  .poster-frame {
    width: min(240px, 18vw);
    min-width: 172px;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-hover);
    background: var(--bg-elev);
    box-shadow: var(--ring-switch), var(--shadow-lift);
  }
  .poster-frame :global(.cached-image) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .poster-mono {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-muted);
    font-family: var(--font-display);
    font-size: 54px;
    font-weight: 800;
    background: linear-gradient(135deg, rgba(232,85,127,.2), rgba(174,186,211,.08));
  }
  .poster-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 12px;
  }
  .poster-meta strong {
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }
  .info { padding: 28px; max-width: 560px; }
  .jp-name { font-family: var(--font-jp); color: var(--text-muted); font-size: 16px; margin-bottom: 8px; }
  h1 { font-size: clamp(30px, 4vw, 48px); line-height: 1.05; letter-spacing: 0; margin: 0; }
  .cn-name { margin-top: 10px; color: var(--text-secondary); }
  .tags { display: flex; flex-wrap: wrap; gap: 8px; margin: 20px 0; }
  .tag-empty {
    display: inline-flex;
    align-items: center;
    min-height: 26px;
    border: 1px dashed var(--border-hover);
    border-radius: var(--radius-full);
    padding: 4px 10px;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 700;
  }
  .score-row { display: flex; align-items: center; gap: 20px; margin-bottom: 22px; }
  .meta-line { display: flex; flex-wrap: wrap; gap: 10px; }
  .meta-line span { border: 1px solid var(--border); border-radius: var(--radius-md); padding: 9px 11px; color: var(--text-muted); font-size: 12px; background: rgba(255,255,255,.035); }
  .meta-line b { font-family: var(--font-mono); color: var(--text-primary); font-variant-numeric: tabular-nums; margin-right: 4px; }
  .description { color: var(--text-secondary); font-size: 14px; line-height: 1.75; max-width: 58ch; }
  .action-bar { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 24px; }

  .coverflow { min-height: 360px; display: flex; align-items: center; justify-content: center; perspective: 1200px; }
  .shot { width: min(42vw, 520px); aspect-ratio: 16 / 9; border-radius: var(--radius-xl); overflow: hidden; border: 1px solid var(--border); box-shadow: var(--shadow-lg); margin-left: -18%; transform: translateX(calc((var(--i) - var(--center)) * 34px)) rotateY(calc((var(--center) - var(--i)) * 12deg)) scale(.82); opacity: .58; }
  .shot:first-child { margin-left: 0; }
  .shot.focus { transform: translateY(-8px) scale(1); opacity: 1; border-color: var(--accent-ring); box-shadow: var(--shadow-focus, 0 0 0 2px var(--accent-ring)); z-index: 2; }
  .shot img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .shot-empty { padding: 40px; color: var(--text-muted); }

  .detail-panels {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(260px, 0.72fr) minmax(260px, 0.82fr);
    gap: 16px;
    padding: 0 44px 42px;
  }

  .info-panel {
    padding: 18px;
    border-radius: 8px;
  }

  .info-panel header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }

  .info-panel header span,
  .panel-grid span {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .info-panel header strong {
    color: var(--text-primary);
    font-size: 18px;
  }

  .panel-grid {
    display: grid;
    grid-template-columns: 1.4fr 0.6fr 0.6fr;
    gap: 10px;
  }

  .panel-grid div {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px;
    background: rgba(255,255,255,.04);
  }

  .panel-grid b {
    display: block;
    margin-top: 5px;
    color: var(--text-primary);
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel-note {
    margin: 12px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.55;
  }

  .panel-note.error {
    color: #fecaca;
  }

  .panel-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
  }

  .achievement-ring {
    width: 112px;
    height: 112px;
    display: grid;
    place-items: center;
    align-content: center;
    gap: 4px;
    margin: 2px 0 14px;
    border-radius: 50%;
    background:
      conic-gradient(var(--accent) var(--achievement), rgba(255,255,255,.09) 0),
      rgba(255,255,255,.04);
    border: 1px solid var(--border);
  }

  .achievement-ring strong {
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 24px;
    line-height: 1;
  }

  .achievement-ring span {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .achievement-bar {
    height: 8px;
    overflow: hidden;
    border-radius: 999px;
    background: rgba(255,255,255,.1);
  }

  .achievement-bar i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }

  .session-list {
    display: grid;
    gap: 8px;
  }

  .session-row {
    display: grid;
    grid-template-columns: 58px minmax(72px, auto) minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: rgba(255,255,255,.04);
    padding: 10px;
  }

  .session-row span,
  .session-row b {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .session-row span {
    color: var(--text-muted);
    font-size: 11px;
  }

  .session-row b {
    color: var(--text-primary);
    font-size: 12px;
  }

  .session-row small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
    font-size: 12px;
  }

  @media (max-width: 1100px) {
    .content { grid-template-columns: 1fr; overflow: auto; align-items: start; }
    .poster-card { justify-self: center; }
    .poster-frame { width: min(220px, 46vw); }
    .detail-panels { grid-template-columns: 1fr; }
    .coverflow { min-height: 260px; }
  }

  /* ── Inline edit overlay ── */
  .edit-overlay {
    position: absolute; inset: 0; z-index: 20;
    display: flex; align-items: center; justify-content: center;
    background: rgba(9, 9, 11, 0.72);
    backdrop-filter: blur(4px);
  }
  .edit-panel {
    width: min(540px, 90vw);
    max-height: 80vh;
    display: flex; flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    background: rgba(13, 17, 30, 0.96);
  }
  .edit-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 20px 24px 16px;
    border-bottom: 1px solid var(--border);
  }
  .edit-header h3 {
    font-size: 17px; font-weight: 700;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 420px;
  }
  .edit-close {
    background: none; border: none; color: var(--text-muted);
    cursor: pointer; display: flex; padding: 4px; border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  .edit-close:hover { color: var(--text-primary); }
  .edit-body {
    flex: 1; padding: 20px 24px; overflow-y: auto;
    display: flex; flex-direction: column; gap: 18px;
  }
  .edit-field { display: flex; flex-direction: column; gap: 6px; }
  .edit-field label {
    font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;
    color: var(--text-muted);
  }
  .edit-field input,
  .edit-field textarea {
    background: rgba(255,255,255,.06);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 10px 14px;
    font-size: 14px; font-family: var(--font-ui);
    outline: none; resize: vertical;
    transition: border-color 0.2s;
  }
  .edit-field input:focus,
  .edit-field textarea:focus { border-color: var(--accent-pink-ring); }
  .edit-field input.mono { font-family: var(--font-mono); font-size: 12px; }
  .edit-footer {
    display: flex; gap: 10px; justify-content: flex-end;
    padding: 14px 24px;
    border-top: 1px solid var(--border);
  }
</style>
