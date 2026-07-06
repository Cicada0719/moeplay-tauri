<script lang="ts">
  import Icon from "../Icon.svelte";
  import { Button, Tag } from "../ui";
  import { DOCK_ITEMS } from "../../nav";

  type DockItem = { id: string; label: string; icon: string; view: string };

  let { items, current, toolsOpen = false, onpick, windowFullscreen = false, ontogglefullscreen }: {
    items: DockItem[];
    current: string;
    toolsOpen?: boolean;
    onpick: (view: string) => void;
    windowFullscreen?: boolean;
    ontogglefullscreen?: () => void;
  } = $props();

  const contentItems = $derived(items.filter(i => ["home", "anime", "comic"].includes(i.id)));
  const utilItems = $derived(items.filter(i => ["tools", "settings"].includes(i.id)));
  const modeItems = $derived(items.filter(i => i.id === "bigpic"));

  const shortcutNumbers = $derived(() => {
    const map = new Map<string, number>();
    for (let i = 0; i < Math.min(5, DOCK_ITEMS.length); i++) {
      map.set(DOCK_ITEMS[i].id, i + 1);
    }
    return map;
  });

  function isActive(it: DockItem): boolean {
    if (it.id === "tools") return toolsOpen || current === "__tools";
    return current === it.view;
  }
</script>

<nav class="dock" aria-label="系统功能">
  <div class="dock-group">
    {#each contentItems as it (it.id)}
      <Button
        variant="ghost"
        size="sm"
        class="dock-btn {isActive(it) ? 'active' : ''}"
        onclick={() => onpick(it.view)}
        title={it.label}
      >
        <span class="ic"><Icon name={it.icon} size={20} /></span>
        <small>{it.label}</small>
        {#if shortcutNumbers().has(it.id)}
          <Tag variant="muted" size="sm" class="shortcut-hint">{shortcutNumbers().get(it.id)}</Tag>
        {/if}
        <span class="indicator" class:visible={isActive(it)}></span>
      </Button>
    {/each}
  </div>

  <div class="dock-sep" aria-hidden="true"></div>

  <div class="dock-group">
    {#each utilItems as it (it.id)}
      <Button
        variant="ghost"
        size="sm"
        class="dock-btn {isActive(it) ? 'active' : ''}"
        onclick={() => onpick(it.view)}
        title={it.label}
      >
        <span class="ic"><Icon name={it.icon} size={20} /></span>
        <small>{it.label}</small>
        {#if shortcutNumbers().has(it.id)}
          <Tag variant="muted" size="sm" class="shortcut-hint">{shortcutNumbers().get(it.id)}</Tag>
        {/if}
        <span class="indicator" class:visible={isActive(it)}></span>
      </Button>
    {/each}
  </div>

  <div class="dock-sep" aria-hidden="true"></div>

  <div class="dock-group">
    {#if ontogglefullscreen}
      <Button
        variant="ghost"
        size="sm"
        class="dock-btn dock-mode"
        onclick={ontogglefullscreen}
        title={windowFullscreen ? '切换到窗口模式' : '切换到全屏模式'}
      >
        <span class="ic"><Icon name={windowFullscreen ? 'shrink' : 'maximize'} size={18} /></span>
        <small>{windowFullscreen ? '窗口' : '全屏'}</small>
      </Button>
    {/if}
    {#each modeItems as it (it.id)}
      <Button
        variant="ghost"
        size="sm"
        class="dock-btn dock-mode"
        onclick={() => onpick(it.view)}
        title={it.label}
      >
        <span class="ic"><Icon name={it.icon} size={18} /></span>
        <small>{it.label}</small>
      </Button>
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

  :global(.ui-button.dock-btn) {
    position: relative;
    padding: 6px 12px 8px;
    color: var(--text-muted);
    flex: 0 0 auto;
  }
  :global(.ui-button.dock-btn .ui-button__content) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    overflow: visible;
  }
  :global(.ui-button.dock-btn) .ic {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: transparent;
    transition: background 0.2s ease, transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.2s ease;
  }
  :global(.ui-button.dock-btn) small {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  :global(.ui-button.dock-btn):hover { color: var(--text-primary); }
  :global(.ui-button.dock-btn):hover .ic {
    background: rgba(255,255,255,0.07);
    transform: translateY(-2px);
  }
  :global(.ui-button.dock-btn):active .ic { transform: translateY(0) scale(0.95); }

  :global(.ui-button.dock-btn.active) { color: var(--accent); }
  :global(.ui-button.dock-btn.active .ic) {
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

  :global(.ui-tag.shortcut-hint) {
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
  :global(.ui-button.dock-btn.active .ui-tag.shortcut-hint) {
    background: rgba(255,255,255,0.2);
    color: #fff;
  }

  /* ── Big picture mode button ── */
  :global(.ui-button.dock-mode) {
    color: var(--accent);
    opacity: 0.65;
    transition: opacity 0.2s ease, color 0.2s ease;
  }
  :global(.ui-button.dock-mode):hover { opacity: 1; }
  :global(.ui-button.dock-mode) .ic {
    background: var(--accent-lo);
    border: 1px solid rgba(232,85,127,0.25);
    border-radius: 10px;
  }
  :global(.ui-button.dock-mode):hover .ic {
    background: var(--accent);
    border-color: transparent;
    transform: translateY(-2px) scale(1.04);
    box-shadow: 0 4px 16px -4px var(--accent, rgba(232,85,127,0.4));
  }
  :global(.ui-button.dock-mode):hover .ic :global(.icon) { stroke: #fff; }

  @media (max-width: 760px) {
    .dock { padding: 6px 10px 8px; gap: 2px; }
    .dock-group { gap: 0; }
    .dock-sep { margin: 0 4px 12px; }
    :global(.ui-button.dock-btn) { padding: 5px 8px 6px; }
    :global(.ui-button.dock-btn) .ic { width: 32px; height: 32px; }
    :global(.ui-button.dock-btn) small { font-size: 9px; }
  }
</style>
