<script lang="ts">
  import { onMount } from "svelte";
  import { attachGamepad } from "./switch/useGamepad.svelte";

  let {
    onChar,
    onBack,
    onSubmit,
    onClose,
  }: {
    onChar: (char: string) => void;
    onBack: () => void;
    onSubmit: () => void;
    onClose: () => void;
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
  let detachGamepad: (() => void) | null = null;

  const currentRows = $derived(isSymbols ? SYMBOLS : ROWS);

  function clampCursor() {
    cursorRow = Math.max(0, Math.min(currentRows.length - 1, cursorRow));
    cursorCol = Math.max(0, Math.min(currentRows[cursorRow].length - 1, cursorCol));
  }

  function press(key: string) {
    if (key === "⌫") { onBack(); return; }
    if (key === "确定") { onSubmit(); return; }
    if (key === "123") { isSymbols = true; clampCursor(); return; }
    if (key === "ABC") { isSymbols = false; clampCursor(); return; }
    onChar(key);
  }

  function pressCurrent() {
    press(currentRows[cursorRow][cursorCol]);
  }

  function move(dr: number, dc: number) {
    cursorRow += dr;
    cursorCol += dc;
    clampCursor();
  }

  function onKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case "ArrowUp": move(-1, 0); e.preventDefault(); break;
      case "ArrowDown": move(1, 0); e.preventDefault(); break;
      case "ArrowLeft": move(0, -1); e.preventDefault(); break;
      case "ArrowRight": move(0, 1); e.preventDefault(); break;
      case "Enter": pressCurrent(); e.preventDefault(); break;
      case "Backspace": onBack(); e.preventDefault(); break;
      case "Escape": onClose(); e.preventDefault(); break;
    }
  }

  onMount(() => {
    detachGamepad = attachGamepad({
      left: () => move(0, -1),
      right: () => move(0, 1),
      pageLeft: () => move(-1, 0),
      pageRight: () => move(1, 0),
      activate: () => pressCurrent(),
      launch: () => pressCurrent(),
      back: () => onClose(),
    });
    return () => detachGamepad?.();
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
<div
  class="vk"
  bind:this={boardEl}
  tabindex="0"
  onkeydown={onKeydown}
  role="application"
  aria-label="屏幕键盘"
>
  {#each currentRows as row, ri}
    <div class="vk-row">
      {#each row as key, ci}
        <button
          class="vk-key"
          class:wide={key === " " || key === "确定"}
          class:active={ri === cursorRow && ci === cursorCol}
          onclick={() => press(key)}
          tabindex="-1"
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
  .vk-key.active {
    background: var(--accent, #e8557f);
    border-color: var(--accent, #e8557f);
    box-shadow: 0 0 8px rgba(232,85,127,0.4);
  }
  .vk-key.wide { min-width: 80px; }
</style>
