<script lang="ts">
  import mascot from "../../assets/handheld/mascot-anime.png";
  import type { HandheldStateModel } from "./mediaTypes";

  interface Props extends HandheldStateModel {
    compact?: boolean;
  }

  let { state, title = "暂无内容", description = "选择一个内容开始探索。", primaryAction, secondaryAction, compact = false }: Props = $props();

  async function run(action: typeof primaryAction | typeof secondaryAction) {
    if (!action) return;
    await action.run();
  }
</script>

<section class:compact class="hh-state-panel hh-state-panel--{state}" data-state={state} aria-live={state === "error" ? "assertive" : "polite"}>
  <div class="hh-state-panel__art"><img src={mascot} alt="" /></div>
  <div class="hh-state-panel__copy">
    <span class="hh-state-panel__kicker">MOEPLAY / {state.toUpperCase()}</span>
    <h2>{title}</h2>
    <p>{description}</p>
  </div>
  {#if primaryAction || secondaryAction}
    <div class="hh-state-panel__actions">
      {#if primaryAction}<button class="hh-state-panel__action primary" type="button" onclick={() => void run(primaryAction)}>{primaryAction.label}</button>{/if}
      {#if secondaryAction}<button class="hh-state-panel__action" type="button" onclick={() => void run(secondaryAction)}>{secondaryAction.label}</button>{/if}
    </div>
  {/if}
</section>

<style>
  .hh-state-panel {
    display: grid;
    grid-template-columns: 112px minmax(0, 1fr) auto;
    align-items: center;
    gap: 22px;
    min-height: 190px;
    padding: 24px 28px;
    border: 1px solid color-mix(in srgb, var(--border, #fff) 18%, transparent);
    background: linear-gradient(115deg, color-mix(in srgb, var(--bg-card, #151820) 94%, transparent), color-mix(in srgb, var(--accent, #e8557f) 7%, transparent));
    color: var(--text-primary, #f8f8fa);
  }
  .hh-state-panel.compact { grid-template-columns: 72px minmax(0, 1fr); min-height: 110px; padding: 16px 18px; gap: 14px; }
  .hh-state-panel__art { display: grid; place-items: center; align-self: stretch; min-width: 0; }
  .hh-state-panel__art img { display: block; max-width: 112px; max-height: 142px; object-fit: contain; filter: drop-shadow(0 12px 20px rgb(0 0 0 / .26)); }
  .compact .hh-state-panel__art img { max-width: 72px; max-height: 84px; }
  .hh-state-panel__copy { min-width: 0; }
  .hh-state-panel__kicker { color: var(--accent, #e8557f); font: 700 10px/1 var(--font-mono, monospace); letter-spacing: .16em; }
  .hh-state-panel h2 { margin: 9px 0 7px; font: 800 clamp(1.2rem, 2.8vw, 2rem)/1.05 var(--font-display, system-ui); letter-spacing: -.035em; }
  .hh-state-panel p { max-width: 560px; margin: 0; color: var(--text-secondary, rgb(255 255 255 / .7)); font-size: .84rem; line-height: 1.65; }
  .hh-state-panel__actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 9px; }
  .hh-state-panel__action { min-height: 44px; padding: 0 16px; border: 1px solid var(--border, rgb(255 255 255 / .16)); background: var(--bg-elev, #202532); color: var(--text-primary, #fff); font: 700 .78rem/1 var(--font-ui, system-ui); cursor: pointer; }
  .hh-state-panel__action.primary { border-color: var(--accent, #e8557f); background: var(--accent, #e8557f); color: #fff; }
  .hh-state-panel__action:hover { border-color: var(--accent-hi, #ff769a); }
  .hh-state-panel__action:focus-visible { outline: 2px solid var(--accent-hi, #ff769a); outline-offset: 3px; }
  @media (max-width: 700px) {
    .hh-state-panel { grid-template-columns: 76px minmax(0, 1fr); padding: 18px; gap: 14px; }
    .hh-state-panel__art img { max-width: 76px; max-height: 100px; }
    .hh-state-panel__actions { grid-column: 2; justify-content: flex-start; }
  }
</style>
