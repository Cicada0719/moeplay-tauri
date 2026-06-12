<script lang="ts">
  type SkeletonVariant = "text" | "block" | "card" | "stat" | "circle";

  let {
    variant = "text",
    count = 1,
    width,
    height,
    animated = true,
    class: className = "",
  }: {
    variant?: SkeletonVariant;
    count?: number;
    width?: string;
    height?: string;
    animated?: boolean;
    class?: string;
  } = $props();

  const items = $derived(Array.from({ length: Math.max(0, count) }));
  const style = $derived(
    [width ? `width: ${width}` : "", height ? `height: ${height}` : ""]
      .filter(Boolean)
      .join("; "),
  );
</script>

{#each items as _}
  <span
    class="ui-skeleton ui-skeleton--{variant} {animated ? 'is-animated' : ''} {className}"
    style={style}
    aria-hidden="true"
  ></span>
{/each}

<style>
  .ui-skeleton {
    display: block;
    min-width: 0;
    background: var(--bg-elev);
    border-radius: var(--radius-sm);
  }

  .ui-skeleton.is-animated {
    background:
      linear-gradient(
        90deg,
        var(--bg-elev) 0%,
        var(--bg-hover, rgba(255, 255, 255, 0.08)) 42%,
        var(--bg-elev) 82%
      )
      0 0 / 720px 100%;
    animation: ui-skeleton-shimmer 1.45s ease-in-out infinite;
  }

  .ui-skeleton--text {
    width: 100%;
    height: 14px;
    margin-bottom: 8px;
  }

  .ui-skeleton--block {
    width: 100%;
    min-height: 72px;
    border-radius: var(--radius-md);
  }

  .ui-skeleton--card {
    width: 100%;
    aspect-ratio: 3 / 4;
    border-radius: var(--radius-lg);
  }

  .ui-skeleton--stat {
    width: 100%;
    height: 92px;
    border-radius: var(--radius-lg);
  }

  .ui-skeleton--circle {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-full, 9999px);
  }

  @keyframes ui-skeleton-shimmer {
    from {
      background-position: -360px 0;
    }

    to {
      background-position: 360px 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ui-skeleton.is-animated {
      animation: none;
    }
  }
</style>
