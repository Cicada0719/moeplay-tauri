<script lang="ts">
  let { count = 18 }: { count?: number } = $props();
  const petals = $derived(Array.from({ length: count }, (_, index) => ({
    left: (index * 37) % 100,
    delay: (index * 0.7) % 8,
    duration: 8 + (index % 5),
  })));
</script>

<div class="sakura-layer" aria-hidden="true">
  {#each petals as petal}
    <span
      style={`left:${petal.left}%; animation-delay:${petal.delay}s; animation-duration:${petal.duration}s`}
    ></span>
  {/each}
</div>

<style>
  .sakura-layer {
    position: fixed;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
    z-index: 0;
  }

  span {
    position: absolute;
    top: -24px;
    width: 10px;
    height: 14px;
    border-radius: 10px 10px 10px 2px;
    background: rgba(255, 183, 197, 0.55);
    animation: fall linear infinite;
  }

  @keyframes fall {
    to {
      transform: translate3d(38px, 110vh, 0) rotate(260deg);
      opacity: 0.15;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    span { animation: none; display: none; }
  }
  :global([data-motion="reduce"]) span { animation: none; display: none; }
</style>
