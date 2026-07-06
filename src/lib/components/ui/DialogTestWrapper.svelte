<script lang="ts">
  import Dialog from "./Dialog.svelte";

  let {
    open = false,
    onClose,
    title = "标题",
  }: {
    open?: boolean;
    onClose: () => void;
    title?: string;
  } = $props();
</script>

<Dialog {open} {onClose} {title}>
  <p>内容</p>
  <button type="button">内部按钮</button>
</Dialog>
