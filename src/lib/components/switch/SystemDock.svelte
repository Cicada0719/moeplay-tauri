<script lang="ts">
  import Icon from "../Icon.svelte";
  import { Tag } from "../ui";

  type DockItem = {
    id: string;
    label: string;
    ariaLabel?: string;
    icon: string;
    view: string;
    shortcut?: string;
    surface?: "content" | "utility" | "mode";
  };

  let {
    items,
    current,
    toolsOpen = false,
    toolsControlsId = "tools-drawer",
    onpick,
    windowFullscreen = false,
    ontogglefullscreen,
  }: {
    items: DockItem[];
    current: string;
    toolsOpen?: boolean;
    toolsControlsId?: string;
    onpick: (view: string) => void;
    windowFullscreen?: boolean;
    ontogglefullscreen?: () => void;
  } = $props();

  const contentItems = $derived(items.filter(item => item.surface === "content"));
  const utilityItems = $derived(items.filter(item => item.surface === "utility"));
  const modeItems = $derived(items.filter(item => item.surface === "mode"));

  function isActive(item: DockItem): boolean {
    if (item.id === "tools") return toolsOpen || current === "__tools";
    return current === item.view;
  }

  function accessibleLabel(item: DockItem): string {
    return item.ariaLabel ?? item.label;
  }
</script>

