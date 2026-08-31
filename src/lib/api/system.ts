// api 域模块：system（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";

export interface AutostartStatus {
  enabled: boolean;
  startup_mode: string;
  exe_path: string;
}

/// 设置开机自动启动

export async function setAutostart(enabled: boolean, startupMode: string): Promise<string> {
  return invokeCmd("set_autostart", { enabled, startupMode });
}

/// 获取当前开机自启状态

export async function getAutostartStatus(): Promise<AutostartStatus> {
  return invokeCmd("get_autostart_status");
}


// ===== 二次元主题与壁纸 =====

export async function listThemePacks(): Promise<import("./types").ThemePackSummary[]> { return invokeCmd("list_theme_packs"); }

export async function listWallpapers(themePack?: string): Promise<import("./types").WallpaperRecord[]> { return invokeCmd("list_wallpapers", { themePack }); }

export async function refreshWallpaperManifest(nsfwMode = "blur"): Promise<import("./types").WallpaperSyncResult> { return invokeCmd("refresh_wallpaper_manifest", { nsfwMode }); }

export async function downloadWallpaper(id: string, nsfwMode = "blur"): Promise<import("./types").WallpaperRecord> { return invokeCmd("download_wallpaper", { id, nsfwMode }); }

export async function deleteWallpaper(id: string): Promise<void> { return invokeCmd("delete_wallpaper", { id }); }

export async function setActiveAppearance(settings: import("./types").AppearanceSettings): Promise<import("./types").AppearanceSettings> { return invokeCmd("set_active_appearance", { settings }); }

export async function getWallpaperAttribution(id: string): Promise<import("./types").WallpaperAttribution> { return invokeCmd("get_wallpaper_attribution", { id }); }

