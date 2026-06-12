<script lang="ts">
  // 标签药丸：透明底+细边（默认）/ 玫红软底（选中）。无渐变。
  let { label = "", active = false }: { label?: string; active?: boolean } = $props();
</script>

<span class="pill" class:active>{label}</span>

<style>
  .pill {
    font-size: 11px;
    line-height: 1.6;
    padding: 2px 10px;
    border-radius: var(--radius-full);
    border: 1px solid var(--border);
    color: var(--text-muted);
    background: transparent;
    white-space: nowrap;
    transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
  }
  .pill.active {
    background: var(--accent-lo);
    border-color: transparent;
    color: var(--accent);
    font-weight: 600;
  }
</style>
