<script lang="ts">
  let { current, previous, fading, isCover }: { current: string; previous: string | null; fading: boolean; isCover: boolean } = $props();
</script>

<div class="bp-bg" aria-hidden="true">
  <div class="bp-color-field"></div>
  {#if previous}
    <div class="bp-art bp-art-prev bp-bg-layer-prev" class:fade-out={fading} class:is-cover={isCover} style={`--art:url("${previous}")`}></div>
  {/if}
  <div class="bp-art bp-art-current bp-bg-layer-current" class:fade-in={fading} class:is-cover={isCover} style={`--art:url("${current}")`}></div>
  <div class="bp-art-echo"></div>
  <div class="bp-cut bp-cut-a"></div>
  <div class="bp-cut bp-cut-b"></div>
  <div class="bp-grain"></div>
</div>
<div class="bp-scrim" aria-hidden="true"></div>

<style>
  .bp-bg { position:absolute; inset:0; z-index:0; overflow:hidden; background:#090a0d; }
  .bp-color-field {
    position:absolute;
    inset:0;
    background:
      radial-gradient(circle at 68% 42%,color-mix(in srgb,var(--scene-accent) 26%,transparent),transparent 48%),
      linear-gradient(128deg,#07080c 0 18%,color-mix(in srgb,var(--scene-accent) 13%,#090a0f) 62%,#08090d 100%);
  }
  .bp-art {
    position:absolute;
    inset:-3%;
    background-image:var(--art);
    background-size:cover;
    background-position:center;
    background-repeat:no-repeat;
    filter:saturate(.82) contrast(1.07) brightness(.84);
    transform:scale(1.045);
    will-change:opacity,transform;
  }
  .bp-art.is-cover {
    inset:-7%;
    background-size:cover;
    background-position:center 32%;
    filter:blur(9px) saturate(.84) contrast(1.08) brightness(.72);
    transform:scale(1.14);
  }
  .bp-art::after {
    content:"";
    position:absolute;
    inset:0;
    background:
      linear-gradient(90deg,rgba(5,6,9,.48),transparent 48%,rgba(5,6,9,.14)),
      linear-gradient(180deg,rgba(5,6,9,.36),transparent 28%,transparent 58%,rgba(5,6,9,.62));
  }
  .bp-art-current.fade-in { animation:sceneIn .7s cubic-bezier(.18,.86,.24,1) both; }
  .bp-art-prev.fade-out { animation:sceneOut .7s cubic-bezier(.18,.86,.24,1) both; }
  @keyframes sceneIn { from { opacity:0; transform:translateX(3%) scale(1.09); } to { opacity:1; transform:translateX(0) scale(1.045); } }
  @keyframes sceneOut { from { opacity:1; transform:translateX(0) scale(1.045); } to { opacity:0; transform:translateX(-3%) scale(1.025); } }
  .bp-art.is-cover.fade-in { animation-name:sceneCoverIn; }
  .bp-art.is-cover.fade-out { animation-name:sceneCoverOut; }
  @keyframes sceneCoverIn { from { opacity:0; transform:translateX(3%) scale(1.18); } to { opacity:1; transform:translateX(0) scale(1.14); } }
  @keyframes sceneCoverOut { from { opacity:1; transform:translateX(0) scale(1.14); } to { opacity:0; transform:translateX(-3%) scale(1.11); } }

  .bp-art-echo {
    position:absolute;
    left:var(--bp-safe-x,48px);
    right:var(--bp-safe-x,48px);
    bottom:clamp(54px,6.8vh,82px);
    height:1px;
    background:linear-gradient(90deg,var(--scene-accent),rgba(255,255,255,.12) 18%,transparent 60%);
    opacity:.55;
  }
  .bp-cut { position:absolute; pointer-events:none; mix-blend-mode:screen; }
  .bp-cut-a { right:5%; top:14%; width:clamp(70px,8vw,150px); height:2px; background:var(--scene-accent); box-shadow:0 0 28px var(--scene-accent); }
  .bp-cut-b { right:8%; top:14%; width:1px; height:clamp(72px,12vh,160px); background:rgba(255,255,255,.3); }
  .bp-grain {
    position:absolute;
    inset:-50%;
    opacity:.045;
    pointer-events:none;
    background-image:url("data:image/svg+xml,%3Csvg viewBox='0 0 180 180' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='.8'/%3E%3C/svg%3E");
    animation:grainShift .24s steps(2) infinite;
  }
  @keyframes grainShift { 0%{transform:translate(0,0)} 25%{transform:translate(2%,-1%)} 50%{transform:translate(-1%,2%)} 75%{transform:translate(1%,1%)} }
  .bp-scrim {
    position:absolute;
    inset:0;
    z-index:1;
    pointer-events:none;
    background:
      radial-gradient(ellipse at 64% 58%,transparent 12%,rgba(5,6,9,.12) 70%),
      linear-gradient(90deg,rgba(5,6,9,.82) 0,rgba(5,6,9,.58) 27%,rgba(5,6,9,.1) 58%,rgba(5,6,9,.22) 100%),
      linear-gradient(180deg,rgba(5,6,9,.55) 0,transparent 24%,transparent 58%,rgba(5,6,9,.78) 100%);
  }

  @media (max-width:1180px) {
    .bp-art { background-position:58% center; }
    .bp-scrim { background:linear-gradient(90deg,rgba(5,6,9,.86),rgba(5,6,9,.48) 38%,rgba(5,6,9,.16) 72%),linear-gradient(180deg,rgba(5,6,9,.52),transparent 23%,transparent 55%,rgba(5,6,9,.8)); }
  }
  @media (prefers-reduced-motion:reduce) { .bp-art-current.fade-in,.bp-art-prev.fade-out,.bp-grain { animation:none; } }
  :global([data-motion="reduce"]) .bp-art-current.fade-in,
  :global([data-motion="reduce"]) .bp-art-prev.fade-out,
  :global([data-motion="reduce"]) .bp-grain { animation:none; }
</style>
