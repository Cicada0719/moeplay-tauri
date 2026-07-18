import { describe, expect, it } from "vitest";
import {
  DEFAULT_APPEARANCE,
  THEME_PACKS,
  getThemePack,
  migrateLegacyTheme,
  normalizeAppearance,
} from "./theme-packs";

describe("theme pack registry", () => {
  it("registers the eight shipped anime theme packs with complete bundled assets", () => {
    expect(THEME_PACKS.map((pack) => pack.id)).toEqual([
      "yozakura",
      "after-school",
      "neon-isekai",
      "shift-editorial",
      "phantom-pop",
      "caution-industrial",
      "astral-rail",
      "borderless-lumen",
    ]);

    for (const pack of THEME_PACKS) {
      expect(pack.label).toBeTruthy();
      expect(pack.description).toBeTruthy();
      expect(pack.preview).toBeTruthy();
      expect(pack.mascot).toBeTruthy();
      expect(pack.wallpapers).toHaveLength(3);
      expect(new Set(pack.wallpapers.map((wallpaper) => wallpaper.id)).size).toBe(3);

      for (const wallpaper of pack.wallpapers) {
        expect(wallpaper.id).toMatch(new RegExp(`^builtin:${pack.id}:`));
        expect(wallpaper.src).toBeTruthy();
        expect(wallpaper.placeholder).toBeTruthy();
        expect(wallpaper.rating).toBe("general");
        expect(wallpaper.author).toBeTruthy();
        expect(wallpaper.licenseId).toBeTruthy();
      }
    }
  });

  it("falls back to the default theme pack for unknown ids", () => {
    expect(getThemePack(undefined).id).toBe("yozakura");
    expect(getThemePack("not-a-theme").id).toBe("yozakura");
    expect(getThemePack("after-school").label).toBe("青空放课后");
  });
});

describe("appearance normalization and legacy migration", () => {
  it.each([
    ["sakura", "yozakura", "pack-default"],
    ["light", "after-school", "light"],
    ["dark", "yozakura", "dark"],
    ["black", "yozakura", "black"],
    ["contrast", "yozakura", "contrast"],
    ["system", "yozakura", "system"],
  ] as const)("migrates legacy %s preferences", (legacyTheme, themePack, colorMode) => {
    expect(migrateLegacyTheme(legacyTheme)).toMatchObject({
      ...DEFAULT_APPEARANCE,
      theme_pack: themePack,
      color_mode: colorMode,
    });
  });

  it("uses the default appearance for missing or invalid legacy preferences", () => {
    expect(migrateLegacyTheme(undefined)).toEqual(DEFAULT_APPEARANCE);
    expect(migrateLegacyTheme("solarized")).toEqual(DEFAULT_APPEARANCE);
  });

  it("normalizes invalid enum values while preserving supported appearance choices", () => {
    expect(normalizeAppearance({
      theme_pack: "unknown" as never,
      color_mode: "sepia" as never,
      wallpaper_rotation: "later" as never,
      mascot_enabled: false,
      decorative_effects: false,
      online_gallery_enabled: false,
    })).toEqual({
      ...DEFAULT_APPEARANCE,
      mascot_enabled: false,
      decorative_effects: false,
      online_gallery_enabled: false,
    });

    expect(normalizeAppearance({
      theme_pack: "neon-isekai",
      color_mode: "contrast",
      wallpaper_rotation: "fixed",
      fixed_wallpaper_id: "builtin:neon-isekai:2",
      custom_wallpaper_path: "C:/MoePlay/custom.webp",
      mascot_enabled: false,
      custom_mascot_path: "C:/MoePlay/mascot.webp",
      decorative_effects: false,
      online_gallery_enabled: false,
    })).toEqual({
      theme_pack: "neon-isekai",
      color_mode: "contrast",
      wallpaper_rotation: "fixed",
      fixed_wallpaper_id: "builtin:neon-isekai:2",
      custom_wallpaper_path: "C:/MoePlay/custom.webp",
      mascot_enabled: false,
      custom_mascot_path: "C:/MoePlay/mascot.webp",
      decorative_effects: false,
      online_gallery_enabled: false,
    });
  });
});