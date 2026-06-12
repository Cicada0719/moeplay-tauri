<script lang="ts">
  let {
    message,
    type = "info",
  }: {
    message: string;
    type?: "info" | "success" | "error";
  } = $props();
</script>

<div class="toast {type}">
  <span>{message}</span>
</div>

<style>
  .toast {
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.08);
    color: var(--color-text);
    backdrop-filter: blur(12px);
  }

  .success {
    border-color: rgba(139, 224, 208, 0.4);
  }

  .error {
    border-color: rgba(255, 120, 140, 0.48);
  }
</style>
