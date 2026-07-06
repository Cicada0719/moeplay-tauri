<script lang="ts">
  import { buildShortcutCatalog } from "../shortcuts";
  import Icon from "./Icon.svelte";

  let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props();

  const catalog = buildShortcutCatalog();
  const global = $derived(catalog.filter((s) => s.scope === "global"));
  const home = $derived(catalog.filter((s) => s.scope === "home"));

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" || e.key === "?") {
      e.preventDefault();
      onclose();
    }
  }
</script>

{#if open}
  <div class="sh-backdrop" role="button" tabindex="-1" aria-label="关闭快捷键帮助" onclick={onclose} onkeydown={(e) => { if (e.key === "Enter" || e.key === " " || e.key === "Escape") { e.preventDefault(); onclose(); } }}>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="sh-card" role="dialog" aria-modal="true" aria-label="快捷键帮助" onclick={(e) => e.stopPropagation()} onkeydown={handleKeydown} tabindex="-1">
      <header class="sh-header">
        <h2><Icon name="gamepad" size={18} /> 快捷键</h2>
        <button class="sh-close" onclick={onclose} aria-label="关闭">
          <Icon name="x" size={16} />
        </button>
      </header>

      <div class="sh-body">
        <section class="sh-group">
          <h3>全局</h3>
          <ul>
            {#each global as s (s.id)}
              <li>
                <kbd>{s.keys}</kbd>
                <span>{s.description}</span>
              </li>
            {/each}
          </ul>
        </section>

        <section class="sh-group">
          <h3>游戏库</h3>
          <ul>
            {#each home as s (s.id)}
              <li>
                <kbd>{s.keys}</kbd>
                <span>{s.description}</span>
              </li>
            {/each}
          </ul>
        </section>
      </div>

      <footer class="sh-footer">
        <button onclick={onclose}>关闭</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .sh-backdrop {
    position: fixed; inset: 0; z-index: 100;
    background: rgba(5, 8, 14, 0.55);
    display: grid; place-items: center;
    padding: 20px;
  }
  .sh-card {
    width: min(480px, calc(100vw - 40px));
    max-height: calc(100vh - 80px);
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 18px;
    background: var(--bg-elev, rgba(22, 26, 36, 0.98));
    backdrop-filter: blur(24px);
    box-shadow: 0 20px 60px rgba(0,0,0,0.4);
  }
  .sh-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 16px 18px;
    border-bottom: 1px solid var(--border);
  }
  .sh-header h2 {
    margin: 0; font-size: 16px; font-weight: 700; color: var(--text-primary);
    display: flex; align-items: center; gap: 8px;
  }
  .sh-close {
    width: 28px; height: 28px; border-radius: 8px;
    border: none; background: transparent; color: var(--text-muted);
    display: grid; place-items: center; cursor: pointer;
  }
  .sh-close:hover { color: var(--text-primary); background: rgba(255,255,255,0.05); }

  .sh-body { padding: 16px 18px; }
  .sh-group { margin-bottom: 16px; }
  .sh-group:last-child { margin-bottom: 0; }
  .sh-group h3 {
    margin: 0 0 10px; font-size: 12px; font-weight: 700;
    color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em;
  }
  .sh-group ul {
    list-style: none; margin: 0; padding: 0;
    display: flex; flex-direction: column; gap: 8px;
  }
  .sh-group li {
    display: flex; align-items: center; gap: 12px;
    font-size: 13px; color: var(--text-secondary);
  }
  kbd {
    min-width: 56px; text-align: center;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: rgba(255,255,255,0.05);
    color: var(--text-primary);
    font-family: var(--font-mono); font-size: 12px; font-weight: 600;
  }

  .sh-footer {
    padding: 12px 18px 16px;
    display: flex; justify-content: flex-end;
    border-top: 1px solid var(--border);
  }
  .sh-footer button {
    padding: 7px 16px; border-radius: 8px;
    border: 1px solid var(--border);
    background: rgba(255,255,255,0.05); color: var(--text-primary);
    font-size: 13px; cursor: pointer;
  }
  .sh-footer button:hover { border-color: var(--accent); color: var(--accent); }
</style>
