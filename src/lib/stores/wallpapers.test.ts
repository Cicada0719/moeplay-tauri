import { beforeEach, describe, expect, it, vi } from "vitest";
import type { WallpaperRecord } from "../api/types";

const wallpaperApi = vi.hoisted(() => ({
  downloadWallpaper: vi.fn(),
  listWallpapers: vi.fn(),
  refreshWallpaperManifest: vi.fn(),
}));

vi.mock("../api", () => wallpaperApi);

function record({
  id,
  themePack = "phantom-pop",
  rating = "general",
  installed = false,
}: {
  id: string;
  themePack?: string;
  rating?: "general" | "suggestive" | "adult";
  installed?: boolean;
}): WallpaperRecord {
  return {
    asset: {
      id,
      theme_pack: themePack,
      title: id,
      download_url: `https://assets.example/${id}.webp`,
      preview_url: `https://assets.example/${id}-preview.webp`,
      sha256: "0".repeat(64),
      byte_size: 1_024,
      width: 2_560,
      height: 1_440,
      rating,
      author: "fixture",
      source_url: "https://assets.example/source",
      license_id: "fixture-license",
      license_url: "https://assets.example/license",
      attribution_required: true,
    },
    installed,
    local_path: installed ? `C:/wallpapers/${id}.webp` : undefined,
    source: installed ? "builtin" : "remote",
  };
}

async function loadStore() {
  const module = await import("./wallpapers.svelte");
  return module.wallpaperStore;
}

beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
  wallpaperApi.listWallpapers.mockResolvedValue([]);
  wallpaperApi.refreshWallpaperManifest.mockResolvedValue({ revision: "fixture", available: 0, downloaded: 0 });
  wallpaperApi.downloadWallpaper.mockResolvedValue(undefined);
});

describe("wallpaperStore defensive loading", () => {
  it("treats a null backend payload as an empty wallpaper list", async () => {
    const store = await loadStore();
    wallpaperApi.listWallpapers.mockResolvedValue(null);

    await expect(store.load()).resolves.toBeUndefined();

    expect(store.records).toEqual([]);
    expect(store.installedFor("phantom-pop")).toEqual([]);
  });
});

describe("wallpaperStore content rating", () => {
  it("keeps installed general wallpapers in hide mode and permits suggestive/adult wallpapers for blur or show", async () => {
    const store = await loadStore();
    wallpaperApi.listWallpapers.mockResolvedValue([
      record({ id: "general", installed: true }),
      record({ id: "suggestive", rating: "suggestive", installed: true }),
      record({ id: "adult", rating: "adult", installed: true }),
      record({ id: "other-pack", themePack: "shift-editorial", installed: true }),
      record({ id: "not-local", installed: true, ...{} }),
    ]);

    await store.load();

    expect(store.installedFor("phantom-pop", "hide").map((item) => item.asset.id)).toEqual(["general", "not-local"]);
    expect(store.installedFor("phantom-pop", "blur").map((item) => item.asset.id)).toEqual([
      "general",
      "suggestive",
      "adult",
      "not-local",
    ]);
    expect(store.installedFor("phantom-pop", "show").map((item) => item.asset.id)).toEqual([
      "general",
      "suggestive",
      "adult",
      "not-local",
    ]);
  });
});

describe("wallpaperStore background sync fallback", () => {
  it("retains already loaded wallpapers and exits syncing quietly when the manifest is unavailable", async () => {
    const store = await loadStore();
    const installed = record({ id: "builtin:phantom-pop:1", installed: true });
    wallpaperApi.listWallpapers.mockResolvedValue([installed]);
    wallpaperApi.refreshWallpaperManifest.mockRejectedValue(new Error("offline"));

    await expect(store.initialize({
      theme_pack: "phantom-pop",
      color_mode: "pack-default",
      wallpaper_rotation: "startup-random",
      mascot_enabled: true,
      decorative_effects: true,
      online_gallery_enabled: true,
    })).resolves.toBeUndefined();

    expect(wallpaperApi.refreshWallpaperManifest).toHaveBeenCalledOnce();
    expect(wallpaperApi.downloadWallpaper).not.toHaveBeenCalled();
    expect(store.syncing).toBe(false);
    expect(store.installedFor("phantom-pop").map((item) => item.asset.id)).toEqual([installed.asset.id]);
  });

  it("continues after individual download failures and never adds failed downloads to the installed pool", async () => {
    const store = await loadStore();
    const candidates = [
      record({ id: "general-remote" }),
      record({ id: "suggestive-remote", rating: "suggestive" }),
    ];
    wallpaperApi.listWallpapers.mockResolvedValue(candidates);
    wallpaperApi.downloadWallpaper.mockRejectedValue(new Error("network interrupted"));

    await expect(store.initialize({
      theme_pack: "phantom-pop",
      color_mode: "pack-default",
      wallpaper_rotation: "startup-random",
      mascot_enabled: true,
      decorative_effects: true,
      online_gallery_enabled: true,
    }, "blur")).resolves.toBeUndefined();

    expect(wallpaperApi.downloadWallpaper).toHaveBeenCalledTimes(2);
    expect(wallpaperApi.downloadWallpaper).toHaveBeenNthCalledWith(1, "general-remote", "blur");
    expect(wallpaperApi.downloadWallpaper).toHaveBeenNthCalledWith(2, "suggestive-remote", "blur");
    expect(store.syncing).toBe(false);
    expect(store.installedFor("phantom-pop", "blur")).toEqual([]);
  });

  it("does not begin remote synchronization when the online gallery is disabled", async () => {
    const store = await loadStore();

    await store.initialize({
      theme_pack: "phantom-pop",
      color_mode: "pack-default",
      wallpaper_rotation: "startup-random",
      mascot_enabled: true,
      decorative_effects: true,
      online_gallery_enabled: false,
    });

    expect(wallpaperApi.listWallpapers).toHaveBeenCalledOnce();
    expect(wallpaperApi.refreshWallpaperManifest).not.toHaveBeenCalled();
    expect(wallpaperApi.downloadWallpaper).not.toHaveBeenCalled();
    expect(store.syncing).toBe(false);
  });
});