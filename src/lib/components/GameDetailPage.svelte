<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { handleBackNavigation } from "../stores/router.svelte";
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
  import { Button, Card, Dialog, Input, Tag } from "./ui";
  import { AsyncState, DetailPanel } from "./ui-v2";
  import RatingRing from "./RatingRing.svelte";
  import Icon from "./Icon.svelte";
  import CachedImage from "./CachedImage.svelte";
  import SavePanel from "./SavePanel.svelte";
  import GameNotes from "./GameNotes.svelte";
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
    tagsOf,
  } from "../utils/game";

  let game = $derived(gameStore.selectedGame);
  let galleryEl = $state<HTMLElement>();
  let saveCandidates = $state<SaveCandidateDir[]>([]);
  let saveSnapshots = $state<SaveSnapshot[]>([]);
  let savesLoading = $state(false);
  let savesError = $state("");
  let lastSaveGameId = "";

  const currentArt = $derived(fileSrc(heroImageOf(game)) ?? "");
  const coverSource = $derived(coverOf(game));
  const screenshots = $derived.by(() => {
    if (!game) return [];
    const seen = new Set<string>();
    const out: string[] = [];
    for (const s of game.screenshots ?? []) {
      if (!s) continue;
      const u = fileSrc(s) ?? s;
      if (u && !seen.has(u)) { seen.add(u); out.push(u); }
    }
    return out;
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
    const node = galleryEl;
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

  function closeDetail() {
    handleBackNavigation();
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

<DetailPanel
  open
  title={game?.name ?? "游戏未找到"}
  description={game ? [developer, releaseYear !== "----" ? releaseYear : "", platform].filter(Boolean).join(" · ") : "该游戏可能已被移除或数据加载失败"}
  onClose={closeDetail}
  side="right"
  size="lg"
  initialFocus=".game-detail-primary"
  returnFocus={false}
  class="game-detail-panel"
>
  {#if game}
    <section class="detail-page" data-testid="game-detail-page" data-game-id={game.id}>
      <div class="hero">
        {#if currentArt}<div class="bg-layer" style={`background-image: url("${currentArt}")`}></div>{/if}
        <div class="hero-scrim"></div>

        <div class="hero-floor">
          <div class="poster">
            <div class="poster-frame">
              {#if coverSource}
                <CachedImage source={coverSource} cacheKey={`detail-cover-${game.id}`} alt={`${game.name} 封面`} loading="eager" />
              {:else}
                <span class="poster-letter">{game.name?.trim()?.[0]?.toUpperCase() ?? "?"}</span>
              {/if}
            </div>
          </div>

          <div class="hero-info">
            {#if originalName}<p class="orig-name">{originalName}</p>{/if}
            <p class="hero-title">{game.name}</p>
            <div class="byline">
              {#if developer && developer !== "未知社团"}<span>{developer}</span>{/if}
              {#if publisher && publisher !== developer}<span class="sep-dot"></span><span>{publisher}</span>{/if}
            </div>

            <div class="meta-row">
              <span class="chip"><b>{releaseYear}</b></span>
              <span class="chip"><b>{playTime}</b></span>
              <span class="chip"><b>{platform}</b></span>
            </div>

            <div class="tags">
              {#each detailTags.slice(0, 6) as tag, index}<Tag active={index === 0}>{tag}</Tag>{/each}
            </div>

            <div class="actions">
              <Button class="game-detail-primary" press={handleLaunch}>启动游戏</Button>
              <Button variant="secondary" press={handleLaunchJP}>日区启动</Button>
              <Button variant="secondary" press={handleScrape}>刮削</Button>
              <Button variant="ghost" press={openEdit}>编辑</Button>
            </div>
          </div>

          {#if rating > 0}<div class="hero-rating"><RatingRing value={rating} max={10} size={68} /></div>{/if}
        </div>
      </div>

      <div class="body">
        <p class="desc">{game.description || "暂无简介。可使用刮削补全剧情简介、标签与截图。"}</p>

        {#if screenshots.length}
          <section class="gallery" bind:this={galleryEl} aria-label="截图画廊">
            <div class="gallery-track">
              {#each screenshots.slice(0, 8) as shot, index}
                <figure class="shot"><img src={shot} alt={`${game.name} 截图 ${index + 1}`} loading="lazy" /></figure>
              {/each}
            </div>
          </section>
        {/if}

        <section class="panels" aria-label="存档与成就">
          <SavePanel gameId={game.id} saveDir={saveDirOf(game)} compact />

          <Card class="panel" padding="none">
            <div class="panel-head"><span class="panel-label">Achievements</span><h3>成就</h3></div>
            <div class="panel-body">
              <div class="achieve-row"><div class="achieve-nums"><b>{achievementTotal ? `${achievementPercent}%` : "--"}</b><span>{achievementUnlocked} / {achievementTotal}</span></div></div>
              <div class="achieve-bar" aria-label={`成就进度 ${achievementPercent}%`}><i style={`width:${achievementPercent}%`}></i></div>
              <p class="panel-note">{achievementTotal ? "来自平台同步的成就数据。" : "暂无成就数据，Steam 同步后自动填充。"}</p>
            </div>
          </Card>

          <Card class="panel" padding="none">
            <div class="panel-head"><span class="panel-label">Recent Play</span><h3>最近会话</h3></div>
            <div class="panel-body">
              {#if recentSessions.length}
                <div class="session-list">
                  {#each recentSessions as session}
                    <div class="session-row">
                      <span class="session-date">{sessionDate(session.start_time)}</span>
                      <b class="session-dur">{formatPlayTime(session.duration_seconds)}</b>
                      <small>{session.notes || (session.end_time ? "已结束" : "进行中")}</small>
                    </div>
                  {/each}
                </div>
              {:else}<p class="panel-note">还没有游玩记录。</p>{/if}
            </div>
          </Card>
        </section>

        <GameNotes gameId={game.id} />
      </div>

      <Dialog open={isEditing} onClose={() => isEditing = false} title={`编辑：${game.name}`}>
        <div class="edit-panel">
          <header class="edit-header">
            <h3>编辑：{game.name}</h3>
            <button class="edit-close" onclick={() => isEditing = false} aria-label="关闭"><Icon name="x" size={18} /></button>
          </header>
          <div class="edit-body">
            <div class="edit-field"><label for="edit-name">游戏名称</label><Input id="edit-name" bind:value={editName} /></div>
            <div class="edit-field"><label for="edit-exe">可执行文件路径</label><Input id="edit-exe" class="mono" bind:value={editExePath} /></div>
            <div class="edit-field"><label for="edit-desc">游戏简介</label><textarea id="edit-desc" bind:value={editDesc} rows={5}></textarea></div>
          </div>
          <footer class="edit-footer">
            <Button press={saveEdit} disabled={isSaving}>{isSaving ? "保存中…" : "保存修改"}</Button>
            <Button variant="ghost" press={() => isEditing = false}>取消</Button>
          </footer>
        </div>
      </Dialog>
    </section>
  {:else}
    <AsyncState
      state="error"
      title="游戏未找到"
      description="该游戏可能已被移除或数据加载失败。"
      primaryAction={{ label: "返回游戏库", onSelect: closeDetail }}
      class="game-detail-missing"
    />
  {/if}
</DetailPanel>

<style>
  :global(.game-detail-panel .v2-detail-panel__body) { padding: 0; background: var(--bg-deep); }
  :global(.game-detail-panel.v2-detail-panel) { width: min(100vw, 56rem); }
  :global(.game-detail-missing) { min-height: 60vh; }

  /* ── Page scaffold ── */
  .detail-page {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: visible;
    background: var(--bg-deep);
  }

  /* ── Background art ── */
  .bg-layer {
    position: absolute;
    inset: 0;
    opacity: 1;
    background-size: cover;
    background-position: center 25%;
    animation: detailBgIn 0.45s ease both;
  }
  @keyframes detailBgIn { from { opacity: 0; } to { opacity: 1; } }

  /* ── Hero ── */
  .hero {
    position: relative;
    min-height: max(48vh, 380px);
    display: flex;
    align-items: flex-end;
  }

  .hero-scrim {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(180deg,
        rgba(8,10,16,0.06) 0%,
        rgba(8,10,16,0.12) 35%,
        rgba(8,10,16,0.50) 65%,
        var(--bg-deep) 100%
      ),
      linear-gradient(90deg,
        rgba(8,10,16,0.40) 0%,
        transparent 55%
      );
    pointer-events: none;
  }

  :global(.ui-button.back-btn) {
    position: absolute;
    top: 20px;
    left: 24px;
    z-index: 4;
    width: 36px;
    height: 36px;
    min-height: 0;
    padding: 0;
    display: grid;
    place-items: center;
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 50%;
    background: rgba(10,12,18,0.50);
    color: var(--text-secondary);
    cursor: pointer;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    transition: color 0.2s, border-color 0.2s, background 0.2s;
  }
  :global(.ui-button.back-btn:hover) {
    color: var(--text-primary);
    border-color: rgba(255,255,255,0.22);
    background: rgba(10,12,18,0.7);
  }

  .hero-floor {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: flex-end;
    gap: 32px;
    width: 100%;
    padding: 0 48px 36px;
  }

  /* ── Poster ── */
  .poster { flex-shrink: 0; }

  .poster-frame {
    width: 190px;
    aspect-ratio: 3 / 4;
    border-radius: 10px;
    overflow: hidden;
    border: 1px solid rgba(255,255,255,0.10);
    box-shadow:
      0 8px 32px rgba(0,0,0,0.40),
      0 2px 8px rgba(0,0,0,0.20);
    background: var(--bg-elev);
    transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.35s ease;
  }
  .poster-frame:hover {
    transform: translateY(-4px) scale(1.02);
    box-shadow:
      0 14px 44px rgba(0,0,0,0.50),
      0 4px 12px rgba(0,0,0,0.25);
  }

  .poster-frame :global(.cached-image) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .poster-letter {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-muted);
    font-family: var(--font-display);
    font-size: 50px;
    font-weight: 800;
    background: linear-gradient(145deg, rgba(232,85,127,0.14), rgba(174,186,211,0.06));
  }

  /* ── Hero info ── */
  .hero-info {
    flex: 1;
    min-width: 0;
    padding-bottom: 2px;
  }

  .orig-name {
    font-family: var(--font-jp);
    color: rgba(255,255,255,0.40);
    font-size: 13px;
    margin-bottom: 4px;
    line-height: 1.3;
  }

  .hero-title {
    font-size: clamp(26px, 3.2vw, 42px);
    font-weight: 800;
    line-height: 1.08;
    letter-spacing: -0.02em;
    color: var(--text-primary);
    margin: 0;
  }

  .byline {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    color: var(--text-secondary);
    font-size: 14px;
  }

  .sep-dot {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--text-muted);
    flex-shrink: 0;
  }

  .meta-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 16px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    padding: 5px 11px;
    border-radius: 6px;
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.08);
    font-size: 12px;
    color: var(--text-secondary);
    transition: background 0.15s, border-color 0.15s;
  }
  .chip:hover {
    background: rgba(255,255,255,0.09);
    border-color: rgba(255,255,255,0.14);
  }

  .chip b {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 14px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 18px;
  }

  /* ── Hero rating ── */
  .hero-rating {
    flex-shrink: 0;
    align-self: flex-end;
    margin-bottom: 2px;
  }

  /* ── Body ── */
  .body {
    position: relative;
    z-index: 1;
    padding: 0 48px 48px;
    background: var(--bg-deep);
  }

  .desc {
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.8;
    max-width: 68ch;
    margin: 0 0 28px;
  }

  /* ── Screenshot gallery ── */
  .gallery { margin-bottom: 32px; }

  .gallery-track {
    display: flex;
    gap: 12px;
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    padding-bottom: 6px;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: thin;
    scrollbar-color: rgba(255,255,255,0.10) transparent;
  }
  .gallery-track::-webkit-scrollbar { height: 5px; }
  .gallery-track::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.10); border-radius: 3px; }
  .gallery-track::-webkit-scrollbar-track { background: transparent; }

  .shot {
    flex-shrink: 0;
    width: min(46vw, 520px);
    aspect-ratio: 16 / 9;
    border-radius: 10px;
    overflow: hidden;
    border: 1px solid rgba(255,255,255,0.07);
    scroll-snap-align: start;
    margin: 0;
    transition: border-color 0.2s, box-shadow 0.3s;
  }
  .shot:hover {
    border-color: rgba(255,255,255,0.16);
    box-shadow: 0 4px 20px rgba(0,0,0,0.3);
  }

  .shot img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  /* ── Panels ── */
  .panels {
    display: grid;
    grid-template-columns: 1.2fr 0.8fr 1fr;
    gap: 12px;
  }

  :global(.ui-card.panel:hover) {
    border-color: rgba(255,255,255,0.10);
  }

  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 16px 18px 0;
  }

  .panel-label {
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }

  .panel-head h3 {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .panel-body { padding: 12px 18px 18px; }

  .panel-note {
    margin: 10px 0 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  /* ── Achievement ── */
  .achieve-row {
    margin-bottom: 10px;
  }

  .achieve-nums b {
    display: block;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 28px;
    color: var(--text-primary);
    line-height: 1;
  }

  .achieve-nums span {
    display: block;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  .achieve-bar {
    height: 5px;
    border-radius: 3px;
    background: rgba(255,255,255,0.07);
    overflow: hidden;
  }

  .achieve-bar i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width 0.6s cubic-bezier(0.16, 1, 0.3, 1);
  }

  /* ── Sessions ── */
  .session-list { display: grid; gap: 6px; }

  .session-row {
    display: grid;
    grid-template-columns: 50px auto 1fr;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 7px;
    background: rgba(255,255,255,0.03);
    border: 1px solid rgba(255,255,255,0.04);
    transition: border-color 0.15s;
  }
  .session-row:hover { border-color: rgba(255,255,255,0.10); }

  .session-date {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 11px;
    color: var(--text-muted);
  }

  .session-dur {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 12px;
    color: var(--text-primary);
  }

  .session-row small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
    font-size: 11px;
  }

  /* ── Responsive ── */
  @media (max-width: 960px) {
    .hero-floor {
      flex-direction: column;
      align-items: flex-start;
      padding: 0 24px 28px;
      gap: 20px;
    }
    .poster-frame { width: 150px; }
    .hero-rating { align-self: flex-start; }
    .body { padding: 0 24px 36px; }
    .panels { grid-template-columns: 1fr; }
    .shot { width: min(72vw, 380px); }
  }

  /* ── Edit dialog ── */
  .edit-panel {
    width: min(520px, 90vw);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 16px;
    background: rgba(13,17,28,0.96);
    box-shadow: 0 24px 64px rgba(0,0,0,0.40);
  }

  .edit-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px 16px;
    border-bottom: 1px solid rgba(255,255,255,0.06);
  }

  .edit-header h3 {
    font-size: 16px;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 400px;
    margin: 0;
  }

  .edit-close {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    padding: 4px;
    border-radius: 6px;
    flex-shrink: 0;
    transition: color 0.15s;
  }
  .edit-close:hover { color: var(--text-primary); }

  .edit-body {
    flex: 1;
    padding: 20px 24px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .edit-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .edit-field label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .edit-field textarea {
    background: rgba(255,255,255,0.05);
    color: var(--text-primary);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 14px;
    font-family: var(--font-ui);
    outline: none;
    resize: vertical;
    transition: border-color 0.2s;
  }

  .edit-field textarea:focus { border-color: var(--accent); }

  .edit-field :global(.ui-input.mono) {
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .edit-footer {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding: 14px 24px;
    border-top: 1px solid rgba(255,255,255,0.06);
  }
</style>
