<script lang="ts">
  import { onMount } from "svelte";
  import { isHandheldActive, onHandheldPrefsChanged, readHandheldKeyboardPreference } from "../platform/handheld";
  import { uiStore } from "../stores/ui.svelte";
  import { platformStore } from "../platform";
  import { backspaceAtCursor, insertTextAtCursor, isTextEntryTarget, type TextTargetLike } from "../utils/textInput";

  // 掌机模式联动：输入框聚焦自动弹出屏幕键盘（掌机没有实体键盘）。
  // 仅当掌机模式生效 + 设置里「自动屏幕键盘」开启 + 非大屏模式时工作
  // （大屏搜索有自己的键盘流，避免双重键盘）。
  let open = $state(false);
  let target = $state<TextTargetLike | null>(null);
  let rootEl = $state<HTMLDivElement>();

  function enabled(): boolean {
    return platformStore.isAndroid && isHandheldActive() && readHandheldKeyboardPreference() && !uiStore.bigPictureActive;
  }

  function tryOpen(candidate: EventTarget | null) {
    if (!enabled()) return;
    const element = candidate instanceof Element ? candidate : null;
    if (!isTextEntryTarget(element)) return;
    target = element;
    open = true;
  }

  function onFocusIn(event: FocusEvent) { tryOpen(event.target); }

  function onFocusOut(event: FocusEvent) {
    // 焦点移动到键盘内部（点击键帽）时保持打开
    const next = event.relatedTarget;
    if (next instanceof Node && rootEl?.contains(next)) return;
    if (open) open = false;
  }

  function close() { open = false; }
  function onChar(ch: string) { if (target) insertTextAtCursor(target, ch); }
  function onBack() { if (target) backspaceAtCursor(target); }
  function onSubmit() { close(); }
  function onClose() { close(); }

  function onKeydown(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      // 只抢关闭键盘这一层，不阻止其他 Escape 行为
      event.stopPropagation();
      close();
    }
  }

  onMount(() => {
    document.addEventListener("focusin", onFocusIn);
    document.addEventListener("focusout", onFocusOut);
    window.addEventListener("keydown", onKeydown, true);
    const offPrefs = onHandheldPrefsChanged(() => {
      if (!enabled()) { open = false; target = null; }
    });
    return () => {
      document.removeEventListener("focusin", onFocusIn);
      document.removeEventListener("focusout", onFocusOut);
      window.removeEventListener("keydown", onKeydown, true);
      offPrefs();
    };
  });
</script>

{#if open && target}
  <div class="hkb-overlay" bind:this={rootEl} data-testid="handheld-keyboard">
    <button class="hkb-close" type="button" onclick={onClose} aria-label="关闭屏幕键盘">⌨ ✕</button>
    {#await import("./VirtualKeyboard.svelte") then { default: VirtualKeyboard }}
      <VirtualKeyboard
        active={true}
        scopeId="handheld-global-keyboard"
        onChar={onChar}
        onBack={onBack}
        onSubmit={onSubmit}
        onClose={onClose}
      />
    {/await}
  </div>
{:else if target}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="hkb-reopen" onclick={() => target?.focus({ preventScroll: true })} onkeydown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); target?.focus({ preventScroll: true }); } }} role="button" tabindex="0" aria-label="打开屏幕键盘">⌨</div>
{/if}

<style>
  .hkb-overlay {
    position: fixed;
    left: 50%;
    bottom: max(12px, env(safe-area-inset-bottom));
    z-index: 400;
    width: min(92vw, 760px);
    display: flex;
    flex-direction: column;
    gap: 6px;
    transform: translateX(-50%);
    filter: drop-shadow(0 18px 40px rgba(0,0,0,.45));
  }
  .hkb-close {
    align-self: flex-end;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid rgba(255,255,255,.18);
    border-radius: 999px;
    color: rgba(255,255,255,.75);
    background: rgba(5,7,10,.88);
    font: 700 11px var(--font-ui, system-ui);
    cursor: pointer;
  }
  .hkb-reopen {
    position: fixed;
    right: 18px;
    bottom: 64px;
    z-index: 399;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border: 1px solid rgba(255,255,255,.2);
    border-radius: 999px;
    color: #fff;
    background: rgba(5,7,10,.86);
    font-size: 20px;
    cursor: pointer;
    backdrop-filter: blur(12px);
  }
</style>
