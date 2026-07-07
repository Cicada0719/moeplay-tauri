// 萌游 MoeGame · API 调用入口（Phase A 拆分类型与核心调用）

import type { CompletionStatus, TagCategory, TagSource, GamePlatform, StoreLink, GameAlias, Tag, GameMetadata, PlaySession, PlaySessionEntry, DailyPlaytime, MonthlyPlaytime, GamePlaytimeRank, PlaytimeSummary, PlayTracker, SaveBackup, SaveData, Game, ImportPreviewCandidate, ScrapeResult, ScrapeSourceStatus, ScrapeResponse, ScrapeDetail, SaveInfo, SaveCandidateDir, SaveSnapshot, SnapshotDiff, SaveConflict, CloudProvider, CloudSyncConfig, Settings, NsfwDisplayMode, NsfwDecision, ChineseMeta, ScrapeMarker, Recommendation, MonthActivity, Collection, DashboardData, ThumbnailInfo, TaskStatus, AppTask, MigrationInfo, ImageCandidate, PerformanceSnapshot, Severity, Issue, SystemInfo, AppInfo, DiagnosticsReport, DownloadStatus, DownloadTask } from "./types";
export type * from "./types";

import { invokeCmd } from "./core";

// ===== 游戏查询 =====

export async function getGames(): Promise<Game[]> {
  return invokeCmd("get_games");
}

export async function getGame(id: string): Promise<Game> {
  return invokeCmd("get_game", { id });
}

export async function searchGames(query: string): Promise<Game[]> {
  return invokeCmd("search_games", { query });
}

// ===== 游戏增删改 =====

export async function addGameByDialog(): Promise<Game> {
  return invokeCmd("add_game_by_dialog");
}

export async function addGameByPath(path: string): Promise<Game> {
  return invokeCmd("add_game_by_path", { path });
}

export async function deleteGame(id: string): Promise<void> {
  return invokeCmd("delete_game", { id });
}

export async function updateGame(game: Game): Promise<Game> {
  return invokeCmd("update_game", { game });
}

export async function importGamesFromDir(dir: string): Promise<Game[]> {
  return invokeCmd("import_games_from_dir", { dir });
}

// ===== 基本信息更新 =====

export async function updateGameName(id: string, name: string): Promise<Game> {
  return invokeCmd("update_game_name", { id, name });
}

export async function updateGameDescription(
  id: string,
  description: string | null
): Promise<Game> {
  return invokeCmd("update_game_description", { id, description });
}

export async function updateGameCover(id: string, cover: string | null): Promise<Game> {
  return invokeCmd("update_game_cover", { id, cover });
}

export async function updateGameBackground(
  id: string,
  background: string | null
): Promise<Game> {
  return invokeCmd("update_game_background", { id, background });
}

export async function updateGameIcon(id: string, icon: string | null): Promise<Game> {
  return invokeCmd("update_game_icon", { id, icon });
}

export async function updateGameType(
  id: string,
  gameType: string | null
): Promise<Game> {
  return invokeCmd("update_game_type", { id, gameType });
}

export async function updateInstallDir(
  id: string,
  installDir: string | null
): Promise<Game> {
  return invokeCmd("update_install_dir", { id, installDir });
}

export async function updateExePath(id: string, exePath: string): Promise<Game> {
  return invokeCmd("update_exe_path", { id, exePath });
}

// ===== 快捷切换 =====

export async function toggleFavorite(id: string): Promise<Game> {
  return invokeCmd("toggle_favorite", { id });
}

export async function toggleHidden(id: string): Promise<Game> {
  return invokeCmd("toggle_hidden", { id });
}

// ===== 简单标签 =====

export async function addSimpleTag(id: string, tag: string): Promise<Game> {
  return invokeCmd("add_simple_tag", { id, tag });
}

export async function removeSimpleTag(id: string, tag: string): Promise<Game> {
  return invokeCmd("remove_simple_tag", { id, tag });
}

export async function setSimpleTags(id: string, tags: string[]): Promise<Game> {
  return invokeCmd("set_simple_tags", { id, tags });
}

// ===== 增强标签 =====

export async function addTagEntry(id: string, tag: Tag): Promise<Game> {
  return invokeCmd("add_tag_entry", { id, tag });
}

