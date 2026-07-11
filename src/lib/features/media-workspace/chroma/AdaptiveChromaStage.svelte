<script lang="ts">
  import type { Snippet } from "svelte";
  import {
    blendPalette,
    fallbackPalette,
    paletteCssVariables,
  } from "../model/adaptiveChroma";
  import type {
    AdaptiveChromaPalette,
    AdaptiveChromaStrength,
  } from "../model/chromaTypes";
  import { loadAdaptiveChromaPalette } from "./imagePalette";

  interface Props {
    src?: string | null;
    strength?: AdaptiveChromaStrength;
    enabled?: boolean;
    themePalette?: AdaptiveChromaPalette;
    children?: Snippet;
    class?: string;
    style?: string;
  }

  let {
    src = null,
    strength = "balanced",
    enabled = true,
    themePalette = fallbackPalette(),
    children,
    class: className = "",
    style = "",
  }: Props = $props();

  let mediaPalette = $state<AdaptiveChromaPalette>(fallbackPalette());
  let loadState = $state<"idle" | "loading" | "ready" | "error">("idle");

  const sourceUrl = $derived(src?.trim() ?? "");
  const shouldLoad = $derived(enabled && strength !== "off" && Boolean(sourceUrl));
  const blendedPalette = $derived(
    shouldLoad
      ? blendPalette(themePalette, mediaPalette, strength)
      : blendPalette(themePalette, fallbackPalette(), "off"),
  );
  const variableStyle = $derived(
    Object.entries(paletteCssVariables(blendedPalette))
      .map(([name, value]) => `${name}: ${value}`)
      .join("; "),
  );
  const containerStyle = $derived([style.trim().replace(/;+$/, ""), variableStyle].filter(Boolean).join("; "));
  const displayState = $derived(!enabled || strength === "off" ? "disabled" : !sourceUrl ? "fallback" : loadState);

  $effect(() => {
    const url = sourceUrl;
    if (!shouldLoad) {
      mediaPalette = fallbackPalette();
      loadState = "idle";
      return;
    }

    let stale = false;
    mediaPalette = fallbackPalette();
    loadState = "loading";

    void loadAdaptiveChromaPalette(url).then(
      (palette) => {
        if (stale) return;
        mediaPalette = palette;
        loadState = palette.source === "media" ? "ready" : "error";
      },
      () => {
        if (stale) return;
        mediaPalette = fallbackPalette();
        loadState = "error";
      },
    );

    return () => {
      stale = true;
    };
  });
</script>

<div
  class={`adaptive-chroma-stage ${className}`.trim()}
  style={containerStyle}
  data-adaptive-chroma-state={displayState}
  data-adaptive-chroma-enabled={enabled && strength !== "off"}
>
  {@render children?.()}
</div>
