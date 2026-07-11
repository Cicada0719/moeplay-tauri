import { describe, expect, it } from "vitest";
import { blendPalette, extractPaletteFromPixels, fallbackPalette, paletteCssVariables } from "./adaptiveChroma";

function pixels(colors: Array<[number, number, number]>) {
  return new Uint8ClampedArray(colors.flatMap(([r, g, b]) => [r, g, b, 255]));
}

describe("adaptive chroma", () => {
  it("extracts a media palette while rejecting flat grayscale noise", () => {
    const palette = extractPaletteFromPixels({
      data: pixels([
        [12, 12, 12], [240, 240, 240], [185, 38, 72], [185, 38, 72],
        [185, 38, 72], [24, 92, 138], [24, 92, 138], [132, 132, 132],
      ]),
      width: 4,
      height: 2,
    });

    expect(palette.source).toBe("media");
    expect(palette.accent.r).toBeGreaterThan(palette.accent.g);
    expect(palette.primary.r + palette.primary.b).toBeGreaterThan(palette.primary.g * 1.5);
  });

  it("falls back for unusable transparent pixels", () => {
    const palette = extractPaletteFromPixels({ data: new Uint8ClampedArray(16), width: 2, height: 2 });
    expect(palette.source).toBe("fallback");
  });

  it("blends by strength without replacing product identity", () => {
    const theme = fallbackPalette();
    const media = { ...theme, primary: { r: 0, g: 160, b: 220 }, accent: { r: 0, g: 210, b: 245 }, source: "media" as const };
    expect(blendPalette(theme, media, "off").primary).toEqual(theme.primary);
    expect(blendPalette(theme, media, "immersive").primary.b).toBeGreaterThan(blendPalette(theme, media, "subtle").primary.b);
  });

  it("exports channel-based CSS variables for alpha composition", () => {
    const variables = paletteCssVariables(fallbackPalette());
    expect(variables["--media-accent-rgb"]).toMatch(/^\d+ \d+ \d+$/);
    expect(variables["--media-on-accent-rgb"]).toBeTruthy();
  });
});
