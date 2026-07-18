<script lang="ts">
  import { onMount } from "svelte";
  import { gameStore, matchesPinyin } from "../stores/games.svelte";
  import type { Game } from "../stores/games.svelte";
  import { coverOf } from "../utils/game";
  import VirtualKeyboard from "./VirtualKeyboard.svelte";
  import Icon from "./Icon.svelte";
  import CachedImage from "./CachedImage.svelte";
  import { focusTrap } from "../actions/a11y/focusTrap";
  import { attachGamepad, type GamepadAttachment } from "./switch/useGamepad.svelte";

  let {
    open = $bindable(false),
    returnFocus = null,
    onselect,
    onclose,
    onzonechange,
  }: {
    open: boolean;
    returnFocus?: HTMLElement | null;
    onselect?: (game: Game) => void;
    onclose?: () => void;
    onzonechange?: (zone: "search" | "keyboard") => void;
  } = $props();

  let query = $state("");
  let focusIdx = $state(0);
  let searchActive = $state(false);
  let modalEl = $state<HTMLDivElement>();
  let resultEl = $state<HTMLDivElement>();
  let inputEl = $state<HTMLInputElement>();
  let scope: GamepadAttachment | null = null;
  let wasOpen = false;

  const allGames = $derived(gameStore.allGames);
  const results = $derived(query.trim() ? allGames.filter((g) => matchesPinyin(g.name, query.trim().toLowerCase())).slice(0, 20) : []);

  function syncResultFocus() {
    if (!searchActive || results.length === 0) return;
    queueMicrotask(() => {
      const target = resultEl?.querySelector<HTMLElement>(`[data-search-index="${focusIdx}"]`);
      target?.scrollIntoView({ inline: "nearest", block: "nearest", behavior: "smooth" });
      target?.focus({ preventScroll: true });
    });
  }

  function activateKeyboard() {
    searchActive = false;
    onzonechange?.("keyboard");
  }

  function activateSearch() {
    searchActive = true;
    onzonechange?.("search");
    if (results.length === 0) inputEl?.focus({ preventScroll: true });
    else syncResultFocus();
  }

  function onInput(char: string) { query += char; focusIdx = 0; }
  function onBackspace() { query = query.slice(0, -1); focusIdx = 0; }
  function onSubmit() { if (results.length > 0) selectResult(focusIdx); }

  function close() {
    if (!open) return;
    onclose?.();
    open = false;
    query = "";
    searchActive = false;
  }

  function selectResult(idx: number) {
    const game = results[idx];
    if (!game) return;
    close();
    queueMicrotask(() => onselect?.(game));
  }

  function moveResult(delta: number) {
    if (results.length === 0) return;
    focusIdx = Math.max(0, Math.min(results.length - 1, focusIdx + delta));
    syncResultFocus();
  }

  function onModalKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") { close(); event.preventDefault(); event.stopPropagation(); return; }
    if (!searchActive) return;
    switch (event.key) {
      case "ArrowRight": case "ArrowDown": moveResult(1); event.preventDefault(); event.stopPropagation(); break;
      case "ArrowLeft": moveResult(-1); event.preventDefault(); event.stopPropagation(); break;
      case "ArrowUp": activateKeyboard(); event.preventDefault(); event.stopPropagation(); break;
      case "Enter": case " ": onSubmit(); event.preventDefault(); event.stopPropagation(); break;
    }
  }

  function onInputKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" && results.length > 0) { activateSearch(); event.preventDefault(); }
  }

  $effect(() => {
    if (open && !wasOpen) {
      wasOpen = true;
      searchActive = true;
      onzonechange?.("search");
      queueMicrotask(() => inputEl?.focus({ preventScroll: true }));
    } else if (!open && wasOpen) {
      wasOpen = false;
    }
    scope?.setEnabled(open && searchActive);
    if (open && searchActive && results.length > 0 && document.activeElement !== inputEl) syncResultFocus();
  });

  $effect(() => {
    if (focusIdx >= results.length) focusIdx = Math.max(0, results.length - 1);
  });

  onMount(() => {
    scope = attachGamepad({
      left: () => moveResult(-1),
      right: () => moveResult(1),
      up: () => activateKeyboard(),
      down: () => { if (results.length > 0) moveResult(1); else activateKeyboard(); },
      launch: () => onSubmit(),
      activate: () => onSubmit(),
      favorite: () => { query = ""; focusIdx = 0; inputEl?.focus({ preventScroll: true }); },
      back: () => close(),
    }, { id: "big-picture-search", zone: "search", overlay: true, priority: 130, enabled: false });
    return () => { scope?.(); scope = null; };
  });
