<script lang="ts">
  import { cacheThumbnail } from "../api";
  import { fileSrc } from "../utils";

  let {
    source,
    cacheKey,
    alt = "",
    loading = "lazy",
  }: {
    source: string | null | undefined;
    cacheKey: string;
    alt?: string;
    loading?: "eager" | "lazy";
  } = $props();

  let cachedSrc = $state<string | null>(null);
  let failed = $state(false);
  let requestSeq = 0;

  const fallbackSrc = $derived(fileSrc(source));
  const displaySrc = $derived(!failed && cachedSrc ? cachedSrc : fallbackSrc);
  const resolvedKey = $derived(source ? `${cacheKey}:${source}` : cacheKey);

  $effect(() => {
    const raw = source;
    const key = resolvedKey;
    const seq = ++requestSeq;

    if (!raw) {
      cachedSrc = null;
      failed = false;
      return;
    }

    cachedSrc = null;
    failed = false;

    void (async () => {
      try {
        const info = await cacheThumbnail(key, raw);
        if (seq === requestSeq) cachedSrc = fileSrc(info.path);
      } catch {
        if (seq === requestSeq) cachedSrc = null;
      }
    })();
  });
</script>

{#if displaySrc}
  <img
    class="cached-image"
    src={displaySrc}
    {alt}
    {loading}
    decoding="async"
    onerror={() => {
      failed = true;
    }}
  />
{/if}

<style>
  .cached-image {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
  }
</style>
