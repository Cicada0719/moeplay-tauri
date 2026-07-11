<script lang="ts">
  import { onMount } from "svelte";

  export let enabled = true;
  export let label = "VIEW";
  export let selector = "[data-concept-cursor]";

  let x = -100;
  let y = -100;
  let visible = false;
  let active = false;
  let currentLabel = label;

  onMount(() => {
    if (!enabled || matchMedia("(pointer: coarse)").matches) return;
    const onMove = (event: PointerEvent) => { x = event.clientX; y = event.clientY; visible = true; };
    const onOut = (event: PointerEvent) => { if (!event.relatedTarget) visible = false; };
    const onOver = (event: PointerEvent) => {
      const target = event.target instanceof Element ? event.target.closest<HTMLElement>(selector) : null;
      active = Boolean(target);
      currentLabel = target?.dataset.conceptCursor || label;
    };
    const onDown = () => { if (active) currentLabel = "HOLD"; };
    const onUp = (event: PointerEvent) => {
      const target = event.target instanceof Element ? event.target.closest<HTMLElement>(selector) : null;
      currentLabel = target?.dataset.conceptCursor || label;
    };
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("pointerout", onOut);
    document.addEventListener("pointerover", onOver, { passive: true });
    document.addEventListener("pointerdown", onDown, { passive: true });
    document.addEventListener("pointerup", onUp, { passive: true });
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerout", onOut);
      document.removeEventListener("pointerover", onOver);
      document.removeEventListener("pointerdown", onDown);
      document.removeEventListener("pointerup", onUp);
    };
  });
</script>

{#if enabled}
  <div class:visible class:active class="media-cursor" style={`transform:translate3d(${x}px,${y}px,0)`} aria-hidden="true">
    <span>{currentLabel}</span>
  </div>
{/if}

<style>
  .media-cursor{position:fixed;z-index:120;left:0;top:0;width:.6rem;height:.6rem;margin:-.3rem 0 0 -.3rem;border:1px solid currentColor;border-radius:50%;color:#fff;pointer-events:none;opacity:0;mix-blend-mode:difference;transition:width 180ms ease,height 180ms ease,margin 180ms ease,opacity 120ms ease,background 180ms ease;will-change:transform}.media-cursor.visible{opacity:1}.media-cursor.active{width:4.75rem;height:4.75rem;margin:-2.375rem 0 0 -2.375rem;background:#fff;color:#000}.media-cursor span{position:absolute;inset:0;display:grid;place-items:center;font-size:.58rem;font-weight:700;letter-spacing:.09em;opacity:0}.media-cursor.active span{opacity:1}
  @media(pointer:coarse),(prefers-reduced-motion:reduce){.media-cursor{display:none}}
</style>
