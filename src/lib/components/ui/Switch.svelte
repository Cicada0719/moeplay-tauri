<script lang="ts">
  let {
    checked = $bindable(false),
    disabled = false,
    class: className = "",
    onchange,
  }: {
    checked?: boolean;
    disabled?: boolean;
    class?: string;
    onchange?: (e: Event) => void;
  } = $props();

  function handleChange(e: Event) {
    checked = (e.target as HTMLInputElement).checked;
    onchange?.(e);
  }
</script>

<label class="ui-switch {className}" class:is-disabled={disabled}>
  <input type="checkbox" role="switch" {checked} {disabled} onchange={handleChange} />
  <span class="ui-switch__track" aria-hidden="true">
    <span class="ui-switch__knob"></span>
  </span>
</label>

<style>
  .ui-switch {
    position: relative;
    display: inline-flex;
    width: 42px;
    height: 24px;
    flex-shrink: 0;
    cursor: pointer;
  }

  .ui-switch.is-disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .ui-switch input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .ui-switch__track {
    display: block;
    width: 42px;
    height: 24px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 24px;
    position: relative;
    transition:
      background 0.25s cubic-bezier(0.16, 1, 0.3, 1),
      border-color 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .ui-switch__knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-muted);
    transition:
      transform 0.25s cubic-bezier(0.16, 1, 0.3, 1),
      background 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .ui-switch input:checked ~ .ui-switch__track {
    background: var(--accent-lo);
    border-color: var(--accent-ring);
  }

  .ui-switch input:checked ~ .ui-switch__track .ui-switch__knob {
    transform: translateX(18px);
    background: var(--accent);
  }

  .ui-switch input:focus-visible ~ .ui-switch__track {
    box-shadow: 0 0 0 2px var(--accent-ring);
  }

  .ui-switch input:disabled ~ .ui-switch__track {
    opacity: 0.45;
  }
</style>
