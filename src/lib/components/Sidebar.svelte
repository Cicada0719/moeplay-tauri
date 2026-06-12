<script lang="ts">
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { gameStore } from "../stores/games.svelte";
  import Icon from "./Icon.svelte";
  import { gameCompletionStatus, gameTotalSeconds } from "../utils/game";

  const menuGroups = [
    {
      label: null,
      items: [
        { id: "home",      label: "游戏库",   icon: "home" },
        { id: "discovery", label: "资源发现", icon: "compass" },
      ],
    },
    {
      label: "工具",
      items: [
        { id: "scraper",   label: "AI 刮削",   icon: "star" },
        { id: "downloads", label: "资源下载",  icon: "download" },
        { id: "backup",    label: "存档管理",  icon: "save" },
        { id: "stats",     label: "统计",      icon: "chart" },
      ],
    },
    {
      label: "导入",
      items: [
        { id: "steam-import", label: "Steam / Epic 导入", icon: "download" },
        { id: "emulator",     label: "模拟器", icon: "gamepad" },
      ],
    },
    {
      label: "系统",
      items: [
        { id: "diagnostics", label: "诊断", icon: "toolbox" },
        { id: "settings",    label: "设置", icon: "gear" },
      ],
    },
  ];

  const totalGames = $derived(gameStore.games.length);
  const completedGames = $derived(
    gameStore.games.filter((g) => gameCompletionStatus(g) === "completed").length
  );
  const totalHours = $derived(
    (gameStore.games.reduce((sum, g) => sum + gameTotalSeconds(g), 0) / 3600).toFixed(0)
  );
</script>

<aside class="sidebar" class:collapsed={uiStore.sidebarCollapsed}>
  <div class="logo">
    <span class="logo-icon" aria-hidden="true"><Icon name="gamepad" size={24} /></span>
    {#if !uiStore.sidebarCollapsed}
      <span class="logo-text">萌游</span>
    {/if}
  </div>

  <nav class="menu">
    {#each menuGroups as group, gi}
      {#if group.label}
        {#if !uiStore.sidebarCollapsed}
          <p class="group-label">{group.label}</p>
        {:else}
          <div class="group-sep"></div>
        {/if}
      {/if}
      {#each group.items as item}
        <button
          class="menu-item"
          class:active={uiStore.currentView === item.id}
          onclick={() => { uiStore.currentView = item.id; if (item.id === "home") gameStore.quickFilter = null; }}
          title={item.label}
        >
          <Icon name={item.icon} size={20} />
          {#if !uiStore.sidebarCollapsed}
            <span class="menu-label">{item.label}</span>
          {/if}
        </button>
      {/each}
    {/each}
  </nav>

  {#if !uiStore.sidebarCollapsed}
    <div class="stats-bar">
      <div class="stat">
        <span class="stat-value">{totalGames}</span>
        <span class="stat-label">游戏</span>
      </div>
      <div class="stat">
        <span class="stat-value">{completedGames}</span>
        <span class="stat-label">通关</span>
      </div>
      <div class="stat">
        <span class="stat-value">{totalHours}h</span>
        <span class="stat-label">时长</span>
      </div>
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    width: 210px;
    min-width: 210px;
    height: 100vh;
    background: var(--bg-surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    transition: width 0.3s cubic-bezier(0.22, 1, 0.36, 1), min-width 0.3s cubic-bezier(0.22, 1, 0.36, 1);
    overflow: hidden;
    position: relative; z-index: 10;
  }
  .sidebar.collapsed {
    width: 0;
    min-width: 0;
    border-right: none;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 20px;
    border-bottom: 1px solid var(--border);
  }
  .logo-icon {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    color: var(--accent-pink);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--accent-pink-lo);
    flex-shrink: 0;
  }
  .logo-text {
    font-size: 16px;
    font-weight: 760;
    color: var(--text-primary);
    letter-spacing: 0;
    white-space: nowrap;
  }

  .menu {
    flex: 1;
    padding: 8px 8px 52px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  /* 分组标题 */
  .group-label {
    font-size: 10px; font-weight: 700; letter-spacing: 0.09em;
    text-transform: uppercase; color: var(--text-dim);
    padding: 12px 12px 4px;
    user-select: none;
  }
  .group-sep {
    height: 1px;
    margin: 6px 10px;
    background: var(--border);
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    border-radius: var(--radius-md, 12px);
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: transform 0.2s cubic-bezier(0.22, 1, 0.36, 1), background 0.18s ease, color 0.18s ease;
    font-size: 14px;
    text-align: left;
    width: 100%;
    white-space: nowrap;
  }
  .menu-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    transform: translateX(3px);
  }
  .menu-item.active {
    position: relative;
    background: var(--accent-pink-lo);
    color: var(--accent-pink);
    font-weight: 650;
  }
  .menu-item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent-pink);
  }

  .stats-bar {
    padding: 14px 12px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: space-around;
  }
  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .stat-value {
    font-size: 15px;
    font-weight: 800;
    color: var(--accent-pink);
    font-variant-numeric: tabular-nums;
  }
  .stat-label {
    font-size: 10px;
    color: var(--text-muted);
    margin-top: 2px;
  }
</style>
