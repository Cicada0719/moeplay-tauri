<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { gameStore } from "./lib/stores/games.svelte";
  import { settingsStore } from "./lib/stores/settings.svelte";
  import { uiStore } from "./lib/stores/ui.svelte";
  import SwitchHome from "./lib/components/switch/SwitchHome.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Notifications from "./lib/components/Notifications.svelte";
  import BigPictureToggle from "./lib/components/BigPictureToggle.svelte";
  import BigPicturePage from "./lib/components/BigPicturePage.svelte";
  import Icon from "./lib/components/Icon.svelte";

  let ScrapeDialog: any = $state(null);
  let ScraperPage: any = $state(null);
  let DownloadPage: any = $state(null);
  let BackupPage: any = $state(null);
  let StatsPage: any = $state(null);
  let DiscoveryPage: any = $state(null);
  let DiagnosticsPage: any = $state(null);
  let SettingsPage: any = $state(null);
  let GameDetailPage: any = $state(null);
  let SteamImportDialog: any = $state(null);
  let MigrationPage: any = $state(null);
  let EmulatorImportDialog: any = $state(null);
  let FirstRunWizard: any = $state(null);
  let CommandDrawer: any = $state(null);

  const isBigPicture = $derived(uiStore.bigPictureActive);
  const showSidebar = $derived(!isBigPicture && uiStore.currentView !== "home");

  $effect(() => {
    if (uiStore.currentView === "scraper" && !ScraperPage) import("./lib/components/ScraperPage.svelte").then(m => ScraperPage = m.default);
    if (uiStore.currentView === "downloads" && !DownloadPage) import("./lib/components/DownloadPage.svelte").then(m => DownloadPage = m.default);
    if (uiStore.currentView === "backup" && !BackupPage) import("./lib/components/BackupPage.svelte").then(m => BackupPage = m.default);
    if (uiStore.currentView === "stats" && !StatsPage) import("./lib/components/StatsPage.svelte").then(m => StatsPage = m.default);
    if (uiStore.currentView === "discovery" && !DiscoveryPage) import("./lib/components/DiscoveryPage.svelte").then(m => DiscoveryPage = m.default);
    if (uiStore.currentView === "diagnostics" && !DiagnosticsPage) import("./lib/components/DiagnosticsPage.svelte").then(m => DiagnosticsPage = m.default);
    if (uiStore.currentView === "settings" && !SettingsPage) import("./lib/components/SettingsPage.svelte").then(m => SettingsPage = m.default);
    if (uiStore.currentView === "game-detail" && !GameDetailPage) import("./lib/components/GameDetailPage.svelte").then(m => GameDetailPage = m.default);
    if (uiStore.currentView === "game-detail" && !gameStore.selectedGame && gameStore.games[0]) gameStore.selectGame(gameStore.games[0].id);
    if (uiStore.currentView === "steam-import" && !SteamImportDialog) import("./lib/components/SteamImportDialog.svelte").then(m => SteamImportDialog = m.default);
    if (uiStore.currentView === "migration" && !MigrationPage) import("./lib/components/MigrationPage.svelte").then(m => MigrationPage = m.default);
    if (uiStore.currentView === "emulator" && !EmulatorImportDialog) import("./lib/components/EmulatorImportDialog.svelte").then(m => EmulatorImportDialog = m.default);
  });

  $effect(() => {
    if (uiStore.showFirstRunWizard && !FirstRunWizard) import("./lib/components/FirstRunWizard.svelte").then(m => FirstRunWizard = m.default);
  });

  $effect(() => {
    if (uiStore.drawerOpen && !CommandDrawer) import("./lib/components/CommandDrawer.svelte").then(m => CommandDrawer = m.default);
  });

  $effect(() => {
    if (uiStore.showScrapeDialog && !ScrapeDialog) {
      import("./lib/components/ScrapeDialog.svelte").then(m => ScrapeDialog = m.default);
    }
  });

  let booted = $state(false);
  onMount(() => {
    if (booted) return;
    booted = true;
    gameStore.load();
    settingsStore.load();
  });

  // 开机自启：根据 startup_mode 切换初始视图 + 窗口模式
  let _startupApplied = false;
  $effect(() => {
    const settings = settingsStore.settings;
    if (!settings || _startupApplied) return;
    _startupApplied = true;

    const mode = settings.startup_mode ?? "dashboard";
    if (mode === "big-picture") {
      uiStore.setBigPicture(true);
      getCurrentWindow().setFullscreen(true).catch(() => {});
    } else if (mode === "fullscreen") {
      getCurrentWindow().setFullscreen(true).catch(() => {});
    } else {
      getCurrentWindow().maximize().catch(() => {});
    }
  });

  // First-run wizard: show once only (dev: ?skip_wizard 跳过)
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

