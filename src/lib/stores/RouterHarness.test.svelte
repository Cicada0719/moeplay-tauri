<script lang="ts">
  import { onMount } from "svelte";
  import { initRouter } from "./router.svelte";

  onMount(() => initRouter());
</script>

<div data-testid="router-harness"></div>
