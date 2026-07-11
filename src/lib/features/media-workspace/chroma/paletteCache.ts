import type { AdaptiveChromaPalette, RgbColor } from "../model/chromaTypes";

export const ADAPTIVE_CHROMA_CACHE_VERSION = 1;
export const ADAPTIVE_CHROMA_CACHE_NAMESPACE = "moeplay:adaptive-chroma";

export interface ChromaStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

interface StoredPalette {
  version: number;
  url: string;
  palette: AdaptiveChromaPalette;
}

function isChannel(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 255;
}

function isRgbColor(value: unknown): value is RgbColor {
  if (!value || typeof value !== "object") return false;
  const color = value as Partial<RgbColor>;
  return isChannel(color.r) && isChannel(color.g) && isChannel(color.b);
}

export function isAdaptiveChromaPalette(value: unknown): value is AdaptiveChromaPalette {
  if (!value || typeof value !== "object") return false;
  const palette = value as Partial<AdaptiveChromaPalette>;
  return isRgbColor(palette.primary)
    && isRgbColor(palette.secondary)
    && isRgbColor(palette.accent)
    && isRgbColor(palette.surface)
    && isRgbColor(palette.foreground)
    && typeof palette.isDark === "boolean"
    && (palette.source === "media" || palette.source === "fallback");
}

function cloneColor(color: RgbColor): RgbColor {
  return { r: color.r, g: color.g, b: color.b };
}

function clonePalette(palette: AdaptiveChromaPalette): AdaptiveChromaPalette {
  return {
    primary: cloneColor(palette.primary),
    secondary: cloneColor(palette.secondary),
    accent: cloneColor(palette.accent),
    surface: cloneColor(palette.surface),
    foreground: cloneColor(palette.foreground),
    isDark: palette.isDark,
    source: palette.source,
  };
}

export class AdaptiveChromaPaletteCache {
  readonly version: number;
  readonly namespace: string;

  private readonly memory = new Map<string, AdaptiveChromaPalette>();
  private readonly storageOverride: ChromaStorage | null | undefined;

  constructor(options: {
    version?: number;
    namespace?: string;
    storage?: ChromaStorage | null;
  } = {}) {
    this.version = options.version ?? ADAPTIVE_CHROMA_CACHE_VERSION;
    this.namespace = options.namespace ?? ADAPTIVE_CHROMA_CACHE_NAMESPACE;
    this.storageOverride = options.storage;
  }

  keyFor(url: string): string {
    return `${this.namespace}:v${this.version}:${encodeURIComponent(url)}`;
  }

  get(url: string): AdaptiveChromaPalette | null {
    const memoryValue = this.memory.get(url);
    if (memoryValue) return clonePalette(memoryValue);

    const storage = this.storage();
    if (!storage) return null;

    const key = this.keyFor(url);
    try {
      const raw = storage.getItem(key);
      if (!raw) return null;
      const stored = JSON.parse(raw) as Partial<StoredPalette>;
      if (stored.version !== this.version || stored.url !== url || !isAdaptiveChromaPalette(stored.palette)) {
        storage.removeItem(key);
        return null;
      }
      const palette = clonePalette(stored.palette);
      this.memory.set(url, palette);
      return clonePalette(palette);
    } catch {
      try {
        storage.removeItem(key);
      } catch {
        // Storage can be unavailable in privacy modes or restricted webviews.
      }
      return null;
    }
  }

  set(url: string, palette: AdaptiveChromaPalette): void {
    if (!url || !isAdaptiveChromaPalette(palette)) return;
    const value = clonePalette(palette);
    this.memory.set(url, value);

    const storage = this.storage();
    if (!storage) return;
    const stored: StoredPalette = { version: this.version, url, palette: value };
    try {
      storage.setItem(this.keyFor(url), JSON.stringify(stored));
    } catch {
      // Keep the memory cache useful even when localStorage is full or blocked.
    }
  }

  delete(url: string): void {
    this.memory.delete(url);
    try {
      this.storage()?.removeItem(this.keyFor(url));
    } catch {
      // Deleting a cache entry is best-effort.
    }
  }

  clearMemory(): void {
    this.memory.clear();
  }

  private storage(): ChromaStorage | null {
    if (this.storageOverride !== undefined) return this.storageOverride;
    try {
      return typeof globalThis.localStorage === "undefined" ? null : globalThis.localStorage;
    } catch {
      return null;
    }
  }
}

export const adaptiveChromaPaletteCache = new AdaptiveChromaPaletteCache();
