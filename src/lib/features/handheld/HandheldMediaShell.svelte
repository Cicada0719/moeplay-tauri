<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import { attachGamepad, type GamepadHandlers } from "../../components/switch/useGamepad.svelte";
  import { gamepadGlyphFor } from "../../platform/gamepadRemap";
  import { orientationStore, platformStore } from "../../platform";
  import { navigateTo } from "../../stores/router.svelte";
  import HandheldArtworkStage from "./HandheldArtworkStage.svelte";
  import HandheldStatePanel from "./HandheldStatePanel.svelte";
  import type { HandheldMediaKind, HandheldMediaShellProps } from "./mediaTypes";

  export type { HandheldMediaAction, HandheldMediaKind, HandheldMediaShellProps, NovelReadingMode } from "./mediaTypes";

  let {
    kind,
    title,
    subtitle = "",
    progress = 0,
    progressLabel = "",
    chromeMode = "auto",
    artwork,
    state: viewState,
    panelOpen = false,
    keepChromeVisible = false,
    onback,
    children,
    headerActions,
    overlay,
    footer,
    handlers = {},
  }: HandheldMediaShellProps = $props();

  let chromeVisible = $state(true);
  let hideTimer: ReturnType<typeof setTimeout> | null = null;
  let pad: ReturnType<typeof attachGamepad> | null = null;

  const labels: Record<HandheldMediaKind, { eyebrow: string; label: string; accent: string }> = {
    anime: { eyebrow: "ANIME / WATCH", label: "番剧", accent: "coral" },
    comic: { eyebrow: "COMIC / READ", label: "漫画", accent: "cyan" },
    novel: { eyebrow: "NOVEL / READ", label: "小说", accent: "amber" },
  };

  const media = $derived(labels[kind]);
  const routeNav = [
    { id: "home", label: "主屏幕", icon: "home", view: "home" },
    { id: "game-library", label: "游戏库", icon: "gamepad", view: "game-library" },
    { id: "anime", label: "番剧", icon: "tv", view: "anime" },
    { id: "comic", label: "漫画", icon: "image", view: "comic" },
    { id: "novel", label: "小说", icon: "book", view: "novel" },
  ] as const;
  const safeProgress = $derived(Math.min(1, Math.max(0, Number.isFinite(progress) ? progress : 0)));
  const isAndroid = $derived(platformStore.isAndroid);
  const autoChrome = $derived(chromeMode === "auto");
  const backDockVisible = $derived(Boolean(onback && autoChrome && !chromeVisible && !panelOpen && !keepChromeVisible));

  function clearHideTimer() {
    if (hideTimer !== null) {
      clearTimeout(hideTimer);
      hideTimer = null;
    }
  }

  function revealChrome() {
    chromeVisible = true;
    clearHideTimer();
    if (autoChrome && !keepChromeVisible && !panelOpen) {
      hideTimer = setTimeout(() => {
        hideTimer = null;
        chromeVisible = false;
      }, 3000);
    }
  }

  function scheduleHide() {
    clearHideTimer();
    if (!autoChrome || keepChromeVisible || panelOpen) {
      chromeVisible = true;
      return;
    }
    hideTimer = setTimeout(() => {
      hideTimer = null;
      chromeVisible = false;
    }, 3000);
  }

  function callHandler(action: keyof GamepadHandlers) {
    revealChrome();
    handlers[action]?.();
  }

  function navigateRoute(view: string) {
    if (view === kind) return;
    navigateTo(view);
  }

  function mediaHandlers(): GamepadHandlers {
    const result: GamepadHandlers = {};
    const available = { ...handlers, ...(handlers.back || !onback ? {} : { back: onback }) };
    for (const action of ["up", "down", "left", "right", "pageLeft", "pageRight", "categoryLeft", "categoryRight", "activate", "launch", "favorite", "filter", "back", "start"] as const) {
      if (available[action]) result[action] = () => callHandler(action);
    }
    return result;
  }

  function onKeydown() {
    revealChrome();
  }

  function onPointerMove(event: PointerEvent) {
    if (event.target instanceof Element && event.target.closest(".hms-back-dock")) return;
    revealChrome();
  }

  function onPointerDown(event: PointerEvent) {
    if (event.target instanceof Element && event.target.closest(".hms-back-dock")) return;
    revealChrome();
  }

  $effect(() => {
    panelOpen;
    keepChromeVisible;
    chromeMode;
    scheduleHide();
  });

  $effect(() => {
    const currentHandlers = mediaHandlers();
    pad?.updateHandlers(currentHandlers);
  });

  onMount(() => {
    if (isAndroid) void orientationStore.enterMediaLandscape();
    pad = attachGamepad(mediaHandlers(), {
      id: `handheld-media-${kind}`,
      zone: "content",
      priority: 80,
    });
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener("pointermove", onPointerMove, { passive: true });
    window.addEventListener("pointerdown", onPointerDown, { passive: true });
    scheduleHide();
    return () => {
      pad?.();
      pad = null;
      window.removeEventListener("keydown", onKeydown, true);
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerdown", onPointerDown);
      clearHideTimer();
      if (isAndroid) void orientationStore.exitMediaLandscape();
    };
  });
