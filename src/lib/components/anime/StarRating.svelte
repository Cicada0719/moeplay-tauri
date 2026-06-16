<script lang="ts">
  let { rating = 0, maxStars = 5, size = 14 }: { rating?: number; maxStars?: number; size?: number } = $props();
  
  const normalized = $derived(Math.max(0, Math.min(rating / 2, maxStars)));
  const fillPercent = $derived((normalized / maxStars) * 100);
</script>

<div class="star-rating" style="--star-size: {size}px; --fill: {fillPercent}%">
  <div class="stars-bg">{'★'.repeat(maxStars)}</div>
  <div class="stars-fill">{'★'.repeat(maxStars)}</div>
</div>

<style>
  .star-rating {
    position: relative;
    display: inline-flex;
    font-size: var(--star-size);
    line-height: 1;
  }
  .stars-bg {
    color: #374151;
    letter-spacing: 2px;
  }
  .stars-fill {
    position: absolute;
    top: 0;
    left: 0;
    overflow: hidden;
    width: var(--fill);
    color: #fbbf24;
    letter-spacing: 2px;
    white-space: nowrap;
  }
</style>
