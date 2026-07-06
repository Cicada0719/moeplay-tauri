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
const VALID_THEMES: AppTheme[] = ["dark", "light", "sakura", "system", "black", "contrast"];

export function isValidTheme(value: string): value is AppTheme {
  return VALID_THEMES.includes(value as AppTheme);
}

export function resolveTheme(theme: AppTheme): Exclude<AppTheme, "system"> {
  if (theme === "system") {
    if (typeof globalThis.matchMedia !== "undefined") {
      return globalThis.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    return "dark";
  }
  return theme;
}

let _systemListener: ((event: MediaQueryListEvent) => void) | null = null;
let _systemQuery: MediaQueryList | null = null;

function clearSystemListener() {
  if (_systemQuery && _systemListener) {
    _systemQuery.removeEventListener("change", _systemListener);
    _systemQuery.removeListener?.(_systemListener);
  }
  _systemQuery = null;
  _systemListener = null;
}

export function applyTheme(theme: AppTheme) {
  const effective = resolveTheme(theme);
  document.documentElement.setAttribute("data-theme", effective);
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // ignore private mode / sandbox errors
  }

  clearSystemListener();

  if (theme === "system" && typeof globalThis.matchMedia !== "undefined") {
    _systemQuery = globalThis.matchMedia("(prefers-color-scheme: dark)");
    _systemListener = () => {
      document.documentElement.setAttribute("data-theme", resolveTheme("system"));
    };
    _systemQuery.addEventListener("change", _systemListener);
  }
}

export function loadStoredTheme(): AppTheme {
  if (typeof localStorage === "undefined") return "dark";
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (stored && isValidTheme(stored)) return stored;
  } catch {
    // ignore
  }
  return "dark";
}

export function getEffectiveTheme(theme: AppTheme): Exclude<AppTheme, "system"> {
  return resolveTheme(theme);
}
