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
import shiftEditorial1 from "./assets/themes/shift-editorial/wallpaper-1.jpg";
import shiftEditorial2 from "./assets/themes/shift-editorial/wallpaper-2.jpg";
import shiftEditorial3 from "./assets/themes/shift-editorial/wallpaper-3.jpg";
import shiftEditorialBlur1 from "./assets/themes/shift-editorial/wallpaper-1-blur.jpg";
import shiftEditorialBlur2 from "./assets/themes/shift-editorial/wallpaper-2-blur.jpg";
import shiftEditorialBlur3 from "./assets/themes/shift-editorial/wallpaper-3-blur.jpg";
import shiftEditorialPreview from "./assets/themes/shift-editorial/preview.jpg";
import shiftEditorialMascot from "./assets/themes/shift-editorial/mascot.png";
import phantomPop1 from "./assets/themes/phantom-pop/wallpaper-1.jpg";
import phantomPop2 from "./assets/themes/phantom-pop/wallpaper-2.jpg";
import phantomPop3 from "./assets/themes/phantom-pop/wallpaper-3.jpg";
import phantomPopBlur1 from "./assets/themes/phantom-pop/wallpaper-1-blur.jpg";
import phantomPopBlur2 from "./assets/themes/phantom-pop/wallpaper-2-blur.jpg";
import phantomPopBlur3 from "./assets/themes/phantom-pop/wallpaper-3-blur.jpg";
import phantomPopPreview from "./assets/themes/phantom-pop/preview.jpg";
import phantomPopMascot from "./assets/themes/phantom-pop/mascot.png";
import cautionIndustrial1 from "./assets/themes/caution-industrial/wallpaper-1.jpg";
import cautionIndustrial2 from "./assets/themes/caution-industrial/wallpaper-2.jpg";
import cautionIndustrial3 from "./assets/themes/caution-industrial/wallpaper-3.jpg";
import cautionIndustrialBlur1 from "./assets/themes/caution-industrial/wallpaper-1-blur.jpg";
import cautionIndustrialBlur2 from "./assets/themes/caution-industrial/wallpaper-2-blur.jpg";
import cautionIndustrialBlur3 from "./assets/themes/caution-industrial/wallpaper-3-blur.jpg";
import cautionIndustrialPreview from "./assets/themes/caution-industrial/preview.jpg";
import cautionIndustrialMascot from "./assets/themes/caution-industrial/mascot.png";
import astralRail1 from "./assets/themes/astral-rail/wallpaper-1.jpg";
import astralRail2 from "./assets/themes/astral-rail/wallpaper-2.jpg";
import astralRail3 from "./assets/themes/astral-rail/wallpaper-3.jpg";
import astralRailBlur1 from "./assets/themes/astral-rail/wallpaper-1-blur.jpg";
import astralRailBlur2 from "./assets/themes/astral-rail/wallpaper-2-blur.jpg";
import astralRailBlur3 from "./assets/themes/astral-rail/wallpaper-3-blur.jpg";
import astralRailPreview from "./assets/themes/astral-rail/preview.jpg";
import astralRailMascot from "./assets/themes/astral-rail/mascot.png";
import borderlessLumen1 from "./assets/themes/borderless-lumen/wallpaper-1.jpg";
import borderlessLumen2 from "./assets/themes/borderless-lumen/wallpaper-2.jpg";
import borderlessLumen3 from "./assets/themes/borderless-lumen/wallpaper-3.jpg";
import borderlessLumenBlur1 from "./assets/themes/borderless-lumen/wallpaper-1-blur.jpg";
import borderlessLumenBlur2 from "./assets/themes/borderless-lumen/wallpaper-2-blur.jpg";
import borderlessLumenBlur3 from "./assets/themes/borderless-lumen/wallpaper-3-blur.jpg";
import borderlessLumenPreview from "./assets/themes/borderless-lumen/preview.jpg";
import borderlessLumenMascot from "./assets/themes/borderless-lumen/mascot.png";

export type ThemePackId = "yozakura" | "after-school" | "neon-isekai" | "shift-editorial" | "phantom-pop" | "caution-industrial" | "astral-rail" | "borderless-lumen";
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
  {
    id: "shift-editorial", label: "素纸编集", description: "纸张、发丝线与编辑排版美学，信号红点睛。", defaultColorMode: "light", decoration: "light-particles",
    preview: shiftEditorialPreview, mascot: shiftEditorialMascot,
    wallpapers: [wallpaper("shift-editorial", 1, "素纸横题", shiftEditorial1, shiftEditorialBlur1), wallpaper("shift-editorial", 2, "网格留白", shiftEditorial2, shiftEditorialBlur2), wallpaper("shift-editorial", 3, "墨渐红线", shiftEditorial3, shiftEditorialBlur3)],
  },
  {
    id: "phantom-pop", label: "魅影波普", description: "斜切构图、黑红撞色的怪盗波普。", defaultColorMode: "dark", decoration: "petals",
    preview: phantomPopPreview, mascot: phantomPopMascot,
    wallpapers: [wallpaper("phantom-pop", 1, "斜切赤带", phantomPop1, phantomPopBlur1), wallpaper("phantom-pop", 2, "撕裂拼贴", phantomPop2, phantomPopBlur2), wallpaper("phantom-pop", 3, "噪点红黑", phantomPop3, phantomPopBlur3)],
  },
  {
    id: "caution-industrial", label: "警戒工业", description: "枪灰金属、技术 HUD 与警戒橙。", defaultColorMode: "dark", decoration: "digital-rain",
    preview: cautionIndustrialPreview, mascot: cautionIndustrialMascot,
    wallpapers: [wallpaper("caution-industrial", 1, "蓝图网格", cautionIndustrial1, cautionIndustrialBlur1), wallpaper("caution-industrial", 2, "警示条纹", cautionIndustrial2, cautionIndustrialBlur2), wallpaper("caution-industrial", 3, "金属走线", cautionIndustrial3, cautionIndustrialBlur3)],
  },
  {
    id: "astral-rail", label: "星穹旅人", description: "深空靛蓝、星轨金线与银河之夜。", defaultColorMode: "dark", decoration: "light-particles",
    preview: astralRailPreview, mascot: astralRailMascot,
    wallpapers: [wallpaper("astral-rail", 1, "银河铁道", astralRail1, astralRailBlur1), wallpaper("astral-rail", 2, "星图连线", astralRail2, astralRailBlur2), wallpaper("astral-rail", 3, "晨曦跃迁", astralRail3, astralRailBlur3)],
  },
  {
    id: "borderless-lumen", label: "无界流光", description: "黑暗中晕开的有机色场与光之呼吸。", defaultColorMode: "dark", decoration: "petals",
    preview: borderlessLumenPreview, mascot: borderlessLumenMascot,
    wallpapers: [wallpaper("borderless-lumen", 1, "花舞光场", borderlessLumen1, borderlessLumenBlur1), wallpaper("borderless-lumen", 2, "水镜流光", borderlessLumen2, borderlessLumenBlur2), wallpaper("borderless-lumen", 3, "萤火之森", borderlessLumen3, borderlessLumenBlur3)],
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
