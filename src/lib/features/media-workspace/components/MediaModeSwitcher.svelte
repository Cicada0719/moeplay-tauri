<script lang="ts">
  import "../styles/media-workspace.css";
  import type { MediaWorkspaceMode } from "./types";

  interface Props {
    mode: MediaWorkspaceMode;
    onModeChange?: (mode: MediaWorkspaceMode) => void;
    disabledModes?: Partial<Record<MediaWorkspaceMode, boolean>>;
    compact?: boolean;
    label?: string;
  }

  let {
    mode,
    onModeChange,
    disabledModes = {},
    compact = false,
    label = "游戏库浏览方式",
  }: Props = $props();

  const modes: Array<{ id: MediaWorkspaceMode; label: string; hint: string }> = [
    { id: "visual", label: "立方舞台", hint: "双面媒体档案" },
    { id: "index", label: "完整索引", hint: "传统游戏库" },
    { id: "scene", label: "影像序列", hint: "连续游戏胶片" },
  ];

  function moveFocus(event: KeyboardEvent, current: number) {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const enabled = modes.filter((item) => !disabledModes[item.id]);
    const activeIndex = enabled.findIndex((item) => item.id === modes[current].id);
    const next = event.key === "Home"
      ? 0
      : event.key === "End"
        ? enabled.length - 1
        : (activeIndex + (event.key === "ArrowRight" ? 1 : -1) + enabled.length) % enabled.length;
    const nextMode = enabled[next];
    if (!nextMode) return;
    onModeChange?.(nextMode.id);
    requestAnimationFrame(() => {
      document.querySelector<HTMLButtonElement>(`[data-media-mode="${nextMode.id}"]`)?.focus();
    });
  }
</script>

<div class:mw-mode-switcher--compact={compact} class="mw-mode-switcher" role="group" aria-label={label}>
  <span class="mw-mode-switcher__rail" aria-hidden="true"></span>
  {#each modes as item, index}
    <button
      type="button"
      class:active={mode === item.id}
      aria-pressed={mode === item.id}
      disabled={disabledModes[item.id]}
      data-media-mode={item.id}
      onclick={() => onModeChange?.(item.id)}
      onkeydown={(event) => moveFocus(event, index)}
    >
      <span class="mw-mode-switcher__number">0{index + 1}</span>
      <span class="mw-mode-switcher__copy">
        <strong>{item.label}</strong>
        {#if !compact}<small>{item.hint}</small>{/if}
      </span>
    </button>
  {/each}
</div>
