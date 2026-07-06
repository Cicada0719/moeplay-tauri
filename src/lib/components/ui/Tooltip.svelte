<script lang="ts">
  import type { Snippet } from "svelte";

  type TooltipPlacement = "top" | "bottom" | "left" | "right";

  let {
    content,
    children,
    placement = "top",
  }: {
    content: string;
    children: Snippet;
    placement?: TooltipPlacement;
  } = $props();
</script>

<span class="ui-tooltip ui-tooltip--{placement}">
  {@render children()}
  <span class="ui-tooltip__bubble" role="tooltip">{content}</span>
</span>
