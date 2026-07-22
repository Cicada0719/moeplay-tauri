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
  <Icon name={active ? "maximize" : "shrink"} size={14} />
  <span>{active ? "退出专注" : "专注模式"}</span>
  {#if controllerActive}<kbd>VIEW</kbd>{/if}
</button>

<style>
  .workspace-focus-toggle {
    position: fixed;
    right: 18px;
    bottom: 60px;
    z-index: 144;
    min-height: 34px;
    display:inline-flex;
    align-items:center;
    gap:7px;
    padding:0 11px;
    max-width:calc(100vw - 36px);
    border:1px solid rgba(255,255,255,.2);
    border-radius:999px;
    background:rgba(5,7,10,.82);
    color:rgba(255,255,255,.74);
    font:700 10px/1 var(--font-ui, system-ui);
    letter-spacing:.04em;
    cursor:pointer;
    backdrop-filter:blur(16px);
    white-space:nowrap;
    box-shadow:0 8px 28px rgba(0,0,0,.3);
  }
  .workspace-focus-toggle:hover, .workspace-focus-toggle:focus-visible, .workspace-focus-toggle.active { color:#fff; border-color:color-mix(in srgb, var(--accent, #e8557f) 62%, white); background:color-mix(in srgb, var(--accent, #e8557f) 18%, rgba(5,7,10,.9)); }
  .workspace-focus-toggle.active { min-height:38px; border-width:2px; color:#fff; }
  .workspace-focus-toggle kbd { min-width:31px; height:19px; display:grid; place-items:center; padding:0 4px; border:1px solid rgba(255,255,255,.25); border-radius:999px; font:800 7px/1 var(--font-mono, monospace); }
  .workspace-focus-toggle.controller-active { bottom:68px; }
  @media (max-width:640px) {
    .workspace-focus-toggle { right:8px; bottom:12px; max-width:calc(100vw - 16px); }
    .workspace-focus-toggle.controller-active { bottom:58px; }
  }
  @media (max-height:560px) and (min-width:641px) {
    .workspace-focus-toggle { right:10px; bottom:10px; }
    .workspace-focus-toggle.controller-active { bottom:58px; }
  }
</style>
