import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  type AppTheme,
  APP_THEMES,
  applyTheme,
  getEffectiveTheme,
  isValidTheme,
  loadStoredTheme,
  resolveTheme,
} from "./theme";

const THEME_STORAGE_KEY = "moegame-theme";

describe("theme utilities", () => {
  let stored: Record<string, string> = {};
  let dataTheme: string | null = null;

  let matchMediaMock = {
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    removeListener: vi.fn(),
  };

  beforeEach(() => {
    stored = {};
    dataTheme = null;

    const fakeElement = {
      setAttribute: (_name: string, value: string) => {
        if (_name === "data-theme") dataTheme = value;
      },
      getAttribute: (name: string) => (name === "data-theme" ? dataTheme : null),
      removeAttribute: (name: string) => {
        if (name === "data-theme") dataTheme = null;
      },
    };

    vi.stubGlobal("document", { documentElement: fakeElement });
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => stored[key] ?? null,
      setItem: (key: string, value: string) => {
        stored[key] = value;
      },
      clear: () => {
        stored = {};
      },
    });

    matchMediaMock = {
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      removeListener: vi.fn(),
    };
    vi.stubGlobal("matchMedia", () => matchMediaMock as unknown as MediaQueryList);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("exposes 6 themes with labels and icons", () => {
    expect(APP_THEMES.length).toBe(6);
    expect(APP_THEMES.map((t) => t.id)).toEqual([
      "dark",
      "light",
      "sakura",
      "system",
      "black",
      "contrast",
    ]);
    for (const theme of APP_THEMES) {
      expect(theme.label).toBeTruthy();
      expect(theme.icon).toBeTruthy();
    }
  });

  it("validates theme names", () => {
    expect(isValidTheme("dark")).toBe(true);
    expect(isValidTheme("system")).toBe(true);
    expect(isValidTheme("black")).toBe(true);
    expect(isValidTheme("contrast")).toBe(true);
    expect(isValidTheme("neon")).toBe(false);
    expect(isValidTheme("")).toBe(false);
  });

  it("resolves system theme from matchMedia", () => {
    matchMediaMock.matches = true;
    expect(resolveTheme("system")).toBe("dark");
    matchMediaMock.matches = false;
    expect(resolveTheme("system")).toBe("light");
  });

  it("resolves non-system themes to themselves", () => {
    const themes: AppTheme[] = ["dark", "light", "sakura", "black", "contrast"];
    for (const theme of themes) {
      expect(resolveTheme(theme)).toBe(theme);
      expect(getEffectiveTheme(theme)).toBe(theme);
    }
  });

  it("loads stored theme from localStorage", () => {
    stored[THEME_STORAGE_KEY] = "light";
    expect(loadStoredTheme()).toBe("light");
  });

  it("falls back to dark for missing or invalid stored theme", () => {
    expect(loadStoredTheme()).toBe("dark");
    stored[THEME_STORAGE_KEY] = "invalid";
    expect(loadStoredTheme()).toBe("dark");
  });

  it("applyTheme sets effective data-theme and persists preference", () => {
    applyTheme("black");
    expect(dataTheme).toBe("black");
    expect(stored[THEME_STORAGE_KEY]).toBe("black");

    applyTheme("system");
    expect(dataTheme).toBe("light");
    expect(stored[THEME_STORAGE_KEY]).toBe("system");
  });

  it("applyTheme listens to system changes only in system mode", () => {
    applyTheme("dark");
    expect(matchMediaMock.addEventListener).not.toHaveBeenCalled();

    applyTheme("system");
    expect(matchMediaMock.addEventListener).toHaveBeenCalledOnce();

    applyTheme("light");
    expect(matchMediaMock.removeEventListener).toHaveBeenCalled();
  });
});
