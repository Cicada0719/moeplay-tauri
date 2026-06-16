<script lang="ts">
  import { cacheThumbnail } from "../api";
  import { fileSrc } from "../utils";

  let {
    source,
    cacheKey,
    alt = "",
    loading = "lazy",
    onfail,
  }: {
    source: string | null | undefined;
    cacheKey: string;
    alt?: string;
    loading?: "eager" | "lazy";
    /** 缓存图与原图都加载失败时触发，便于上层退回占位图（如磁贴首字母） */
    onfail?: () => void;
  } = $props();

  let cachedSrc = $state<string | null>(null);
  let failed = $state(false);          // 缓存缩略图加载失败 → 退回原图
  let fallbackFailed = $state(false);  // 原图也失败 → 彻底放弃
  let requestSeq = 0;

  const fallbackSrc = $derived(fileSrc(source));
  const displaySrc = $derived(
    fallbackFailed ? null : (!failed && cachedSrc ? cachedSrc : fallbackSrc),
  );
  const resolvedKey = $derived(source ? `${cacheKey}:${source}` : cacheKey);

  $effect(() => {
    const raw = source;
    const key = resolvedKey;
    const seq = ++requestSeq;

    if (!raw) {
      cachedSrc = null;
      failed = false;
      fallbackFailed = false;
      return;
    }

    cachedSrc = null;
    failed = false;
    fallbackFailed = false;

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
      // 先坏的是缩略图 → 退回原图重试；原图再坏 → 通知上层换占位
      if (!failed && cachedSrc) {
        failed = true;
      } else {
        fallbackFailed = true;
        onfail?.();
      }
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
