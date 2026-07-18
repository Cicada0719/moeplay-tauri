<script lang="ts">
  let { variant = "text", count = 1, className = "" } = $props();
</script>

{#if variant === "card"}
  {#each Array(count) as _}
    <div class="skeleton skeleton-card {className}">
      <div class="skeleton-img"></div>
      <div class="skeleton-body">
        <div class="skeleton skeleton-text w-80"></div>
        <div class="skeleton skeleton-text w-50"></div>
        <div class="skeleton skeleton-text w-30"></div>
      </div>
    </div>
  {/each}
{:else if variant === "detail"}
  <div class="skeleton-detail {className}">
    <div class="skeleton skeleton-img detail-cover"></div>
    <div class="skeleton skeleton-text w-90"></div>
    <div class="skeleton skeleton-text w-70"></div>
    <div class="skeleton skeleton-text w-100"></div>
    <div class="skeleton skeleton-text w-60"></div>
    <div class="skeleton skeleton-text w-40"></div>
    <div class="skeleton-row">
      <div class="skeleton skeleton-chip"></div>
      <div class="skeleton skeleton-chip"></div>
      <div class="skeleton skeleton-chip"></div>
    </div>
  </div>
{:else if variant === "stats"}
  {#each Array(count) as _}
    <div class="skeleton skeleton-stat {className}">
      <div class="skeleton skeleton-text w-40 sm"></div>
      <div class="skeleton skeleton-text w-80 lg"></div>
    </div>
  {/each}
{:else}
  {#each Array(count) as _}
    <div class="skeleton skeleton-text {className}"></div>
  {/each}
{/if}

<style>
  .skeleton {
    background: linear-gradient(90deg, var(--bg-elev) 0%, var(--bg-hover) 40%, var(--bg-elev) 80%) 0 0 / 800px 100%;
    animation: shimmer 1.6s ease-in-out infinite;
    border-radius: var(--radius-sm);
  }
  .skeleton-text { height: 14px; margin-bottom: 8px; }
  .skeleton-text.sm { height: 11px; }
  .skeleton-text.lg { height: 24px; margin-bottom: 12px; }
  .w-30 { width: 30%; }
  .w-40 { width: 40%; }
  .w-50 { width: 50%; }
  .w-60 { width: 60%; }
  .w-70 { width: 70%; }
  .w-80 { width: 80%; }
  .w-90 { width: 90%; }
  .w-100 { width: 100%; }
  .skeleton-card {
    border-radius: var(--radius-lg);
    overflow: hidden;
    padding: 0;
    background: var(--bg-elev);
  }
  .skeleton-img {
    width: 100%;
    aspect-ratio: 3 / 4;
    background: linear-gradient(90deg, var(--bg-elev) 0%, var(--bg-hover) 40%, var(--bg-elev) 80%) 0 0 / 800px 100%;
    animation: shimmer 1.6s ease-in-out infinite;
  }
  .skeleton-body { padding: 12px; }
  .skeleton-row { display: flex; gap: 8px; flex-wrap: wrap; }
  .skeleton-chip { width: 60px; height: 24px; border-radius: var(--radius-full);
    background: linear-gradient(90deg, var(--bg-elev) 0%, var(--bg-hover) 40%, var(--bg-elev) 80%) 0 0 / 800px 100%;
    animation: shimmer 1.6s ease-in-out infinite; }
  .skeleton-stat { padding: 20px; border-radius: var(--radius-lg); background: var(--bg-card); }
  .skeleton-detail { display: flex; flex-direction: column; gap: 12px; padding: 20px; }
  .detail-cover { width: 100%; aspect-ratio: 16 / 9; border-radius: var(--radius-lg); }

  @keyframes shimmer {
    0%   { background-position: -400px 0; }
    100% { background-position: 400px 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .skeleton, .skeleton-img, .skeleton-chip { animation: none; }
  }
  :global([data-motion="reduce"]) .skeleton,
  :global([data-motion="reduce"]) .skeleton-img,
  :global([data-motion="reduce"]) .skeleton-chip { animation: none; }
</style>
