<script lang="ts">
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { gameStore } from "../stores/games.svelte";
  import { i18n } from "../stores/i18n.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { handleBackNavigation } from "../stores/router.svelte";
  import { platformStore } from "../platform";
  import { formatPlayTime, updateGame } from "../api";
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
  const privateReview = $derived(game?.play_tracker?.review?.trim() ?? "");
  const archiveFacts = $derived.by(() => {
    if (!game) return [] as Array<{ label: string; value: string }>;
    const facts = [
      { label: "开发", value: developer || "" },
      { label: "发行", value: publisher || "" },
      { label: "引擎", value: game.metadata?.engine || game.engine || "" },
      { label: "系列", value: game.metadata?.series || "" },
      { label: "发售", value: game.metadata?.release_date || releaseYear },
      { label: "分级", value: game.metadata?.age_rating || "" },
      { label: "语言", value: (game.metadata?.languages || []).slice(0, 3).join(" / ") },
      { label: "首次游玩", value: game.play_tracker?.first_played ? sessionDate(game.play_tracker.first_played) : "" },
      { label: "完成次数", value: game.play_tracker?.completion_count ? `${game.play_tracker.completion_count} 次` : "" },
    ];
    return facts.filter((fact) => fact.value);
  });
  const recentSessions = $derived.by(() =>
    [...(game?.play_tracker?.sessions ?? [])]
      .sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime())
      .slice(0, 3)
  );

  function sessionDate(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value || "未记录";
    return date.toLocaleDateString(i18n.locale, { month: "2-digit", day: "2-digit" });
  }

  onMount(() => {
    const reduce =
      window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches ||
      document.documentElement.dataset.motion === "reduce";
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
  title={game?.name ?? i18n.t("gamedetail.missing_title")}
  description={game ? [developer, releaseYear !== "----" ? releaseYear : "", platform].filter(Boolean).join(" · ") : i18n.t("gamedetail.missing_desc")}
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

        <aside class="hero-contact-sheet" aria-label={i18n.t("gamedetail.visual_archive_aria")}>
          <span class="contact-label">VISUAL CONTACT / {String(game.id).padStart(3, "0")}</span>
          <div class="contact-grid">
            {#each screenshots.slice(0, 3) as shot, index}
              <figure class:contact-wide={index === 0}><img src={shot} alt={i18n.t("gamedetail.visual_archive_alt", { name: game.name, index: index + 1 })} loading={index === 0 ? "eager" : "lazy"} /></figure>
            {/each}
            {#if screenshots.length === 0 && coverSource}
              <figure class="contact-wide"><CachedImage source={coverSource} cacheKey={`detail-contact-${game.id}`} alt={i18n.t("gamedetail.cover_alt", { name: game.name })} loading="eager" /></figure>
            {/if}
          </div>
        </aside>

        <div class="hero-floor">
          <div class="poster">
            <div class="poster-frame">
              {#if coverSource}
                <CachedImage source={coverSource} cacheKey={`detail-cover-${game.id}`} alt={i18n.t("gamedetail.cover_alt", { name: game.name })} loading="eager" />
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
              {#if platformStore.capabilities.gameLaunch}
                <Button class="game-detail-primary" press={handleLaunch}>{i18n.t("gamedetail.launch")}</Button>
                <Button variant="secondary" press={handleLaunchJP}>{i18n.t("gamedetail.launch_jp")}</Button>
              {:else}
                <div class="mobile-companion-note" role="status">
                  <Icon name="smartphone" size={18} />
                  <span>{i18n.t("gamedetail.mobile_note")}</span>
                </div>
              {/if}
              <Button variant="secondary" press={handleScrape}>{i18n.t("button.scrape")}</Button>
              <Button variant="ghost" press={openEdit}>{i18n.t("gamedetail.edit")}</Button>
            </div>
          </div>

          {#if rating > 0}<div class="hero-rating"><RatingRing value={rating} max={10} size={68} /></div>{/if}
        </div>
      </div>

      <div class="body">
        <section class="editorial-dossier" aria-label="作品档案">
          <div class="dossier-index"><span>01 / SYNOPSIS</span><strong>{i18n.t("gamedetail.synopsis_title")}</strong></div>
          <p class="desc">{game.description || i18n.t("gamedetail.synopsis_empty")}</p>
          {#if archiveFacts.length}
            <dl class="archive-facts">
              {#each archiveFacts as fact}<div><dt>{fact.label}</dt><dd>{fact.value}</dd></div>{/each}
            </dl>
          {/if}
          {#if privateReview}
            <blockquote class="private-review">
              <span>02 / MY REVIEW</span>
              <p>{privateReview}</p>
              {#if rating > 0}<footer>PRIVATE SCORE · {rating.toFixed(1)} / 10</footer>{/if}
            </blockquote>
          {/if}
        </section>

        {#if screenshots.length}
          <section class="gallery" bind:this={galleryEl} aria-label={i18n.t("gamedetail.screenshots_aria")}>
            <div class="gallery-track">
              {#each screenshots.slice(0, 8) as shot, index}
                <figure class="shot"><img src={shot} alt={i18n.t("gamedetail.screenshot_alt", { name: game.name, index: index + 1 })} loading="lazy" /></figure>
              {/each}
            </div>
          </section>
        {/if}

        <section class="panels" aria-label={i18n.t("gamedetail.panels_aria")}>
          <SavePanel gameId={game.id} saveDir={saveDirOf(game)} compact />

          <Card class="panel" padding="none">
            <div class="panel-head"><span class="panel-label">Achievements</span><h3>{i18n.t("gamedetail.achievements_title")}</h3></div>
            <div class="panel-body">
              <div class="achieve-row"><div class="achieve-nums"><b>{achievementTotal ? `${achievementPercent}%` : "--"}</b><span>{achievementUnlocked} / {achievementTotal}</span></div></div>
              <div class="achieve-bar" aria-label={i18n.t("gamedetail.achievements_progress_aria", { percent: achievementPercent })}><i style={`width:${achievementPercent}%`}></i></div>
              <p class="panel-note">{achievementTotal ? i18n.t("gamedetail.achievements_note") : i18n.t("gamedetail.achievements_none")}</p>
            </div>
          </Card>

          <Card class="panel" padding="none">
            <div class="panel-head"><span class="panel-label">Recent Play</span><h3>{i18n.t("gamedetail.sessions_title")}</h3></div>
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
              {:else}<p class="panel-note">{i18n.t("gamedetail.sessions_empty")}</p>{/if}
            </div>
          </Card>
        </section>

        <GameNotes gameId={game.id} />
      </div>

      <Dialog open={isEditing} onClose={() => isEditing = false} title={i18n.t("gamedetail.edit_title", { name: game.name })}>
        <div class="edit-panel">
          <header class="edit-header">
            <h3>{i18n.t("gamedetail.edit_title", { name: game.name })}</h3>
            <button class="edit-close" onclick={() => isEditing = false} aria-label={i18n.t("gamedetail.edit_close_aria")}><Icon name="x" size={18} /></button>
          </header>
          <div class="edit-body">
            <div class="edit-field"><label for="edit-name">{i18n.t("gamedetail.edit_name")}</label><Input id="edit-name" bind:value={editName} /></div>
            <div class="edit-field"><label for="edit-exe">{i18n.t("gamedetail.edit_exe")}</label><Input id="edit-exe" class="mono" bind:value={editExePath} /></div>
            <div class="edit-field"><label for="edit-desc">{i18n.t("gamedetail.edit_desc")}</label><textarea id="edit-desc" bind:value={editDesc} rows={5}></textarea></div>
          </div>
          <footer class="edit-footer">
            <Button press={saveEdit} disabled={isSaving}>{isSaving ? i18n.t("gamedetail.edit_saving") : i18n.t("gamedetail.edit_save")}</Button>
            <Button variant="ghost" press={() => isEditing = false}>{i18n.t("button.cancel")}</Button>
          </footer>
        </div>
      </Dialog>
    </section>
  {:else}
    <AsyncState
      state="error"
      title={i18n.t("gamedetail.missing_title")}
      description={i18n.t("gamedetail.missing_desc")}
      primaryAction={{ label: i18n.t("gamedetail.back_to_library"), onSelect: closeDetail }}
      class="game-detail-missing"
    />
  {/if}
</DetailPanel>

<style>
  :global(.game-detail-panel .v2-detail-panel__body) { padding: 0; background: var(--bg-deep); }
  :global(.game-detail-panel.v2-detail-panel) { width: 100vw; min-width: 100vw; border-left: 0; }
  :global(.game-detail-panel .v2-detail-panel__header) { min-height: 52px; padding: 10px 22px; background: rgba(7,9,13,.94); backdrop-filter: blur(18px); }
  :global(.game-detail-panel .v2-detail-panel__title) { font: 700 11px/1 var(--font-mono); letter-spacing: .1em; text-transform: uppercase; }
  :global(.game-detail-panel .v2-detail-panel__description) { font-size: 10px; }
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

  .hero-contact-sheet {
    position: absolute;
    z-index: 2;
    top: clamp(20px, 4vh, 48px);
    left: clamp(24px, 4vw, 72px);
    width: min(32vw, 520px);
    display: grid;
    gap: 8px;
  }
  .contact-label { color: rgba(255,255,255,.72); font: 700 8px/1 var(--font-mono); letter-spacing: .14em; }
  .contact-grid { display: grid; grid-template-columns: 1.45fr .75fr; grid-template-rows: repeat(2, minmax(58px, 1fr)); gap: 6px; height: clamp(130px, 23vh, 240px); padding: 6px; border: 1px solid rgba(255,255,255,.22); background: rgba(7,9,13,.24); backdrop-filter: blur(10px); }
  .contact-grid figure { min-width: 0; min-height: 0; margin: 0; overflow: hidden; background: rgba(255,255,255,.05); }
  .contact-grid .contact-wide { grid-row: 1 / span 2; }
  .contact-grid img, .contact-grid :global(.cached-image) { width: 100%; height: 100%; display: block; object-fit: cover; filter: saturate(.88) contrast(1.03); transition: transform .5s cubic-bezier(.22,.75,.18,1), filter .3s ease; }
  .contact-grid figure:hover img, .contact-grid figure:hover :global(.cached-image) { transform: scale(1.035); filter: saturate(1) contrast(1.05); }

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
    .hero-contact-sheet { width: min(42vw, 420px); }
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
  @media (max-width: 720px) {
    .hero-contact-sheet { display: none; }
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

  .editorial-dossier { display:grid; grid-template-columns:clamp(7rem,16vw,11rem) minmax(0,1fr); gap:18px 28px; padding:clamp(22px,4vw,44px); border-block:1px solid var(--border); background:linear-gradient(120deg,color-mix(in srgb,var(--bg-card) 84%,transparent),transparent); }
  .dossier-index { display:grid; align-content:start; gap:8px; padding-top:4px; border-top:2px solid var(--accent); }
  .dossier-index span,.private-review>span { color:var(--accent); font:700 8px/1 var(--font-mono); letter-spacing:.14em; }
  .dossier-index strong { font:700 13px/1.2 var(--font-ui); }
  .editorial-dossier>.desc { max-width:70ch; margin:0; color:var(--text-secondary); font-size:14px; line-height:1.78; text-wrap:pretty; }
  .archive-facts { grid-column:2; display:grid; grid-template-columns:repeat(auto-fit,minmax(9rem,1fr)); margin:0; border-top:1px solid var(--border); }
  .archive-facts div { min-width:0; padding:12px 12px 12px 0; border-bottom:1px solid var(--border); }
  .archive-facts dt { color:var(--text-muted); font:700 8px/1 var(--font-mono); letter-spacing:.1em; }
  .archive-facts dd { margin:7px 0 0; overflow:hidden; color:var(--text-primary); font-size:12px; text-overflow:ellipsis; white-space:nowrap; }
  .private-review { grid-column:1/-1; margin:6px 0 0; padding:18px 20px; border-left:3px solid var(--accent); background:color-mix(in srgb,var(--accent) 7%,var(--bg-card)); }
  .private-review p { max-width:74ch; margin:10px 0; color:var(--text-primary); font:500 clamp(1rem,1.5vw,1.25rem)/1.65 var(--font-ui); }
  .private-review footer { color:var(--text-muted); font:700 8px/1 var(--font-mono); letter-spacing:.12em; }
  @media(max-width:42rem){.editorial-dossier{grid-template-columns:1fr}.archive-facts{grid-column:1}.private-review{grid-column:1}}
  .mobile-companion-note { display: inline-flex; align-items: center; gap: 8px; min-height: 44px; padding: 0 14px; border: 1px solid color-mix(in srgb, var(--accent) 32%, transparent); border-radius: 10px; color: var(--text-secondary); background: color-mix(in srgb, var(--surface-2) 82%, transparent); font-size: 13px; }

  /* ── Reduced motion：media query 与 data-motion 双信号降级 ── */
  @media (prefers-reduced-motion: reduce) {
    .bg-layer { animation: none; }
    .poster-frame,
    .chip,
    .shot,
    .session-row,
    .achieve-bar i,
    .contact-grid img,
    .contact-grid :global(.cached-image) { transition: none; }
    .poster-frame:hover { transform: none; }
    .contact-grid figure:hover img,
    .contact-grid figure:hover :global(.cached-image) { transform: none; }
  }
  :global([data-motion="reduce"]) .bg-layer { animation: none; }
  :global([data-motion="reduce"]) .poster-frame,
  :global([data-motion="reduce"]) .chip,
  :global([data-motion="reduce"]) .shot,
  :global([data-motion="reduce"]) .session-row,
  :global([data-motion="reduce"]) .achieve-bar i,
  :global([data-motion="reduce"]) .contact-grid img,
  :global([data-motion="reduce"]) .contact-grid :global(.cached-image) { transition: none; }
  :global([data-motion="reduce"]) .poster-frame:hover { transform: none; }
  :global([data-motion="reduce"]) .contact-grid figure:hover img,
  :global([data-motion="reduce"]) .contact-grid figure:hover :global(.cached-image) { transform: none; }
</style>