export async function removeTagEntry(id: string, tagName: string): Promise<Game> {
  return invokeCmd("remove_tag_entry", { id, tagName });
}

export async function updateTagEntry(
  id: string,
  tagName: string,
  tag: Tag
): Promise<Game> {
  return invokeCmd("update_tag_entry", { id, tagName, tag });
}

export async function setTagEntries(id: string, tags: Tag[]): Promise<Game> {
  return invokeCmd("set_tag_entries", { id, tags });
}

// ===== 别名 =====

export async function addGameAlias(id: string, alias: GameAlias): Promise<Game> {
  return invokeCmd("add_game_alias", { id, alias });
}

export async function removeGameAlias(id: string, aliasName: string): Promise<Game> {
  return invokeCmd("remove_game_alias", { id, aliasName });
}

export async function setPrimaryAlias(id: string, aliasName: string): Promise<Game> {
  return invokeCmd("set_primary_alias", { id, aliasName });
}

export async function setGameAliases(id: string, aliases: GameAlias[]): Promise<Game> {
  return invokeCmd("set_game_aliases", { id, aliases });
}

// ===== 元数据 =====

export async function updateGameMetadata(
  id: string,
  metadata: GameMetadata
): Promise<Game> {
  return invokeCmd("update_game_metadata", { id, metadata });
}

export async function updateDeveloper(
  id: string,
  developer: string | null
): Promise<Game> {
  return invokeCmd("update_developer", { id, developer });
}

export async function updatePublisher(
  id: string,
  publisher: string | null
): Promise<Game> {
  return invokeCmd("update_publisher", { id, publisher });
}

export async function updateEngine(id: string, engine: string | null): Promise<Game> {
  return invokeCmd("update_engine", { id, engine });
}

export async function updateGameVersion(
  id: string,
  version: string | null
): Promise<Game> {
  return invokeCmd("update_game_version", { id, version });
}

export async function updateOriginalName(
  id: string,
  originalName: string | null
): Promise<Game> {
  return invokeCmd("update_original_name", { id, originalName });
}

export async function updateHomepage(
  id: string,
  homepage: string | null
): Promise<Game> {
  return invokeCmd("update_homepage", { id, homepage });
}

export async function updateDeveloperHomepage(
  id: string,
  homepage: string | null
): Promise<Game> {
  return invokeCmd("update_developer_homepage", { id, homepage });
}

export async function updateAgeRating(
  id: string,
  ageRating: string | null
): Promise<Game> {
  return invokeCmd("update_age_rating", { id, ageRating });
}

export async function updateSeries(id: string, series: string | null): Promise<Game> {
  return invokeCmd("update_series", { id, series });
}

export async function updateReleaseDate(
  id: string,
  releaseDate: string | null
): Promise<Game> {
  return invokeCmd("update_release_date", { id, releaseDate });
}

export async function updateReleaseYear(
  id: string,
  releaseYear: number | null
): Promise<Game> {
  return invokeCmd("update_release_year", { id, releaseYear });
}

export async function updateEstimatedHours(
  id: string,
  hours: number | null
): Promise<Game> {
  return invokeCmd("update_estimated_hours", { id, hours });
}

export async function updateVndbRating(
  id: string,
  rating: number | null
): Promise<Game> {
  return invokeCmd("update_vndb_rating", { id, rating });
}

export async function updateBangumiRating(
  id: string,
  rating: number | null
): Promise<Game> {
  return invokeCmd("update_bangumi_rating", { id, rating });
}

export async function updateVndbId(
  id: string,
  vndbId: string | null
): Promise<Game> {
  return invokeCmd("update_vndb_id", { id, vndbId });
}

export async function updateBangumiId(
  id: string,
  bangumiId: string | null
): Promise<Game> {
  return invokeCmd("update_bangumi_id", { id, bangumiId });
}

export async function setGenres(id: string, genres: string[]): Promise<Game> {
  return invokeCmd("set_genres", { id, genres });
}

export async function setLanguages(id: string, languages: string[]): Promise<Game> {
  return invokeCmd("set_languages", { id, languages });
}

export async function setVoiceLanguages(
  id: string,
  voiceLanguages: string[]
): Promise<Game> {
  return invokeCmd("set_voice_languages", { id, voiceLanguages });
}

