<script lang="ts">
  let {
    rows = 3,
    columns = 1,
    class: className = "",
  }: {
    rows?: number;
    columns?: number;
    class?: string;
  } = $props();

  const rowItems = $derived(Array.from({ length: Math.max(0, rows) }));
</script>

<div
  class="ui-skeleton-stack {className}"
  style="grid-template-columns: repeat({columns}, minmax(0, 1fr));"
  aria-hidden="true"
>
  {#each rowItems as _}
    <div class="skeleton ui-skeleton-stack__item"></div>
  {/each}
</div>
