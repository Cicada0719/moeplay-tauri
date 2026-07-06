<script lang="ts">
  import Button from "./Button.svelte";

  let {
    label = "保存",
    onclick,
    loading = false,
    disabled = false,
    ariaLabel,
  }: {
    label?: string;
    onclick?: (event: MouseEvent) => void;
    loading?: boolean;
    disabled?: boolean;
    ariaLabel?: string;
  } = $props();
</script>

<Button {loading} {disabled} {onclick} {ariaLabel}>{label}</Button>
