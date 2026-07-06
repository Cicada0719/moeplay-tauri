// 萌游 MoeGame · API 类型定义（从 api/index.ts 拆分）


// ===== 枚举类型 =====

export type CompletionStatus =
  | "not_started"
  | "playing"
  | "completed"
  | "dropped"
  | "on_hold"
  | "plan_to_play"
  | "replaying";

export type TagCategory = "genre" | "theme" | "mood" | "feature" | "content" | "custom";

export type TagSource = "vndb" | "bangumi" | "steam" | "dlsite" | "ai" | "user";

export type GamePlatform = "pc" | "web" | "mobile" | "console" | "other";

// ===== 子模型 =====

export interface StoreLink {
  name: string;
  url: string;
  price?: string;
  currency?: string;
}

export interface GameAlias {
  name: string;
  language?: string;
  source?: string;
  is_primary: boolean;
}

export interface Tag {
  name: string;
  color?: string;
  category: TagCategory;
  source: TagSource;
}

export interface GameMetadata {
  developer?: string;
  publisher?: string;
  platform?: GamePlatform;
  engine?: string;
  genres: string[];
  languages: string[];
  voice_languages: string[];
  version?: string;
  original_name?: string;
  homepage?: string;
  developer_homepage?: string;
  stores: StoreLink[];
  age_rating?: string;
  series?: string;
  release_date?: string;
  release_year?: number;
  estimated_hours?: number;
  vndb_rating?: number;
  bangumi_rating?: number;
  vndb_id?: string;
  bangumi_id?: string;
  cover?: string;
  background?: string;
}

export interface PlaySession {
  id: string;
  start_time: string;
  end_time?: string;
  duration_seconds: number;
  notes?: string;
}

export interface PlaySessionEntry {
  game_id: string;
  game_name: string;
  session: PlaySession;
}

export interface DailyPlaytime {
  date: string;
  seconds: number;
  sessions: number;
}

export interface MonthlyPlaytime {
  month: string;
  seconds: number;
  sessions: number;
}

export interface GamePlaytimeRank {
  game_id: string;
  game_name: string;
  total_seconds: number;
  sessions: number;
  last_played?: string;
}

export interface PlaytimeSummary {
  total_seconds: number;
  session_count: number;
  play_days: number;
  average_session_seconds: number;
  daily: DailyPlaytime[];
  monthly: MonthlyPlaytime[];
  recent_sessions: PlaySessionEntry[];
  top_games: GamePlaytimeRank[];
}

export interface PlayTracker {
  total_seconds: number;
  sessions: PlaySession[];
  completion_status: CompletionStatus;
  last_played?: string;
  first_played?: string;
  user_rating?: number;
  review?: string;
  achievements_total: number;
  achievements_unlocked: number;
  finished: boolean;
  completion_count: number;
}

export interface SaveBackup {
  id: string;
  name: string;
  path: string;
  size: number;
  created: string;
  progress_note?: string;
  note?: string;
}

export interface SaveData {
  save_dir?: string;
  auto_backup: boolean;
  backup_interval_minutes: number;
  max_backups: number;
  backups: SaveBackup[];
  cloud_sync: boolean;
  cloud_provider?: string;
}

// ===== 核心模型 =====

export interface Game {
  id: string;
  name: string;
  exe_path: string;
  install_dir?: string;
  game_type?: string;
  library_source?: "steam" | "epic" | string;
  library_id?: string;
  launch_uri?: string;
  last_imported_at?: string;
  platform?: string;
  created_at: string;
  updated_at: string;
  description?: string;
  cover?: string;
  background?: string;
  icon?: string;
  screenshots: string[];
  favorite: boolean;
  hidden: boolean;
  tags: string[];
  genres?: string[];
  metadata: GameMetadata;
  play_tracker: PlayTracker;
  save_data: SaveData;
  aliases: GameAlias[];
  tag_entries: Tag[];
  // 向后兼容（已弃用）
  original_name?: string;
  developer?: string;
  publisher?: string;
  engine?: string;
  release_year?: number;
  rating?: number;
  last_played?: string;
  vndb_id?: string;
  bangumi_id?: string;
  play_time_seconds: number;
  add_date?: string;
}

export interface ScrapeResult {
  title: string;
  description?: string;
  cover?: string;
  background?: string;
  tags: string[];
  rating?: number;
  release_year?: number;
  source: string;
  source_id: string;
  detail?: ScrapeDetail;
}

export interface ScrapeSourceStatus {
  source: string;
  ok: boolean;
  count: number;
  error?: string;
}

export interface ScrapeResponse {
  results: ScrapeResult[];
  source_status: ScrapeSourceStatus[];
}

export interface ScrapeDetail {
  developer?: string;
  publisher?: string;
  original_name?: string;
  aliases: string[];
  genres: string[];
  homepage?: string;
  screenshots: string[];
  languages: string[];
  engine?: string;
  age_rating?: string;
  series?: string;
  release_date?: string;
  vndb_id?: string;
  bangumi_id?: string;
  dl_site_id?: string;
  voice_languages: string[];
}

