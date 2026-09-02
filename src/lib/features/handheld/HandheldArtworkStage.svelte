<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HandheldArtworkRole, HandheldArtworkSource } from "./mediaTypes";
  import homeAmbient from "../../assets/handheld/home-ambient.webp";
  import animeAmbient from "../../assets/handheld/anime-ambient.webp";
  import comicAmbient from "../../assets/handheld/comic-ambient.webp";
  import novelAmbient from "../../assets/handheld/novel-ambient.webp";

  interface Props {
    source?: HandheldArtworkSource;
    role?: HandheldArtworkRole;
    children?: Snippet;
    class?: string;
    strength?: "soft" | "balanced" | "immersive";
    fill?: boolean;
  }

  let {
    source,
    role = "home",
    children,
    class: className = "",
    strength = "balanced",
    fill = false,
  }: Props = $props();

  const fallbackByRole: Record<HandheldArtworkRole, string> = {
    home: homeAmbient,
    anime: animeAmbient,
    comic: comicAmbient,
    novel: novelAmbient,
    empty: homeAmbient,
  };

  let failed = $state(false);
  const artworkRole = $derived(source?.role ?? role);
  const cover = $derived(source?.cover?.trim() ?? "");
  const fallback = $derived(source?.fallback ?? fallbackByRole[artworkRole]);
  const imageSrc = $derived(!failed && cover ? cover : fallback);
  const imageAlt = $derived(source?.alt ?? "");

  function handleError() {
    if (!failed && fallback && imageSrc !== fallback) {
      failed = true;
    }
  }

  $effect(() => {
    cover;
    failed = false;
  });
</script>

<div
  class={`hh-artwork-stage hh-artwork-stage--${strength} ${fill ? "hh-artwork-stage--fill" : ""} ${className}`.trim()}
  data-artwork-role={artworkRole}
  data-artwork-source={cover ? "cover" : "fallback"}
>
  <img class="hh-artwork-stage__image" src={imageSrc} alt={imageAlt} aria-hidden={imageAlt ? undefined : "true"} onerror={handleError} />
  <div class="hh-artwork-stage__wash" aria-hidden="true"></div>
  <div class="hh-artwork-stage__grain v2-grain" aria-hidden="true"></div>
  <div class="hh-artwork-stage__content">
    {@render children?.()}
  </div>
</div>

<style>
  .hh-artwork-stage {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    isolation: isolate;
    background: var(--hh-canvas, #08090c);
  }
  .hh-artwork-stage--fill { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }

  .hh-artwork-stage__image,
  .hh-artwork-stage__wash,
  .hh-artwork-stage__grain {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .hh-artwork-stage__image {
    z-index: -3;
    object-fit: cover;
    opacity: .46;
    filter: blur(18px) saturate(1.08);
    transform: scale(1.06);
  }

  .hh-artwork-stage--soft .hh-artwork-stage__image { opacity: .28; filter: blur(14px) saturate(1.04); }
  .hh-artwork-stage--immersive .hh-artwork-stage__image { opacity: .62; filter: blur(22px) saturate(1.14); }

  .hh-artwork-stage__wash {
    z-index: -2;
    background:
      linear-gradient(90deg, color-mix(in srgb, var(--hh-canvas, #08090c) 96%, transparent), color-mix(in srgb, var(--hh-canvas, #08090c) 48%, transparent) 52%, color-mix(in srgb, var(--hh-canvas, #08090c) 84%, transparent)),
      linear-gradient(180deg, color-mix(in srgb, var(--hh-canvas, #08090c) 48%, transparent), color-mix(in srgb, var(--hh-canvas, #08090c) 94%, transparent));
  }

  .hh-artwork-stage__grain { z-index: -1; }
  .hh-artwork-stage__content { position: relative; z-index: 0; min-width: 0; min-height: 0; height: 100%; }

  @media (prefers-reduced-motion: reduce) {
    .hh-artwork-stage__image { transform: none; }
  }
</style>
