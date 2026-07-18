<script lang="ts">
  import { onMount } from "svelte";
  import { attachGamepad, type GamepadAttachment } from "./switch/useGamepad.svelte";

  let {
    active = true,
    onChar,
    onBack,
    onSubmit,
    onClose,
    onExitUp,
    onzonechange,
  }: {
    active?: boolean;
    onChar: (char: string) => void;
    onBack: () => void;
    onSubmit: () => void;
    onClose: () => void;
    onExitUp?: () => void;
    onzonechange?: (zone: "keyboard") => void;
  } = $props();

  const ROWS = [
    ["Q","W","E","R","T","Y","U","I","O","P"],
    ["A","S","D","F","G","H","J","K","L"],
    ["Z","X","C","V","B","N","M","⌫"],
    ["123"," ",".","确定"],
  ];
  const SYMBOLS = [
    ["1","2","3","4","5","6","7","8","9","0"],
    ["-","/",":",";","(",")","$","&","@"],
    [".",",","!","?","'","\"","\\","⌫"],
    ["ABC"," ","_","确定"],
  ];

  let cursorRow = $state(0);
  let cursorCol = $state(0);
  let isSymbols = $state(false);
  let boardEl = $state<HTMLDivElement>();
  let scope: GamepadAttachment | null = null;

  const currentRows = $derived(isSymbols ? SYMBOLS : ROWS);

  function clampCursor() {
    cursorRow = Math.max(0, Math.min(currentRows.length - 1, cursorRow));
    cursorCol = Math.max(0, Math.min(currentRows[cursorRow].length - 1, cursorCol));
  }

  function focusCurrent() {
    if (!active) return;
    queueMicrotask(() => boardEl?.querySelector<HTMLElement>(`[data-key-row="${cursorRow}"][data-key-col="${cursorCol}"]`)?.focus({ preventScroll: true }));
  }

  function press(key: string) {
    if (key === "⌫") { onBack(); return; }
    if (key === "确定") { onSubmit(); return; }
    if (key === "123") { isSymbols = true; clampCursor(); focusCurrent(); return; }
    if (key === "ABC") { isSymbols = false; clampCursor(); focusCurrent(); return; }
    onChar(key);
  }

  function pressCurrent() { press(currentRows[cursorRow][cursorCol]); }

  function move(dr: number, dc: number) {
    if (dr < 0 && cursorRow === 0) { onExitUp?.(); return; }
    cursorRow += dr;
    cursorCol += dc;
    clampCursor();
    focusCurrent();
  }

  function onKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowUp": move(-1, 0); event.preventDefault(); event.stopPropagation(); break;
      case "ArrowDown": move(1, 0); event.preventDefault(); event.stopPropagation(); break;
      case "ArrowLeft": move(0, -1); event.preventDefault(); event.stopPropagation(); break;
      case "ArrowRight": move(0, 1); event.preventDefault(); event.stopPropagation(); break;
      case "Enter": case " ": pressCurrent(); event.preventDefault(); event.stopPropagation(); break;
      case "Backspace": onBack(); event.preventDefault(); event.stopPropagation(); break;
      case "Escape": onClose(); event.preventDefault(); event.stopPropagation(); break;
    }
  }

  $effect(() => {
    scope?.setEnabled(active);
    if (active) { onzonechange?.("keyboard"); focusCurrent(); }
  });

  onMount(() => {
    scope = attachGamepad({
      left: () => move(0, -1),
      right: () => move(0, 1),
      up: () => move(-1, 0),
      down: () => move(1, 0),
      activate: () => pressCurrent(),
      launch: () => pressCurrent(),
      favorite: () => onBack(),
      back: () => onClose(),
    }, { id: "big-picture-keyboard", zone: "keyboard", overlay: true, priority: 120, enabled: active });
    focusCurrent();
    return () => { scope?.(); scope = null; };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="vk"
  bind:this={boardEl}
  tabindex="-1"
  onkeydown={onKeydown}
  role="application"
  aria-label="屏幕键盘"
  data-focus-zone="keyboard"
  data-active={active ? "true" : "false"}
>
  {#each currentRows as row, ri}
    <div class="vk-row" role="group" aria-label={`键盘第 ${ri + 1} 行`}>
      {#each row as key, ci}
        <button
          class="vk-key"
          class:wide={key === " " || key === "确定"}
          class:active={ri === cursorRow && ci === cursorCol}
          data-key-row={ri}
          data-key-col={ci}
          aria-pressed={active && ri === cursorRow && ci === cursorCol}
          onclick={() => press(key)}
          onfocus={() => { cursorRow = ri; cursorCol = ci; }}
          tabindex={active && ri === cursorRow && ci === cursorCol ? 0 : -1}
          aria-label={key === " " ? "空格" : key === "⌫" ? "退格" : key}
        >
          {key}
        </button>
      {/each}
    </div>
  {/each}
</div>

<style>
  .vk {
    display: flex; flex-direction: column; gap: 4px;
    padding: 8px;
    background: rgba(0, 0, 0, 0.85);
    border-radius: 10px;
    outline: none;
  }
  .vk-row { display: flex; gap: 4px; justify-content: center; }
  .vk-key {
    min-width: 36px; height: 36px;
    display: flex; align-items: center; justify-content: center;
    border: 1px solid rgba(255,255,255,0.15);
    border-radius: 6px;
    background: rgba(255,255,255,0.08);
    color: #fff; font-size: 14px; font-weight: 600;
    cursor: pointer; transition: all 0.1s;
  }
  .vk-key:hover { background: rgba(255,255,255,0.15); }
  .vk-key.active, .vk-key:focus-visible {
    background: var(--accent, #e8557f);
    border-color: var(--accent, #e8557f);
    box-shadow: 0 0 8px rgba(232,85,127,0.4);
  }
  .vk-key.wide { min-width: 80px; }

  @media (prefers-reduced-motion: reduce) {
    .vk-key { transition: none; }
  }
  :global([data-motion="reduce"]) .vk-key { transition: none; }
</style>
