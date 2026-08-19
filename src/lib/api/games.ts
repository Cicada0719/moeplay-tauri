// api 域模块：games（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";
import type { CompletionStatus, GameAlias, Tag, GameMetadata, PlaySession, PlaySessionEntry, PlaytimeSummary, PlayTracker, SaveBackup, SaveData, Game } from "./types";

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

