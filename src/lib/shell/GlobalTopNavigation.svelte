<script module lang="ts">
  export interface GlobalTopNavigationItem {
    id: string;
    label: string;
    view: string;
    ariaLabel?: string;
    icon?: string;
    shortcut?: string;
  }

  export interface GlobalTopNavigationProps {
    currentView: string;
    contentItems: readonly GlobalTopNavigationItem[];
    onNavigate: (view: string) => void;
    onSearch: () => void;
    onStatus: () => void;
    onTools: () => void;
    onSettings: () => void;
    onToggleFullscreen: () => void;
    onBigPicture: () => void;
    windowFullscreen: boolean;
    toolsOpen: boolean;
    taskActiveCount: number;
    taskFailedCount: number;
  }
</script>

<script lang="ts">
  import Icon from "../components/Icon.svelte";

  let {
    currentView,
    contentItems,
    onNavigate,
    onSearch,
    onStatus,
    onTools,
    onSettings,
    onToggleFullscreen,
    onBigPicture,
    windowFullscreen,
    toolsOpen,
    taskActiveCount,
    taskFailedCount,
  }: GlobalTopNavigationProps = $props();

  let menuOpen = $state(false);

  const currentItem = $derived(
    contentItems.find((item) => item.view === currentView),
  );
  const statusTotal = $derived(taskActiveCount + taskFailedCount);
  const hasTaskActivity = $derived(statusTotal > 0);
  const hasFailures = $derived(taskFailedCount > 0);

  function navigate(view: string) {
    menuOpen = false;
    onNavigate(view);
  }

  function closeMenu() {
    menuOpen = false;
  }

  function handleNavKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && menuOpen) {
      event.preventDefault();
      closeMenu();
      document.getElementById("global-top-navigation-menu-toggle")?.focus();
    }
  }
</script>

<svelte:window onkeydown={handleNavKeydown} />

