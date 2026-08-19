// api 域模块：settings（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";
import type { ImportPreviewCandidate, Settings } from "./types";

export async function getSettings(): Promise<Settings> {
  return invokeCmd("get_settings");
}


export async function updateSettings(settings: Settings): Promise<Settings> {
  return invokeCmd("update_settings", { settings });
}


export async function getAppCacheStats(): Promise<import("./types").AppCacheStats> {
  return invokeCmd("get_app_cache_stats");
}


export async function clearAppCache(): Promise<import("./types").CacheClearResult> {
  return invokeCmd("clear_app_cache");
}


export async function restoreDefaultSettings(): Promise<Settings> {
  return invokeCmd("restore_default_settings");
}


export async function addWatchDir(dir: string): Promise<Settings> {
  return invokeCmd("add_watch_dir", { dir });
}


export async function removeWatchDir(dir: string): Promise<Settings> {
  return invokeCmd("remove_watch_dir", { dir });
}


export async function pickDirectory(): Promise<string> {
  return invokeCmd("pick_directory");
}


export async function pickImageFile(): Promise<string> {
  return invokeCmd("pick_image_file");
}


export async function scanDirectoryForGames(dir: string): Promise<{ imported: number; skipped: number }> {
  return invokeCmd("scan_directory_for_games", { dir });
}


export async function previewDirectoryForGames(dir: string): Promise<ImportPreviewCandidate[]> {
  return invokeCmd("preview_directory_for_games", { dir });
}


export async function importSelectedCandidates(paths: string[]): Promise<{ imported: number; skipped: number }> {
  return invokeCmd("import_selected_candidates", { paths });
}

// ===== 数据库信息 =====


export async function getSchemaVersion(): Promise<number> {
  return invokeCmd("get_schema_version");
}


export async function getGameCount(): Promise<number> {
  return invokeCmd("get_game_count");
}

// ===== P1 增强体验 =====

