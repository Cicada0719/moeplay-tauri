<script lang="ts">
  let {
    current,
    previous,
    fading,
    isCover,
  }: {
    current: string;
    previous: string | null;
    fading: boolean;
    isCover: boolean;
  } = $props();
</script>

<div class="bp-bg">
  {#if previous}
    <div class="bp-bg-layer bp-bg-layer-prev" class:fade-out={fading} class:is-cover={isCover} style={`background-image: url("${previous}")`}></div>
  {/if}
  <div class="bp-bg-layer bp-bg-layer-current" class:fade-in={fading} class:is-cover={isCover} style={`background-image: url("${current}")`}></div>
</div>
<div class="bp-scrim"></div>

<style>
  .bp-bg { position: absolute; inset: 0; z-index: 0; }
  .bp-bg-layer {
    position: absolute; inset: 0;
    background-size: cover;
    background-position: center center;
    background-repeat: no-repeat;
    background-color: var(--bg-void);
    opacity: 1;
    will-change: opacity;
  }
  .bp-bg-layer.is-cover { background-size: contain; }
  .bp-bg-layer-current.fade-in { animation: bpBgIn 0.6s cubic-bezier(0.45, 0, 0.2, 1) both; }
  .bp-bg-layer-prev.fade-out { animation: bpBgOut 0.6s cubic-bezier(0.45, 0, 0.2, 1) both; }
  @keyframes bpBgIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes bpBgOut { from { opacity: 1; } to { opacity: 0; } }

  .bp-scrim {
    position: absolute; inset: 0; z-index: 1; pointer-events: none;
    background:
      linear-gradient(90deg, rgba(7,9,15,0.50) 0%, rgba(7,9,15,0.22) 18%, transparent 50%, transparent 100%),
      linear-gradient(180deg, rgba(7,9,15,0.18) 0%, transparent 30%, rgba(7,9,15,0.35) 80%, var(--bg-void) 100%);
  }
</style>
