<script lang="ts">
  import Button from "./Button.svelte";

  let {
    label = "保存",
    onclick,
    loading = false,
    disabled = false,
    ariaLabel,
    gamepadLabel,
    gamepadActivate,
    gamepadSecondaryAction = false,
  }: {
    label?: string;
    onclick?: (event: MouseEvent) => void;
    loading?: boolean;
    disabled?: boolean;
    ariaLabel?: string;
    gamepadLabel?: string;
    gamepadActivate?: string;
    gamepadSecondaryAction?: boolean;
  } = $props();
</script>

<Button {loading} {disabled} {onclick} {ariaLabel} {gamepadLabel} {gamepadActivate} {gamepadSecondaryAction}>{label}</Button>
