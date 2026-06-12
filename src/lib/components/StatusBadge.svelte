<script lang="ts">
  // 状态角标：thin-line 图标 + 中文。游玩中带"呼吸"点。无 emoji。
  import Icon from "./Icon.svelte";
  let { status = "NotStarted" }: { status?: string } = $props();

  const MAP: Record<string, { label: string; icon: string; cls: string }> = {
    Playing: { label: "游玩中", icon: "play", cls: "running" },
    Completed: { label: "已通关", icon: "check", cls: "done" },
    OnHold: { label: "暂停", icon: "chevronDown", cls: "" },
    Dropped: { label: "已弃", icon: "x", cls: "" },
    PlanToPlay: { label: "计划玩", icon: "star", cls: "" },
  };
  const s = $derived(MAP[status]);
</script>

{#if s}
  <span class="badge {s.cls}">
    {#if s.cls === "running"}<span class="dot"></span>{/if}
    <Icon name={s.icon} size={11} />
    <span class="lbl">{s.label}</span>
  </span>
{/if}

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elev);
    color: var(--text-secondary);
  }
  .badge.running { color: var(--accent); border-color: var(--accent-lo); }
  .badge.done { color: var(--color-success); }
  .lbl { line-height: 1; }
  .dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    animation: pulse-dot 2s ease-in-out infinite;
  }
</style>
