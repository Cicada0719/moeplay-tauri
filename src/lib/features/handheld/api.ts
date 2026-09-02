// 安卓掌机（ES-DE 式）API 封装。
// 原生侧命令分两类：
//   - plugin:handheld|*  —— Kotlin HandheldPlugin（模拟器探测 / Intent 启动 / 存储授权）
//   - handheld_*         —— Rust 命令（ROM 目录扫描 / 平台推断 / 批量入库）

import { invoke } from "@tauri-apps/api/core";
import { invokeCmd } from "../../api/core";

export interface InstalledEmulator {
  packageName: string;
  label: string;
  versionName: string;
  platforms: string[];
}

export interface RomRootInfo {
  path: string;
  label: string;
  exists: boolean;
}

export interface ScannedRom {
  path: string;
  name: string;
  platform: string;
  sizeBytes: number;
  cover: string | null;
  /** ES-DE gamelist 中的最后游玩时间（"YYYY-MM-DD HH:MM"） */
  lastPlayed: string | null;
}

export interface ImportRomInput {
  path: string;
  platform: string;
  name?: string;
  /** 该 ROM 指定的启动模拟器包名（缺省用顶层 emulatorPackage 兜底） */
  emulatorPackage?: string;
  lastPlayed?: string | null;
  /** 扫描阶段匹配到的封面路径（含 ES-DE downloaded_media 结果） */
  cover?: string | null;
}

export interface ImportRomsResult {
  ok: number;
  fail: number;
}

export async function listInstalledEmulators(): Promise<InstalledEmulator[]> {
  const res = await invoke<{ emulators: InstalledEmulator[] }>("plugin:handheld|list_emulators");
  return res.emulators;
}

export async function hasAllFilesAccess(): Promise<boolean> {
  const res = await invoke<{ granted: boolean }>("plugin:handheld|has_all_files_access");
  return res.granted;
}

/** 跳转到系统「所有文件访问」设置页；授权结果需稍后重新查询。 */
export async function requestAllFilesAccess(): Promise<boolean> {
  const res = await invoke<{ granted: boolean }>("plugin:handheld|request_all_files_access");
  return res.granted;
}

export function handheldRomRoots(): Promise<RomRootInfo[]> {
  return invokeCmd<RomRootInfo[]>("handheld_rom_roots");
}

/** 通过 StorageManager 枚举存储卷上的 ROM 目录（绕过部分设备 /storage 根目录不可列举的限制）。 */
export async function discoverRomRoots(): Promise<RomRootInfo[]> {
  const res = await invoke<{ roots: { path: string; label: string }[] }>(
    "plugin:handheld|discover_rom_roots",
  );
  return res.roots.map((r) => ({ ...r, exists: true }));
}

/** 掌机沉浸式显示：隐藏/恢复 Android 状态栏与底部导航栏。 */
export async function setHandheldSystemBars(immersive: boolean): Promise<boolean> {
  const res = await invoke<{ immersive: boolean }>("plugin:handheld|set_system_bars", {
    request: { immersive },
  });
  return res.immersive;
}

export function handheldScanRoms(dir: string): Promise<ScannedRom[]> {
  return invokeCmd<ScannedRom[]>("handheld_scan_roms", { dir });
}

export function handheldImportRoms(
  emulatorPackage: string,
  roms: ImportRomInput[],
): Promise<ImportRomsResult> {
  return invokeCmd<ImportRomsResult>("handheld_import_roms", {
    emulatorPackage,
    roms,
  });
}
