<script lang="ts">
  import { writeLibraryV2Flag } from "./feature-flag";

  let {
    enabled = false,
    onChange,
  }: {
    enabled?: boolean;
    onChange?: (enabled: boolean) => void;
  } = $props();

  function toggle(event: Event) {
    const next = (event.currentTarget as HTMLInputElement).checked;
    writeLibraryV2Flag(next);
    onChange?.(next);
  }
</script>

<label class="feature-toggle">
  <span class="copy">
    <strong>Library v2</strong>
    <small>{enabled ? "预览后确认写入" : "旧导入流程 fallback"}</small>
  </span>
  <input type="checkbox" role="switch" checked={enabled} aria-label="启用 Library v2" onchange={toggle} />
  <span class="track" aria-hidden="true"><span></span></span>
</label>

<style>
  .feature-toggle {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 7px 9px 7px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-elev);
    color: var(--text-primary);
    cursor: pointer;
  }
  .copy { min-width: 0; display: grid; gap: 2px; }
  strong { font: 700 12px/1.2 var(--font-ui); }
  small { color: var(--text-muted); font: 500 10px/1.2 var(--font-ui); white-space: nowrap; }
  input { position: absolute; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
  .track {
    position: relative;
    width: 34px;
    height: 20px;
    flex: 0 0 auto;
    border: 1px solid var(--border-hover);
    border-radius: 999px;
    background: color-mix(in srgb, var(--text-muted) 24%, transparent);
    transition: background .16s ease, border-color .16s ease;
  }
  .track span {
    position: absolute;
    width: 14px;
    height: 14px;
    left: 2px;
    top: 2px;
    border-radius: 50%;
    background: #fff;
    transition: transform .16s ease;
  }
  input:checked + .track { border-color: var(--accent-ring); background: var(--accent); }
  input:checked + .track span { transform: translateX(14px); }
  input:focus-visible + .track { box-shadow: var(--focus-ring); }
  @media (prefers-reduced-motion: reduce) {
    .track, .track span { transition: none; }
  }
</style>