</script>

  <section
    class="handheld-media-shell handheld-media-shell--{media.accent}"
    class:hms-mode-minimal={chromeMode === "minimal"}
    class:chrome-hidden={autoChrome && !chromeVisible && !panelOpen && !keepChromeVisible}
  data-testid="handheld-media-shell"
  data-media-kind={kind}
  data-media-mode="handheld"
  aria-label={`${media.label}掌机页面`}
>
  {#if artwork}
    <HandheldArtworkStage class="hms-artwork" source={artwork} role={artwork.role} strength="soft" fill />
  {/if}

  <header class="hms-chrome hms-header">
    <div class="hms-header-main hms-brand-group">
      {#if onback}
        <button class="hms-back" type="button" aria-label="返回" data-gamepad-activate="返回" onclick={onback}>
          <Icon name="arrowLeft" size={18} />
        </button>
      {/if}
      <button class="hms-brand" type="button" aria-label="返回掌机主屏幕" onclick={() => navigateTo("home")}>
        <strong>萌游</strong><small>PORTABLE / XMB</small>
      </button>
      <div class="hms-title-block">
        <div class="hms-eyebrow"><span>{media.eyebrow}</span><b>{media.label}</b></div>
        <strong>{title || media.label}</strong>
        {#if subtitle}<small>{subtitle}</small>{/if}
      </div>
    </div>
    <nav class="hms-nav" aria-label="掌机主导航">
      {#each routeNav as item (item.id)}
        <button type="button" class:active={item.id === kind} aria-current={item.id === kind ? "page" : undefined} onclick={() => navigateRoute(item.view)}>
          <Icon name={item.icon} size={14} /><span>{item.label}</span>
        </button>
      {/each}
    </nav>
    <div class="hms-header-side">
      <button class="hms-quick-action" type="button" aria-label="打开模拟器导入" onclick={() => navigateTo("handheld-import")}><Icon name="database" size={14} /><span>导入</span></button>
      <button class="hms-quick-action" type="button" aria-label="打开掌机设置" onclick={() => navigateTo("settings")}><Icon name="settings" size={15} /><span>设置</span></button>
      {#if progressLabel}<span class="hms-progress-label">{progressLabel}</span>{/if}
      {#if headerActions}{@render headerActions()}{/if}
    </div>
  </header>

  {#if onback}
    <button
      class="hms-back-dock"
      class:visible={backDockVisible}
      type="button"
      aria-label="返回"
      aria-hidden={backDockVisible ? undefined : "true"}
      tabindex={backDockVisible ? 0 : -1}
      data-gamepad-activate="返回"
      onclick={onback}
    >
      <Icon name="arrowLeft" size={18} />
    </button>
  {/if}

  <div class="hms-stage" onclick={revealChrome} onkeydown={onKeydown} role="presentation">
    {#if viewState && viewState.state !== "ready" && viewState.state !== "partial"}
      <HandheldStatePanel
        state={viewState.state}
        title={viewState.title}
        description={viewState.description}
        primaryAction={viewState.primaryAction}
        secondaryAction={viewState.secondaryAction}
      />
    {:else}
      {@render children()}
    {/if}
  </div>

  <footer class="hms-chrome hms-footer">
    <div class="hms-progress" aria-label={progressLabel || "阅读进度"}>
      <span style={`width:${safeProgress * 100}%`}></span>
    </div>
    <div class="hms-hints">
      <span><kbd>{gamepadGlyphFor("launch", "xbox")}</kbd>确认</span>
      <span><kbd>{gamepadGlyphFor("back", "xbox")}</kbd>返回</span>
      <span><kbd>{gamepadGlyphFor("pageLeft", "xbox")}</kbd><kbd>{gamepadGlyphFor("pageRight", "xbox")}</kbd>上一页 / 下一页</span>
      <span><kbd>{gamepadGlyphFor("favorite", "xbox")}</kbd>选集</span>
      <span><kbd>{gamepadGlyphFor("activate", "xbox")}</kbd>显示设置</span>
      <span><kbd>START</kbd>更多</span>
    </div>
  </footer>

  {#if footer}{@render footer()}{/if}
  {#if overlay}{@render overlay()}{/if}
</section>

<style>
  .handheld-media-shell {
    --hms-accent: #f06d58;
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    isolation: isolate;
    background:
      radial-gradient(circle at 76% 8%, color-mix(in srgb, var(--hms-accent) 13%, transparent), transparent 34%),
      #07090c;
    color: #f4f5f7;
  }

  .hms-artwork { position: absolute; inset: 0; z-index: 0; pointer-events: none; }
  .hms-mode-minimal { grid-template-rows: auto minmax(0, 1fr) 0; }
  .hms-mode-minimal .hms-footer { display: none; }

  .handheld-media-shell--coral { --hms-accent: #f06d58; }
  .handheld-media-shell--cyan { --hms-accent: #55c4d8; }
  .handheld-media-shell--amber { --hms-accent: #d5a35d; }

  .hms-chrome {
    position: relative;
    z-index: 10;
    transition: opacity 180ms ease, transform 220ms ease;
  }
  .chrome-hidden { grid-template-rows: 0 minmax(0, 1fr) 0; }
  .chrome-hidden .hms-header { opacity: 0; transform: translateY(-105%); pointer-events: none; }
  .chrome-hidden .hms-footer { opacity: 0; transform: translateY(105%); pointer-events: none; }

  .hms-header {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr) max-content;
    align-items: center;
    gap: clamp(8px, 1.4vw, 18px);
    min-height: 64px;
    padding: max(8px, env(safe-area-inset-top)) max(14px, env(safe-area-inset-right)) 8px max(14px, env(safe-area-inset-left));
    border-bottom: 1px solid rgba(255, 255, 255, .1);
    background: rgba(7, 9, 12, .88);
    backdrop-filter: blur(14px) saturate(1.1);
  }
  .hms-header-main, .hms-header-side, .hms-eyebrow, .hms-hints { display: flex; align-items: center; }
  .hms-header-main { min-width: 0; gap: 10px; }
  .hms-header-side { flex: 0 0 auto; gap: 6px; min-width: max-content; }
  .hms-back {
    display: grid;
    width: 44px;
    height: 44px;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid rgba(255, 255, 255, .16);
    border-radius: 12px;
    background: rgba(255, 255, 255, .04);
    color: #fff;
    cursor: pointer;
  }
  .hms-back:hover { border-color: var(--hms-accent); background: color-mix(in srgb, var(--hms-accent) 14%, transparent); }
  .hms-brand {
    display: grid;
    gap: 3px;
    min-width: 78px;
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .hms-brand strong { color: var(--hms-accent); font: 850 1.08rem/1 var(--font-display, system-ui); letter-spacing: .05em; text-shadow: 0 0 18px color-mix(in srgb, var(--hms-accent) 34%, transparent); }
  .hms-brand small { color: rgba(255, 255, 255, .42); font: 700 7px/1 var(--font-mono, monospace); letter-spacing: .1em; white-space: nowrap; }
  .hms-nav { display: flex; align-items: center; justify-content: center; gap: 3px; min-width: 0; overflow-x: auto; scrollbar-width: none; }
  .hms-nav::-webkit-scrollbar { display: none; }
  .hms-nav button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    min-width: 58px;
    min-height: 40px;
    padding: 0 7px;
    border: 1px solid transparent;
    border-radius: 7px;
    background: transparent;
    color: rgba(255, 255, 255, .48);
    font: 700 .66rem/1 var(--font-ui, system-ui);
    white-space: nowrap;
    cursor: pointer;
    transition: color 160ms ease, background 160ms ease, border-color 160ms ease, transform 160ms ease;
  }
  .hms-nav button:hover { color: #fff; background: rgba(255, 255, 255, .04); }
  .hms-nav button.active { border-color: color-mix(in srgb, var(--hms-accent) 68%, transparent); background: color-mix(in srgb, var(--hms-accent) 14%, transparent); color: #fff; transform: translateY(-1px); }
  .hms-quick-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    min-height: 36px;
    padding: 0 8px;
    border: 1px solid rgba(255, 255, 255, .14);
    border-radius: 7px;
    background: rgba(255, 255, 255, .04);
    color: rgba(255, 255, 255, .72);
    font: 700 .64rem/1 var(--font-ui, system-ui);
    white-space: nowrap;
    cursor: pointer;
  }
  .hms-quick-action:first-child { border-color: color-mix(in srgb, var(--hms-accent) 50%, transparent); color: var(--hms-accent); }
  .hms-quick-action:hover, .hms-quick-action:focus-visible { border-color: var(--hms-accent); color: #fff; background: color-mix(in srgb, var(--hms-accent) 12%, transparent); }
  .hms-back-dock {
    position: absolute;
    z-index: 25;
    top: max(10px, env(safe-area-inset-top));
    left: max(10px, env(safe-area-inset-left));
    display: grid;
    width: 44px;
    height: 44px;
    place-items: center;
    border: 1px solid rgba(255, 255, 255, .2);
    border-radius: 12px;
    background: rgba(7, 9, 12, .78);
    color: #fff;
    cursor: pointer;
    opacity: 0;
    pointer-events: none;
    transform: translateY(-6px);
    transition: opacity 180ms ease, transform 220ms ease, border-color 180ms ease, background 180ms ease;
  }
  .hms-back-dock.visible { opacity: 1; pointer-events: auto; transform: none; }
  .hms-back-dock:hover { border-color: var(--hms-accent); background: color-mix(in srgb, var(--hms-accent) 14%, rgba(7, 9, 12, .78)); }
  .hms-title-block { min-width: 0; display: grid; gap: 3px; }
  .hms-title-block { max-width: min(22vw, 220px); }
  .hms-title-block strong, .hms-title-block small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hms-title-block strong { font: 750 15px/1.15 var(--font-display, system-ui); letter-spacing: -.02em; }
  .hms-title-block small { color: rgba(255, 255, 255, .54); font-size: 10px; }
  .hms-eyebrow { gap: 8px; color: var(--hms-accent); font: 700 8px/1 var(--font-mono, monospace); letter-spacing: .14em; }
  .hms-eyebrow b { color: rgba(255, 255, 255, .4); font-weight: 700; }
  .hms-eyebrow b::before { content: "/"; margin-right: 8px; color: rgba(255, 255, 255, .24); }
  .hms-progress-label { color: rgba(255, 255, 255, .62); font: 700 10px/1 var(--font-mono, monospace); }

  .hms-stage { position: relative; z-index: 1; min-width: 0; min-height: 0; overflow: hidden; }
  .hms-footer {
    padding: 0 max(14px, env(safe-area-inset-right)) max(9px, env(safe-area-inset-bottom)) max(14px, env(safe-area-inset-left));
    border-top: 1px solid rgba(255, 255, 255, .1);
    background: rgba(7, 9, 12, .9);
    backdrop-filter: blur(14px);
  }
  .hms-progress { height: 3px; margin-bottom: 9px; overflow: hidden; background: rgba(255, 255, 255, .1); }
  .hms-progress span { display: block; height: 100%; background: var(--hms-accent); box-shadow: 0 0 14px color-mix(in srgb, var(--hms-accent) 60%, transparent); }
  .hms-hints { flex-wrap: wrap; gap: 8px 16px; color: rgba(255, 255, 255, .58); font: 600 10px/1 var(--font-ui, system-ui); }
  .hms-hints span { display: inline-flex; align-items: center; gap: 5px; white-space: nowrap; }
  kbd { min-width: 22px; height: 20px; display: inline-grid; place-items: center; padding: 0 5px; border: 1px solid rgba(255, 255, 255, .2); border-radius: 5px; background: rgba(255, 255, 255, .08); color: #fff; font: 800 9px/1 var(--font-mono, monospace); }
  button:focus-visible { outline: 2px solid var(--hms-accent); outline-offset: 2px; }

  @media (max-width: 760px) {
    .hms-header { min-height: 58px; gap: 8px; padding-inline: max(10px, env(safe-area-inset-left)) max(10px, env(safe-area-inset-right)); }
    .hms-nav { justify-content: flex-start; }
    .hms-nav button { min-width: 54px; padding-inline: 5px; }
    .hms-quick-action span { display: none; }
    .hms-quick-action { width: 34px; padding-inline: 0; }
    .hms-title-block strong { font-size: 13px; }
    .hms-progress-label { display: none; }
    .hms-hints { gap: 6px 9px; font-size: 9px; }
    .hms-hints span:nth-child(n+5) { display: none; }
  }

  @media (max-height: 560px) and (orientation: landscape) {
    .hms-header { min-height: 50px; padding-block: max(5px, env(safe-area-inset-top)) 5px; }
    .hms-back { width: 38px; height: 38px; border-radius: 9px; }
    .hms-brand { min-width: 70px; }
    .hms-brand strong { font-size: 1rem; }
    .hms-nav button { min-height: 36px; min-width: 54px; padding-inline: 5px; font-size: .6rem; }
    .hms-quick-action { min-height: 32px; padding-inline: 7px; }
    .hms-back-dock { width: 40px; height: 40px; border-radius: 9px; }
    .hms-title-block strong { font-size: 13px; }
    .hms-eyebrow { font-size: 7px; }
    .hms-footer { padding-bottom: max(5px, env(safe-area-inset-bottom)); }
    .hms-progress { margin-bottom: 5px; }
    .hms-hints { gap: 5px 12px; font-size: 9px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .hms-chrome, .hms-nav button { transition: none; }
  }
</style>
