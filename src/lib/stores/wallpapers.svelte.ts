import { downloadWallpaper, listWallpapers, refreshWallpaperManifest, type WallpaperRecord } from "../api";
import type { AppearanceSettings } from "../api/types";

let records = $state<WallpaperRecord[]>([]);
let syncing = $state(false);
let initialized = false;

function ratingAllowed(rating: string, mode: string): boolean {
  if (mode === "hide") return rating === "general";
  return true;
}

export const wallpaperStore = {
  get records() { return records; },
  get syncing() { return syncing; },
  installedFor(themePack: string, nsfwMode = "blur") {
    const available = Array.isArray(records) ? records : [];
    return available.filter((record) => record.installed && record.local_path && record.asset.theme_pack === themePack && ratingAllowed(record.asset.rating, nsfwMode));
  },
  async load(themePack?: string) {
    try {
      const result = await listWallpapers(themePack);
      records = Array.isArray(result) ? result : [];
    } catch {
      records = [];
    }
  },
  async initialize(appearance: AppearanceSettings, nsfwMode = "blur") {
    if (initialized) return;
    initialized = true;
    await this.load();
    if (!appearance.online_gallery_enabled) return;
    syncing = true;
    try {
      await refreshWallpaperManifest(nsfwMode);
      await this.load();
      const candidates = records.filter((record) => !record.installed && record.asset.theme_pack === appearance.theme_pack && ratingAllowed(record.asset.rating, nsfwMode)).slice(0, 2);
      for (const candidate of candidates) {
        try { await downloadWallpaper(candidate.asset.id, nsfwMode); } catch { /* keep built-ins */ }
      }
      await this.load();
    } catch { /* offline startup remains quiet */ }
    finally { syncing = false; }
  },
};
