import type { AdaptiveChromaPalette, AdaptiveChromaStrength, RgbColor } from "./chromaTypes";

const FALLBACK_PALETTE: AdaptiveChromaPalette = {
  primary: { r: 203, g: 59, b: 94 },
  secondary: { r: 58, g: 92, b: 128 },
  accent: { r: 234, g: 109, b: 131 },
  surface: { r: 15, g: 17, b: 22 },
  foreground: { r: 248, g: 247, b: 244 },
  isDark: true,
  source: "fallback",
};

export interface PalettePixelSource {
  data: Uint8ClampedArray | number[];
  width: number;
  height: number;
}

interface WeightedColor extends RgbColor {
  weight: number;
  saturation: number;
  luminance: number;
}

function clampChannel(value: number): number {
  return Math.max(0, Math.min(255, Math.round(value)));
}

function relativeLuminance({ r, g, b }: RgbColor): number {
  const channels = [r, g, b].map((value) => {
    const normalized = value / 255;
    return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  });
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

function saturationOf({ r, g, b }: RgbColor): number {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  return max === 0 ? 0 : (max - min) / max;
}

function distance(a: RgbColor, b: RgbColor): number {
  const dr = a.r - b.r;
  const dg = a.g - b.g;
  const db = a.b - b.b;
  return Math.sqrt(dr * dr + dg * dg + db * db);
}

function mix(a: RgbColor, b: RgbColor, amount: number): RgbColor {
  const t = Math.max(0, Math.min(1, amount));
  return {
    r: clampChannel(a.r + (b.r - a.r) * t),
    g: clampChannel(a.g + (b.g - a.g) * t),
    b: clampChannel(a.b + (b.b - a.b) * t),
  };
}

function quantize(value: number): number {
  return Math.min(255, Math.round(value / 24) * 24);
}

function css(color: RgbColor): string {
  return `${color.r} ${color.g} ${color.b}`;
}

function strengthAmount(strength: AdaptiveChromaStrength): number {
  if (strength === "immersive") return 0.72;
  if (strength === "balanced") return 0.56;
  if (strength === "subtle") return 0.28;
  return 0;
}

export function extractPaletteFromPixels(source: PalettePixelSource): AdaptiveChromaPalette {
  const buckets = new Map<string, WeightedColor>();
  const pixels = Math.max(0, Math.min(source.width * source.height, Math.floor(source.data.length / 4)));
  const stride = Math.max(1, Math.floor(pixels / 4096));

  for (let index = 0; index < pixels; index += stride) {
    const offset = index * 4;
    const alpha = Number(source.data[offset + 3] ?? 255);
    if (alpha < 180) continue;
    const color = {
      r: Number(source.data[offset] ?? 0),
      g: Number(source.data[offset + 1] ?? 0),
      b: Number(source.data[offset + 2] ?? 0),
    };
    const luminance = relativeLuminance(color);
    const saturation = saturationOf(color);
    if (luminance < 0.012 || luminance > 0.94) continue;
    if (saturation < 0.06 && luminance > 0.12 && luminance < 0.78) continue;

    const key = `${quantize(color.r)}:${quantize(color.g)}:${quantize(color.b)}`;
    const current = buckets.get(key);
    const importance = 0.45 + saturation * 1.25 + (1 - Math.abs(luminance - 0.42)) * 0.35;
    if (current) {
      current.weight += importance;
    } else {
      buckets.set(key, { ...color, weight: importance, saturation, luminance });
    }
  }

  const candidates = [...buckets.values()].sort((a, b) => b.weight - a.weight);
  if (!candidates.length) return { ...FALLBACK_PALETTE };

  const primary = candidates[0];
  const secondary = candidates.find((item) => distance(item, primary) > 92) ?? candidates[1] ?? mix(primary, FALLBACK_PALETTE.secondary, 0.55);
  const accentCandidate = candidates
    .filter((item) => item.saturation >= 0.32 && item.luminance >= 0.08 && item.luminance <= 0.72)
    .sort((a, b) => (b.saturation * b.weight) - (a.saturation * a.weight))[0] ?? primary;
  const averageLuminance = candidates.slice(0, 8).reduce((sum, item) => sum + item.luminance * item.weight, 0)
    / candidates.slice(0, 8).reduce((sum, item) => sum + item.weight, 0);
  const isDark = averageLuminance < 0.44;
  const surfaceBase = isDark ? { r: 8, g: 10, b: 14 } : { r: 241, g: 239, b: 234 };

  return {
    primary: mix(primary, isDark ? { r: 178, g: 178, b: 178 } : { r: 86, g: 86, b: 86 }, 0.12),
    secondary: mix(secondary, surfaceBase, 0.18),
    accent: mix(accentCandidate, isDark ? { r: 255, g: 255, b: 255 } : { r: 20, g: 20, b: 20 }, 0.12),
    surface: mix(surfaceBase, primary, isDark ? 0.16 : 0.1),
    foreground: isDark ? { r: 248, g: 247, b: 244 } : { r: 19, g: 20, b: 23 },
    isDark,
    source: "media",
  };
}

export function blendPalette(
  theme: AdaptiveChromaPalette,
  media: AdaptiveChromaPalette,
  strength: AdaptiveChromaStrength,
): AdaptiveChromaPalette {
  const amount = strengthAmount(strength);
  if (!amount) return { ...theme };
  return {
    primary: mix(theme.primary, media.primary, amount),
    secondary: mix(theme.secondary, media.secondary, amount * 0.86),
    accent: mix(theme.accent, media.accent, Math.min(0.76, amount + 0.08)),
    surface: mix(theme.surface, media.surface, amount * 0.38),
    foreground: mix(theme.foreground, media.foreground, amount * 0.18),
    isDark: theme.isDark,
    source: media.source,
  };
}

export function paletteCssVariables(palette: AdaptiveChromaPalette): Record<string, string> {
  const onAccent = relativeLuminance(palette.accent) > 0.42 ? { r: 12, g: 13, b: 16 } : { r: 255, g: 255, b: 255 };
  return {
    "--media-primary-rgb": css(palette.primary),
    "--media-secondary-rgb": css(palette.secondary),
    "--media-accent-rgb": css(palette.accent),
    "--media-surface-rgb": css(palette.surface),
    "--media-foreground-rgb": css(palette.foreground),
    "--media-on-accent-rgb": css(onAccent),
  };
}

export function fallbackPalette(): AdaptiveChromaPalette {
  return { ...FALLBACK_PALETTE };
}

/**
 * Parse a theme token color (hex or rgb()/rgba()) into channels.
 * Returns null for anything unrecognized so callers keep their fallback.
 */
export function parseCssColorToRgb(value: string): RgbColor | null {
  const input = value.trim();
  if (!input) return null;
  const hex = /^#([0-9a-f]{3,8})$/i.exec(input);
  if (hex) {
    const raw = hex[1];
    const normalized = raw.length <= 4 ? [...raw].map((channel) => channel + channel).join("") : raw;
    if (normalized.length !== 6 && normalized.length !== 8) return null;
    return {
      r: parseInt(normalized.slice(0, 2), 16),
      g: parseInt(normalized.slice(2, 4), 16),
      b: parseInt(normalized.slice(4, 6), 16),
    };
  }
  const fn = /^rgba?\(\s*([^)]+?)\s*\)$/i.exec(input);
  if (fn) {
    const channels = fn[1]
      .replace(/\//g, " ")
      .split(/[\s,]+/)
      .filter(Boolean)
      .slice(0, 3)
      .map((part) => (part.endsWith("%") ? (parseFloat(part) / 100) * 255 : parseFloat(part)));
    if (channels.length === 3 && channels.every((channel) => Number.isFinite(channel))) {
      return { r: clampChannel(channels[0]), g: clampChannel(channels[1]), b: clampChannel(channels[2]) };
    }
  }
  return null;
}
