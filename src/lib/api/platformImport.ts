// api 域模块：platformImport（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";

export interface ImportedGame {
  name: string;
  install_path: string;
  platform: string;
  app_id: string | null;
  cover_url: string | null;
}


export type PlatformImportSource = "steam" | "epic";

export type PlatformImportMode = "local" | "account" | "combined";


export interface PlatformImportStatus {
  steam_path?: string | null;
  steam_id?: string | null;
  has_steam_api_key: boolean;
  steam_api_key_validated: boolean;
  steam_can_sync_account: boolean;
  epic_manifest_path?: string | null;
  epic_manifest_available: boolean;
}


export interface PlatformGameCandidate {
  source: PlatformImportSource | string;
  library_id: string;
  name: string;
  install_dir?: string | null;
  launch_uri: string;
  cover_url?: string | null;
  icon_url?: string | null;
  store_url?: string | null;
  playtime_minutes?: number | null;
  last_played?: string | null;
  achievements_total?: number | null;
  achievements_unlocked?: number | null;
  installed: boolean;
  selected: boolean;
  skip_reason?: string | null;
}


export interface PlatformScanResult {
  source: PlatformImportSource | string;
  mode: PlatformImportMode | string;
  candidates: PlatformGameCandidate[];
  skipped: string[];
  errors: string[];
}


export interface PlatformImportResult {
  source: PlatformImportSource | string;
  imported: number;
  updated: number;
  skipped: number;
  failed: number;
  total: number;
  imported_ids: string[];
  updated_ids: string[];
  skipped_reasons: string[];
  errors: string[];
}


export async function getPlatformImportStatus(): Promise<PlatformImportStatus> {
  return invokeCmd("get_platform_import_status");
}


export async function resolveSteamId(input: string): Promise<SteamLoginResult> {
  return invokeCmd("resolve_steam_id", { input });
}


export async function validateSteamApiKey(apiKey: string): Promise<string> {
  return invokeCmd("validate_steam_api_key", { apiKey });
}


export async function steamLoginOpenid(): Promise<string> {
  return invokeCmd("steam_login_openid");
}


export async function scanPlatformLibrary(
  source: PlatformImportSource,
  mode: PlatformImportMode,
  steamId?: string,
): Promise<PlatformScanResult> {
  return invokeCmd("scan_platform_library", { source, mode, steamId });
}


export async function importPlatformLibrary(
  source: PlatformImportSource,
  candidates: PlatformGameCandidate[],
): Promise<PlatformImportResult> {
  return invokeCmd("import_platform_library", { source, candidates });
}

/// 从已登录 WebView 会话抓取到的 Steam 全库（Playnite 式，无需 API Key）

export interface SteamSessionGame {
  appid: number;
  name: string;
  playtime_forever: number;
  last_played: number;
}


export async function importSteamSessionGames(
  games: SteamSessionGame[],
): Promise<PlatformImportResult> {
  return invokeCmd("import_steam_session_games", { games });
}


export interface SyncAchievementsResult {
  synced: number;
  skipped: number;
  failed: number;
  errors: string[];
}


export async function syncSteamAchievements(): Promise<SyncAchievementsResult> {
  return invokeCmd("sync_steam_achievements");
}

/// 发现本地 Steam 安装路径

export async function findSteamPath(): Promise<string | null> {
  return invokeCmd("find_steam_path");
}

/// 扫描 Steam 库中已安装的游戏

export async function scanSteamLibrary(): Promise<ImportedGame[]> {
  return invokeCmd("scan_steam_library");
}

/// 扫描 Epic 库中已安装的游戏

export async function scanEpicLibrary(): Promise<ImportedGame[]> {
  return invokeCmd("scan_epic_library");
}

/// 导入单个 Steam/Epic 游戏到本地库

export async function importSteamGame(
  name: string,
  installPath: string,
  appId?: string,
  coverUrl?: string,
  platform?: string,
): Promise<any> {
  return invokeCmd("import_steam_game", { name, installPath, appId, coverUrl, platform });
}

// ===== M6 自动入库刮削 =====


export interface PipelineState {
  stage: string;
  current: number;
  total: number;
  detected: string[];
  imported: number;
  updated: number;
  skipped: number;
  errors: string[];
}

/// 对指定目录运行完整自动入库管线

export async function runAutoScrapePipeline(dir: string, autoScrape?: boolean): Promise<PipelineState> {
  return invokeCmd("run_auto_scrape_pipeline", { dir, autoScrape });
}

// ===== M6 Steam 身份认证 + Web API =====


export interface SteamLoginResult {
  steam_id: string;
  personaname: string;
  avatar: string;
  profile_url: string;
  login_method: string;
}


export interface SteamOwnedGame {
  app_id: number;
  name: string;
  playtime_forever: number;
  playtime_2weeks: number | null;
  rtime_last_played: number | null;
  img_icon_url: string | null;
  img_logo_url: string | null;
  achievements_total: number | null;
  achievements_unlocked: number | null;
}


export interface SteamOwnedGamesResponse {
  game_count: number;
  games: SteamOwnedGame[];
  imported_count?: number;
  updated_count?: number;
  skipped_count?: number;
}

/// 方式 A: 在浏览器打开 Steam 社区（用户手动获取 SteamID）

export async function steamOpenCommunity(mode?: string): Promise<string> {
  return invokeCmd("steam_open_community", { mode });
}

/// 方式 D: 【推荐】在 App 内嵌 WebView 打开 Steam 登录（支持扫码）

export async function steamLoginWebview(): Promise<string> {
  return invokeCmd("steam_login_webview");
}

/// 方式 B: 从粘贴的 URL 解析 SteamID64（推荐，100% 可靠）

export async function steamResolveUrl(url: string): Promise<SteamLoginResult> {
  return invokeCmd("steam_resolve_url", { url });
}

/// 方式 C: 尝试 OpenID 一键登录（部分网络可能被拦截）

export async function steamOpenidLogin(): Promise<SteamLoginResult> {
  return invokeCmd("steam_openid_login");
}

/// 验证 Steam API Key 是否有效

export async function steamVerifyApiKey(apiKey: string): Promise<string> {
  return invokeCmd("steam_verify_api_key", { apiKey });
}

/// 检测本地 Steam 客户端是否已登录，返回 SteamID64 或 null

export async function steamDetectLocal(): Promise<string | null> {
  return invokeCmd("steam_detect_local");
}

/// 一步完成：获取+导入 Steam 全库游戏

export async function steamFetchAndImport(steamId: string): Promise<SteamOwnedGamesResponse> {
  return invokeCmd("steam_fetch_and_import", { steamId });
}

/// 批量导入 Steam 全库游戏

export async function steamImportOwnedGames(games: SteamOwnedGame[]): Promise<SteamOwnedGamesResponse> {
  return invokeCmd("steam_import_owned_games", { games });
}

// ===== 模拟器检测与 ROM 导入 =====