export interface SaveInfo {
  name: string;
  path: string;
  size: number;
  created: string;
}

export interface SaveCandidateDir {
  path: string;
  category: string;
  score: number;
  write_count: number;
  last_write_time?: string;
  file_count: number;
  total_size_bytes: number;
  matched_rule: string;
}

export interface SaveSnapshot {
  id: string;
  file_path: string;
  file_name: string;
  created_at: string;
  file_size_bytes: number;
  note?: string;
  file_count: number;
}

export interface SnapshotDiff {
  added: string[];
  removed: string[];
  changed: string[];
  unchanged: number;
}

export interface SaveConflict {
  relative_path: string;
  local_path: string;
  remote_path: string;
  local_modified?: string;
  remote_modified?: string;
  local_size: number;
  remote_size: number;
  reason: string;
}

export type CloudProvider = "None" | "WebDAV" | "AliyunOSS" | "LocalFolder";

export interface CloudSyncConfig {
  enabled: boolean;
  provider: CloudProvider;
  server_url?: string;
  username?: string;
  password?: string;
  sync_directory: string;
  auto_sync: boolean;
  sync_interval_seconds: number;
}

export interface Settings {
  theme: string;
  watch_dirs: string[];
  auto_scrape: boolean;
  language: string;
  minimize_to_tray: boolean;
  vndb_enabled: boolean;
  bangumi_enabled: boolean;
  dlsite_enabled?: boolean;
  touchgal_enabled?: boolean;
  erogamescape_enabled?: boolean;
  ymgal_enabled?: boolean;
  kungal_enabled?: boolean;
  steam_enabled?: boolean;
  pcgw_enabled?: boolean;
  scraper_proxy?: string;
  ai_enabled: boolean;
  ai_api_url: string;
  ai_api_key: string;
  ai_model: string;
  nsfw_display_mode?: NsfwDisplayMode;
  steam_id?: string;
  steam_api_key?: string;
  autostart_enabled?: boolean;
  startup_mode?: string;
}

export type NsfwDisplayMode = "show" | "blur" | "hide";

export interface NsfwDecision {
  is_nsfw: boolean;
  display_mode: NsfwDisplayMode;
  should_show: boolean;
  should_blur: boolean;
  reasons: string[];
}

export interface ChineseMeta {
  name_cn?: string;
  desc_cn?: string;
}

export interface ScrapeMarker {
  scraped_at?: string;
  source?: string;
  metadata_hash?: string;
  cover_image: boolean;
  background_image: boolean;
}

export interface Recommendation {
  game_id: string;
  name: string;
  score: number;
  reasons: string[];
}

export interface MonthActivity {
  month: string;
  sessions: number;
  hours: number;
}

export interface Collection {
  id: string;
  name: string;
  description: string;
  game_count: number;
  icon: string;
}

export interface DashboardData {
  total_games: number;
  installed_games: number;
  completed_games: number;
  playtime_hours: number;
  completion_rate: number;
  scrape_coverage: number;
  disk_usage_gb: number;
  recent_games: string[];
  top_tags: [string, number][];
  completion_distribution: [string, number][];
  monthly_heatmap: MonthActivity[];
  collections: Collection[];
}

export interface ThumbnailInfo {
  key: string;
  source: string;
  path: string;
  size: number;
  cached: boolean;
}

export type TaskStatus = "pending" | "running" | "completed" | "failed" | "cancelled";

export interface AppTask {
  id: string;
  title: string;
  kind: string;
  status: TaskStatus;
  progress: number;
  created_at: string;
  updated_at: string;
  message?: string;
}

export interface MigrationInfo {
  version: number;
  description: string;
  applied: boolean;
}

export interface ImageCandidate {
  path: string;
  kind: string;
  score: number;
  size: number;
}

export interface PerformanceSnapshot {
  timestamp: number;
  game_count: number;
  database_size_bytes: number;
  cache_size_bytes: number;
  target_dir_size_bytes: number;
}

export type Severity = "Info" | "Warning" | "Error" | "Critical";

export interface Issue {
  severity: Severity;
  category: string;
  message: string;
  solution?: string;
}

export interface SystemInfo {
  os: string;
  arch: string;
  memory_gb: number;
  disk_free_gb: number;
  locale_emulator_installed: boolean;
}

export interface AppInfo {
  version: string;
  database_size_mb: number;
  game_count: number;
  scrape_sources: string[];
}

export interface DiagnosticsReport {
  system_info: SystemInfo;
  app_info: AppInfo;
  issues: Issue[];
  recommendations: string[];
}

export type DownloadStatus =
  | "Pending"
  | "Downloading"
  | "Paused"
  | "Completed"
  | "Failed"
  | "Extracting"
  | "Importing"
  | "Cancelled";

export interface DownloadTask {
  id: string;
  url: string;
  filename: string;
  save_path: string;
  total_size: number;
  downloaded_size: number;
  progress: number;
  speed: number;
  status: DownloadStatus;
  retry_count: number;
  max_retries: number;
  error?: string;
  auto_extract: boolean;
  auto_import: boolean;
  headers: Record<string, string>;
}

