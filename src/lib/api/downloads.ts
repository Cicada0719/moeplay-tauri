// api 域模块：downloads（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";
import type { DownloadTask } from "./types";

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

