// api 域模块：emulators（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";

export interface ScannedEmulator {
  id: string;
  name: string;
  install_dir: string;
  executable: string;
  profiles: ScannedProfile[];
}


export interface ScannedProfile {
  profile_name: string;
  platform_ids: string[];
  image_extensions: string[];
  startup_arguments: string | null;
}


export interface RomFile {
  path: string;
  filename: string;
  name: string;
  extension: string;
  size_bytes: number;
  platform: string | null;
}

/// 扫描已安装的模拟器

export async function searchEmulators(searchPaths: string[]): Promise<ScannedEmulator[]> {
  return invokeCmd("search_emulators", { searchPaths });
}

/// 扫描 ROM 文件

export async function scanRoms(dir: string, extensions: string[], recursive?: boolean): Promise<RomFile[]> {
  return invokeCmd("scan_roms", { dir, extensions, recursive });
}

/// 导入 ROM 游戏（关联模拟器启动）

export async function importRomGame(
  name: string, romPath: string, emulatorExe: string,
  startupArgs: string, platform: string, coverUrl?: string,
): Promise<any> {
  return invokeCmd("import_rom_game", { name, romPath, emulatorExe, startupArgs, platform, coverUrl });
}

// ===== 开机自启管理 =====

