<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";

  type InputType = "text" | "password" | "number" | "search" | "url";

  let {
    value = $bindable(""),
    type = "text",
    placeholder = "",
    class: className = "",
    ariaLabel,
    onblur,
    oninput,
    onkeydown,
    ...rest
  }: HTMLInputAttributes & {
    value?: string;
    type?: InputType;
    class?: string;
    ariaLabel?: string;
  } = $props();

  function handleInput(e: Event & { currentTarget: HTMLInputElement }) {
    value = e.currentTarget.value;
    oninput?.(e);
  }
</script>

<input
  class="ui-input {className}"
  {type}
  {value}
  {placeholder}
  aria-label={ariaLabel}
  oninput={handleInput}
  onblur={onblur}
  onkeydown={onkeydown}
  {...rest}
/>

<style>
  .ui-input {
    width: 100%;
    min-width: 0;
    padding: 10px 14px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: 13px;
    outline: none;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      box-shadow 0.18s ease;
  }

  .ui-input::placeholder {
    color: var(--text-muted);
  }

  .ui-input:focus-visible {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-ring);
  }

  .ui-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