</script>

{#if open}
  <div class="bps-overlay" onclick={close} role="presentation">
    <div
      class="bps-modal"
      bind:this={modalEl}
      use:focusTrap={{
        enabled: open,
        trapFocus: true,
        closeOnEscape: true,
        initialFocus: () => inputEl,
        returnFocus: () => returnFocus,
        onEscape: () => close(),
      }}
      onclick={(event) => event.stopPropagation()}
      onkeydown={onModalKeydown}
      role="dialog"
      aria-modal="true"
      aria-labelledby="bp-search-title"
      tabindex="-1"
      data-focus-zone={searchActive ? "search" : "keyboard"}
    >
      <header class="bps-header">
        <div class="bps-input-row">
          <Icon name="search" size={18} />
          <label id="bp-search-title" class="sr-only" for="bp-search-input">搜索游戏</label>
          <input
            id="bp-search-input"
            class="bps-input"
            bind:this={inputEl}
            bind:value={query}
            oninput={() => { focusIdx = 0; }}
            onkeydown={onInputKeydown}
            placeholder="输入游戏名称或拼音首字母"
            autocomplete="off"
          />
          {#if query}
            <button class="bps-clear" onclick={() => { query = ""; focusIdx = 0; inputEl?.focus(); }} aria-label="清空搜索">
              <Icon name="x" size={14} />
            </button>
          {/if}
        </div>
        <button class="bps-close" onclick={close} aria-label="关闭搜索"><Icon name="x" size={16} /></button>
      </header>

      {#if results.length > 0}
        <div class="bps-results" bind:this={resultEl} role="listbox" aria-label="搜索结果">
          {#each results as game, index}
            <button
              class="bps-result"
              class:focused={searchActive && index === focusIdx}
              data-search-index={index}
              role="option"
              aria-selected={index === focusIdx}
              tabindex={searchActive && index === focusIdx ? 0 : -1}
              onclick={() => selectResult(index)}
              onfocus={() => { focusIdx = index; searchActive = true; onzonechange?.("search"); }}
            >
              <div class="bps-thumb">
                {#if coverOf(game)}<CachedImage source={coverOf(game)} cacheKey={`bps-${game.id}`} alt={game.name} />
                {:else}<span class="bps-mono">{(game.name?.[0] ?? "?").toUpperCase()}</span>{/if}
              </div>
              <div class="bps-info"><span class="bps-name">{game.name}</span></div>
            </button>
          {/each}
        </div>
      {:else if query}<div class="bps-empty">没有找到匹配的游戏</div>
      {:else}<div class="bps-hint">输入游戏名称或使用屏幕键盘</div>{/if}

      <VirtualKeyboard
        active={!searchActive}
        onChar={onInput}
        onBack={onBackspace}
        onSubmit={onSubmit}
        onClose={close}
        onExitUp={activateSearch}
        onzonechange={(zone) => onzonechange?.(zone)}
      />
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
  .bps-input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--text-primary); font: inherit; }
  .bps-input::placeholder { color: var(--text-muted); }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
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
  .bps-result.focused, .bps-result:focus-visible { border-color: var(--accent); background: rgba(232,85,127,0.1); }
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

  @media (prefers-reduced-motion: reduce) {
    .bps-overlay { animation: none; }
    .bps-result { transition: none; }
  }
  :global([data-motion="reduce"]) .bps-overlay { animation: none; }
  :global([data-motion="reduce"]) .bps-result { transition: none; }
</style>
