<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore, type Game } from "../../stores/games.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { navigateTo } from "../../stores/router.svelte";
  import { i18n } from "../../stores/i18n.svelte";
  import { formatPlayTime } from "../../api";
  import CachedImage from "../CachedImage.svelte";
  import Icon from "../Icon.svelte";
  import { attachGamepad } from "./useGamepad.svelte";
  import {
    coverOf,
    developerOf,
    gameRating,
    gameTotalSeconds,
    heroImageOf,
    originalNameOf,
  } from "../../utils/game";

  type HeroSession = Game["play_tracker"]["sessions"][number];
  type HeroItem = { game: Game; session: HeroSession };

  const MAX_ITEMS = 5;

  let scroller = $state<HTMLDivElement>();
  let focusIndex = $state(0);

  function sessionTime(value: string | null | undefined): number {
    return value ? new Date(value).getTime() || 0 : 0;
  }

  function latestSessionOf(game: Game): HeroSession | null {
    let best: HeroSession | null = null;
    for (const session of game.play_tracker?.sessions ?? []) {
      if (!best || sessionTime(session.start_time) >= sessionTime(best.start_time)) best = session;
    }
    return best;
  }

  // 最近 session start_time 降序取 top 5；无 sessions 的游戏直接排除。
  const items = $derived.by<HeroItem[]>(() =>
    gameStore.games
      .map((game) => ({ game, session: latestSessionOf(game) }))
      .filter((item): item is HeroItem => item.session !== null)
      .sort((a, b) => sessionTime(b.session.start_time) - sessionTime(a.session.start_time))
      .slice(0, MAX_ITEMS),
  );

  // 滚动平滑同样双通道降级：OS media query + 应用内 data-motion="reduce"。
  function prefersReducedMotion(): boolean {
    if (typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches) return true;
    return typeof document !== "undefined" && document.documentElement.dataset.motion === "reduce";
  }

  function focusCard(index = focusIndex) {
    queueMicrotask(() => {
      scroller?.querySelector<HTMLElement>(`[data-idx="${index}"] [data-focus-key]`)?.focus({ preventScroll: true });
    });
  }

  function syncIndex(index: number, moveFocus = false) {
    focusIndex = Math.max(0, Math.min(items.length - 1, index));
    if (moveFocus) focusCard(focusIndex);
  }

  function move(delta: number) {
    syncIndex(focusIndex + delta, true);
  }

  // 与 GameDetailPage.handleLaunch 同款：直接交给 store 启动并给出提示。
  function launchGame(game: Game) {
    void gameStore.launch(game.id);
    uiStore.notify(i18n.t("home.continue_hero.launching", { name: game.name }), "info");
  }

  // 与 SwitchHome.onactivate / GameCard 进入详情同款调用：先选中，再进档案。
  function openProfile(game: Game) {
    gameStore.selectGame(game.id);
    navigateTo("game-detail", { entity: { kind: "game", id: game.id }, focus: "start" });
  }

  function launchCurrent() {
    const item = items[focusIndex];
    if (item) {
      focusCard();
      launchGame(item.game);
    }
  }

  function activateCurrent() {
    const item = items[focusIndex];
    if (item) {
      focusCard();
      openProfile(item.game);
    }
  }

  // 与 GameDetailPage.sessionDate 同款：MM/DD，随 i18n.locale 联动。
  function sessionDate(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value || "未记录";
    return date.toLocaleDateString(i18n.locale, { month: "2-digit", day: "2-digit" });
  }

  function handleRailKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowRight":
        move(1); event.preventDefault(); break;
      case "ArrowLeft":
        move(-1); event.preventDefault(); break;
      case "Home":
        syncIndex(0, true); event.preventDefault(); break;
      case "End":
        syncIndex(items.length - 1, true); event.preventDefault(); break;
    }
  }

  function handleWheel(event: WheelEvent) {
    if (Math.abs(event.deltaY) < 1 && Math.abs(event.deltaX) < 1) return;
    event.preventDefault();
    move(event.deltaY > 0 || event.deltaX > 0 ? 1 : -1);
  }

  function railInteraction(node: HTMLElement) {
    const wheel = (event: WheelEvent) => handleWheel(event);
    const keydown = (event: KeyboardEvent) => handleRailKeydown(event);
    node.addEventListener("wheel", wheel, { passive: false });
    node.addEventListener("keydown", keydown);
    return {
      destroy() {
        node.removeEventListener("wheel", wheel);
        node.removeEventListener("keydown", keydown);
      },
    };
  }

  $effect(() => {
    if (focusIndex > items.length - 1) focusIndex = Math.max(0, items.length - 1);
  });

  $effect(() => {
    const index = focusIndex;
    if (index < 0 || index >= items.length) return;
    queueMicrotask(() => {
      scroller?.querySelector<HTMLElement>(`[data-idx="${index}"]`)?.scrollIntoView({
        inline: "center",
        block: "nearest",
        behavior: prefersReducedMotion() ? "auto" : "smooth",
      });
    });
  });

  onMount(() => {
    const detachGamepad = attachGamepad({
      left: () => move(-1),
      right: () => move(1),
      activate: activateCurrent,
      launch: launchCurrent,
    });
    return () => {
      detachGamepad();
    };
  });
