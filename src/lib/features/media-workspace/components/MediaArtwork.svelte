<script lang="ts">
  interface Props {
    src?: string | null;
    alt: string;
    title: string;
    kicker?: string;
    eager?: boolean;
    class?: string;
  }

  let {
    src = null,
    alt,
    title,
    kicker = "MOEPLAY ARCHIVE",
    eager = false,
    class: className = "",
  }: Props = $props();

  let failed = $state(false);
  let lastSrc = $state<string | null>(null);

  $effect(() => {
    if (src !== lastSrc) {
      lastSrc = src;
      failed = false;
    }
  });
</script>

<div class={`mw-artwork ${className}`} data-has-media={Boolean(src) && !failed}>
  {#if src && !failed}
    <img
      {src}
      {alt}
      loading={eager ? "eager" : "lazy"}
      decoding="async"
      draggable="false"
      onerror={() => (failed = true)}
    />
  {:else}
    <div class="mw-artwork__fallback" aria-label={`${title} 暂无图片`}>
      <span>{kicker}</span>
      <strong>{title}</strong>
      <i aria-hidden="true"></i>
    </div>
  {/if}
</div>

