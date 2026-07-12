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
  let reducedMotion = $state(false);
  let highContrast = $state(false);

  const sourceUrl = $derived(src?.trim() ?? "");
  const shouldLoad = $derived(enabled && strength !== "off" && Boolean(sourceUrl));
  const blendedPalette = $derived(
    shouldLoad
      ? blendPalette(themePalette, mediaPalette, strength)
      : blendPalette(themePalette, fallbackPalette(), "off"),
  );
  const displayState = $derived(!enabled || strength === "off" ? "disabled" : !sourceUrl ? "fallback" : loadState);
  const contrastMode = $derived(highContrast ? "high" : "normal");

  function cssUrl(value: string): string {
    const escaped = value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/[\r\n]/g, " ");
    return `url("${escaped}")`;
  }


  const variableStyle = $derived.by(() => {
    const legacyVariables = paletteCssVariables(blendedPalette);
    const semanticVariables: Record<string, string> = {
      "--adaptive-chroma-source": sourceUrl ? cssUrl(sourceUrl) : "none",
      "--adaptive-chroma-background-image": sourceUrl && !highContrast ? cssUrl(sourceUrl) : "none",
      "--adaptive-chroma-primary-rgb": legacyVariables["--media-primary-rgb"],
      "--adaptive-chroma-secondary-rgb": legacyVariables["--media-secondary-rgb"],
      "--adaptive-chroma-accent-rgb": legacyVariables["--media-accent-rgb"],
      "--adaptive-chroma-surface-rgb": legacyVariables["--media-surface-rgb"],
      "--adaptive-chroma-foreground-rgb": legacyVariables["--media-foreground-rgb"],
      "--adaptive-chroma-on-accent-rgb": legacyVariables["--media-on-accent-rgb"],
      "--adaptive-chroma-strength": enabled ? strength : "off",
      "--adaptive-chroma-contrast-mode": contrastMode,
      "--adaptive-chroma-transition-duration": reducedMotion ? "0ms" : "180ms",
    };
    return Object.entries({ ...legacyVariables, ...semanticVariables })
      .map(([name, value]) => `${name}: ${value}`)
      .join("; ");
  });

  const containerStyle = $derived([
    style.trim().replace(/;+$/, ""),
    "background-image: var(--adaptive-chroma-background-image)",
    "background-size: cover",
    "background-position: center",
    "background-repeat: no-repeat",
    variableStyle,
  ].filter(Boolean).join("; "));

  $effect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const contrast = window.matchMedia("(prefers-contrast: more)");
    const forced = window.matchMedia("(forced-colors: active)");
    const sync = () => {
      reducedMotion = reduced.matches;
      highContrast = contrast.matches || forced.matches;
    };
    sync();
    reduced.addEventListener?.("change", sync);
    contrast.addEventListener?.("change", sync);
    forced.addEventListener?.("change", sync);
    return () => {
      reduced.removeEventListener?.("change", sync);
      contrast.removeEventListener?.("change", sync);
      forced.removeEventListener?.("change", sync);
    };
  });

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
  data-adaptive-chroma-contrast={contrastMode}
  data-adaptive-chroma-reduced-motion={reducedMotion}
>
  {@render children?.()}
</div>
