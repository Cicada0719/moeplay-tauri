import yozakura1 from "./assets/themes/yozakura/wallpaper-1.webp";
import yozakura2 from "./assets/themes/yozakura/wallpaper-2.webp";
import yozakura3 from "./assets/themes/yozakura/wallpaper-3.webp";
import yozakuraBlur1 from "./assets/themes/yozakura/wallpaper-1-blur.webp";
import yozakuraBlur2 from "./assets/themes/yozakura/wallpaper-2-blur.webp";
import yozakuraBlur3 from "./assets/themes/yozakura/wallpaper-3-blur.webp";
import yozakuraPreview from "./assets/themes/yozakura/preview.webp";
import yozakuraMascot from "./assets/themes/yozakura/mascot.webp";
import afterSchool1 from "./assets/themes/after-school/wallpaper-1.webp";
import afterSchool2 from "./assets/themes/after-school/wallpaper-2.webp";
import afterSchool3 from "./assets/themes/after-school/wallpaper-3.webp";
import afterSchoolBlur1 from "./assets/themes/after-school/wallpaper-1-blur.webp";
import afterSchoolBlur2 from "./assets/themes/after-school/wallpaper-2-blur.webp";
import afterSchoolBlur3 from "./assets/themes/after-school/wallpaper-3-blur.webp";
import afterSchoolPreview from "./assets/themes/after-school/preview.webp";
import afterSchoolMascot from "./assets/themes/after-school/mascot.webp";
import neon1 from "./assets/themes/neon-isekai/wallpaper-1.webp";
import neon2 from "./assets/themes/neon-isekai/wallpaper-2.webp";
import neon3 from "./assets/themes/neon-isekai/wallpaper-3.webp";
import neonBlur1 from "./assets/themes/neon-isekai/wallpaper-1-blur.webp";
import neonBlur2 from "./assets/themes/neon-isekai/wallpaper-2-blur.webp";
import neonBlur3 from "./assets/themes/neon-isekai/wallpaper-3-blur.webp";
import neonPreview from "./assets/themes/neon-isekai/preview.webp";
import neonMascot from "./assets/themes/neon-isekai/mascot.webp";

export type ThemePackId = "yozakura" | "after-school" | "neon-isekai";
export type ColorMode = "pack-default" | "system" | "light" | "dark" | "black" | "contrast";
export type WallpaperRotation = "startup-random" | "fixed";
export type WallpaperRating = "general" | "suggestive" | "adult";
export type ThemeDecoration = "petals" | "light-particles" | "digital-rain";

export type { AppearanceSettings } from "./api/types";
import type { AppearanceSettings } from "./api/types";

export interface BuiltinWallpaper {
  id: string;
  title: string;
  src: string;
  placeholder: string;
  rating: WallpaperRating;
  author: string;
  licenseId: string;
}

export interface ThemePackDefinition {
  id: ThemePackId;
  label: string;
  description: string;
  defaultColorMode: "light" | "dark";
  wallpapers: BuiltinWallpaper[];
  mascot: string;
  preview: string;
  decoration: ThemeDecoration;
}

function wallpaper(pack: ThemePackId, index: number, title: string, src: string, placeholder: string): BuiltinWallpaper {
  return { id: `builtin:${pack}:${index}`, title, src, placeholder, rating: "general", author: "MoePlay Original", licenseId: "MoePlay-Bundled" };
}

export const THEME_PACKS: ThemePackDefinition[] = [
  {
    id: "yozakura", label: "夜樱终端", description: "夜樱、日式城市与克制的玫红终端光。", defaultColorMode: "dark", decoration: "petals",
    preview: yozakuraPreview, mascot: yozakuraMascot,
    wallpapers: [wallpaper("yozakura", 1, "月下鸟居", yozakura1, yozakuraBlur1), wallpaper("yozakura", 2, "樱夜窗景", yozakura2, yozakuraBlur2), wallpaper("yozakura", 3, "夜桥流光", yozakura3, yozakuraBlur3)],
  },
  {
    id: "after-school", label: "青空放课后", description: "晴空、海风与轻盈的校园午后。", defaultColorMode: "light", decoration: "light-particles",
    preview: afterSchoolPreview, mascot: afterSchoolMascot,
    wallpapers: [wallpaper("after-school", 1, "海边校舍", afterSchool1, afterSchoolBlur1), wallpaper("after-school", 2, "沿海列车", afterSchool2, afterSchoolBlur2), wallpaper("after-school", 3, "晴空教室", afterSchool3, afterSchoolBlur3)],
  },
  {
    id: "neon-isekai", label: "霓虹异界", description: "雨夜都市、全息界面与电青霓虹。", defaultColorMode: "dark", decoration: "digital-rain",
    preview: neonPreview, mascot: neonMascot,
    wallpapers: [wallpaper("neon-isekai", 1, "环城终端", neon1, neonBlur1), wallpaper("neon-isekai", 2, "雨夜控制室", neon2, neonBlur2), wallpaper("neon-isekai", 3, "异界数据港", neon3, neonBlur3)],
  },
];

export const DEFAULT_APPEARANCE: AppearanceSettings = {
  theme_pack: "yozakura", color_mode: "pack-default", wallpaper_rotation: "startup-random",
  mascot_enabled: true, decorative_effects: true, online_gallery_enabled: true,
};

export function getThemePack(id: string | null | undefined): ThemePackDefinition {
  return THEME_PACKS.find((pack) => pack.id === id) ?? THEME_PACKS[0];
}

export function isThemePackId(value: unknown): value is ThemePackId {
  return typeof value === "string" && THEME_PACKS.some((pack) => pack.id === value);
}

export function isColorMode(value: unknown): value is ColorMode {
  return ["pack-default", "system", "light", "dark", "black", "contrast"].includes(String(value));
}

export function normalizeAppearance(value: Partial<AppearanceSettings> | null | undefined): AppearanceSettings {
  return {
    ...DEFAULT_APPEARANCE,
    ...value,
    theme_pack: isThemePackId(value?.theme_pack) ? value.theme_pack : DEFAULT_APPEARANCE.theme_pack,
    color_mode: isColorMode(value?.color_mode) ? value.color_mode : DEFAULT_APPEARANCE.color_mode,
    wallpaper_rotation: value?.wallpaper_rotation === "fixed" ? "fixed" : "startup-random",
  };
}

export function migrateLegacyTheme(theme: string | null | undefined): AppearanceSettings {
  const mapping: Record<string, Pick<AppearanceSettings, "theme_pack" | "color_mode">> = {
    sakura: { theme_pack: "yozakura", color_mode: "pack-default" },
    light: { theme_pack: "after-school", color_mode: "light" },
    dark: { theme_pack: "yozakura", color_mode: "dark" },
    black: { theme_pack: "yozakura", color_mode: "black" },
    contrast: { theme_pack: "yozakura", color_mode: "contrast" },
    system: { theme_pack: "yozakura", color_mode: "system" },
  };
  return { ...DEFAULT_APPEARANCE, ...(mapping[theme ?? ""] ?? {}) };
}
