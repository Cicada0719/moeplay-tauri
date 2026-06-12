<script lang="ts">
  import Icon from "../Icon.svelte";

  type DockItem = { id: string; label: string; icon: string; view: string };

  let { items, current, onpick }: {
    items: DockItem[];
    current: string;
    onpick: (view: string) => void;
  } = $props();
</script>

<nav class="dock" aria-label="系统功能">
  {#each items as it (it.id)}
    <button
      class="dock-btn"
      class:active={current === it.view}
      onclick={() => onpick(it.view)}
      title={it.label}
    >
      <span class="ic"><Icon name={it.icon} size={20} /></span>
      <small>{it.label}</small>
    </button>
  {/each}
</nav>

<style>
  .dock {
    display: flex;
    justify-content: center;
    gap: 6px;
    padding: 10px 16px 18px;
    flex-shrink: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .dock::-webkit-scrollbar { display: none; }
  .dock-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--text-muted);
    padding: 6px 8px;
    border-radius: var(--radius-md);
    transition: color 0.18s ease;
    flex: 0 0 auto;
  }
  .dock-btn .ic {
    width: var(--sw-dock-icon);
    height: var(--sw-dock-icon);
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: transparent;
    transition: background 0.18s ease, transform 0.18s ease;
  }
  .dock-btn small { font-size: 11px; }
  .dock-btn:hover { color: var(--text-primary); }
  .dock-btn:hover .ic { background: var(--sw-dock-hover); transform: translateY(-1px); }
  .dock-btn.active { color: var(--accent); }
  .dock-btn.active .ic { background: var(--accent-lo); }
  .dock-btn:focus-visible { outline: none; }
  .dock-btn:focus-visible .ic { box-shadow: var(--focus-ring); }

  @media (max-width: 760px) {
    .dock {
      justify-content: flex-start;
      padding: 8px 12px 12px;
    }
    .dock-btn {
      min-width: 54px;
      padding-inline: 5px;
    }
    .dock-btn .ic {
      width: 40px;
      height: 40px;
    }
  }
</style>