// ===== 游玩追踪 =====

export async function updatePlayTracker(
  id: string,
  tracker: PlayTracker
): Promise<Game> {
  return invokeCmd("update_play_tracker", { id, tracker });
}

export async function startPlaySession(id: string): Promise<string> {
  return invokeCmd("start_play_session", { id });
}

export async function endPlaySession(
  id: string,
  sessionId: string,
  durationSeconds: number
): Promise<Game> {
  return invokeCmd("end_play_session", { id, sessionId, durationSeconds });
}

export async function updateCompletionStatus(
  id: string,
  status: CompletionStatus
): Promise<Game> {
  return invokeCmd("update_completion_status", { id, status });
}

export async function updateUserRating(
  id: string,
  rating: number | null
): Promise<Game> {
  return invokeCmd("update_user_rating", { id, rating });
}

export async function updateReview(
  id: string,
  review: string | null
): Promise<Game> {
  return invokeCmd("update_review", { id, review });
}

export async function updateAchievements(
  id: string,
  total: number,
  unlocked: number
): Promise<Game> {
  return invokeCmd("update_achievements", { id, total, unlocked });
}

export async function markGameFinished(
  id: string,
  finished: boolean
): Promise<Game> {
  return invokeCmd("mark_game_finished", { id, finished });
}

export async function getPlaySessions(id: string): Promise<PlaySession[]> {
  return invokeCmd("get_play_sessions", { id });
}

export async function updatePlaySession(
  id: string,
  sessionId: string,
  session: PlaySession
): Promise<Game> {
  return invokeCmd("update_play_session", { id, sessionId, session });
}

export async function removePlaySession(id: string, sessionId: string): Promise<Game> {
  return invokeCmd("remove_play_session", { id, sessionId });
}

export async function setPlaySessions(
  id: string,
  sessions: PlaySession[]
): Promise<Game> {
  return invokeCmd("set_play_sessions", { id, sessions });
}

export async function updateTotalPlaytime(
  id: string,
  totalSeconds: number
): Promise<Game> {
  return invokeCmd("update_total_playtime", { id, totalSeconds });
}

export async function updateFirstPlayed(
  id: string,
  firstPlayed: string | null
): Promise<Game> {
  return invokeCmd("update_first_played", { id, firstPlayed });
}

export async function updateLastPlayed(
  id: string,
  lastPlayed: string | null
): Promise<Game> {
  return invokeCmd("update_last_played", { id, lastPlayed });
}

export async function updateCompletionCount(id: string, count: number): Promise<Game> {
  return invokeCmd("update_completion_count", { id, count });
}

export async function getRecentPlaySessions(
  days = 30,
  limit = 50
): Promise<PlaySessionEntry[]> {
  return invokeCmd("get_recent_play_sessions", { days, limit });
}

export async function getPlaytimeSummary(
  days = 30,
  months = 12,
  topLimit = 10
): Promise<PlaytimeSummary> {
  return invokeCmd("get_playtime_summary", { days, months, topLimit });
}

// ===== 截图 =====

export async function addScreenshot(id: string, path: string): Promise<Game> {
  return invokeCmd("add_screenshot", { id, path });
}

export async function removeScreenshot(id: string, index: number): Promise<Game> {
  return invokeCmd("remove_screenshot", { id, index });
}

export async function removeScreenshotByPath(
  id: string,
  path: string
): Promise<Game> {
  return invokeCmd("remove_screenshot_by_path", { id, path });
}

export async function setScreenshots(
  id: string,
  screenshots: string[]
): Promise<Game> {
  return invokeCmd("set_screenshots", { id, screenshots });
}

// ===== 存档数据 =====

export async function updateSaveData(id: string, saveData: SaveData): Promise<Game> {
  return invokeCmd("update_save_data", { id, saveData });
}

export async function setSaveDir(
  id: string,
  saveDir: string | null
): Promise<Game> {
  return invokeCmd("set_save_dir", { id, saveDir });
}

export async function configureAutoBackup(
  id: string,
  autoBackup: boolean,
  intervalMinutes: number,
  maxBackups: number
): Promise<Game> {
  return invokeCmd("configure_auto_backup", {
    id,
    autoBackup,
    intervalMinutes,
    maxBackups,
  });
}