<nav class="dock" aria-label="主导航" data-testid="system-dock">
  <div class="dock-group" aria-label="内容入口">
    {#each contentItems as item (item.id)}
      <button
        id="system-dock-{item.id}"
        type="button"
        class="dock-btn {isActive(item) ? 'active' : ''}"
        onclick={() => onpick(item.view)}
        title={item.label}
        aria-label={accessibleLabel(item)}
        aria-current={isActive(item) ? "page" : undefined}
      >
        <span class="dock-btn-content">
          <span class="ic"><Icon name={item.icon} size={20} /></span>
          <small>{item.label}</small>
          {#if item.shortcut}
            <Tag variant="muted" size="sm" class="shortcut-hint">{item.shortcut}</Tag>
          {/if}
          <span class="indicator" class:visible={isActive(item)} aria-hidden="true"></span>
        </span>
      </button>
    {/each}
  </div>

  <div class="dock-sep" aria-hidden="true"></div>

  <div class="dock-group" aria-label="工具与设置">
    {#each utilityItems as item (item.id)}
      <button
        id="system-dock-{item.id}"
        type="button"
        class="dock-btn {isActive(item) ? 'active' : ''}"
        onclick={() => onpick(item.view)}
        title={item.label}
        aria-label={accessibleLabel(item)}
        aria-current={item.id !== "tools" && isActive(item) ? "page" : undefined}
        aria-expanded={item.id === "tools" ? toolsOpen : undefined}
        aria-controls={item.id === "tools" ? toolsControlsId : undefined}
      >
        <span class="dock-btn-content">
          <span class="ic"><Icon name={item.icon} size={20} /></span>
          <small>{item.label}</small>
          {#if item.shortcut}
            <Tag variant="muted" size="sm" class="shortcut-hint">{item.shortcut}</Tag>
          {/if}
          <span class="indicator" class:visible={isActive(item)} aria-hidden="true"></span>
        </span>
      </button>
    {/each}
  </div>

  <div class="dock-sep" aria-hidden="true"></div>

  <div class="dock-group" aria-label="显示模式">
    {#if ontogglefullscreen}
      <button
        id="system-dock-fullscreen"
        type="button"
        class="dock-btn dock-mode"
        onclick={ontogglefullscreen}
        title={windowFullscreen ? "切换到窗口模式" : "切换到全屏模式"}
        aria-label={windowFullscreen ? "切换到窗口模式" : "切换到全屏模式"}
      >
        <span class="dock-btn-content">
          <span class="ic"><Icon name={windowFullscreen ? "shrink" : "maximize"} size={18} /></span>
          <small>{windowFullscreen ? "窗口" : "全屏"}</small>
        </span>
      </button>
    {/if}
    {#each modeItems as item (item.id)}
      <button
        id="system-dock-{item.id}"
        type="button"
        class="dock-btn dock-mode"
        onclick={() => onpick(item.view)}
        title={item.label}
        aria-label={accessibleLabel(item)}
      >
        <span class="dock-btn-content">
          <span class="ic"><Icon name={item.icon} size={18} /></span>
          <small>{item.label}</small>
        </span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .dock {
    display: flex;
    justify-content: center;
    align-items: flex-end;
    gap: 4px;
    padding: 8px 16px 12px;
    flex-shrink: 0;
  }
  .dock-group {
    display: flex;
    gap: 2px;
  }
  .dock-sep {
    width: 1px;
    height: 28px;
    margin: 0 8px 14px;
    background: var(--border);
    opacity: 0.4;
    flex-shrink: 0;
  }

  .dock-btn {
    min-width: 52px;
    min-height: 52px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: transparent;
    position: relative;
    padding: 6px 12px 8px;
    color: var(--text-muted);
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-ui);
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
    user-select: none;
    transition: color 0.2s ease, background 0.2s ease, border-color 0.2s ease, transform 0.12s ease;
  }
  .dock-btn:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: 2px;
  }
  .dock-btn-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    overflow: visible;
  }
  .dock-btn .ic {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: transparent;
    transition: background 0.2s ease, transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.2s ease;
  }
  .dock-btn small {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .dock-btn:hover { color: var(--text-primary); }
  .dock-btn:hover .ic {
    background: rgba(255,255,255,0.07);
    transform: translateY(-2px);
  }
  .dock-btn:active .ic { transform: translateY(0) scale(0.95); }

  .dock-btn.active { color: var(--accent); }
  .dock-btn.active .ic {
    background: var(--accent-lo);
    box-shadow: 0 0 12px -4px var(--accent, rgba(232,85,127,0.3));
  }

  .indicator {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent);
    opacity: 0;
    transform: scale(0);
    transition: opacity 0.25s ease, transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .indicator.visible {
    opacity: 1;
    transform: scale(1);
  }

  .dock-btn :global(.ui-tag.shortcut-hint) {
    position: absolute;
    top: 4px;
    right: 4px;
    min-width: 14px;
    padding: 1px 4px;
    border-radius: 999px;
    background: rgba(255,255,255,0.1);
    color: var(--text-muted);
    font-size: 9px;
    font-weight: 700;
    font-family: var(--font-mono);
    line-height: 1;
    border: 0;
  }
  .dock-btn.active :global(.ui-tag.shortcut-hint) {
    background: rgba(255,255,255,0.2);
    color: #fff;
  }

  .dock-mode {
    color: var(--accent);
    opacity: 0.65;
    transition: opacity 0.2s ease, color 0.2s ease;
  }
  .dock-mode:hover { opacity: 1; }
  .dock-mode .ic {
    background: var(--accent-lo);
    border: 1px solid rgba(232,85,127,0.25);
    border-radius: 10px;
  }
  .dock-mode:hover .ic {
    background: var(--accent);
    border-color: transparent;
    transform: translateY(-2px) scale(1.04);
    box-shadow: 0 4px 16px -4px var(--accent, rgba(232,85,127,0.4));
  }
  .dock-mode:hover .ic :global(.icon) { stroke: #fff; }

  @media (max-width: 760px) {
    .dock { padding: 6px 10px 8px; gap: 2px; }
    .dock-group { gap: 0; }
    .dock-sep { margin: 0 4px 12px; }
    .dock-btn { min-width: 48px; min-height: 48px; padding: 5px 8px 6px; }
    .dock-btn .ic { width: 32px; height: 32px; }
    .dock-btn small { font-size: 9px; }
  }

  @media (max-width: 620px) {
    .dock { overflow-x: auto; justify-content: flex-start; scrollbar-width: none; }
    .dock::-webkit-scrollbar { display: none; }
    .dock-btn small { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  }

  @media (prefers-reduced-motion: reduce) {
    .dock-btn,
    .dock-btn .ic,
    .indicator { transition-duration: 0.01ms; }
  }
</style>
