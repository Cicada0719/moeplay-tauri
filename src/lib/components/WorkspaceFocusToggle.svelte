<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    active = false,
    view = "home",
    controllerActive = false,
    onToggle,
  }: {
    active?: boolean;
    view?: string;
    controllerActive?: boolean;
    onToggle?: () => void;
  } = $props();
</script>

<button
  type="button"
  class="workspace-focus-toggle"
  class:active
  class:controller-active={controllerActive}
  aria-pressed={active}
  aria-label={active ? "退出专注模式并恢复全部界面控件" : "进入专注模式并临时隐藏辅助控件"}
  title={active ? "退出专注模式（View）" : "进入专注模式（View）"}
  data-state={active ? "focus" : "standard"}
  data-gamepad-label={active ? "退出专注模式" : "进入专注模式"}
  data-gamepad-activate={active ? "退出专注" : "进入专注"}
  data-gamepad-skip="true"
  data-workspace-focus-toggle
  data-view={view}
  onclick={onToggle}
>
  <Icon name={active ? "maximize" : "shrink"} size={13} />
  <span class="workspace-focus-toggle__label">{active ? "退出专注" : "专注模式"}</span>
  <span class="workspace-focus-toggle__short" aria-hidden="true">{active ? "退出" : "专注"}</span>
  {#if controllerActive}<kbd>VIEW</kbd>{/if}
</button>

<style>
  .workspace-focus-toggle {
    position: fixed;
    right: max(12px, env(safe-area-inset-right));
    bottom: calc(4.75rem + env(safe-area-inset-bottom));
    z-index: 144;
    min-width: 3.5rem;
    min-height: 1.875rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.3rem;
    padding: 0 0.55rem;
    border: 1px solid rgba(255,255,255,.2);
    border-radius: 999px;
    background: rgba(5,7,10,.78);
    color: rgba(255,255,255,.74);
    font: 700 10px/1 var(--font-ui, system-ui);
    cursor: pointer;
    backdrop-filter: blur(16px);
    box-shadow: 0 6px 20px rgba(0,0,0,.24);
    transition: transform 160ms ease, background 160ms ease, border-color 160ms ease;
  }
  .workspace-focus-toggle__label {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  .workspace-focus-toggle__short {
    font-size: 0.6875rem;
    font-weight: 750;
    letter-spacing: .02em;
  }
  .workspace-focus-toggle:hover,
  .workspace-focus-toggle:focus-visible,
  .workspace-focus-toggle.active {
    color: #fff;
    border-color: color-mix(in srgb, var(--accent, #e8557f) 62%, white);
    background: color-mix(in srgb, var(--accent, #e8557f) 18%, rgba(5,7,10,.9));
  }
  .workspace-focus-toggle:hover { transform: translateY(-1px); }
  .workspace-focus-toggle.active { color: #fff; }
  .workspace-focus-toggle kbd { display: none; }
  .workspace-focus-toggle.controller-active { bottom: calc(5.75rem + env(safe-area-inset-bottom)); }
  @media (max-width:640px) {
    .workspace-focus-toggle {
      right: max(10px, env(safe-area-inset-right));
      bottom: calc(4rem + env(safe-area-inset-bottom));
    }
    .workspace-focus-toggle.controller-active { bottom: calc(6rem + env(safe-area-inset-bottom)); }
  }
  @media (max-height:560px) and (min-width:641px) {
    .workspace-focus-toggle { right: 12px; bottom: 3.5rem; }
    .workspace-focus-toggle.controller-active { bottom: 5rem; }
  }
  @media (prefers-reduced-motion: reduce) {
    .workspace-focus-toggle { transition: none; }
  }
</style>
