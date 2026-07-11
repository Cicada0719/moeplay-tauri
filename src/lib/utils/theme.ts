import {
  DEFAULT_APPEARANCE,
  getThemePack,
  migrateLegacyTheme,
  normalizeAppearance,
  type AppearanceSettings,
  type ColorMode,
} from "../theme-packs";

export type AppTheme = "dark" | "light" | "sakura" | "system" | "black" | "contrast";

export const APP_THEMES: { id: AppTheme; label: string; icon: string }[] = [
  { id: "dark", label: "深色", icon: "home" },
  { id: "light", label: "浅色", icon: "lightbulb" },
  { id: "sakura", label: "樱夜", icon: "heart" },
  { id: "system", label: "跟随系统", icon: "monitor" },
  { id: "black", label: "纯黑", icon: "moon" },
  { id: "contrast", label: "高对比", icon: "contrast" },
];

const THEME_STORAGE_KEY = "moegame-theme";
const APPEARANCE_STORAGE_KEY = "moeplay-appearance-v1";
const VALID_THEMES: AppTheme[] = ["dark", "light", "sakura", "system", "black", "contrast"];

export function isValidTheme(value: string): value is AppTheme { return VALID_THEMES.includes(value as AppTheme); }
export function resolveTheme(theme: AppTheme): Exclude<AppTheme, "system"> {
  if (theme === "system") return typeof globalThis.matchMedia !== "undefined" && globalThis.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  return theme;
}

let _systemListener: ((event: MediaQueryListEvent) => void) | null = null;
let _systemQuery: MediaQueryList | null = null;
function clearSystemListener() {
  if (_systemQuery && _systemListener) { _systemQuery.removeEventListener("change", _systemListener); _systemQuery.removeListener?.(_systemListener); }
  _systemQuery = null; _systemListener = null;
}

function resolveColorMode(appearance: AppearanceSettings): Exclude<AppTheme, "system" | "sakura"> {
  const pack = getThemePack(appearance.theme_pack);
  const mode: ColorMode = appearance.color_mode;
  if (mode === "pack-default") return pack.defaultColorMode;
  if (mode === "system") return typeof globalThis.matchMedia !== "undefined" && globalThis.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  return mode;
}

export function applyAppearance(input: Partial<AppearanceSettings>): AppearanceSettings {
  const appearance = normalizeAppearance(input);
  const effective = resolveColorMode(appearance);
  const root = document.documentElement;
  root.setAttribute("data-theme-pack", appearance.theme_pack);
  root.setAttribute("data-color-mode", effective);
  root.setAttribute("data-theme", effective);
  root.setAttribute("data-wallpaper-rating", "general");
  root.setAttribute("data-decoration", appearance.decorative_effects && effective !== "contrast" ? getThemePack(appearance.theme_pack).decoration : "none");
  try { localStorage.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify(appearance)); } catch { /* private mode */ }
  clearSystemListener();
  if (appearance.color_mode === "system" && typeof globalThis.matchMedia !== "undefined") {
    _systemQuery = globalThis.matchMedia("(prefers-color-scheme: dark)");
    _systemListener = () => applyAppearance(appearance);
    _systemQuery.addEventListener("change", _systemListener);
  }
  return appearance;
}

export function loadStoredAppearance(legacyTheme?: string): AppearanceSettings {
  if (typeof localStorage !== "undefined") {
    try {
      const stored = localStorage.getItem(APPEARANCE_STORAGE_KEY);
      if (stored) return normalizeAppearance(JSON.parse(stored));
    } catch { /* malformed storage */ }
  }
  return legacyTheme ? migrateLegacyTheme(legacyTheme) : DEFAULT_APPEARANCE;
}

/** Legacy compatibility for older call sites and persisted settings. */
export function applyTheme(theme: AppTheme) { applyAppearance(migrateLegacyTheme(theme)); try { localStorage.setItem(THEME_STORAGE_KEY, theme); } catch {} }
export function loadStoredTheme(): AppTheme {
  if (typeof localStorage === "undefined") return "dark";
  try { const stored = localStorage.getItem(THEME_STORAGE_KEY); return stored && isValidTheme(stored) ? stored : "dark"; } catch { return "dark"; }
}
export function getEffectiveTheme(theme: AppTheme): Exclude<AppTheme, "system"> { return resolveTheme(theme); }