export async function addGameBackup(id: string, backup: SaveBackup): Promise<Game> {
  return invokeCmd("add_game_backup", { id, backup });
}

export async function removeGameBackup(
  id: string,
  backupId: string
): Promise<Game> {
  return invokeCmd("remove_game_backup", { id, backupId });
}

export async function updateBackupNote(
  id: string,
  backupId: string,
  note: string | null
): Promise<Game> {
  return invokeCmd("update_backup_note", { id, backupId, note });
}

export async function configureCloudSync(
  id: string,
  cloudSync: boolean,
  cloudProvider: string | null
): Promise<Game> {
  return invokeCmd("configure_cloud_sync", { id, cloudSync, cloudProvider });
}

// ===== 启动 =====

export async function launchGame(
  id: string,
  forceLocaleJp?: boolean
): Promise<void> {
  const args: { id: string; forceLocaleJp?: boolean } = { id };
  if (forceLocaleJp !== undefined) args.forceLocaleJp = forceLocaleJp;
  return invokeCmd("launch_game", args);
}

// ===== 刮削 =====

export interface ScrapeSourceOptions {
  dlsite?: boolean;
  touchgal?: boolean;
  erogamescape?: boolean;
  ymgal?: boolean;
  kungal?: boolean;
  steam?: boolean;
  pcgw?: boolean;
}

export async function scrapeGames(
  query: string,
  vndb: boolean,
  bangumi: boolean,
  sources: ScrapeSourceOptions = {}
): Promise<ScrapeResponse> {
  return invokeCmd("scrape_games", { query, vndb, bangumi, ...sources });
}

export async function scrapeGame(
  query: string,
  vndb: boolean,
  bangumi: boolean,
  sources: ScrapeSourceOptions = {}
): Promise<ScrapeResponse> {
  return invokeCmd("scrape_game", { query, vndb, bangumi, ...sources });
}

export async function scrapeKungalDetail(gameId: string): Promise<ScrapeResult> {
  return invokeCmd("scrape_kungal_detail", { gameId });
}

export async function scrapeSteamApp(appId: string): Promise<ScrapeResult> {
  return invokeCmd("scrape_steam_app", { appId });
}

export async function scrapePcgwPage(title: string): Promise<ScrapeResult> {
  return invokeCmd("scrape_pcgw_page", { title });
}

/** 在系统默认浏览器中打开 URL */
export async function openUrl(url: string): Promise<void> {
  return invokeCmd("open_url", { url });
}

/** 在系统文件管理器中打开路径（文件夹/文件） */
export async function openPath(path: string): Promise<void> {
  return invokeCmd("open_path", { path });
}

/** 搜索 Galgame 下载资源（TouchGAL/Kungal） */
export async function searchGameDownloads(
  name: string,
  kungalId?: string,
  patchId?: string,
): Promise<DownloadSearchResult> {
  return invokeCmd("search_game_downloads", { name, kungalId, patchId });
}

/** 直搜 TouchGAL 下载资源（多候选名回退，不依赖 Kungal API） */
export async function searchDownloadsDirect(candidates: string[]): Promise<DownloadSearchResult> {
  return invokeCmd("search_downloads_direct", { candidates });
}

export interface DownloadEntry {
  label: string;
  url: string;
  type: DownloadKind;
  size?: string;
  note?: string;
  direct_download: boolean;
}

export type DownloadKind = "magnet" | "http" | "baidu_pan" | "one_drive" | "google_drive" | "patch" | "translation_patch" | "official_site" | "other";

export interface DownloadSearchResult {
  game_name: string;
  entries: DownloadEntry[];
  source: string;
  source_url?: string;
}

/** 根据刮削结果构建源站页面 URL */
export function buildSourceUrl(r: ScrapeResult): string | null {
  switch (r.source) {
    case "vndb":          return `https://vndb.org/${r.source_id}`;
    case "bangumi":       return `https://bgm.tv/subject/${r.source_id}`;
    case "steam":         return `https://store.steampowered.com/app/${r.source_id}`;
    case "dlsite":        return r.detail?.homepage ?? (r.source_id ? `https://www.dlsite.com/maniax/work/=/product_id/${r.source_id}.html` : null);
    case "erogamescape":  return `https://erogamescape.dyndns.org/~ap2/ero/toukei_kaiseki/game.php?game=${r.source_id}`;
    case "pcgw":          return `https://www.pcgamingwiki.com/wiki/${encodeURIComponent(r.title)}`;
    case "kungal":         return r.detail?.homepage ?? null;
    case "ymgal":          return r.detail?.homepage ?? null;
    case "touchgal":       return r.detail?.homepage ?? null;
    case "ai":             return null;
    default:               return r.detail?.homepage ?? null;
  }
}

