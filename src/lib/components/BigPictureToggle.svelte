<script lang="ts">
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";
</script>

<button
  class="bp-toggle"
  onclick={() => uiStore.setBigPicture(true)}
  title="进入大屏模式"
  aria-label="进入大屏模式"
>
  <Icon name="gamepad" size={16} />
  <span class="bp-toggle-label">大屏模式</span>
</button>

<style>
  .bp-toggle {
    position: fixed;
    left: 18px;
    bottom: 58px;
    z-index: 90;
    display: inline-flex; align-items: center; gap: 8px;
    padding: 9px 14px;
    border-radius: 999px;
    background: rgba(20, 31, 49, 0.78);
    color: var(--text-primary);
    border: 1px solid var(--border);
    box-shadow: 0 8px 28px -8px rgba(0, 0, 0, .6);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 12px; font-weight: 700;
    transition: transform .2s cubic-bezier(.22,1,.36,1), background .2s ease, border-color .2s ease, color .2s ease;
  }
  .bp-toggle:hover {
    transform: translateY(-2px);
    border-color: var(--accent-ring);
    color: var(--accent);
  }
  .bp-toggle-label { white-space: nowrap; }
</style>
