// api 域模块：saves（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";
import type { SaveInfo, SaveCandidateDir, SaveSnapshot, SnapshotDiff, SaveConflict, CloudSyncConfig } from "./types";

export async function getGameSaves(gameId: string): Promise<SaveInfo[]> {
  return invokeCmd("get_game_saves", { gameId });
}


export async function backupSave(savePath: string): Promise<string> {
  return invokeCmd("backup_save", { savePath });
}


export async function restoreSave(
  backupPath: string,
  targetPath: string
): Promise<void> {
  return invokeCmd("restore_save", { backupPath, targetPath });
}


export async function detectSaveCandidates(gameId: string): Promise<SaveCandidateDir[]> {
  return invokeCmd("detect_save_candidates", { gameId });
}


export async function scanSaveDir(saveDir: string): Promise<SaveInfo[]> {
  return invokeCmd("scan_save_dir", { saveDir });
}


export async function createSaveSnapshot(
  gameId: string,
  saveDir: string | null = null,
  note: string | null = null
): Promise<SaveSnapshot> {
  return invokeCmd("create_save_snapshot", { gameId, saveDir, note });
}


export async function listSaveSnapshots(gameId: string): Promise<SaveSnapshot[]> {
  return invokeCmd("list_save_snapshots", { gameId });
}


export async function restoreSaveSnapshot(
  gameId: string,
  snapshotPath: string,
  saveDir: string | null = null,
  createSafety = true
): Promise<void> {
  return invokeCmd("restore_save_snapshot", {
    gameId,
    snapshotPath,
    saveDir,
    createSafety,
  });
}


export async function deleteSaveSnapshot(snapshotPath: string): Promise<void> {
  return invokeCmd("delete_save_snapshot", { snapshotPath });
}


export async function compareSaveSnapshot(
  snapshotPath: string,
  saveDir: string
): Promise<SnapshotDiff> {
  return invokeCmd("compare_save_snapshot", { snapshotPath, saveDir });
}


export async function detectSaveConflicts(
  localDir: string,
  remoteDir: string
): Promise<SaveConflict[]> {
  return invokeCmd("detect_save_conflicts", { localDir, remoteDir });
}


export async function syncSaveSnapshotsToCloud(
  gameId: string,
  config: CloudSyncConfig
): Promise<number> {
  return invokeCmd("sync_save_snapshots_to_cloud", { gameId, config });
}


export async function restoreLatestSaveSnapshotFromCloud(
  gameId: string,
  cloudDir: string,
  saveDir: string | null = null
): Promise<SaveSnapshot | null> {
  return invokeCmd("restore_latest_save_snapshot_from_cloud", { gameId, cloudDir, saveDir });
}

// ===== NSFW / 翻译 =====

