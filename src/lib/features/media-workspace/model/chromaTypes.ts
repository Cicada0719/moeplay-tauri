export type AdaptiveChromaStrength = "off" | "subtle" | "balanced" | "immersive";

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export interface AdaptiveChromaPalette {
  primary: RgbColor;
  secondary: RgbColor;
  accent: RgbColor;
  surface: RgbColor;
  foreground: RgbColor;
  isDark: boolean;
  source: "media" | "fallback";
}

export interface PaletteExtractionOptions {
  maxSamples?: number;
  fallback?: Partial<AdaptiveChromaPalette>;
}

export interface MediaChromeContext {
  palette: AdaptiveChromaPalette;
  strength: AdaptiveChromaStrength;
}
