import { describe, expect, it, vi } from "vitest";
import type { AdaptiveChromaPalette } from "../model/chromaTypes";
import { calculateSampleSize, resolvePaletteWithCache } from "./imagePalette";
import { AdaptiveChromaPaletteCache, type ChromaStorage } from "./paletteCache";

const MEDIA_PALETTE: AdaptiveChromaPalette = {
  primary: { r: 210, g: 40, b: 80 },
  secondary: { r: 30, g: 90, b: 170 },
  accent: { r: 245, g: 120, b: 145 },
  surface: { r: 18, g: 20, b: 26 },
  foreground: { r: 248, g: 247, b: 244 },
  isDark: true,
  source: "media",
};

const FALLBACK_PALETTE: AdaptiveChromaPalette = {
  ...MEDIA_PALETTE,
  source: "fallback",
};

class MemoryStorage implements ChromaStorage {
  readonly values = new Map<string, string>();

  getItem(key: string) {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string) {
    this.values.set(key, value);
  }

  removeItem(key: string) {
    this.values.delete(key);
  }
}

describe("calculateSampleSize", () => {
  it("preserves aspect ratio and limits the longest edge to 32 pixels", () => {
    expect(calculateSampleSize(1920, 1080)).toEqual({ width: 32, height: 18 });
    expect(calculateSampleSize(800, 1200)).toEqual({ width: 21, height: 32 });
  });

  it("does not upscale small images and never returns a zero dimension", () => {
    expect(calculateSampleSize(16, 8)).toEqual({ width: 16, height: 8 });
    expect(calculateSampleSize(0, 0)).toEqual({ width: 1, height: 1 });
  });
});

describe("AdaptiveChromaPaletteCache", () => {
  it("uses a URL-derived versioned key and restores a persisted palette", () => {
    const storage = new MemoryStorage();
    const first = new AdaptiveChromaPaletteCache({ storage, version: 7, namespace: "test-chroma" });
    const url = "https://cdn.example.test/cover art.jpg?v=2";

    first.set(url, MEDIA_PALETTE);
    expect(first.keyFor(url)).toBe(`test-chroma:v7:${encodeURIComponent(url)}`);

    const second = new AdaptiveChromaPaletteCache({ storage, version: 7, namespace: "test-chroma" });
    expect(second.get(url)).toEqual(MEDIA_PALETTE);
  });

  it("ignores and removes malformed persisted values", () => {
    const storage = new MemoryStorage();
    const cache = new AdaptiveChromaPaletteCache({ storage });
    const url = "asset://localhost/broken.png";
    storage.setItem(cache.keyFor(url), "{not-json");

    expect(cache.get(url)).toBeNull();
    expect(storage.getItem(cache.keyFor(url))).toBeNull();
  });

  it("keeps working when localStorage access fails", () => {
    const storage: ChromaStorage = {
      getItem: () => { throw new Error("blocked"); },
      setItem: () => { throw new Error("full"); },
      removeItem: () => { throw new Error("blocked"); },
    };
    const cache = new AdaptiveChromaPaletteCache({ storage });

    expect(() => cache.set("tauri://cover", MEDIA_PALETTE)).not.toThrow();
    expect(cache.get("tauri://cover")).toEqual(MEDIA_PALETTE);
  });
});

describe("resolvePaletteWithCache", () => {
  it("reuses successful extraction without invoking the extractor again", async () => {
    const cache = new AdaptiveChromaPaletteCache({ storage: null });
    const extract = vi.fn(async () => MEDIA_PALETTE);

    await expect(resolvePaletteWithCache("cover-a", extract, cache)).resolves.toEqual(MEDIA_PALETTE);
    await expect(resolvePaletteWithCache("cover-a", extract, cache)).resolves.toEqual(MEDIA_PALETTE);
    expect(extract).toHaveBeenCalledTimes(1);
  });

  it("does not pollute the cache after a rejected extraction", async () => {
    const cache = new AdaptiveChromaPaletteCache({ storage: null });
    const failure = vi.fn(async () => { throw new Error("CORS"); });

    await expect(resolvePaletteWithCache("cover-b", failure, cache)).rejects.toThrow("CORS");
    expect(cache.get("cover-b")).toBeNull();

    const retry = vi.fn(async () => MEDIA_PALETTE);
    await expect(resolvePaletteWithCache("cover-b", retry, cache)).resolves.toEqual(MEDIA_PALETTE);
    expect(retry).toHaveBeenCalledOnce();
  });

  it("returns extraction fallback without caching it", async () => {
    const cache = new AdaptiveChromaPaletteCache({ storage: null });
    const extract = vi.fn(async () => FALLBACK_PALETTE);

    await expect(resolvePaletteWithCache("cover-c", extract, cache)).resolves.toEqual(FALLBACK_PALETTE);
    await expect(resolvePaletteWithCache("cover-c", extract, cache)).resolves.toEqual(FALLBACK_PALETTE);
    expect(extract).toHaveBeenCalledTimes(2);
  });
});
