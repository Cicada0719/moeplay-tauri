<script lang="ts">
  type SegmentOption = { value: string; label: string };
  type SegmentSize = "sm" | "md";

  let {
    options,
    value,
    onChange,
    size = "md",
    class: className = "",
  }: {
    options: SegmentOption[];
    value: string;
    onChange: (value: string) => void;
    size?: SegmentSize;
    class?: string;
  } = $props();
</script>

<div
  class="ui-segment ui-segment--{size} {className}"
  role="radiogroup"
  aria-label="切换视图"
>
  {#each options as option (option.value)}
    <button
      class="ui-segment__option"
      class:is-selected={value === option.value}
      type="button"
      role="radio"
      aria-checked={value === option.value}
      onclick={() => onChange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>
