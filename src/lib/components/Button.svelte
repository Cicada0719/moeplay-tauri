<script lang="ts">
  // 统一按钮：主=实色玫红、次=透明描边、ghost=纯文字；:active 下沉 1px；无渐变、无外发光。
  import type { Snippet } from "svelte";
  let {
    variant = "primary",
    disabled = false,
    title = "",
    onclick,
    children,
  }: {
    variant?: "primary" | "secondary" | "ghost";
    disabled?: boolean;
    title?: string;
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
  } = $props();
</script>

<button class="btn {variant}" {disabled} {title} {onclick}>
  {@render children?.()}
</button>

<style>
  .btn {
    font-family: var(--font-ui);
    font-weight: 600;
    font-size: 13px;
    padding: 9px 18px;
    border-radius: var(--radius-md);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 1px solid transparent;
    white-space: nowrap;
    transition: transform 0.12s ease, background 0.18s ease, border-color 0.18s ease, color 0.18s ease, opacity 0.18s ease;
  }
  .btn:active:not(:disabled) { transform: translateY(1px); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--accent-ring); }

  .btn.primary {
    background: var(--accent-pink);
    color: #fff;
  }
  .btn.primary:hover:not(:disabled) {
    background: var(--accent-pink-hi);
    box-shadow: var(--shadow-accent);
  }

  .btn.secondary { background: transparent; color: var(--text-primary); border-color: var(--border); }
  .btn.secondary:hover:not(:disabled) { border-color: var(--border-hover); background: var(--bg-hover); }

  .btn.ghost { background: transparent; color: var(--text-secondary); }
  .btn.ghost:hover:not(:disabled) { color: var(--text-primary); background: var(--bg-hover); }
</style>
