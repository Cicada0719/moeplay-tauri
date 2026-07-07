<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";

  let ScraperPage: any = $state(null);
  let DownloadPage: any = $state(null);
  let BackupPage: any = $state(null);
  let StatsPage: any = $state(null);
  let DiscoveryPage: any = $state(null);
  let DiagnosticsPage: any = $state(null);
  let SettingsPage: any = $state(null);
  let SteamImportDialog: any = $state(null);
  let EmulatorImportDialog: any = $state(null);

  const navItems = [
    { id: "discovery",    label: "资源发现", icon: "compass" },
    { id: "scraper",      label: "AI 刮削",  icon: "star" },
    { id: "downloads",    label: "资源下载", icon: "download" },
    { id: "backup",       label: "存档管理", icon: "save" },
    { id: "stats",        label: "统计",     icon: "chart" },
    { id: "steam-import", label: "平台导入", icon: "database" },
    { id: "emulator",     label: "模拟器",   icon: "gamepad" },
    { id: "diagnostics",  label: "诊断",     icon: "toolbox" },
    { id: "settings",     label: "设置",     icon: "gear" },
  ];

  const activeView = $derived(uiStore.drawerView ?? "settings");

  $effect(() => {
    if (!uiStore.drawerOpen) return;
    const v = activeView;
    if (v === "scraper" && !ScraperPage) import("./ScraperPage.svelte").then(m => ScraperPage = m.default);
    if (v === "downloads" && !DownloadPage) import("./DownloadPage.svelte").then(m => DownloadPage = m.default);
    if (v === "backup" && !BackupPage) import("./BackupPage.svelte").then(m => BackupPage = m.default);
    if (v === "stats" && !StatsPage) import("./StatsPage.svelte").then(m => StatsPage = m.default);
    if (v === "discovery" && !DiscoveryPage) import("./DiscoveryPage.svelte").then(m => DiscoveryPage = m.default);
    if (v === "diagnostics" && !DiagnosticsPage) import("./DiagnosticsPage.svelte").then(m => DiagnosticsPage = m.default);
    if (v === "settings" && !SettingsPage) import("./SettingsPage.svelte").then(m => SettingsPage = m.default);
    if (v === "steam-import" && !SteamImportDialog) import("./SteamImportDialog.svelte").then(m => SteamImportDialog = m.default);
    if (v === "emulator" && !EmulatorImportDialog) import("./EmulatorImportDialog.svelte").then(m => EmulatorImportDialog = m.default);
  });

  function close() {
    uiStore.closeDrawer();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") close();
  }

  function switchView(id: string) {
    uiStore.openDrawer(id);
  }
</script>

{#if uiStore.drawerOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="drawer-overlay" transition:fade={{ duration: 220 }} onkeydown={onKeydown} onclick={close}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="drawer-panel"
      transition:fly={{ x: 0, y: 32, duration: 320, easing: quintOut }}
      onclick={(e) => e.stopPropagation()}
      onkeydown={onKeydown}
    >
      <nav class="drawer-nav">
        <div class="drawer-nav-header">
          <span class="drawer-logo">萌</span>
          <span class="drawer-title">萌游</span>
        </div>
        <div class="drawer-nav-list">
          {#each navItems as item (item.id)}
            <button
              class="drawer-nav-item"
              class:active={activeView === item.id}
              onclick={() => switchView(item.id)}
            >
              <Icon name={item.icon} size={18} />
              <span>{item.label}</span>
            </button>
          {/each}
        </div>
      </nav>

      <div class="drawer-content">
        <header class="drawer-content-header">
          <h2>{navItems.find(n => n.id === activeView)?.label ?? ""}</h2>
          <button class="drawer-close" onclick={close} title="关闭">
            <Icon name="x" size={18} />
          </button>
        </header>
        <div class="drawer-content-body">
          {#if activeView === "scraper" && ScraperPage}
            <ScraperPage />
          {:else if activeView === "downloads" && DownloadPage}
            <DownloadPage />
          {:else if activeView === "backup" && BackupPage}
            <BackupPage />
          {:else if activeView === "stats" && StatsPage}
            <StatsPage />
          {:else if activeView === "discovery" && DiscoveryPage}
            <DiscoveryPage />
          {:else if activeView === "diagnostics" && DiagnosticsPage}
            <DiagnosticsPage />
          {:else if activeView === "settings" && SettingsPage}
            <SettingsPage />
          {:else if activeView === "steam-import" && SteamImportDialog}
            <SteamImportDialog />
          {:else if activeView === "emulator" && EmulatorImportDialog}
            <EmulatorImportDialog />
          {:else}
            <div class="drawer-loading">
              <div class="drawer-spinner"></div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .drawer-overlay {
    position: fixed;
    inset: 0;
    z-index: 900;
    background: rgba(5, 7, 12, 0.72);
    backdrop-filter: blur(12px);
    display: grid;
    place-items: center;
  }

  .drawer-panel {
    width: min(92vw, 1100px);
    height: min(88vh, 760px);
    display: flex;
    border-radius: var(--radius-xl);
    overflow: hidden;
    background: var(--bg-deep);
    border: 1px solid var(--border-hover);
    box-shadow: 0 32px 80px -20px rgba(0, 0, 0, 0.7);
  }

  .drawer-nav {
    width: 200px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border-right: 1px solid var(--border);
  }

  .drawer-nav-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 20px 16px;
    border-bottom: 1px solid var(--border);
  }

  .drawer-logo {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--accent-lo);
    border: 1px solid var(--accent-ring);
    display: grid;
    place-items: center;
    color: var(--accent);
    font-weight: 700;
    font-size: 13px;
    flex-shrink: 0;
  }

  .drawer-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .drawer-nav-list {
    flex: 1;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .drawer-nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    font-weight: 550;
    text-align: left;
    width: 100%;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .drawer-nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .drawer-nav-item.active {
    background: var(--accent-lo);
    color: var(--accent);
    font-weight: 650;
  }

  .drawer-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .drawer-content-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 24px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .drawer-content-header h2 {
    font-size: 16px;
    font-weight: 700;
    margin: 0;
    color: var(--text-primary);
  }

  .drawer-close {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: color 0.15s ease, border-color 0.15s ease;
  }

  .drawer-close:hover {
    color: var(--text-primary);
    border-color: var(--border-hover);
  }

  .drawer-content-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) transparent;
  }

  .drawer-loading {
    display: grid;
    place-items: center;
    height: 200px;
  }

  .drawer-spinner {
    width: 28px;
    height: 28px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: drawer-spin 0.7s linear infinite;
  }

  @keyframes drawer-spin {
    to { transform: rotate(360deg); }
  }
</style>
