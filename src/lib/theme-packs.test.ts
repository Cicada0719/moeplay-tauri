import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  DEFAULT_APPEARANCE,
  THEME_PACKS,
  getThemePack,
  migrateLegacyTheme,
  normalizeAppearance,
} from "./theme-packs";

describe("theme pack registry", () => {
  it("registers the five shipped anime theme packs with complete bundled assets", () => {
    expect(THEME_PACKS.map((pack) => pack.id)).toEqual([
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
    expect(getThemePack(undefined).id).toBe("phantom-pop");
    expect(getThemePack("not-a-theme").id).toBe("phantom-pop");
    expect(getThemePack("shift-editorial").label).toBe("素纸编集");
  });

  it("keeps the Rust ThemePackId enum in sync with the frontend registry", () => {
    // 防漂移契约：前端发的新包 id 必须能被 Rust 端 serde 反序列化，
    // 否则 update_settings 失败会导致主题切换静默失效（0.15.x 热修根因）。
    const rustVariants: Record<string, string> = {
      "shift-editorial": "ShiftEditorial",
      "phantom-pop": "PhantomPop",
      "caution-industrial": "CautionIndustrial",
      "astral-rail": "AstralRail",
      "borderless-lumen": "BorderlessLumen",
    };
    const modelsRs = readFileSync(
      path.join(process.cwd(), "src-tauri", "src", "models.rs"),
      "utf8",
    );
    expect(modelsRs).toContain('rename_all = "kebab-case"');
    for (const pack of THEME_PACKS) {
      const variant = rustVariants[pack.id];
      expect(variant, `missing Rust variant mapping for ${pack.id}`).toBeTruthy();
      // kebab-case 是 serde 的线上格式：变体名转 kebab 必须等于前端 id。
      expect(variant.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase()).toBe(pack.id);
      expect(modelsRs).toContain(variant);
    }
  });
});

describe("appearance normalization and legacy migration", () => {
  it.each([
    ["sakura", "phantom-pop", "pack-default"],
    ["light", "shift-editorial", "light"],
    ["dark", "phantom-pop", "dark"],
    ["black", "phantom-pop", "black"],
    ["contrast", "phantom-pop", "contrast"],
    ["system", "phantom-pop", "system"],
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

  it.each([
    ["yozakura", "phantom-pop"],
    ["after-school", "shift-editorial"],
    ["neon-isekai", "borderless-lumen"],
  ] as const)("redirects retired theme pack %s to %s", (retiredId, replacement) => {
    expect(normalizeAppearance({ theme_pack: retiredId as never }).theme_pack).toBe(replacement);
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
      theme_pack: "astral-rail",
      color_mode: "contrast",
      wallpaper_rotation: "fixed",
      fixed_wallpaper_id: "builtin:astral-rail:2",
      custom_wallpaper_path: "C:/MoePlay/custom.webp",
      mascot_enabled: false,
      custom_mascot_path: "C:/MoePlay/mascot.webp",
      decorative_effects: false,
      online_gallery_enabled: false,
    })).toEqual({
      theme_pack: "astral-rail",
      color_mode: "contrast",
      wallpaper_rotation: "fixed",
      fixed_wallpaper_id: "builtin:astral-rail:2",
      custom_wallpaper_path: "C:/MoePlay/custom.webp",
      mascot_enabled: false,
      custom_mascot_path: "C:/MoePlay/mascot.webp",
      decorative_effects: false,
      online_gallery_enabled: false,
    });
  });
});
