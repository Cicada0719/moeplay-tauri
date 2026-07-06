<script lang="ts">
  import Button from "./Button.svelte";
  import Icon from "../Icon.svelte";

  let {
    icon,
    title,
    description,
    action,
    class: className = "",
  }: {
    icon?: string;
    title: string;
    description?: string;
    action?: { label: string; onclick: () => void };
    class?: string;
  } = $props();
</script>

<div class="ui-empty {className}" role="status">
  {#if icon}
    <div class="ui-empty__icon">
      <Icon name={icon} size={40} stroke={1.25} />
    </div>
  {/if}
  <h3 class="ui-empty__title">{title}</h3>
  {#if description}
    <p class="ui-empty__desc">{description}</p>
  {/if}
  {#if action}
    <div class="ui-empty__action">
      <Button variant="secondary" onclick={action.onclick}>
        {action.label}
      </Button>
    </div>
  {/if}
</div>
