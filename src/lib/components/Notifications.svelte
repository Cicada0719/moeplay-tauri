<script lang="ts">
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";

  const icons: Record<string, string> = { success: "check", error: "x", info: "lightbulb" };
</script>

{#if uiStore.notifications.length > 0}
  <div class="notifications">
    {#each uiStore.notifications as notification (notification.id)}
      <div class="notification {notification.type}">
        <span class="notif-icon">
          <Icon name={icons[notification.type] ?? "lightbulb"} size={15} />
        </span>
        <span class="notif-msg">{notification.message}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .notifications {
    position: fixed;
    top: 20px;
    right: 20px;
    z-index: 2000;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .notification {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 20px;
    border-radius: var(--radius-md);
    font-size: 14px;
    backdrop-filter: blur(12px);
    animation: slideIn 0.3s cubic-bezier(0.22,1,0.36,1);
    max-width: 360px;
  }

  .notification.success {
    background: rgba(34,197,94,0.15);
    border: 1px solid rgba(34,197,94,0.28);
    color: #86efac;
  }

  .notification.error {
    background: rgba(239,68,68,0.15);
    border: 1px solid rgba(239,68,68,0.28);
    color: #fca5a5;
  }

  .notification.info {
    background: rgba(59,130,246,0.15);
    border: 1px solid rgba(59,130,246,0.28);
    color: #93c5fd;
  }

  .notif-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  @keyframes slideIn {
    from { transform: translateX(40px); opacity: 0; }
    to   { transform: translateX(0); opacity: 1; }
  }
</style>
