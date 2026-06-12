<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open = false,
    title = "",
    onClose = () => {},
    children,
  }: {
    open?: boolean;
    title?: string;
    onClose?: () => void;
    children?: Snippet;
  } = $props();
</script>

{#if open}
  <div class="overlay">
    <button class="backdrop" aria-label="关闭弹窗" onclick={onClose}></button>
    <div class="modal" role="dialog" aria-modal="true" aria-label={title}>
      <header>
        <h2>{title}</h2>
        <button aria-label="关闭" onclick={onClose}>×</button>
      </header>
      <div class="body">
        {@render children?.()}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(12px);
  }

  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border: 0;
    border-radius: 0;
    background: transparent;
    cursor: default;
  }

  .modal {
    position: relative;
    z-index: 1;
    width: min(680px, 100%);
    max-height: min(760px, 90vh);
    overflow: hidden;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--bg-elev);
    box-shadow: var(--shadow-lg);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  h2 {
    font-size: 17px;
  }

  header button {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
    color: var(--color-text);
    cursor: pointer;
  }

  .body {
    padding: 20px;
    overflow: auto;
    max-height: calc(90vh - 72px);
  }
</style>
