<script lang="ts">
  // 评分环（设计稿签名元素）：玫红圆环 + 居中等宽数字。
  // GSAP：入场扫环 + 数字 count-up（power3.out），尊重 prefers-reduced-motion。
  import { onMount } from "svelte";
  import { gsap } from "gsap";

  let {
    value = 0,
    max = 10,
    size = 48,
  }: { value?: number; max?: number; size?: number } = $props();

  let ringEl: SVGCircleElement;
  let numEl: HTMLSpanElement;

  const stroke = $derived(Math.max(3, Math.round(size * 0.07)));
  const radius = $derived((size - stroke) / 2 - 1);
  const circ = $derived(2 * Math.PI * radius);
  const pct = $derived(Math.max(0, Math.min(1, max ? value / max : 0)));
  const fmt = (v: number) => (max <= 10 ? v.toFixed(1) : Math.round(v).toString());

  onMount(() => {
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
    if (reduce || !ringEl) return;
    const ctx = gsap.context(() => {
      gsap.fromTo(
        ringEl,
        { strokeDashoffset: circ },
        { strokeDashoffset: circ * (1 - pct), duration: 0.9, ease: "power3.out" }
      );
      const o = { v: 0 };
      gsap.to(o, {
        v: value,
        duration: 0.9,
        ease: "power3.out",
        onUpdate: () => { if (numEl) numEl.textContent = fmt(o.v); },
      });
    });
    return () => ctx.revert();
  });
</script>

<div class="ring" style="width:{size}px;height:{size}px">
  <svg width={size} height={size} viewBox="0 0 {size} {size}" aria-hidden="true">
    <circle cx={size / 2} cy={size / 2} r={radius} fill="none" stroke="var(--border)" stroke-width={stroke} />
    <circle
      bind:this={ringEl}
      cx={size / 2}
      cy={size / 2}
      r={radius}
      fill="none"
      stroke="var(--accent)"
      stroke-width={stroke}
      stroke-linecap="round"
      stroke-dasharray={circ}
      stroke-dashoffset={circ * (1 - pct)}
      transform="rotate(-90 {size / 2} {size / 2})"
    />
  </svg>
  <span class="num" bind:this={numEl} style="font-size:{Math.round(size * 0.28)}px">{fmt(value)}</span>
</div>

<style>
  .ring { position: relative; display: inline-grid; place-items: center; }
  .ring svg { position: absolute; inset: 0; }
  .num {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-weight: 700;
    color: var(--accent);
    line-height: 1;
  }
</style>
