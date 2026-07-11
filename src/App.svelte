<script lang="ts">
  import { onMount } from "svelte";
  import { check } from "@tauri-apps/plugin-updater";
  import { fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { shortcut } from "@svelte-put/shortcut";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { gameStore } from "./lib/stores/games.svelte";
  import { settingsStore } from "./lib/stores/settings.svelte";
  import { uiStore } from "./lib/stores/ui.svelte";
  import { continueStore } from "./lib/stores/continue.svelte";
  import SwitchHome from "./lib/components/switch/SwitchHome.svelte";
  import SystemDock from "./lib/components/switch/SystemDock.svelte";
  import Notifications from "./lib/components/Notifications.svelte";
  import BigPicturePage from "./lib/components/BigPicturePage.svelte";
  import ShortcutHelp from "./lib/components/ShortcutHelp.svelte";
  import UpdateDialog from "./lib/components/UpdateDialog.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import { Drawer } from "./lib/components/ui-v2";
  import { attachGamepad } from "./lib/components/switch/useGamepad.svelte";
  import { DOCK_ITEMS, TOOL_ITEMS, getViewLabel } from "./lib/nav";
  import { buildShortcutParameter, type ShortcutActions } from "./lib/shortcuts";
  import {
    closeOverlay,
    focusCurrentRouteSearch,
    handleBackNavigation,
    initRouter,
    navigateTo,
    openOverlay,
  } from "./lib/stores/router.svelte";
  import { motionStore } from "./lib/stores/motion.svelte";

  const TOOLS_DRAWER_ID = "tools-drawer";
  const SHORTCUT_HELP_OVERLAY_ID = "shortcut-help";
  const SCRAPE_OVERLAY_ID = "scrape-dialog";
  const UPDATE_OVERLAY_ID = "update-dialog";

  continueStore.start();
  const isBigPicture = $derived(uiStore.bigPictureActive);
  const toolsDrawerOpen = $derived(uiStore.drawerOpen && uiStore.drawerView === "tools");
  let isWindowFullscreen = $state(false);

  const TOOL_VIEWS = new Set(TOOL_ITEMS.map(item => item.view));
  const isToolView = $derived(TOOL_VIEWS.has(uiStore.currentView));

  function appWindow() {
    if (typeof window === "undefined" || !(window as any).__TAURI_INTERNALS__) return null;
    try {
      return getCurrentWindow();
    } catch {
      return null;
    }
  }

  function openToolsDrawer() {
    uiStore.openDrawer("tools");
    openOverlay(
      { id: TOOLS_DRAWER_ID, kind: "drawer", returnFocusKey: "system-dock-tools" },
      () => uiStore.closeDrawer(),
    );
  }

  function closeToolsDrawer() {
    uiStore.closeDrawer();
    closeOverlay(TOOLS_DRAWER_ID);
  }

  function toggleToolsDrawer() {
    if (toolsDrawerOpen) closeToolsDrawer();
    else openToolsDrawer();
  }

  function setShortcutHelp(open: boolean) {
    showShortcutHelp = open;
    if (open) {
      openOverlay(
        { id: SHORTCUT_HELP_OVERLAY_ID, kind: "dialog" },
        () => { showShortcutHelp = false; },
      );
    } else {
      closeOverlay(SHORTCUT_HELP_OVERLAY_ID);
    }
  }

  function pickDock(view: string) {
    if (view === "__bigpicture") {
      closeToolsDrawer();
      uiStore.setBigPicture(true);
      return;
    }
    if (view === "__tools") {
      toggleToolsDrawer();
      return;
    }
    closeToolsDrawer();
    if (view === "home") gameStore.quickFilter = null;
    navigateTo(view);
  }

  function pickTool(view: string) {
    closeToolsDrawer();
    navigateTo(view);
  }

  function focusCurrentSearch() {
    if (isBigPicture) return;
    const view = uiStore.currentView;
    uiStore.requestFocusSearch(view);
    let attempt = 0;
    const focus = () => {
      if (uiStore.currentView !== view) {
        uiStore.consumeFocusSearchSignal();
        return;
      }
      const active = document.activeElement;
      const searchFocused = active instanceof HTMLInputElement
        && (active.type === "search" || active.placeholder.includes("搜索"));
      if (searchFocused) {
        uiStore.consumeFocusSearchSignal();
        return;
      }
      const userMovedFocus = active instanceof HTMLElement
        && /^(BUTTON|A|INPUT|TEXTAREA|SELECT)$/.test(active.tagName);
      if (attempt > 0 && userMovedFocus) {
        uiStore.consumeFocusSearchSignal();
        return;
      }
      focusCurrentRouteSearch(view);
      attempt++;
      if (attempt < 4) window.setTimeout(focus, attempt * 75);
      else uiStore.consumeFocusSearchSignal();
    };
    queueMicrotask(focus);
  }

  function layeredBack(): boolean {
    if (uiStore.showFirstRunWizard) return true;
    const result = handleBackNavigation();
    if (result !== "none") return true;
    if (isBigPicture) {
      uiStore.setBigPicture(false);
      return true;
    }
    return false;
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape" || event.defaultPrevented) return;
    if (layeredBack()) event.preventDefault();
  }

  const shortcutActions: ShortcutActions = {
    navigate(view: string) {
      if (isBigPicture) return;
      pickDock(view);
    },
    toggleTools() {
      if (isBigPicture) return;
      toggleToolsDrawer();
    },
    focusSearch: focusCurrentSearch,
    toggleHelp() {
      if (isBigPicture) return;
      setShortcutHelp(!showShortcutHelp);
    },
    goBack() {
      if (isBigPicture) return;
      layeredBack();
    },
  };

  const shortcutParameter = $derived(buildShortcutParameter(shortcutActions));

  $effect(() => {
    if (uiStore.currentView === "game-detail" && !gameStore.selectedGame && gameStore.games[0]) {
      gameStore.selectGame(gameStore.games[0].id);
    }
  });

  async function toggleWindowFullscreen() {
    try {
      const win = appWindow();
      if (!win) return;
      if (isWindowFullscreen) {
        await win.setFullscreen(false);
        await win.maximize();
      } else {
        await win.setFullscreen(true);
      }
      isWindowFullscreen = !isWindowFullscreen;
    } catch {}
  }

  let booted = $state(false);
  let _detachGamepad = () => {};

  $effect(() => {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.uiReady = booted ? "true" : "false";
  });
  let showShortcutHelp = $state(false);
  let showUpdateDialog = $state(false);

  $effect(() => {
    if (uiStore.showScrapeDialog) {
      openOverlay({ id: SCRAPE_OVERLAY_ID, kind: "dialog" }, () => uiStore.closeScrapeDialog());
    } else {
      closeOverlay(SCRAPE_OVERLAY_ID);
    }
  });

  $effect(() => {
    if (showUpdateDialog) {
      openOverlay({ id: UPDATE_OVERLAY_ID, kind: "dialog" }, () => { showUpdateDialog = false; });
    } else {
      closeOverlay(UPDATE_OVERLAY_ID);
    }
  });

  onMount(() => {
    const releaseMotion = motionStore.initialize();
    if (!booted) {
      booted = true;
      gameStore.load();
      settingsStore.load();
    }
    const releaseRouter = initRouter();
    appWindow()?.isFullscreen().then(value => { isWindowFullscreen = value; }).catch(() => {});
    window.addEventListener("keydown", onKeydown);
    const updateTimer = setTimeout(async () => {
      try {
        const update = await check();
        if (update) showUpdateDialog = true;
      } catch {}
    }, 5000);
    _detachGamepad = attachGamepad({
      back: layeredBack,
      start: () => {
        if (!isBigPicture) uiStore.setBigPicture(true);
      },
    }, { id: "app-global-gamepad", priority: -100 });
    return () => {
      releaseMotion();
      releaseRouter();
      clearTimeout(updateTimer);
      window.removeEventListener("keydown", onKeydown);
      _detachGamepad();
    };
  });

  let _startupApplied = false;
  $effect(() => {
    if (_startupApplied) return;
    if (!settingsStore.loaded) return;
    _startupApplied = true;

    const mode = settingsStore.settings.startup_mode ?? "fullscreen";
    const win = appWindow();
    if (mode === "big-picture") {
      uiStore.setBigPicture(true);
    } else if (mode === "fullscreen") {
      // 已由 tauri.conf.json fullscreen:true 原生全屏，无需处理
    } else if (mode === "windowed") {
      if (!win) return;
      import("@tauri-apps/api/dpi").then(({ LogicalSize }) => {
        win.setFullscreen(false).then(() =>
          win.setSize(new LogicalSize(1200, 800)).then(() => win.center())
        ).catch(() => {});
      });
    } else {
      if (!win) return;
      win.setFullscreen(false).then(() => win.maximize()).catch(() => {});
    }
  });

  const _skipWizard = typeof window !== "undefined" && new URLSearchParams(window.location.search).has("skip_wizard");
  let _firstRunWizardShown = $state(false);
  $effect(() => {
    if (_skipWizard || _firstRunWizardShown) return;
    if (!booted || gameStore.loading || settingsStore.loading) return;
    if (settingsStore.settings && gameStore.games.length === 0 && !(settingsStore.settings.watch_dirs?.length)) {
      _firstRunWizardShown = true;
      uiStore.showFirstRunWizard = true;
    }
  });
</script>

<svelte:window use:shortcut={shortcutParameter} />

<div
  class="app-container"
  class:fullscreen={isBigPicture}
  data-testid="app-shell"
  data-ui-ready={booted ? "true" : "false"}
>
  {#if !isBigPicture}
    <div class="bg-layers">
      <div class="bg-gradient"></div>
      <div class="bg-scrim"></div>
    </div>

    <div class="main-content" data-testid="main-content">
      {#key uiStore.currentView}
        <div
          class="view-wrapper"
          data-route-root
          data-route-view={uiStore.currentView}
          aria-label={getViewLabel(uiStore.currentView)}
          tabindex="-1"
          in:fade={{ duration: 240, easing: cubicOut }}
          out:fade={{ duration: 160 }}
        >
          {#if uiStore.currentView === "scraper"}
            {#await import("./lib/components/ScraperPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "tasks"}
            {#await import("./lib/features/jobs/TaskCenterPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "downloads"}
            {#await import("./lib/components/DownloadPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "backup"}
            {#await import("./lib/components/BackupPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "stats"}
            {#await import("./lib/components/StatsPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "discovery"}
            {#await import("./lib/components/DiscoveryPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "records"}
            {#await import("./lib/components/PlayRecordsDashboard.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "anime"}
            {#await import("./lib/components/AnimePage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "continue"}
            {#await import("./lib/components/ContinueHub.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "comic"}
            {#await import("./lib/components/ComicPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "diagnostics"}
            {#await import("./lib/components/DiagnosticsPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "settings"}
            {#await import("./lib/components/SettingsPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "game-detail"}
            {#await import("./lib/components/GameDetailPage.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "steam-import"}
            {#await import("./lib/components/SteamImportDialog.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else if uiStore.currentView === "emulator"}
            {#await import("./lib/components/EmulatorImportDialog.svelte") then { default: Comp }}
              <Comp />
            {/await}
          {:else}
            <SwitchHome />
          {/if}
        </div>
      {/key}
    </div>

    <Drawer
      id={TOOLS_DRAWER_ID}
      open={toolsDrawerOpen}
      title="工具"
      description="从当前内容入口打开辅助功能。"
      side="bottom"
      size="sm"
      initialFocus="[data-tool-item]"
      returnFocus="#system-dock-tools"
      onClose={closeToolsDrawer}
    >
      <div class="tools-grid">
        {#each TOOL_ITEMS as tool (tool.id)}
          <button
            type="button"
            class="tool-cell"
            class:active={uiStore.currentView === tool.view}
            data-tool-item
            aria-label={tool.ariaLabel}
            aria-current={uiStore.currentView === tool.view ? "page" : undefined}
            onclick={() => pickTool(tool.view)}
          >
            <span class="tool-icon tool-icon--{tool.group}"><Icon name={tool.icon} size={22} /></span>
            <span class="tool-label">{tool.label}</span>
          </button>
        {/each}
      </div>
    </Drawer>

    <div class="global-dock">
      <SystemDock
        items={DOCK_ITEMS}
        current={isToolView ? "__tools" : uiStore.currentView}
        toolsOpen={toolsDrawerOpen}
        onpick={pickDock}
        windowFullscreen={isWindowFullscreen}
        ontogglefullscreen={toggleWindowFullscreen}
      />
    </div>
  {:else}
    <BigPicturePage />
  {/if}
</div>

{#if uiStore.showScrapeDialog}
  {#await import("./lib/components/ScrapeDialog.svelte") then { default: Comp }}
    <Comp />
  {/await}
{/if}

{#if uiStore.showFirstRunWizard}
  {#await import("./lib/components/FirstRunWizard.svelte") then { default: Comp }}
    <Comp />
  {/await}
{/if}

<ShortcutHelp open={showShortcutHelp} onclose={() => setShortcutHelp(false)} />

<UpdateDialog bind:open={showUpdateDialog} />

<Notifications />

<style>
  .app-container {
    display: flex; flex-direction: column; height: 100dvh; width: 100vw;
    background: var(--bg-deep);
    position: relative; overflow: hidden;
  }
  .app-container.fullscreen {
    background: #050914;
  }

  .bg-layers { position: absolute; inset: 0; pointer-events: none; z-index: 0; }
  .bg-gradient {
    position: absolute; inset: 0;
    background: linear-gradient(135deg, var(--bg-deep) 0%, var(--bg) 50%, var(--bg-deep) 100%);
  }
  .bg-scrim {
    position: absolute; inset: 0;
    background: rgba(11, 14, 20, 0.36);
  }

  .main-content {
    flex: 1; display: flex; flex-direction: column; overflow: hidden;
    min-width: 0; position: relative; z-index: 1;
  }

  .view-wrapper {
    position: absolute; inset: 0;
    display: flex; flex-direction: column; overflow: hidden;
    z-index: 1;
  }

  /* ── Tools drawer content ── */
  .tools-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }

  @keyframes toolCellIn {
    from { opacity: 0; transform: translateY(8px) scale(0.95); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  .tool-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 14px 8px;
    border: none;
    border-radius: 14px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease, transform 0.15s ease;
    animation: toolCellIn 0.28s cubic-bezier(0.34, 1.56, 0.64, 1) both;
  }
  .tool-cell:hover {
    background: rgba(255,255,255,0.06);
    color: var(--text-primary);
    transform: translateY(-2px);
  }
  .tool-cell:active {
    transform: scale(0.96);
  }
  .tool-cell.active {
    color: var(--accent);
    background: var(--accent-lo);
  }
  .tool-icon {
    width: 44px; height: 44px;
    display: grid; place-items: center;
    border-radius: 12px;
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.06);
    transition: background 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease;
  }
  .tool-icon--tools { color: rgba(139,166,245,0.85); }
  .tool-icon--import { color: rgba(94,211,172,0.85); }
  .tool-icon--system { color: rgba(245,179,100,0.85); }
  .tool-cell:hover .tool-icon {
    background: rgba(255,255,255,0.08);
    border-color: rgba(255,255,255,0.1);
    box-shadow: 0 2px 8px -2px rgba(0,0,0,0.2);
  }
  .tool-cell.active .tool-icon {
    background: var(--accent-lo);
    border-color: var(--accent-ring, rgba(232,85,127,0.3));
    color: var(--accent);
  }
  .tool-label {
    font-size: 11px;
    font-weight: 600;
  }

  /* ── Global dock ── */
  .global-dock {
    flex-shrink: 0;
    position: relative; z-index: 90;
    border-top: 1px solid var(--border);
    background: rgba(10, 13, 20, 0.72);
    backdrop-filter: blur(16px) saturate(1.1);
    -webkit-backdrop-filter: blur(16px) saturate(1.1);
  }

  @media (max-width: 520px) {
    .tools-grid { grid-template-columns: repeat(3, 1fr); }
  }
</style>
