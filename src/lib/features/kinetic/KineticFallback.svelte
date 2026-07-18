<!--
  Kinetic 舞台 CSS/2D 兜底版（概念站 MediaStageFallback 的生产硬化移植）。
  纯 token 渐变 + transform/opacity 缓动；reduced-motion 双写
  （media query + [data-motion="reduce"]），data-quality="reduced" 时静止。
-->
<script lang="ts">
  import type { KineticFallbackReason, KineticQuality } from "./types";

  let {
    quality = "low",
    reason = "init",
    animated = true,
  }: {
    quality?: KineticQuality | "reduced";
    reason?: KineticFallbackReason;
    animated?: boolean;
  } = $props();

  const still = $derived(!animated || quality === "reduced");
</script>

<div
  class="kinetic-fallback"
  data-quality={quality}
  data-reason={reason}
  data-animated={String(!still)}
  aria-hidden="true"
>
  <div class="kinetic-fallback__layer kinetic-fallback__layer--far"></div>
  <div class="kinetic-fallback__layer kinetic-fallback__layer--near"></div>
  <div class="kinetic-fallback__vignette"></div>
</div>

<style>
  .kinetic-fallback {
    position: absolute;
    inset: 0;
    overflow: hidden;
    isolation: isolate;
    background: var(--bg-deep, var(--bg-base, #05070a));
    pointer-events: none;
  }

  .kinetic-fallback__layer,
  .kinetic-fallback__vignette {
    position: absolute;
    inset: -14%;
    pointer-events: none;
  }

  .kinetic-fallback__layer--far {
    background:
      radial-gradient(58% 72% at 24% 68%, color-mix(in srgb, var(--bg-elev, #202634) 62%, transparent), transparent 70%),
      linear-gradient(112deg, transparent 30%, color-mix(in srgb, var(--bg-elev, #202634) 42%, transparent) 52%, transparent 74%);
    opacity: 0.85;
    animation: kinetic-fallback-drift-far 34s ease-in-out infinite alternate;
    will-change: transform;
  }

  .kinetic-fallback__layer--near {
    background:
      radial-gradient(44% 56% at 72% 30%, color-mix(in srgb, var(--accent, #e8557f) 24%, transparent), transparent 72%),
      linear-gradient(66deg, transparent 42%, color-mix(in srgb, var(--accent, #e8557f) 14%, transparent) 58%, transparent 76%);
    opacity: 0.8;
    animation: kinetic-fallback-drift-near 22s ease-in-out infinite alternate;
    will-change: transform;
  }

  .kinetic-fallback__vignette {
    inset: 0;
    background: radial-gradient(120% 90% at 50% 46%, transparent 52%, rgb(0 0 0 / 0.42) 100%);
  }

  .kinetic-fallback[data-animated="false"] .kinetic-fallback__layer {
    animation: none;
    will-change: auto;
  }

  @keyframes kinetic-fallback-drift-far {
    from { transform: translate3d(-1.6%, -1%, 0) scale(1.02); }
    to { transform: translate3d(1.6%, 1%, 0) scale(1.07); }
  }

  @keyframes kinetic-fallback-drift-near {
    from { transform: translate3d(1.4%, 0.8%, 0) scale(1.04); }
    to { transform: translate3d(-1.4%, -0.8%, 0) scale(1.09); }
  }

  @media (prefers-reduced-motion: reduce) {
    .kinetic-fallback__layer {
      animation: none !important;
      transform: none !important;
      will-change: auto;
    }
  }

  :global([data-motion="reduce"]) .kinetic-fallback__layer {
    animation: none !important;
    transform: none !important;
    will-change: auto;
  }
</style>