export async function applyScrapeResult(
  gameId: string,
  result: ScrapeResult
): Promise<Game> {
  return invokeCmd("apply_scrape_result", { gameId, result });
}

/** 取某条搜索结果的全量详情（截图/开发商/发行商/流派/别名/发行日期等）。
 *  搜索只回浅层结果，落库前先用它把富字段补全。 */
export async function fetchFullDetail(
  source: string,
  sourceId: string
): Promise<ScrapeResult> {
  return invokeCmd("fetch_full_detail", { source, sourceId });
}

// ===== 存档（文件系统扫描） =====

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

export async function getNsfwDecision(
  gameId: string,
  mode?: NsfwDisplayMode
): Promise<NsfwDecision> {
  return invokeCmd("get_nsfw_decision", { gameId, mode });
}

export async function classifyNsfwGame(
  game: Game,
  mode: NsfwDisplayMode = "blur"
): Promise<NsfwDecision> {
  return invokeCmd("classify_nsfw_game", { game, mode });
}

export async function getGamesNsfwFiltered(mode?: NsfwDisplayMode): Promise<Game[]> {
  return invokeCmd("get_games_nsfw_filtered", { mode });
}

export async function updateNsfwDisplayMode(mode: NsfwDisplayMode): Promise<Settings> {
  return invokeCmd("update_nsfw_display_mode", { mode });
}

export async function translateScrapeMetadata(
  result: ScrapeResult,
  targetLanguage: string | null = null
): Promise<ChineseMeta> {
  return invokeCmd("translate_scrape_metadata", { result, targetLanguage });
}

export async function translateText(
  text: string,
  targetLanguage: string | null = null
): Promise<string> {
  return invokeCmd("translate_text", { text, targetLanguage });
}

export async function parseChineseMetadata(text: string): Promise<ChineseMeta> {
  return invokeCmd("parse_chinese_metadata", { text });
}

export async function embedChineseMetadata(
  text: string | null,
  meta: ChineseMeta
): Promise<string> {
  return invokeCmd("embed_chinese_metadata", { text, meta });
}

export async function stripMetadataMarkers(text: string): Promise<string> {
  return invokeCmd("strip_metadata_markers", { text });
}

export async function parseScrapeMarker(text: string): Promise<ScrapeMarker> {
  return invokeCmd("parse_scrape_marker", { text });
}

export async function embedScrapeMarker(
  text: string | null,
  source: string | null,
  metadataHash: string | null,
  coverImage: boolean,
  backgroundImage: boolean
): Promise<string> {
  return invokeCmd("embed_scrape_marker", {
    text,
    source,
    metadataHash,
    coverImage,
    backgroundImage,
  });
}

// ===== 设置 =====

export async function getSettings(): Promise<Settings> {
  return invokeCmd("get_settings");
}

