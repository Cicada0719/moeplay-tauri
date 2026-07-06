<script lang="ts">
  let {
    onClose,
    ariaLabel = "关闭",
  }: {
    onClose: () => void;
    ariaLabel?: string;
  } = $props();
</script>

<button type="button" class="ui-overlay" aria-label={ariaLabel} onclick={onClose}></button>

<style>
  .ui-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(6px);
    border: none;
    padding: 0;
    margin: 0;
    cursor: default;
    pointer-events: auto;
  }

  @media (prefers-reduced-motion: reduce) {
    .ui-overlay {
      transition: none;
    }
  }
</style>