<div
  class="app-container"
  class:fullscreen={isBigPicture}
  class:console-home={!isBigPicture && uiStore.currentView === "home"}
  class:sidebar-open={showSidebar && !uiStore.sidebarCollapsed}
  data-testid="app-shell"
>
  {#if !isBigPicture}
    <!-- PS5-style restrained depth background -->
    <div class="bg-layers">
      <div class="bg-gradient"></div>
      <div class="bg-scrim"></div>
    </div>

    {#if showSidebar}
      <!-- 侧边栏（工具页保留，主页使用主机式全屏布局） -->
      <Sidebar />

      <!-- 侧边栏切换钮 -->
      <button
        class="sidebar-toggle"
        onclick={() => uiStore.sidebarCollapsed = !uiStore.sidebarCollapsed}
        title={uiStore.sidebarCollapsed ? "展开侧边栏" : "收起侧边栏"}
        aria-label="切换侧边栏"
      >
        <Icon name={uiStore.sidebarCollapsed ? "chevronRight" : "chevronLeft"} size={14} />
      </button>
    {/if}

    <main class="main-content" data-testid="main-content">
      {#key uiStore.currentView}
        <div class="view-wrapper" in:fly={{ y: 12, duration: 320, easing: quintOut }} out:fly={{ y: -12, duration: 200, easing: quintOut }}>
          {#if uiStore.currentView === "scraper" && ScraperPage}
            <ScraperPage />
          {:else if uiStore.currentView === "downloads" && DownloadPage}
            <DownloadPage />
          {:else if uiStore.currentView === "backup" && BackupPage}
            <BackupPage />
          {:else if uiStore.currentView === "stats" && StatsPage}
            <StatsPage />
          {:else if uiStore.currentView === "discovery" && DiscoveryPage}
            <DiscoveryPage />
          {:else if uiStore.currentView === "diagnostics" && DiagnosticsPage}
            <DiagnosticsPage />
          {:else if uiStore.currentView === "settings" && SettingsPage}
            <SettingsPage />
          {:else if uiStore.currentView === "game-detail" && GameDetailPage}
            <GameDetailPage />
          {:else if uiStore.currentView === "steam-import" && SteamImportDialog}
            <SteamImportDialog />
          {:else if uiStore.currentView === "migration" && MigrationPage}
            <MigrationPage />
          {:else if uiStore.currentView === "emulator" && EmulatorImportDialog}
            <EmulatorImportDialog />
          {:else}
            <SwitchHome />
          {/if}
        </div>
      {/key}
    </main>
  {:else}
    <BigPicturePage />
  {/if}
</div>

{#if !isBigPicture}
  <BigPictureToggle />
{/if}

{#if uiStore.showScrapeDialog && ScrapeDialog}
  <ScrapeDialog />
{/if}

{#if uiStore.drawerOpen && CommandDrawer}
  <CommandDrawer />
{/if}

{#if uiStore.showFirstRunWizard && FirstRunWizard}
  <FirstRunWizard />
{/if}

<Notifications />

<style>
  .app-container {
    display: flex; flex-direction: row; height: 100vh; width: 100vw;
    background: var(--bg-deep);
    position: relative; overflow: hidden;
  }
  .app-container.fullscreen {
    background: #050914;
  }

  /* ── 侧边栏切换按钮（紧随sidebar右侧边缘） ── */
  .sidebar-toggle {
    position: absolute; top: 14px; left: 210px; z-index: 30;
    width: 22px; height: 38px;
    display: grid; place-items: center;
    border: 1px solid var(--border); border-left: none;
    background: rgba(17, 24, 39, 0.78);
    color: var(--text-muted);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    cursor: pointer;
    backdrop-filter: blur(10px);
    transition: color 0.2s, left 0.3s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .sidebar-toggle:hover { color: var(--accent-pink); }

  /* 侧边栏收起时，按钮回到左边缘 */
  .app-container:not(.sidebar-open) .sidebar-toggle { left: 0; }

  /* ── 原版 WPF 背景叠层 ── */
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
    min-width: 0; position: relative; z-index: 40;
  }

  .view-wrapper {
    flex: 1; display: flex; flex-direction: column; overflow: hidden;
    position: relative; z-index: 1;
  }
</style>
