<script lang="ts">
  import { gameStore, matchesPinyin } from "../stores/games.svelte";
  import type { Game } from "../stores/games.svelte";
  import { fileSrc } from "../utils";
  import { coverOf } from "../utils/game";
  import VirtualKeyboard from "./VirtualKeyboard.svelte";
  import Icon from "./Icon.svelte";
  import CachedImage from "./CachedImage.svelte";

  let {
    open = $bindable(false),
    onselect,
  }: {
    open: boolean;
    onselect?: (game: Game) => void;
  } = $props();

  let query = $state("");
  let focusIdx = $state(0);
  let resultEl = $state<HTMLDivElement>();

  const allGames = $derived(gameStore.allGames);
  const results = $derived(
    query.trim()
      ? allGames.filter(g => matchesPinyin(g.name, query.trim().toLowerCase())).slice(0, 20)
      : []
  );

  function onInput(char: string) { query += char; focusIdx = 0; }
  function onBackspace() { query = query.slice(0, -1); focusIdx = 0; }
  function onSubmit() {
    if (results.length > 0) selectResult(focusIdx);
  }
  function onClose() { open = false; query = ""; }

  function selectResult(idx: number) {
    const g = results[idx];
    if (!g) return;
    onselect?.(g);
    onClose();
  }

  function moveResult(d: number) {
    focusIdx = Math.max(0, Math.min(results.length - 1, focusIdx + d));
  }

  function onOverlayKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") { onClose(); e.preventDefault(); }
    if (e.key === "ArrowDown") { moveResult(1); e.preventDefault(); }
    if (e.key === "ArrowUp") { moveResult(-1); e.preventDefault(); }
    if (e.key === "Enter") { onSubmit(); e.preventDefault(); }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="bps-overlay" onclick={onClose} onkeydown={onOverlayKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="bps-modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <header class="bps-header">
        <div class="bps-input-row">
          <Icon name="search" size={18} />
          <span class="bps-query">{query}<span class="bps-cursor">|</span></span>
          {#if query}
            <button class="bps-clear" onclick={() => { query = ""; focusIdx = 0; }}>
              <Icon name="x" size={14} />
            </button>
          {/if}
        </div>
        <button class="bps-close" onclick={onClose}>
          <Icon name="x" size={16} />
        </button>
      </header>

      {#if results.length > 0}
        <div class="bps-results" bind:this={resultEl}>
          {#each results as g, i}
            <button
              class="bps-result"
              class:focused={i === focusIdx}
              onclick={() => selectResult(i)}
              onfocus={() => (focusIdx = i)}
            >
              <div class="bps-thumb">
                {#if coverOf(g)}
                  <CachedImage source={coverOf(g)} cacheKey={`bps-${g.id}`} alt={g.name} />
                {:else}
                  <span class="bps-mono">{(g.name?.[0] ?? "?").toUpperCase()}</span>
                {/if}
              </div>
              <div class="bps-info">
                <span class="bps-name">{g.name}</span>
              </div>
            </button>
          {/each}
        </div>
      {:else if query}
        <div class="bps-empty">没有找到匹配的游戏</div>
      {:else}
        <div class="bps-hint">输入游戏名称或拼音首字母</div>
      {/if}

      <VirtualKeyboard onChar={onInput} onBack={onBackspace} onSubmit={onSubmit} onClose={onClose} />
    </div>
  </div>
{/if}

<style>
  .bps-overlay {
    position: fixed; inset: 0; z-index: 1000;
    background: rgba(0,0,0,0.8); backdrop-filter: blur(12px);
    display: flex; align-items: flex-end; justify-content: center;
    padding-bottom: 20px;
    animation: fade-in 0.15s ease;
  }
  .bps-modal {
    width: 680px; max-width: 95vw;
    background: rgba(20, 22, 30, 0.98);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 16px;
    overflow: hidden;
    display: flex; flex-direction: column;
    box-shadow: 0 24px 64px rgba(0,0,0,0.5);
  }

  .bps-header {
    display: flex; align-items: center; gap: 8px;
    padding: 12px 16px; border-bottom: 1px solid rgba(255,255,255,0.06);
  }
  .bps-input-row {
    flex: 1; display: flex; align-items: center; gap: 8px;
    padding: 8px 12px; border-radius: 8px;
    background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08);
    color: var(--text-primary); font-size: 16px;
  }
  .bps-query { flex: 1; }
  .bps-cursor { animation: blink 0.8s infinite; color: var(--accent); }
  @keyframes blink { 0%,100% { opacity: 1; } 50% { opacity: 0; } }
  .bps-clear {
    display: flex; align-items: center; justify-content: center;
    width: 24px; height: 24px; border: none; border-radius: 4px;
    background: rgba(255,255,255,0.08); color: var(--text-muted); cursor: pointer;
  }
  .bps-close {
    display: flex; align-items: center; justify-content: center;
    width: 32px; height: 32px; border: none; border-radius: 8px;
    background: rgba(255,255,255,0.06); color: var(--text-muted); cursor: pointer;
  }
  .bps-close:hover { background: rgba(255,255,255,0.12); }

  .bps-results {
    display: flex; gap: 8px; padding: 10px 12px;
    overflow-x: auto; scrollbar-width: none;
    min-height: 80px;
  }
  .bps-results::-webkit-scrollbar { display: none; }
  .bps-result {
    flex: 0 0 auto; width: 60px;
    display: flex; flex-direction: column; align-items: center; gap: 4px;
    background: none; border: 2px solid transparent; border-radius: 8px;
    padding: 4px; cursor: pointer; transition: all 0.15s;
  }
  .bps-result:hover { border-color: rgba(255,255,255,0.2); }
  .bps-result.focused { border-color: var(--accent); background: rgba(232,85,127,0.1); }
  .bps-thumb {
    width: 52px; height: 69px; border-radius: 6px; overflow: hidden;
    background: rgba(255,255,255,0.06); flex-shrink: 0;
  }
  .bps-thumb :global(.cached-image) { width: 100%; height: 100%; object-fit: cover; }
  .bps-mono {
    width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;
    font-size: 20px; font-weight: 700; color: var(--text-muted);
  }
  .bps-info { text-align: center; }
  .bps-name {
    font-size: 10px; color: var(--text-secondary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    display: block; max-width: 56px;
  }
  .bps-empty, .bps-hint {
    padding: 20px; text-align: center; font-size: 14px; color: var(--text-muted);
  }

  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
</style>
