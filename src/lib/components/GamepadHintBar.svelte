<script lang="ts">
  import { onMount } from "svelte";
  import { gamepadElementLabel, gamepadPrimaryActionLabel, gamepadSecondaryActionLabel } from "../actions/a11y/gamepadSemantics";
  import type { GamepadInputMode } from "../actions/a11y/gamepadFocus";

  let {
    connected = false,
    inputMode = "keyboard",
    currentView = "home",
    focusModeAvailable = false,
    focusMode = false,
  }: {
    connected?: boolean;
    inputMode?: GamepadInputMode;
    currentView?: string;
    focusModeAvailable?: boolean;
    focusMode?: boolean;
  } = $props();

  let focused = $state<HTMLElement | null>(null);
  const active = $derived(connected && inputMode === "gamepad");
  const focusLabel = $derived(gamepadElementLabel(focused));
  const primaryLabel = $derived(gamepadPrimaryActionLabel(focused));
  const secondaryLabel = $derived(gamepadSecondaryActionLabel(focused));
  const controlKind = $derived(
    focused instanceof HTMLInputElement && focused.type === "range"
      ? "range"
      : focused instanceof HTMLSelectElement
        ? "select"
        : "default",
  );

  onMount(() => {
    const update = () => {
      focused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    };
    const onFocusIn = (event: FocusEvent) => {
      focused = event.target instanceof HTMLElement ? event.target : null;
    };
    update();
    document.addEventListener("focusin", onFocusIn);
    document.addEventListener("click", update, true);
    return () => {
      document.removeEventListener("focusin", onFocusIn);
      document.removeEventListener("click", update, true);
    };
  });
</script>

{#if connected}
  <aside
    class="gamepad-hints"
    class:active
    data-testid="gamepad-hints"
    data-view={currentView}
    aria-label="手柄操作提示"
    aria-live="polite"
  >
    {#if active}
      <div class="focus-context"><span>当前</span><strong>{focusLabel}</strong></div>
      <div class="prompt-list">
        <span class="prompt prompt--primary"><kbd>A</kbd>{primaryLabel}</span>
        {#if secondaryLabel}<span class="prompt"><kbd>Y</kbd>{secondaryLabel}</span>{/if}
        {#if controlKind === "range" || controlKind === "select"}<span class="prompt"><kbd>◀▶</kbd>调整</span>{/if}
        <span class="prompt"><kbd>B</kbd>返回</span>
        <span class="prompt"><kbd>X</kbd>搜索</span>
        <span class="prompt"><kbd>LB</kbd><kbd>RB</kbd>切换栏目</span>
        {#if focusModeAvailable}<span class="prompt"><kbd>VIEW</kbd>{focusMode ? "退出专注" : "进入专注"}</span>{/if}
        <span class="prompt"><kbd>START</kbd>大屏</span>
      </div>
    {:else}
      <div class="connected-note"><span class="connected-dot"></span><strong>手柄已连接</strong><small>按任意键显示操作提示</small></div>
    {/if}
  </aside>
{/if}

<style>
  .gamepad-hints {
    position: fixed;
    right: 18px;
    bottom: 14px;
    z-index: 145;
    max-width: min(calc(100vw - 36px), 980px);
    pointer-events: none;
    color: #f4f1e8;
    font-family: var(--font-ui, system-ui);
  }
  .connected-note, .focus-context, .prompt-list {
    border: 1px solid rgba(255,255,255,.18);
    background: rgba(5,7,10,.88);
    backdrop-filter: blur(18px) saturate(1.2);
    box-shadow: 0 12px 38px rgba(0,0,0,.32);
  }
  .connected-note { display:flex; align-items:center; gap:8px; min-height:34px; padding:0 12px; }
  .connected-note strong { font-size:11px; letter-spacing:.06em; }
  .connected-note small { color:rgba(255,255,255,.56); font-size:10px; }
  .connected-dot { width:7px; height:7px; border-radius:50%; background:#79e6a7; box-shadow:0 0 12px rgba(121,230,167,.72); }
  .gamepad-hints.active { left: 18px; display:grid; grid-template-columns:minmax(120px, .34fr) minmax(0, 1fr); }
  .focus-context { min-width:0; display:grid; align-content:center; gap:3px; min-height:42px; padding:7px 12px; border-right:0; }
  .focus-context span { color:var(--accent, #e8557f); font:700 8px/1 var(--font-mono, monospace); letter-spacing:.14em; text-transform:uppercase; }
  .focus-context strong { overflow:hidden; font-size:11px; line-height:1.15; text-overflow:ellipsis; white-space:nowrap; }
  .prompt-list { min-width:0; display:flex; align-items:center; justify-content:flex-end; gap:10px; padding:7px 10px; overflow:hidden; }
  .prompt { flex:0 0 auto; display:inline-flex; align-items:center; gap:4px; color:rgba(255,255,255,.7); font-size:10px; white-space:nowrap; }
  .prompt--primary { color:#fff; }
  kbd { min-width:22px; height:22px; display:inline-grid; place-items:center; padding:0 5px; border:1px solid rgba(255,255,255,.28); border-radius:999px; background:rgba(255,255,255,.08); color:#fff; font:800 8px/1 var(--font-mono, monospace); box-shadow:inset 0 -1px rgba(255,255,255,.1); }
  .prompt--primary kbd { border-color:color-mix(in srgb, var(--accent, #e8557f) 75%, white); background:color-mix(in srgb, var(--accent, #e8557f) 72%, transparent); }
  @media (max-width: 980px) {
    .gamepad-hints.active { grid-template-columns:minmax(104px,.28fr) minmax(0,1fr); }
    .prompt-list { justify-content:flex-start; overflow-x:auto; scrollbar-width:none; }
    .prompt-list::-webkit-scrollbar { display:none; }
  }
  @media (max-width: 640px) {
    .gamepad-hints.active { left:8px; right:8px; bottom:8px; grid-template-columns:1fr; }
    .focus-context { display:none; }
  }
  @media (max-height:560px) and (min-width:641px) {
    .gamepad-hints { right:10px; bottom:8px; }
    .gamepad-hints.active { left:10px; }
  }
  @media (prefers-reduced-motion: reduce) { .connected-dot { box-shadow:none; } }
</style>
