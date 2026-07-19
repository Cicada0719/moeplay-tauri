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
  aria-label={active ? "退出专注布局并显示全部控件" : "进入专注布局并隐藏辅助控件"}
  data-gamepad-label={active ? "显示全部控件" : "隐藏辅助控件"}
  data-gamepad-activate={active ? "显示控件" : "专注布局"}
  data-gamepad-skip="true"
  data-workspace-focus-toggle
  data-view={view}
  onclick={onToggle}
>
  <Icon name={active ? "maximize" : "shrink"} size={14} />
  <span>{active ? "显示控件" : "专注布局"}</span>
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
    border:1px solid rgba(255,255,255,.2);
    border-radius:999px;
    background:rgba(5,7,10,.82);
    color:rgba(255,255,255,.74);
    font:700 10px/1 var(--font-ui, system-ui);
    letter-spacing:.04em;
    cursor:pointer;
    backdrop-filter:blur(16px);
  }
  .workspace-focus-toggle:hover, .workspace-focus-toggle:focus-visible, .workspace-focus-toggle.active { color:#fff; border-color:color-mix(in srgb, var(--accent, #e8557f) 62%, white); background:color-mix(in srgb, var(--accent, #e8557f) 18%, rgba(5,7,10,.9)); }
  .workspace-focus-toggle kbd { min-width:31px; height:19px; display:grid; place-items:center; padding:0 4px; border:1px solid rgba(255,255,255,.25); border-radius:999px; font:800 7px/1 var(--font-mono, monospace); }
  .workspace-focus-toggle.controller-active { bottom:68px; }
  @media (max-width:640px) { .workspace-focus-toggle { right:8px; bottom:54px; } .workspace-focus-toggle.controller-active { bottom:58px; } }
</style>