export async function updateSettings(settings: Settings): Promise<Settings> {
  return invokeCmd("update_settings", { settings });
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

export async function getRecommendations(
  seedGameId: string | null = null,
  limit = 12
): Promise<Recommendation[]> {
  return invokeCmd("get_recommendations", { seedGameId, limit });
}

export async function getDashboardData(): Promise<DashboardData> {
  return invokeCmd("get_dashboard_data");
}

export async function getSmartCollections(): Promise<Collection[]> {
  return invokeCmd("get_smart_collections");
}

export async function getCollectionGames(collectionId: string): Promise<Game[]> {
  return invokeCmd("get_collection_games", { collectionId });
}

export async function cacheThumbnail(
  key: string,
  source: string
): Promise<ThumbnailInfo> {
  return invokeCmd("cache_thumbnail", { key, source });
}

export async function getThumbnail(key: string): Promise<ThumbnailInfo | null> {
  return invokeCmd("get_thumbnail", { key });
}

export async function clearThumbnailCache(): Promise<number> {
  return invokeCmd("clear_thumbnail_cache");
}

export async function enqueueTask(title: string, kind: string): Promise<AppTask> {
  return invokeCmd("enqueue_task", { title, kind });
}

export async function getTasks(): Promise<AppTask[]> {
  return invokeCmd("get_tasks");
}

export async function updateTask(
  id: string,
  status: TaskStatus | null = null,
  progress: number | null = null,
  message: string | null = null
): Promise<AppTask> {
  return invokeCmd("update_task", { id, status, progress, message });
}

export async function cancelTask(id: string): Promise<AppTask> {
  return invokeCmd("cancel_task", { id });
}

export async function clearFinishedTasks(): Promise<void> {
  return invokeCmd("clear_finished_tasks");
}

export async function getMigrationStatus(): Promise<MigrationInfo[]> {
  return invokeCmd("get_migration_status");
}

export async function exportDatabase(exportPath: string | null = null): Promise<string> {
  return invokeCmd("export_database", { exportPath });
}

export async function importDatabase(
  importPath: string,
  merge = true
): Promise<unknown> {
  return invokeCmd("import_database", { importPath, merge });
}

export async function scanImagesDir(dir: string): Promise<ImageCandidate[]> {
  return invokeCmd("scan_images_dir", { dir });
}

export async function scanGameImages(gameId: string): Promise<ImageCandidate[]> {
  return invokeCmd("scan_game_images", { gameId });
}

export async function getPerformanceSnapshot(): Promise<PerformanceSnapshot> {
  return invokeCmd("get_performance_snapshot");
}

export async function runDiagnostics(): Promise<DiagnosticsReport> {
  return invokeCmd("run_diagnostics");
}

// ===== 下载管理 =====

export async function downloadStart(
  url: string,
  filename: string,
  autoExtract = false,
  autoImport = false
): Promise<DownloadTask> {
  return invokeCmd("download_start", { url, filename, autoExtract, autoImport });
}

export async function downloadPause(taskId: string): Promise<void> {
  return invokeCmd("download_pause", { taskId });
}

export async function downloadResume(taskId: string): Promise<void> {
  return invokeCmd("download_resume", { taskId });
}

export async function downloadCancel(taskId: string): Promise<void> {
  return invokeCmd("download_cancel", { taskId });
}

export async function downloadRetry(taskId: string): Promise<void> {
  return invokeCmd("download_retry", { taskId });
}

export async function downloadRemove(taskId: string): Promise<void> {
  return invokeCmd("download_remove", { taskId });
}

export async function downloadClearFinished(): Promise<void> {
  return invokeCmd("download_clear_finished");
}

export async function getDownloads(): Promise<DownloadTask[]> {
  return invokeCmd("get_downloads");
}

export async function setDownloadSpeedLimit(bytesPerSec: number): Promise<void> {
  return invokeCmd("set_download_speed_limit", { bytesPerSec });
}

export async function getDownloadSpeedLimit(): Promise<number> {
  return invokeCmd("get_download_speed_limit");
}

// ===== 番剧下载管理 =====

export type AnimeDownloadStatus =
  | "Pending"
  | "Parsing"
  | "Downloading"
  | "Merging"
  | "Completed"
  | "Failed"
  | "Paused"
  | "Cancelled";

export interface AnimeDownloadTask {
  id: string;
  url: string;
  filename: string;
  output_path: string;
  status: AnimeDownloadStatus;
  progress: number;
  total_segments: number;
  downloaded_segments: number;
  total_size: number;
  downloaded_size: number;
  speed: number;
  error?: string;
  is_m3u8: boolean;
  anime_name?: string;
  episode_name?: string;
}

export async function animeDownloadEpisode(
  url: string,
  filename: string,
  outputDir?: string,
  animeName?: string,
  episodeName?: string,
  referer?: string
): Promise<AnimeDownloadTask> {
  return invokeCmd("anime_download_episode", {
    url,
    filename,
    outputDir,
    animeName,
    episodeName,
    referer,
  });
}

export async function animeGetDownloads(): Promise<AnimeDownloadTask[]> {
  return invokeCmd("anime_get_downloads");
}

export async function animeCancelDownload(downloadId: string): Promise<void> {
  return invokeCmd("anime_cancel_download", { downloadId });
}

export async function animePauseDownload(downloadId: string): Promise<void> {
  return invokeCmd("anime_pause_download", { downloadId });
}

export async function animeResumeDownload(downloadId: string): Promise<void> {
  return invokeCmd("anime_resume_download", { downloadId });
}

export async function animeRemoveDownload(downloadId: string): Promise<void> {
  return invokeCmd("anime_remove_download", { downloadId });
}

export async function animeClearFinishedDownloads(): Promise<void> {
  return invokeCmd("anime_clear_finished_downloads");
}

export async function animeOpenDownloadFolder(downloadId: string): Promise<void> {
  return invokeCmd("anime_open_download_folder", { downloadId });
}

// ===== 工具函数 =====

export function formatPlayTime(seconds: number): string {
  if (seconds === 0) return "未游玩";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/** 获取有效的评分（优先用户评分 → VNDB → Bangumi → 旧字段） */
export function effectiveRating(game: Game): number | null {
  return (
    game.play_tracker?.user_rating ??
    game.metadata?.vndb_rating ??
    game.metadata?.bangumi_rating ??
    game.rating ??
    null
  );
}

/** 获取有效的发行年份 */
export function effectiveReleaseYear(game: Game): number | null {
  return game.metadata?.release_year ?? game.release_year ?? null;
}

/** 获取有效的最后游玩时间 */
export function effectiveLastPlayed(game: Game): string | null {
  return game.play_tracker?.last_played ?? game.last_played ?? null;
}

/** 获取完成状态的中文描述 */
export function completionStatusLabel(status: CompletionStatus): string {
  const labels: Record<CompletionStatus, string> = {
    not_started: "未开始",
    playing: "游玩中",
    completed: "已通关",
    dropped: "已弃坑",
    on_hold: "搁置中",
    plan_to_play: "计划玩",
    replaying: "重温中",
  };
  return labels[status] ?? status;
}

// ===== M6 Steam 集成 =====

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

export async function resolveSteamId(input: string, apiKey?: string): Promise<SteamLoginResult> {
  return invokeCmd("resolve_steam_id", { input, apiKey });
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
  apiKey?: string,
): Promise<PlatformScanResult> {
  return invokeCmd("scan_platform_library", { source, mode, steamId, apiKey });
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

// ===== M1 C# 迁移桥 =====

export interface MigrationReport {
  total_found: number;
  imported: number;
  updated: number;
  skipped: number;
  media_copied: number;
  media_missing: number;
  errors: string[];
  duration_secs: number;
  source_label: string;
  source_ids: string[];
  backup_dir: string | null;
}

export interface MigrationVerifyReport {
  expected_count: number;
  actual_count: number;
  matched_count: number;
  missing_count: number;
  missing_ids: string[];
  count_match: boolean;
  with_cover: number;
  with_background: number;
  with_description: number;
  cover_rate: number;
  issues: string[];
}

/// 从 C# 旧版数据迁移
export async function migrateFromCsharp(sourcePath: string): Promise<MigrationReport> {
  return invokeCmd("migrate_from_csharp", { sourcePath });
}

/// 校验迁移结果
export async function verifyMigration(expectedCount: number): Promise<MigrationVerifyReport> {
  return invokeCmd("verify_migration", { expectedCount });
}

export async function verifyMigrationIds(expectedCount: number, sourceIds: string[]): Promise<MigrationVerifyReport> {
  return invokeCmd("verify_migration_ids", { expectedCount, sourceIds });
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
export async function steamResolveUrl(url: string, apiKey?: string): Promise<SteamLoginResult> {
  return invokeCmd("steam_resolve_url", { url, apiKey });
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
export async function steamFetchAndImport(steamId: string, apiKey: string): Promise<SteamOwnedGamesResponse> {
  return invokeCmd("steam_fetch_and_import", { steamId, apiKey });
}

/// 批量导入 Steam 全库游戏
export async function steamImportOwnedGames(games: SteamOwnedGame[]): Promise<SteamOwnedGamesResponse> {
  return invokeCmd("steam_import_owned_games", { games });
}

// ===== 模拟器检测与 ROM 导入 =====

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
