<script lang="ts">
  import Icon from "../Icon.svelte";

  type RailItem = {
    id: string;
    label: string;
    ariaLabel?: string;
    icon: string;
    view: string;
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
    items: RailItem[];
    current: string;
    toolsOpen?: boolean;
    toolsControlsId?: string;
    onpick: (view: string) => void;
    windowFullscreen?: boolean;
    ontogglefullscreen?: () => void;
  } = $props();

  function isActive(item: RailItem) {
    return item.id === "tools" ? toolsOpen || current === "__tools" : current === item.view;
  }
</script>

<nav class="context-rail" aria-label="系统快捷操作" data-testid="system-dock">
  <div class="rail-register" aria-hidden="true">
    <span>MP</span><b>SYS</b>
  </div>

  <div class="rail-actions">
    {#each items.filter((item) => item.surface === "utility") as item, index (item.id)}
      <button
        id="system-dock-{item.id}"
        type="button"
        class:active={isActive(item)}
        data-index={`U${String(index + 1).padStart(2, "0")}`}
        aria-label={item.ariaLabel ?? item.label}
        aria-current={item.id !== "tools" && isActive(item) ? "page" : undefined}
        aria-expanded={item.id === "tools" ? toolsOpen : undefined}
        aria-controls={item.id === "tools" ? toolsControlsId : undefined}
        onclick={() => onpick(item.view)}
      >
        <Icon name={item.icon} size={17} stroke={1.5} />
        <span>{item.label}</span>
      </button>
    {/each}
  </div>

  <div class="rail-modes">
    {#if ontogglefullscreen}
      <button type="button" aria-label={windowFullscreen ? "退出全屏" : "进入全屏"} onclick={ontogglefullscreen}>
        <Icon name={windowFullscreen ? "shrink" : "maximize"} size={17} stroke={1.5} />
        <span>{windowFullscreen ? "窗口" : "全屏"}</span>
      </button>
    {/if}
    {#each items.filter((item) => item.surface === "mode") as item (item.id)}
      <button id="system-dock-{item.id}" type="button" aria-label={item.ariaLabel ?? item.label} onclick={() => onpick(item.view)}>
        <Icon name={item.icon} size={17} stroke={1.5} />
        <span>{item.label}</span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .context-rail {
    --rail-line: var(--c-line, rgba(238, 234, 224, .16));
    width: 100%; height: 100%; min-width: 0; display: grid;
    grid-template-rows: 64px minmax(0, 1fr) auto; color: var(--c-paper, #eeeae0);
    font-family: var(--font-ui, "Outfit", sans-serif);
  }
  .rail-register { display: grid; place-items: center; align-content: center; gap: 2px; border-bottom: 1px solid var(--rail-line); font: 700 11px/1 var(--font-mono, monospace); letter-spacing: .12em; }
  .rail-register b { color: var(--c-accent, #c7472f); font-size: 6px; letter-spacing: .2em; }
  .rail-actions, .rail-modes { display: flex; flex-direction: column; }
  .rail-actions { padding-top: 7px; }
  .rail-modes { border-top: 1px solid var(--rail-line); }
  button { position: relative; min-height: 62px; display: grid; place-items: center; align-content: center; gap: 6px; padding: 5px 2px; border: 0; border-bottom: 1px solid var(--rail-line); background: transparent; color: var(--c-muted, #99958c); cursor: pointer; transition: color 160ms ease, background 160ms ease; }
  button::before { content: attr(data-index); position: absolute; top: 5px; left: 6px; color: var(--c-dim, #67645e); font: 500 5px/1 var(--font-mono, monospace); letter-spacing: .08em; }
  button span { font-size: 9px; letter-spacing: .08em; }
  button:hover, button.active { color: var(--c-paper, #eeeae0); background: rgba(238, 234, 224, .04); }
  button.active { box-shadow: inset 2px 0 var(--c-accent, #c7472f); }
  button:focus-visible { outline: 1px solid var(--c-paper, #eeeae0); outline-offset: -3px; }
  @media (max-width: 760px) {
    .context-rail { grid-template-columns: auto 1fr auto; grid-template-rows: 1fr; }
    .rail-register { min-width: 48px; border-bottom: 0; border-right: 1px solid var(--rail-line); }
    .rail-actions, .rail-modes { flex-direction: row; padding: 0; border: 0; }
    .rail-modes { border-left: 1px solid var(--rail-line); }
    button { min-width: 58px; min-height: 100%; border-bottom: 0; border-right: 1px solid var(--rail-line); }
    button.active { box-shadow: inset 0 2px var(--c-accent, #c7472f); }
  }
  @media (prefers-reduced-motion: reduce) { button { transition: none; } }
</style>