</script>

{#if items.length > 0}
  <section class="continue-hero" aria-label={i18n.t("home.continue_hero.aria")} data-testid="continue-hero-rail">
    <header class="continue-hero__head">
      <span class="continue-hero__kicker" aria-hidden="true">CONTINUE</span>
      <h2 class="continue-hero__title">{i18n.t("home.continue_hero.title")}</h2>
      <span class="continue-hero__count">{i18n.t("home.continue_hero.count", { count: items.length })}</span>
    </header>
    <div class="continue-hero__rail" role="list" bind:this={scroller} use:railInteraction>
      {#each items as item, index (item.game.id)}
        {@const hero = heroImageOf(item.game)}
        {@const cover = coverOf(item.game)}
        {@const orig = originalNameOf(item.game)}
        {@const developer = developerOf(item.game)}
        {@const rating = gameRating(item.game)}
        {@const totalSeconds = gameTotalSeconds(item.game)}
        {@const achTotal = item.game.play_tracker?.achievements_total ?? 0}
        {@const achUnlocked = item.game.play_tracker?.achievements_unlocked ?? 0}
        {@const achPercent = achTotal > 0 ? Math.min(100, Math.round((achUnlocked / achTotal) * 100)) : 0}
        <div class="continue-hero__slot" data-idx={index} role="listitem">
          <article class="hero-card">
            <div class="hero-card__bg" aria-hidden="true">
              {#if hero}<CachedImage source={hero} cacheKey={`continue-hero-bg-${item.game.id}`} loading="lazy" alt="" />{/if}
            </div>
            <div class="hero-card__scrim" aria-hidden="true"></div>
            <div class="hero-card__cover">
              {#if cover}
                <CachedImage source={cover} cacheKey={`continue-hero-cover-${item.game.id}`} loading="lazy" alt={item.game.name} />
              {:else}
                <span class="hero-card__monogram" aria-hidden="true">{(item.game.name.trim()[0] ?? "?").toUpperCase()}</span>
              {/if}
            </div>
            <div class="hero-card__info">
              {#if orig && orig !== item.game.name}<p class="hero-card__orig">{orig}</p>{/if}
              <h3 class="hero-card__name">{item.game.name}</h3>
              <p class="hero-card__meta">
                {#if developer}<span>{developer}</span>{/if}
                {#if rating > 0}<span>★ {rating.toFixed(1)}</span>{/if}
                <span>{i18n.t("home.continue_hero.last_session")} {sessionDate(item.session.start_time)}</span>
                <span>{i18n.t("home.continue_hero.total_playtime")} {formatPlayTime(totalSeconds)}</span>
              </p>
              {#if achTotal > 0}
                <div class="hero-card__achievements">
                  <span class="hero-card__achievements-label">{i18n.t("home.continue_hero.achievements")} {achUnlocked}/{achTotal}</span>
                  <span
                    class="hero-card__achievements-track"
                    role="progressbar"
                    aria-valuenow={achPercent}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-label={i18n.t("home.continue_hero.achievements_aria", { percent: achPercent })}
                  >
                    <span class="hero-card__achievements-fill" style={`width:${achPercent}%`}></span>
                  </span>
                  <span class="hero-card__achievements-percent">{achPercent}%</span>
                </div>
              {/if}
              <div class="hero-card__actions">
                <button
                  type="button"
                  class="hero-card__btn hero-card__btn--primary"
                  data-focus-key={`continue-hero-${item.game.id}`}
                  tabindex={index === focusIndex ? 0 : -1}
                  onfocus={() => syncIndex(index)}
                  onclick={() => launchGame(item.game)}
                ><Icon name="play" size={13} /> {i18n.t("home.continue_hero.launch")}</button>
                <button
                  type="button"
                  class="hero-card__btn hero-card__btn--ghost"
                  tabindex={index === focusIndex ? 0 : -1}
                  onfocus={() => syncIndex(index)}
                  onclick={() => openProfile(item.game)}
                >{i18n.t("home.continue_hero.open_profile")}</button>
              </div>
            </div>
          </article>
        </div>
      {/each}
    </div>
  </section>
{/if}

<style>
  .continue-hero { display: grid; gap: 10px; padding: 14px 26px 10px; border-bottom: 1px solid var(--border); background: color-mix(in srgb, var(--bg-deep) 55%, transparent); }
  .continue-hero__head { display: flex; align-items: baseline; gap: 12px; }
  .continue-hero__kicker { color: var(--accent); font: 600 9px/1 var(--font-mono); letter-spacing: .18em; }
  .continue-hero__title { margin: 0; color: var(--text-primary); font-family: var(--font-display); font-size: 1.05rem; letter-spacing: -.02em; }
  .continue-hero__count { margin-left: auto; color: var(--text-muted); font: 600 10px/1 var(--font-mono); letter-spacing: .08em; }

  .continue-hero__rail { display: flex; gap: 14px; overflow-x: auto; overflow-y: hidden; scroll-snap-type: x mandatory; scrollbar-width: none; outline: none; }
  .continue-hero__rail::-webkit-scrollbar { display: none; }
  .continue-hero__slot { flex: 0 0 100%; min-width: 0; scroll-snap-align: center; }

  .hero-card { position: relative; min-height: 188px; display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 20px; align-items: end; padding: 18px 22px; border: 1px solid var(--border); background: var(--bg-deep); overflow: hidden; isolation: isolate; transition: border-color .22s ease; }
  .hero-card:hover, .hero-card:focus-within { border-color: var(--border-hover); }
  .hero-card__bg { position: absolute; inset: 0; z-index: -2; opacity: .9; transform: scale(1.02); transition: transform .6s ease, opacity .3s ease; }
  .hero-card:hover .hero-card__bg, .hero-card:focus-within .hero-card__bg { transform: scale(1.06); opacity: 1; }
  .hero-card__bg :global(.cached-image) { filter: brightness(var(--wallpaper-brightness, .78)) saturate(.9); }
  .hero-card__scrim { position: absolute; inset: 0; z-index: -1; background:
    linear-gradient(90deg, color-mix(in srgb, var(--bg-deep) 88%, transparent) 0%, color-mix(in srgb, var(--bg-deep) 38%, transparent) 46%, transparent 74%),
    linear-gradient(0deg, color-mix(in srgb, var(--bg-deep) 86%, transparent) 0%, transparent 60%);
    transition: opacity .3s ease; }
  .hero-card__cover { position: relative; z-index: 1; width: 118px; aspect-ratio: 3 / 4; display: grid; place-items: center; border: 1px solid var(--border); background: var(--bg-elev); overflow: hidden; transition: transform .24s ease; }
  .hero-card:hover .hero-card__cover, .hero-card:focus-within .hero-card__cover { transform: translateY(-2px); }
  .hero-card__monogram { color: var(--text-muted); font: 700 2rem/1 var(--font-display); }
  .hero-card__info { position: relative; z-index: 1; min-width: 0; display: grid; gap: 7px; }
  .hero-card__orig { margin: 0; color: var(--text-muted); font: 600 10px/1.3 var(--font-mono); letter-spacing: .06em; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hero-card__name { margin: 0; color: var(--text-primary); font-family: var(--font-display); font-size: clamp(1.1rem, 2vw, 1.5rem); letter-spacing: -.03em; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hero-card__meta { margin: 0; display: flex; flex-wrap: wrap; gap: 5px 14px; color: var(--text-secondary); font: 600 11px/1.4 var(--font-ui); }
  .hero-card__achievements { display: flex; align-items: center; gap: 10px; max-width: 330px; }
  .hero-card__achievements-label { color: var(--text-muted); font: 600 10px/1 var(--font-mono); letter-spacing: .06em; white-space: nowrap; }
  .hero-card__achievements-track { flex: 1; height: 4px; background: color-mix(in srgb, var(--text-primary) 14%, transparent); overflow: hidden; }
  .hero-card__achievements-fill { display: block; height: 100%; background: var(--accent); transition: width .3s ease; }
  .hero-card__achievements-percent { color: var(--accent); font: 700 10px/1 var(--font-mono); }
  .hero-card__actions { display: flex; gap: 10px; margin-top: 3px; }
  .hero-card__btn { min-height: 2.25rem; display: inline-flex; align-items: center; gap: 6px; padding: 0 14px; border: 1px solid var(--border); background: color-mix(in srgb, var(--bg-deep) 55%, transparent); color: var(--text-secondary); font: 650 12px/1 var(--font-ui); cursor: pointer; transition: transform .18s ease, border-color .18s ease, background .18s ease, color .18s ease, opacity .18s ease; }
  .hero-card__btn:hover { border-color: var(--border-hover); color: var(--text-primary); }
  .hero-card__btn:active { transform: translateY(1px); }
  .hero-card__btn:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  .hero-card__btn--primary { border-color: transparent; background: var(--accent); color: var(--bg-deep); }
  .hero-card__btn--primary:hover { background: var(--accent-hi); color: var(--bg-deep); }

  @media (max-width: 760px) {
    .continue-hero { padding: 12px 14px 8px; }
    .hero-card { gap: 12px; min-height: 160px; padding: 14px; }
    .hero-card__cover { width: 84px; }
    .hero-card__meta { gap: 4px 10px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .continue-hero__rail { scroll-behavior: auto; }
    .hero-card, .hero-card__bg, .hero-card__scrim, .hero-card__cover, .hero-card__btn, .hero-card__achievements-fill { transition: none; }
  }
  :global([data-motion="reduce"]) .continue-hero__rail { scroll-behavior: auto; }
  :global([data-motion="reduce"]) .hero-card,
  :global([data-motion="reduce"]) .hero-card__bg,
  :global([data-motion="reduce"]) .hero-card__scrim,
  :global([data-motion="reduce"]) .hero-card__cover,
  :global([data-motion="reduce"]) .hero-card__btn,
  :global([data-motion="reduce"]) .hero-card__achievements-fill { transition: none; }
</style>
