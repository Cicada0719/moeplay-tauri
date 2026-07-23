<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { shortcut } from "@svelte-put/shortcut";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onBackButtonPress } from "@tauri-apps/api/app";
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { gameStore } from "./lib/stores/games.svelte";
  import { settingsStore } from "./lib/stores/settings.svelte";
  import { uiStore } from "./lib/stores/ui.svelte";
  import { continueStore } from "./lib/stores/continue.svelte";
  import SwitchHome from "./lib/components/switch/SwitchHome.svelte";
  import { GlobalTopNavigation, MobileAppShell } from "./lib/shell";
  import Notifications from "./lib/components/Notifications.svelte";
  import WallpaperStage from "./lib/components/WallpaperStage.svelte";
  import BigPicturePage from "./lib/components/BigPicturePage.svelte";
  import ShortcutHelp from "./lib/components/ShortcutHelp.svelte";
  import UpdateDialog from "./lib/components/UpdateDialog.svelte";
  import GamepadHintBar from "./lib/components/GamepadHintBar.svelte";
  import WorkspaceFocusToggle from "./lib/components/WorkspaceFocusToggle.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import { Drawer } from "./lib/components/ui-v2";
  import { attachGamepad } from "./lib/components/switch/useGamepad.svelte";
  import { activateGamepadFocus, activateGamepadSecondaryFocus, collectGamepadFocusable, focusGamepadSearch, moveGamepadFocus } from "./lib/actions/a11y/domGamepadNavigation";
  import { controllerSurfaceFor, dispatchSurfaceDirection, dispatchSurfaceKey, findControllerSurface } from "./lib/actions/a11y/controllerSurface";
  import { adjustFocusedGamepadControl } from "./lib/actions/a11y/gamepadSemantics";
  import { getDefaultGamepadFocusRuntime, type GamepadInputMode } from "./lib/actions/a11y/gamepadFocus";
  import { DOCK_ITEMS, PRIMARY_CONTENT_VIEWS, TOOL_ITEMS, getViewLabel } from "./lib/nav";
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
  import { createJobsStore } from "./lib/features/jobs";
  import { invokeCmd } from "./lib/api/core";
  import { wallpaperStore } from "./lib/stores/wallpapers.svelte";
  import { workspaceFocusStore } from "./lib/stores/workspaceFocus.svelte";
  import { nativeFullscreenHealthy, reassertNativeFullscreen } from "./lib/utils/window-fullscreen";
  import { isViewSupportedOnPlatform, orientationStore, platformStore } from "./lib/platform";

  const TOOLS_DRAWER_ID = "tools-drawer";
  const SHORTCUT_HELP_OVERLAY_ID = "shortcut-help";
  const SCRAPE_OVERLAY_ID = "scrape-dialog";
  const UPDATE_OVERLAY_ID = "update-dialog";

  continueStore.start();
  const isAndroid = $derived(platformStore.isAndroid);
  const isBigPicture = $derived(uiStore.bigPictureActive && !isAndroid);
  const toolsDrawerOpen = $derived(uiStore.drawerOpen && uiStore.drawerView === "tools");
  const managementViews = new Set(["scraper","tasks","sources","downloads","backup","stats","diagnostics","settings","steam-import","emulator"]);
  const wallpaperSurface = $derived(managementViews.has(uiStore.currentView) ? "management" : uiStore.currentView === "game-detail" ? "immersive" : "browse");
  let isWindowFullscreen = $state(false);
  let gamepadInputMode = $state<GamepadInputMode>("keyboard");
  let gamepadConnected = $state(false);
  const workspaceFocusAvailable = $derived(workspaceFocusStore.supports(uiStore.currentView));
  const workspaceFocusEnabled = $derived(workspaceFocusStore.isEnabled(uiStore.currentView));
  const taskBadgeStore = createJobsStore();
  let taskActiveCount = $state(0);
  let taskFailedCount = $state(0);
  const unsubscribeTaskBadge = taskBadgeStore.subscribe((snapshot) => {
    taskActiveCount = snapshot.counts.active;
    taskFailedCount = snapshot.counts.failed;
  });

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
      if (isAndroid) return;
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

  function toggleWorkspaceFocus() {
    if (!workspaceFocusAvailable) return;
    const enabled = workspaceFocusStore.toggle(uiStore.currentView);
    uiStore.notify(enabled ? "已进入专注模式；按 View 或右下角“退出专注”恢复" : "已退出专注模式", "info");
    requestAnimationFrame(() => {
      const active = document.activeElement;
      if (active instanceof HTMLElement && active.getClientRects().length === 0) {
        visibleNavigationTarget(uiStore.currentView)?.focus({ preventScroll: true });
      }
    });
  }

  function refreshGamepadConnection() {
    if (typeof navigator === "undefined" || typeof navigator.getGamepads !== "function") {
      gamepadConnected = false;
      return;
    }
    gamepadConnected = Array.from(navigator.getGamepads()).some((gamepad) => Boolean(gamepad?.connected));
  }

  function gamepadNavigationRoot(): ParentNode {
    const dialogs = Array.from(document.querySelectorAll<HTMLElement>("[aria-modal='true']"))
      .filter((dialog) => {
        const style = getComputedStyle(dialog);
        const rect = dialog.getBoundingClientRect();
        return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
      });
    return dialogs.at(-1) ?? document.querySelector("[data-testid='app-shell']") ?? document;
  }

  function visibleNavigationTarget(view: string): HTMLElement | null {
    return Array.from(document.querySelectorAll<HTMLElement>(`[data-nav-view="${view}"]`))
      .find((element) => {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
      }) ?? null;
  }

  function moveNormalModeFocus(direction: "up" | "down" | "left" | "right") {
    if (adjustFocusedGamepadControl(direction)) return;
    const root = gamepadNavigationRoot();
    const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    // Inside a controller surface (home visual/scene stage) the stick drives the
    // stage directly: up/down switch the selected game, left/right step media.
    // Focus is pinned to the stable surface root so per-game button re-keys can
    // no longer drop the gamepad context back to the global dock.
    const surface = controllerSurfaceFor(active);
    if (surface) {
      dispatchSurfaceDirection(surface, direction);
      return;
    }
    const focusable = collectGamepadFocusable({ root });
    const hasUsableFocus = active != null && focusable.includes(active);
    if (!hasUsableFocus) {
      // First stick press enters a visible controller surface right away (and
      // already steps the selection) instead of landing on the dock.
      const entrySurface = findControllerSurface(root);
      if (entrySurface) {
        dispatchSurfaceDirection(entrySurface, direction);
        return;
      }
    }
    const initial = hasUsableFocus ? active : visibleNavigationTarget(uiStore.currentView) ?? focusable[0] ?? null;
    if (!initial) return;
    if (!hasUsableFocus) initial.focus({ preventScroll: true });
    moveGamepadFocus(direction, { root, activeElement: initial });
  }

  function activateNormalModeFocus() {
    const surface = controllerSurfaceFor(document.activeElement);
    if (surface) {
      // A on a home stage behaves like Enter: open the featured archive.
      dispatchSurfaceKey(surface, "Enter");
      return;
    }
    activateGamepadFocus({ root: gamepadNavigationRoot() });
  }

  /** B while focused inside a home stage escapes back to the global dock. */
  function escapeControllerSurface(): boolean {
    const surface = controllerSurfaceFor(document.activeElement);
    if (!surface) return false;
    const target = visibleNavigationTarget(uiStore.currentView);
    if (target) {
      target.focus({ preventScroll: true });
      target.scrollIntoView({ block: "nearest", inline: "nearest" });
    } else {
      (document.activeElement instanceof HTMLElement ? document.activeElement : null)?.blur();
    }
    return true;
  }

  function cyclePrimaryContent(delta: number) {
    const available = PRIMARY_CONTENT_VIEWS.filter((view) => isViewSupportedOnPlatform(view, platformStore.capabilities));
    if (available.length === 0) return;
    const current = available.indexOf(uiStore.currentView as (typeof PRIMARY_CONTENT_VIEWS)[number]);
    const nextIndex = current < 0 ? 0 : (current + delta + available.length) % available.length;
    const nextView = available[nextIndex];
    pickDock(nextView);
    requestAnimationFrame(() => {
      // Landing on a view with a controller surface (home stages) should hand
      // focus straight to the stage so the next stick press switches games.
      const root = gamepadNavigationRoot();
      const surface = findControllerSurface(root);
      const target = surface ?? visibleNavigationTarget(nextView);
      if (target) {
        target.focus({ preventScroll: true });
        target.scrollIntoView({ block: "nearest", inline: "nearest" });
      }
    });
  }

  function focusCurrentSearch() {
    if (isBigPicture) return;
    const view = uiStore.currentView;
    uiStore.requestFocusSearch(view);
    const originActive = document.activeElement;
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
      if (attempt > 0 && userMovedFocus && active !== originActive) {
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

  async function layeredBack(): Promise<boolean> {
    if (uiStore.showFirstRunWizard) return true;
    if (document.fullscreenElement) {
      await document.exitFullscreen().catch(() => {});
      return true;
    }
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
    void layeredBack().then((handled) => {
      if (handled) event.preventDefault();
    });
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
      void layeredBack();
    },
  };

  const shortcutParameter = $derived(buildShortcutParameter(shortcutActions));

  $effect(() => {
    if (isAndroid && !isViewSupportedOnPlatform(uiStore.currentView, platformStore.capabilities)) {
      navigateTo("home", { replace: true });
      return;
    }
    if (uiStore.currentView === "game-detail" && !gameStore.selectedGame && gameStore.games[0]) {
      gameStore.selectGame(gameStore.games[0].id);
    }
  });

  $effect(() => {
    workspaceFocusStore.reconcile(uiStore.currentView);
  });

  async function toggleWindowFullscreen() {
    if (!platformStore.capabilities.desktopWindowControl) return;
    try {
      const win = appWindow();
      if (!win) return;
      const reportedFullscreen = await win.isFullscreen();
      if (reportedFullscreen && !(await nativeFullscreenHealthy(win))) {
        await reassertNativeFullscreen(win, true);
        isWindowFullscreen = true;
      } else if (reportedFullscreen) {
        await win.setFullscreen(false);
        await win.maximize();
        isWindowFullscreen = false;
      } else {
        await reassertNativeFullscreen(win, true);
        isWindowFullscreen = true;
      }
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
    const platformReady = platformStore.initialize();
    void platformReady.then(() => orientationStore.initialize());
    const releaseMotion = motionStore.initialize();
    if (!booted) {
      booted = true;
      gameStore.load();
      settingsStore.load();
    }
    const releaseRouter = initRouter();
    let androidBackListener: { unregister: () => Promise<void> } | null = null;
    void platformReady.then(async () => {
      if (!platformStore.isAndroid) return;
      androidBackListener = await onBackButtonPress(() => {
        void layeredBack().then((handled) => {
          if (handled) return;
          if (uiStore.currentView !== "home") navigateTo("home", { replace: true });
          else void tauriInvoke("plugin:app|exit");
        });
      }).catch(() => null);
    });
    const win = platformStore.capabilities.desktopWindowControl ? appWindow() : null;
    win?.isFullscreen().then(value => { isWindowFullscreen = value; }).catch(() => {});
    window.addEventListener("keydown", onKeydown);
    void taskBadgeStore.load();
    const taskBadgeTimer = window.setInterval(() => void taskBadgeStore.refresh(), 5000);
    const updateTimer = window.setTimeout(async () => {
      await platformReady.catch(() => {});
      if (!platformStore.capabilities.desktopUpdater) return;
      try {
        const result = await invokeCmd<{ available: boolean }>("start_update_check_task");
        if (result.available) showUpdateDialog = true;
      } catch {
        // The backend records a redacted failed task; startup remains quiet.
      }
    }, 5000);
    let releaseGamepadMode = () => {};
    if (!isAndroid) {
      const runtime = getDefaultGamepadFocusRuntime();
      refreshGamepadConnection();
      window.addEventListener("gamepadconnected", refreshGamepadConnection);
      window.addEventListener("gamepaddisconnected", refreshGamepadConnection);
      releaseGamepadMode = runtime?.subscribeInputMode((mode) => {
        gamepadInputMode = mode;
        document.documentElement.dataset.inputMode = mode;
        if (mode === "gamepad") refreshGamepadConnection();
      }) ?? (() => {});
      _detachGamepad = attachGamepad({
        up: () => { if (!isBigPicture) moveNormalModeFocus("up"); },
        down: () => { if (!isBigPicture) moveNormalModeFocus("down"); },
        left: () => { if (!isBigPicture) moveNormalModeFocus("left"); },
        right: () => { if (!isBigPicture) moveNormalModeFocus("right"); },
        launch: () => { if (!isBigPicture) activateNormalModeFocus(); },
        activate: () => { if (!isBigPicture) activateGamepadSecondaryFocus({ root: gamepadNavigationRoot() }); },
        favorite: () => { if (!isBigPicture) focusGamepadSearch(gamepadNavigationRoot()); },
        filter: () => { if (!isBigPicture) toggleWorkspaceFocus(); },
        pageLeft: () => { if (!isBigPicture) cyclePrimaryContent(-1); },
        pageRight: () => { if (!isBigPicture) cyclePrimaryContent(1); },
        back: () => { if (!escapeControllerSurface()) void layeredBack(); },
        start: () => {
          if (!isBigPicture) uiStore.setBigPicture(true);
        },
      }, { id: "app-global-gamepad", priority: 10 });
    }
    return () => {
      releaseMotion();
      releaseRouter();
      void androidBackListener?.unregister();
      clearTimeout(updateTimer);
      clearInterval(taskBadgeTimer);
      unsubscribeTaskBadge();
      window.removeEventListener("keydown", onKeydown);
      window.removeEventListener("gamepadconnected", refreshGamepadConnection);
      window.removeEventListener("gamepaddisconnected", refreshGamepadConnection);
      _detachGamepad();
      releaseGamepadMode();
    };
  });

  let _wallpaperSyncStarted = false;
  $effect(() => {
    if (_wallpaperSyncStarted || !settingsStore.loaded) return;
    _wallpaperSyncStarted = true;
    if (!(window as any).__MOEPLAY_TEST__) void wallpaperStore.initialize(settingsStore.appearance, settingsStore.settings.nsfw_display_mode ?? "blur");
  });

  let _startupApplied = false;
  $effect(() => {
    if (_startupApplied) return;
    if (!settingsStore.loaded) return;
    _startupApplied = true;

    if (!platformStore.capabilities.desktopWindowControl) return;
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
    if (_skipWizard || _firstRunWizardShown || isAndroid) return;
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
  class:mobile-shell={isAndroid}
  data-testid="app-shell"
  data-ui-ready={booted ? "true" : "false"}
  data-gamepad-connected={gamepadConnected ? "true" : "false"}
  data-shell-mode={workspaceFocusEnabled ? "focus" : "standard"}
  data-workspace-focus={workspaceFocusEnabled ? "true" : undefined}
  data-workspace-focus-view={workspaceFocusEnabled ? (workspaceFocusStore.scopeFor(uiStore.currentView) ?? undefined) : undefined}
>
  {#if !isBigPicture}
    <WallpaperStage surface={wallpaperSurface} />

    {#if isAndroid}
      <MobileAppShell
        currentView={uiStore.currentView}
        onNavigate={pickDock}
        onSearch={focusCurrentSearch}
        {taskActiveCount}
        {taskFailedCount}
      />
    {:else}
      <div class="global-top-navigation">
        <GlobalTopNavigation
          currentView={uiStore.currentView}
          contentItems={DOCK_ITEMS.filter(item => item.surface === "content")}
          onNavigate={pickDock}
          onSearch={focusCurrentSearch}
          onStatus={() => navigateTo("tasks")}
          onTools={openToolsDrawer}
          onSettings={() => navigateTo("settings")}
          onToggleFullscreen={toggleWindowFullscreen}
          onBigPicture={() => pickDock("__bigpicture")}
          windowFullscreen={isWindowFullscreen}
          toolsOpen={toolsDrawerOpen}
          {taskActiveCount}
          {taskFailedCount}
        />
      </div>
    {/if}

    <div id="main-content" class="main-content" data-testid="main-content">
      {#key uiStore.currentView}
        <div
          class="view-wrapper"
          data-route-root
          data-route-view={uiStore.currentView}
          data-module-style={uiStore.currentView === "home" || uiStore.currentView === "game-detail" ? "cinematic" : uiStore.currentView === "anime" || uiStore.currentView === "novel" ? "editorial" : uiStore.currentView === "comic" ? "kinetic" : "system"}
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
          {:else if uiStore.currentView === "sources"}
            {#await import("./lib/features/sources/SourceCenterPage.svelte") then { default: Comp }}
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
          {:else if uiStore.currentView === "novel"}
            {#await import("./lib/components/NovelPage.svelte") then { default: Comp }}
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

    {#if !isAndroid}
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
      {#if workspaceFocusAvailable}
        <WorkspaceFocusToggle
          active={workspaceFocusEnabled}
          view={workspaceFocusStore.scopeFor(uiStore.currentView) ?? uiStore.currentView}
          controllerActive={gamepadConnected && gamepadInputMode === "gamepad"}
          onToggle={toggleWorkspaceFocus}
        />
      {/if}
      <GamepadHintBar
        connected={gamepadConnected}
        inputMode={gamepadInputMode}
        currentView={uiStore.currentView}
        focusModeAvailable={workspaceFocusAvailable}
        focusMode={workspaceFocusEnabled}
      />
    {/if}

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

{#if !isAndroid}
  <ShortcutHelp open={showShortcutHelp} onclose={() => setShortcutHelp(false)} />
  <UpdateDialog bind:open={showUpdateDialog} />
{/if}

<Notifications />

<style>
  .app-container {
    --app-topbar-size: 64px;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: var(--app-topbar-size) minmax(0, 1fr);
    inline-size: 100%;
    max-inline-size: 100vw;
    block-size: 100dvh;
    min-block-size: 0;
    position: relative;
    overflow: hidden;
    background: var(--c-black, #050505);
  }
  .app-container.fullscreen { display: block; background: #050914; }
  .app-container.mobile-shell { display: block; height: 100dvh; min-height: 100svh; }
  .app-container.mobile-shell .main-content { position: absolute; inset: calc(56px + env(safe-area-inset-top)) 0 calc(64px + env(safe-area-inset-bottom)); overflow: hidden; }
  .app-container.mobile-shell .view-wrapper { touch-action: pan-x pan-y; }
  .global-top-navigation { grid-column: 1; grid-row: 1; position: relative; z-index: 95; min-width: 0; }
  .main-content { grid-column: 1; grid-row: 2; min-width: 0; min-height: 0; max-width: 100%; position: relative; z-index: 1; overflow: hidden; isolation: isolate; }
  .view-wrapper { position: absolute; inset: 0; display: flex; min-width: 0; min-height: 0; max-width: 100%; flex-direction: column; overflow: hidden; outline: none; z-index: 1; }

  .tools-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); border-top: 1px solid var(--c-line, var(--border)); border-left: 1px solid var(--c-line, var(--border)); }
  .tool-cell { min-height: 94px; display: grid; grid-template-columns: 34px 1fr; align-items: center; gap: 12px; padding: 14px 16px; border: 0; border-right: 1px solid var(--c-line, var(--border)); border-bottom: 1px solid var(--c-line, var(--border)); border-radius: 0; background: transparent; color: var(--c-muted, var(--text-secondary)); text-align: left; cursor: pointer; transition: color 160ms ease, background 160ms ease; }
  .tool-cell:hover, .tool-cell.active { color: var(--c-paper, var(--text-primary)); background: rgba(238,234,224,.04); }
  .tool-cell.active { box-shadow: inset 2px 0 var(--c-accent, var(--accent)); }
  .tool-icon { width: 34px; height: 34px; display: grid; place-items: center; border: 1px solid var(--c-line, var(--border)); border-radius: 0; }
  .tool-label { font: 650 12px/1 var(--font-ui); letter-spacing: .08em; }
  .tool-cell:focus-visible { outline: 1px solid var(--c-paper, white); outline-offset: -3px; }

  @media (orientation: landscape) and (max-height: 600px) {
    .app-container.mobile-shell .main-content { inset: 0 0 0 calc(72px + env(safe-area-inset-left)); }
  }

  @media (max-width: 760px) {
    .app-container { --app-topbar-size: 56px; grid-template-columns: minmax(0, 1fr); }
    .global-top-navigation { grid-column: 1; grid-row: 1; }
    .main-content { grid-column: 1; grid-row: 2; }
    .tools-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  @media (max-height: 560px) { .app-container { --app-topbar-size: 56px; } }
  @media (max-width: 520px) { .tools-grid { grid-template-columns: 1fr; } }
  @media (prefers-reduced-motion: reduce) { .tool-cell { transition: none; } }
</style>
