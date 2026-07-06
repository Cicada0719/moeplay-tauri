<script lang="ts">
  import Icon from "../Icon.svelte";

  let {
    value = $bindable(""),
    placeholder = "搜索...",
    shortcut,
    onsearch,
    onclear,
    class: className = "",
    ariaLabel = "搜索",
  }: {
    value?: string;
    placeholder?: string;
    shortcut?: string;
    onsearch?: (value: string) => void;
    onclear?: () => void;
    class?: string;
    ariaLabel?: string;
  } = $props();

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    value = target.value;
    onsearch?.(value);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      onsearch?.(value);
    }
  }

  function clear() {
    value = "";
    onclear?.();
    onsearch?.("");
  }
</script>

<div class="ui-search {className}">
  <span class="ui-search__icon" aria-hidden="true">
    <Icon name="search" size={16} />
  </span>
  <input
    class="ui-search__input"
    type="search"
    {value}
    {placeholder}
    aria-label={ariaLabel}
    oninput={handleInput}
    onkeydown={handleKeydown}
  />
  {#if value}
    <button
      class="ui-search__clear"
      type="button"
      aria-label="清空"
      onclick={clear}
    >
      <Icon name="x" size={14} />
    </button>
  {:else if shortcut}
    <kbd class="ui-search__shortcut">{shortcut}</kbd>
  {/if}
</div>
