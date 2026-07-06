<script lang="ts">
  import { onMount } from "svelte";
  import { check } from "@tauri-apps/plugin-updater";
  import { fly, fade, scale } from "svelte/transition";
  import { quintOut, cubicOut } from "svelte/easing";
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
  import { attachGamepad } from "./lib/components/switch/useGamepad.svelte";
  import { DOCK_ITEMS, TOOL_ITEMS } from "./lib/nav";
  import { buildShortcutParameter, type ShortcutActions } from "./lib/shortcuts";
  import { initRouter } from "./lib/stores/router.svelte";

  continueStore.start();
  const isBigPicture = $derived(uiStore.bigPictureActive);
  let toolsDrawerOpen = $state(false);
  let isWindowFullscreen = $state(false);

  // tool drawer contains these views — used to highlight "工具" in dock
  const TOOL_VIEWS = new Set(TOOL_ITEMS.map(t => t.view));
  const isToolView = $derived(TOOL_VIEWS.has(uiStore.currentView));

  function pickDock(view: string) {
    if (view === "__bigpicture") {
      toolsDrawerOpen = false;
      uiStore.setBigPicture(true);
      return;
    }
    if (view === "__tools") {
      toolsDrawerOpen = !toolsDrawerOpen;
      return;
    }
    toolsDrawerOpen = false;
    if (view === "home") gameStore.quickFilter = null;
    uiStore.currentView = view;
  }

  function pickTool(view: string) {
    toolsDrawerOpen = false;
    uiStore.currentView = view;
  }

  function goHome() {
    toolsDrawerOpen = false;
    gameStore.quickFilter = null;
    uiStore.currentView = "home";
  }

  function exitable(): boolean {
    return !isBigPicture
      && uiStore.currentView !== "home"
      && !uiStore.showScrapeDialog
      && !uiStore.showFirstRunWizard
      && !toolsDrawerOpen;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (showShortcutHelp) {
        e.preventDefault();
        showShortcutHelp = false;
        return;
      }
      if (toolsDrawerOpen) {
        e.preventDefault();
        toolsDrawerOpen = false;
        return;
      }
      if (exitable()) {
        e.preventDefault();
        goHome();
      }
    }
  }

  const shortcutActions: ShortcutActions = {
    navigate(view: string) {
      if (isBigPicture) return;
      pickDock(view);
    },
    toggleTools() {
      if (isBigPicture) return;
      toolsDrawerOpen = !toolsDrawerOpen;
    },
    focusSearch() {
      if (isBigPicture) return;
      if (uiStore.currentView !== "home") uiStore.currentView = "home";
      uiStore.requestFocusSearch();
    },
    toggleHelp() {
      if (isBigPicture) return;
      showShortcutHelp = !showShortcutHelp;
    },
    goHome() {
      if (isBigPicture) return;
      goHome();
    },
  };

  const shortcutParameter = $derived(buildShortcutParameter(shortcutActions));

  $effect(() => {
    if (uiStore.currentView === "game-detail" && !gameStore.selectedGame && gameStore.games[0]) gameStore.selectGame(gameStore.games[0].id);
  });

  async function toggleWindowFullscreen() {
    try {
      const win = getCurrentWindow();
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
  let showShortcutHelp = $state(false);
  let showUpdateDialog = $state(false);
  onMount(() => {
    if (!booted) {
      booted = true;
      gameStore.load();
      settingsStore.load();
      initRouter();
    }
    getCurrentWindow().isFullscreen().then(v => { isWindowFullscreen = v; }).catch(() => {});
    window.addEventListener("keydown", onKeydown);
    // 启动 5 秒后静默检查更新（避免和首屏加载冲突）
    const updateTimer = setTimeout(async () => {
      try {
        const update = await check();
        if (update) showUpdateDialog = true;
      } catch {}
    }, 5000);
    // disable shortcuts while help is open to avoid double-firing from action
    _detachGamepad = attachGamepad({ back: () => {
      if (toolsDrawerOpen) { toolsDrawerOpen = false; return; }
      if (exitable()) goHome();
    }});
    return () => {
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
    const win = getCurrentWindow();
    if (mode === "big-picture") {
      uiStore.setBigPicture(true);
    } else if (mode === "fullscreen") {
      // 已由 tauri.conf.json fullscreen:true 原生全屏，无需处理
    } else if (mode === "windowed") {
      import("@tauri-apps/api/dpi").then(({ LogicalSize }) => {
        win.setFullscreen(false).then(() =>
          win.setSize(new LogicalSize(1200, 800)).then(() => win.center())
        ).catch(() => {});
      });
    } else {
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
>
  {#if !isBigPicture}
    <div class="bg-layers">
      <div class="bg-gradient"></div>
      <div class="bg-scrim"></div>
    </div>

    <main class="main-content" data-testid="main-content">
      {#key uiStore.currentView}
        <div class="view-wrapper" in:fade={{ duration: 240, easing: cubicOut }} out:fade={{ duration: 160 }}>
          {#if uiStore.currentView === "scraper"}
            {#await import("./lib/components/ScraperPage.svelte") then { default: Comp }}
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
          {:else if uiStore.currentView === "migration"}
            {#await import("./lib/components/MigrationPage.svelte") then { default: Comp }}
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
    </main>

    <!-- Tools drawer overlay -->
    {#if toolsDrawerOpen}
      <button class="drawer-backdrop" transition:fade={{ duration: 200 }} onclick={() => (toolsDrawerOpen = false)} aria-label="关闭工具面板"></button>
      <div class="tools-drawer" transition:fly={{ y: 60, duration: 300, easing: quintOut }}>
        <div class="tools-grid">
          {#each TOOL_ITEMS as tool, idx (tool.id)}
            <button
              class="tool-cell"
              class:active={uiStore.currentView === tool.view}
              style="animation-delay: {idx * 35}ms"
              onclick={() => pickTool(tool.view)}
            >
              <span class="tool-icon tool-icon--{tool.group}"><Icon name={tool.icon} size={22} /></span>
              <span class="tool-label">{tool.label}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

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

<ShortcutHelp open={showShortcutHelp} onclose={() => (showShortcutHelp = false)} />

<UpdateDialog bind:open={showUpdateDialog} />

<Notifications />

<style>
  .app-container {
    display: flex; flex-direction: column; height: 100vh; width: 100vw;
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
    flex: 1; display: flex; flex-direction: column; overflow: hidden;
    position: relative; z-index: 1;
  }

  /* ── Tools drawer ── */
  .drawer-backdrop {
    position: fixed; inset: 0; z-index: 80;
    background: rgba(5, 8, 14, 0.45);
    border: none; cursor: default;
  }

  .tools-drawer {
    position: fixed;
    bottom: 70px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 81;
    width: min(520px, calc(100vw - 32px));
    padding: 18px 20px;
    background: var(--bg-elev, rgba(22, 26, 36, 0.95));
    border: 1px solid var(--border);
    border-radius: 18px;
    backdrop-filter: blur(24px) saturate(1.15);
    -webkit-backdrop-filter: blur(24px) saturate(1.15);
    box-shadow: 0 -8px 40px rgba(0,0,0,0.35);
  }

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
    .tools-drawer { bottom: 62px; padding: 14px 12px; border-radius: 14px; }
  }
</style>