<header class="top-navigation" data-current-view={currentView}>
  <a class="brand" href="#main-content" aria-label="MoePlay，跳到主要内容">
    <span class="brand-mark" aria-hidden="true"></span>
    <span class="brand-type">MOEPLAY</span>
    <span class="brand-code">MP / 13</span>
  </a>

  <nav class="content-navigation" aria-label="主要模块">
    <div class="desktop-content-items">
      {#each contentItems as item, index (item.id)}
        <button
          type="button"
          class="content-link"
          class:active={item.view === currentView}
          aria-label={item.ariaLabel ?? `打开${item.label}`}
          aria-current={item.view === currentView ? "page" : undefined}
          onclick={() => navigate(item.view)}
        >
          <span class="content-index">0{index + 1}</span>
          <span>{item.label}</span>
        </button>
      {/each}
    </div>

    <div class="compact-content-items">
      <button
        id="global-top-navigation-menu-toggle"
        type="button"
        class="current-module"
        aria-label="切换主要模块"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        aria-controls="global-top-navigation-menu"
        onclick={() => (menuOpen = !menuOpen)}
      >
        <span class="current-kicker">当前模块</span>
        <span class="current-label">{currentItem?.label ?? "导航"}</span>
        <Icon name="chevronDown" size={14} stroke={1.6} />
      </button>

      {#if menuOpen}
        <div id="global-top-navigation-menu" class="module-menu" role="menu" aria-label="选择主要模块">
          {#each contentItems as item, index (item.id)}
            <button
              type="button"
              role="menuitem"
              class="module-menu-item"
              class:active={item.view === currentView}
              aria-current={item.view === currentView ? "page" : undefined}
              onclick={() => navigate(item.view)}
            >
              <span class="menu-index">0{index + 1}</span>
              <span>{item.label}</span>
              {#if item.view === currentView}<span class="menu-state">当前</span>{/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </nav>

  <div class="utility-navigation" aria-label="全局功能">
    <button type="button" class="utility-button search-button" aria-label="全局搜索" onclick={onSearch}>
      <Icon name="search" size={18} stroke={1.6} />
      <span class="utility-label">搜索</span>
      <span class="utility-code">FIND</span>
    </button>

    <button
      type="button"
      class="utility-button status-button"
      class:has-activity={hasTaskActivity}
      class:has-failures={hasFailures}
      aria-label={`任务状态：进行中 ${taskActiveCount}，失败 ${taskFailedCount}`}
      onclick={onStatus}
    >
      <span class="status-glyph" aria-hidden="true">
        <span class="status-line"></span>
        <span class="status-line"></span>
        <span class="status-line"></span>
      </span>
      <span class="utility-label">状态</span>
      {#if hasTaskActivity}
        <span class="task-count" aria-hidden="true">{statusTotal > 99 ? "99+" : statusTotal}</span>
        <span class="task-summary" aria-hidden="true">A{taskActiveCount} / F{taskFailedCount}</span>
      {:else}
        <span class="utility-code">LIVE</span>
      {/if}
    </button>

    <button id="system-dock-tools" type="button" class="utility-button" aria-label="打开工具抽屉" aria-expanded={toolsOpen} aria-controls="tools-drawer" onclick={onTools}>
      <Icon name="grid" size={18} stroke={1.6} />
      <span class="utility-label">工具</span>
      <span class="utility-code">TOOLS</span>
    </button>

    <button type="button" class="utility-button mode-button" aria-label={windowFullscreen ? "退出窗口全屏" : "进入窗口全屏"} onclick={onToggleFullscreen}>
      <Icon name={windowFullscreen ? "shrink" : "maximize"} size={18} stroke={1.6} />
      <span class="utility-label">{windowFullscreen ? "窗口" : "全屏"}</span>
      <span class="utility-code">FULL</span>
    </button>

    <button type="button" class="utility-button mode-button" aria-label="进入大屏模式" onclick={onBigPicture}>
      <Icon name="tv" size={18} stroke={1.6} />
      <span class="utility-label">大屏</span>
      <span class="utility-code">BIG</span>
    </button>

    <button type="button" class="utility-button settings-button" aria-label="打开设置" onclick={onSettings}>
      <Icon name="gear" size={18} stroke={1.6} />
      <span class="utility-label">设置</span>
      <span class="utility-code">SET</span>
    </button>
  </div>
</header>

{#if menuOpen}
  <button class="menu-scrim" type="button" aria-label="关闭模块菜单" onclick={closeMenu}></button>
{/if}

<style>
  .top-navigation {
    --nav-black: var(--c-black, #050505);
    --nav-paper: var(--c-paper, #eeeae0);
    --nav-muted: var(--c-muted, #99958c);
    --nav-dim: var(--c-dim, #67645e);
    --nav-line: var(--c-line, rgba(238, 234, 224, 0.16));
    --nav-line-strong: var(--c-line-strong, rgba(238, 234, 224, 0.42));
    --nav-accent: var(--c-accent, #c7472f);
    position: relative;
    z-index: 50;
    display: grid;
    grid-template-columns: minmax(168px, .78fr) auto minmax(520px, 1.42fr);
    align-items: stretch;
    width: 100%;
    height: 64px;
    border-bottom: 1px solid var(--nav-line);
    background: rgba(5, 5, 5, 0.96);
    color: var(--nav-paper);
    font-family: var(--font-ui, "Outfit", system-ui, sans-serif);
  }

  .brand {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 clamp(14px, 2.1vw, 34px);
    color: inherit;
    text-decoration: none;
  }

  .brand-mark {
    width: 9px;
    height: 9px;
    border: 1px solid var(--nav-accent);
    background: linear-gradient(135deg, transparent 46%, var(--nav-accent) 47% 100%);
    transform: rotate(45deg);
  }

  .brand-type {
    font-family: var(--font-display, "Outfit", system-ui, sans-serif);
    font-size: 15px;
    font-weight: 700;
    letter-spacing: -0.025em;
  }

  .brand-code,
  .utility-code,
  .content-index,
  .menu-index,
  .current-kicker {
    font-family: var(--font-mono, "JetBrains Mono", monospace);
    text-transform: uppercase;
  }

  .brand-code {
    color: var(--nav-dim);
    font-size: 7px;
    letter-spacing: 0.12em;
  }

  .content-navigation {
    position: relative;
    min-width: 0;
  }

  .desktop-content-items {
    height: 100%;
    display: flex;
    align-items: stretch;
    border-left: 1px solid var(--nav-line);
    border-right: 1px solid var(--nav-line);
  }

  .content-link,
  .utility-button,
  .current-module,
  .module-menu-item {
    appearance: none;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .content-link {
    position: relative;
    min-width: clamp(76px, 7.1vw, 112px);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0 16px;
    border-right: 1px solid var(--nav-line);
    color: var(--nav-muted);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    transition: color 150ms ease, background 150ms ease;
  }

  .content-link:last-child { border-right: 0; }
  .content-link:hover { color: var(--nav-paper); background: rgba(238, 234, 224, 0.04); }
  .content-link.active { color: var(--nav-paper); background: rgba(199, 71, 47, 0.08); }
  .content-link.active::after {
    content: "";
    position: absolute;
    right: 14px;
    bottom: -1px;
    left: 14px;
    height: 2px;
    background: var(--nav-accent);
  }

  .content-index { color: var(--nav-dim); font-size: 7px; letter-spacing: 0.04em; }
  .content-link.active .content-index { color: var(--nav-accent); }

  .utility-navigation {
    min-width: 0;
    display: flex;
    justify-content: flex-end;
    align-items: stretch;
  }

  .utility-button {
    position: relative;
    min-width: 72px;
    display: grid;
    grid-template-columns: auto auto;
    grid-template-rows: 1fr auto;
    align-items: center;
    justify-content: center;
    column-gap: 8px;
    padding: 9px 13px 8px;
    border-left: 1px solid var(--nav-line);
    color: var(--nav-muted);
    transition: color 150ms ease, background 150ms ease;
  }

  .utility-button:hover { color: var(--nav-paper); background: rgba(238, 234, 224, 0.045); }
  .utility-button :global(svg), .status-glyph { align-self: end; }
  .utility-label { align-self: end; font-size: 10px; font-weight: 600; letter-spacing: 0.08em; }
  .utility-code,
  .task-summary {
    grid-column: 1 / -1;
    justify-self: end;
    color: var(--nav-dim);
    font-size: 6px;
    letter-spacing: 0.12em;
  }

  .search-button { min-width: 98px; }
  .settings-button { margin-right: clamp(0px, 1vw, 16px); }

  .status-glyph {
    width: 18px;
    height: 18px;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding: 3px 2px;
  }

  .status-line { width: 3px; height: 5px; background: currentColor; }
  .status-line:nth-child(2) { height: 10px; }
  .status-line:nth-child(3) { height: 7px; }
  .status-button.has-activity { color: var(--nav-paper); }
  .status-button.has-failures { color: var(--nav-accent); }
  .task-summary {
    justify-self: end;
    color: var(--nav-dim);
    font: 500 6px/1 var(--font-mono, "JetBrains Mono", monospace);
    letter-spacing: 0.08em;
  }
  .has-failures .task-summary { color: var(--nav-accent); }

  .task-count {
    position: absolute;
    top: 7px;
    right: 7px;
    min-width: 15px;
    height: 15px;
    display: grid;
    place-items: center;
    padding: 0 3px;
    border: 1px solid currentColor;
    background: var(--nav-black);
    color: inherit;
    font: 600 7px/1 var(--font-mono, "JetBrains Mono", monospace);
  }

  .compact-content-items { display: none; }
  .menu-scrim { display: none; }

  :is(.brand, .content-link, .utility-button, .current-module, .module-menu-item):focus-visible {
    outline: 1px solid var(--nav-paper);
    outline-offset: -3px;
  }

  @media (max-width: 1180px) {
    .top-navigation { grid-template-columns: minmax(132px, .7fr) auto minmax(360px, 1.3fr); }
    .brand-code, .utility-code { display: none; }
    .content-link { min-width: 70px; padding-inline: 11px; }
    .utility-button, .search-button { min-width: 62px; padding-inline: 9px; }
  }

  @media (max-width: 760px) {
    .top-navigation {
      grid-template-columns: auto minmax(104px, 1fr) auto;
      height: 56px;
    }

    .brand { padding: 0 12px; border-right: 1px solid var(--nav-line); }
    .brand-type { font-size: 0; }
    .brand-type::after { content: "MP"; font-size: 13px; letter-spacing: -0.04em; }
    .brand-mark { width: 8px; height: 8px; }
    .desktop-content-items { display: none; }
    .compact-content-items { display: block; height: 100%; }

    .current-module {
      width: 100%;
      height: 100%;
      display: grid;
      grid-template-columns: 1fr auto;
      grid-template-rows: auto auto;
      align-content: center;
      column-gap: 8px;
      padding: 7px 12px 6px;
      text-align: left;
    }

    .current-kicker { color: var(--nav-accent); font-size: 6px; letter-spacing: 0.13em; }
    .current-label { font-size: 12px; font-weight: 650; letter-spacing: 0.08em; }
    .current-module :global(svg) { grid-column: 2; grid-row: 1 / 3; transition: transform 150ms ease; }
    .current-module[aria-expanded="true"] :global(svg) { transform: rotate(180deg); }

    .module-menu {
      position: absolute;
      top: calc(100% + 1px);
      left: 0;
      z-index: 52;
      width: min(260px, calc(100vw - 24px));
      border: 1px solid var(--nav-line-strong);
      border-top: 0;
      background: rgba(5, 5, 5, 0.99);
      box-shadow: 16px 18px 0 rgba(0, 0, 0, 0.28);
    }

    .module-menu-item {
      width: 100%;
      min-height: 48px;
      display: grid;
      grid-template-columns: 28px 1fr auto;
      align-items: center;
      gap: 8px;
      padding: 0 14px;
      border-bottom: 1px solid var(--nav-line);
      color: var(--nav-muted);
      text-align: left;
      font-size: 12px;
      letter-spacing: 0.08em;
    }

    .module-menu-item:last-child { border-bottom: 0; }
    .module-menu-item:hover, .module-menu-item.active { color: var(--nav-paper); background: rgba(199, 71, 47, 0.09); }
    .module-menu-item.active { box-shadow: inset 2px 0 var(--nav-accent); }
    .menu-index { color: var(--nav-dim); font-size: 7px; }
    .menu-state { color: var(--nav-accent); font: 600 7px/1 var(--font-mono, "JetBrains Mono", monospace); letter-spacing: 0.1em; }

    .utility-button, .search-button {
      min-width: 43px;
      grid-template-columns: 1fr;
      grid-template-rows: 1fr;
      padding: 0;
    }

    .utility-button :global(svg), .status-glyph { align-self: center; justify-self: center; }
    .utility-label, .utility-code, .task-summary { display: none; }
    .settings-button { margin-right: 0; }
    .task-count { top: 5px; right: 4px; }

    .menu-scrim {
      position: fixed;
      inset: 56px 0 0;
      z-index: 49;
      display: block;
      width: 100%;
      border: 0;
      border-radius: 0;
      background: rgba(0, 0, 0, 0.56);
      cursor: default;
    }
  }

  @media (max-width: 470px) {
    .brand { gap: 7px; padding-inline: 9px; }
    .brand-mark { display: none; }
    .current-module { padding-inline: 9px; }
    .utility-button, .search-button { min-width: 39px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .content-link, .utility-button, .current-module :global(svg) { transition: none; }
  }

  @media (forced-colors: active) {
    .top-navigation, .module-menu { border-color: CanvasText; background: Canvas; }
    .content-link.active::after { background: Highlight; }
  }
</style>



