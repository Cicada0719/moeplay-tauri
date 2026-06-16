<script lang="ts">
  import Icon from "../Icon.svelte";

  type DockItem = { id: string; label: string; icon: string; view: string };

  let { items, current, toolsOpen = false, onpick }: {
    items: DockItem[];
    current: string;
    toolsOpen?: boolean;
    onpick: (view: string) => void;
  } = $props();

  const contentItems = $derived(items.filter(i => ["home", "anime", "comic"].includes(i.id)));
  const utilItems = $derived(items.filter(i => ["tools", "settings"].includes(i.id)));
  const modeItems = $derived(items.filter(i => i.id === "bigpic"));

  function isActive(it: DockItem): boolean {
    if (it.id === "tools") return toolsOpen || current === "__tools";
    return current === it.view;
  }
</script>

<nav class="dock" aria-label="系统功能">
  <div class="dock-group">
    {#each contentItems as it (it.id)}
      <button
        class="dock-btn"
        class:active={isActive(it)}
        onclick={() => onpick(it.view)}
        title={it.label}
      >
        <span class="ic"><Icon name={it.icon} size={20} /></span>
        <small>{it.label}</small>
        <span class="indicator" class:visible={isActive(it)}></span>
      </button>
    {/each}
  </div>

  <div class="dock-sep" aria-hidden="true"></div>

  <div class="dock-group">
    {#each utilItems as it (it.id)}
      <button
        class="dock-btn"
        class:active={isActive(it)}
        onclick={() => onpick(it.view)}
        title={it.label}
      >
        <span class="ic"><Icon name={it.icon} size={20} /></span>
        <small>{it.label}</small>
        <span class="indicator" class:visible={isActive(it)}></span>
      </button>
    {/each}
  </div>

  <div class="dock-sep" aria-hidden="true"></div>

  <div class="dock-group">
    {#each modeItems as it (it.id)}
      <button
        class="dock-btn dock-mode"
        onclick={() => onpick(it.view)}
        title={it.label}
      >
        <span class="ic"><Icon name={it.icon} size={18} /></span>
        <small>{it.label}</small>
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
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--text-muted);
    padding: 6px 12px 8px;
    border-radius: var(--radius-md);
    transition: color 0.2s ease;
    flex: 0 0 auto;
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

  .dock-btn:focus-visible { outline: none; }
  .dock-btn:focus-visible .ic { box-shadow: var(--focus-ring); }

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

  /* ── Big picture mode button ── */
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
    .dock-btn { padding: 5px 8px 6px; }
    .dock-btn .ic { width: 32px; height: 32px; }
    .dock-btn small { font-size: 9px; }
  }
</style>
