<script lang="ts">
  import type { ActivityEventPatch, ActivityEventView, DurationQuality } from "../../features/activity";
  import { focusTrap } from "../../actions/a11y/focusTrap";

  let { event, saving = false, error = null, onCancel, onSave }: {
    event: ActivityEventView; saving?: boolean; error?: string | null; onCancel: () => void; onSave: (patch: ActivityEventPatch) => void | Promise<void>;
  } = $props();

  let startedAt = $state("");
  let endedAt = $state("");
  let durationSeconds = $state("");
  let durationQuality = $state<DurationQuality>("progress_only");
  let validationError = $state<string | null>(null);
  let initializedEventId = $state<string | null>(null);

  $effect.pre(() => {
    if (initializedEventId === event.id) return;
    initializedEventId = event.id;
    startedAt = toLocalDateTimeInput(event.startedAt);
    endedAt = event.endedAt ? toLocalDateTimeInput(event.endedAt) : "";
    durationSeconds = event.durationSeconds === null ? "" : String(event.durationSeconds);
    durationQuality = event.durationQuality;
    validationError = null;
  });
function toLocalDateTimeInput(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value.slice(0, 16);
    const pad = (part: number) => String(part).padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
  }

  function submit(formEvent: SubmitEvent) {
    formEvent.preventDefault(); validationError = null;
    const started = new Date(startedAt); const ended = endedAt ? new Date(endedAt) : null;
    if (Number.isNaN(started.getTime()) || (ended && Number.isNaN(ended.getTime()))) { validationError = "请输入有效的开始和结束时间。"; return; }
    const duration = durationSeconds.trim() === "" ? null : Number(durationSeconds);
    if (duration !== null && (!Number.isFinite(duration) || duration < 0)) { validationError = "时长必须是大于或等于 0 的数字。"; return; }
    void onSave({ startedAt: started.toISOString(), endedAt: ended?.toISOString() ?? null, durationQuality, durationSeconds: durationQuality === "progress_only" ? null : duration });
  }
</script>

<div class="activity-editor-backdrop" aria-hidden="true"></div>
<div class="activity-editor" role="dialog" aria-modal="true" aria-labelledby="activity-editor-title" aria-describedby="activity-editor-description" tabindex="-1" use:focusTrap={{ initialFocus: '[name="startedAt"]', returnFocus: true, closeOnEscape: true, onEscape: () => { if (!saving) onCancel(); } }}>
  <header><div><span>Activity v2</span><h2 id="activity-editor-title">编辑活动记录</h2></div><button type="button" aria-label="关闭编辑器" onclick={onCancel} disabled={saving}>×</button></header>
  <p id="activity-editor-description">{event.resourceKind} · {event.eventType} · {event.id}</p>
  <form onsubmit={submit}>
    <label>开始时间<input name="startedAt" type="datetime-local" bind:value={startedAt} required /></label>
    <label>结束时间<input name="endedAt" type="datetime-local" bind:value={endedAt} /></label>
    <label>时长质量<select name="durationQuality" bind:value={durationQuality}><option value="exact">精确</option><option value="estimated">估算</option><option value="progress_only">仅进度</option></select></label>
    <label>时长（秒）<input name="durationSeconds" type="number" min="0" step="1" bind:value={durationSeconds} disabled={durationQuality === "progress_only"} /></label>
    {#if validationError || error}<p class="activity-editor__error" role="alert">{validationError ?? error}</p>{/if}
    <div class="activity-editor__actions"><button type="button" class="secondary" onclick={onCancel} disabled={saving}>取消</button><button type="submit" class="primary" disabled={saving}>{saving ? "保存中…" : "保存"}</button></div>
  </form>
</div>

<style>
  .activity-editor-backdrop { position: fixed; inset: 0; z-index: 1200; background: rgba(0, 0, 0, .66); backdrop-filter: blur(5px); }
  .activity-editor { position: fixed; z-index: 1201; inset: 50% auto auto 50%; translate: -50% -50%; display: grid; gap: var(--v2-space-4); width: min(30rem, calc(100vw - 2rem)); max-height: calc(100vh - 2rem); overflow: auto; padding: var(--v2-space-5); border: 1px solid var(--v2-color-border-strong); border-radius: var(--v2-radius-xl); background: var(--v2-color-surface); color: var(--v2-color-text); box-shadow: var(--v2-shadow-lg); outline: none; }
  header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--v2-space-4); } header span { color: var(--v2-color-accent); font-size: var(--v2-text-xs); font-weight: 800; letter-spacing: .08em; text-transform: uppercase; } h2 { margin-top: var(--v2-space-1); font-size: var(--v2-text-xl); }
  header button { display: grid; place-items: center; width: 2.5rem; min-height: 2.5rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: transparent; color: inherit; font-size: 1.35rem; cursor: pointer; }
  p { margin: 0; color: var(--v2-color-text-secondary); overflow-wrap: anywhere; } form, label { display: grid; gap: var(--v2-space-2); } form { gap: var(--v2-space-4); } label { color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); font-weight: 700; }
  input, select { min-height: 2.75rem; padding: .65rem .75rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: var(--v2-color-surface-subtle); color: var(--v2-color-text); font: inherit; } input:focus-visible, select:focus-visible, button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .activity-editor__error { padding: var(--v2-space-3); border: 1px solid color-mix(in srgb, #ef6a7d 55%, var(--v2-color-border)); border-radius: var(--v2-radius-md); color: #ffb5c0; background: color-mix(in srgb, #5d1724 50%, transparent); }
  .activity-editor__actions { display: flex; justify-content: flex-end; gap: var(--v2-space-2); } .activity-editor__actions button { min-height: 2.75rem; padding: .6rem 1rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); color: inherit; font: inherit; font-weight: 800; cursor: pointer; } .activity-editor__actions .secondary { background: transparent; } .activity-editor__actions .primary { border-color: var(--v2-color-accent); background: var(--v2-color-accent); color: var(--v2-color-on-accent, #fff); } button:disabled { cursor: wait; opacity: .6; }
  @media (prefers-reduced-motion: reduce) { .activity-editor-backdrop, .activity-editor { transition: opacity 80ms linear; } } :global([data-motion="reduce"]) .activity-editor-backdrop, :global([data-motion="reduce"]) .activity-editor { transition: opacity 80ms linear; }
</style>




