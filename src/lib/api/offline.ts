import { invokeCmd } from "./core";

export type OfflineState = "queued" | "downloading" | "paused" | "failed" | "cancelled" | "complete";
export type OfflineResourceState = "pending" | "staging" | "complete" | "failed";
export type OfflineContentType = "manga" | "novel";

export interface OfflineResourceSpec { resourceKey: string; filename?: string; }
export interface OfflineChapterRequest { chapterId: string; order?: number; title?: string; resources?: OfflineResourceSpec[]; }
export interface OfflineEnqueueRequest {
  contentType: OfflineContentType;
  sourceId: string;
  contentId: string;
  title: string;
  chapters: OfflineChapterRequest[];
}
export interface OfflineResourceInput { resourceKey: string; bytesBase64?: string; filename?: string; }
export interface OfflineSupplyRequest { chapterKey: string; body?: string; resources?: OfflineResourceInput[]; }
export interface OfflineControlRequest { chapterKey: string; action: "pause" | "resume" | "retry" | "cancel" | "delete"; }
export interface OfflineResource { resourceKey: string; filename: string; bytes: number; state: OfflineResourceState; }
export interface OfflineChapter {
  offlineBundleId: string; offlineChapterKey: string; contentType: OfflineContentType;
  sourceId: string; contentId: string; chapterId: string; order: number; title: string;
  state: OfflineState; resourceState: OfflineResourceState; readable: boolean; bytes: number;
  resources: OfflineResource[]; savePath: string; stagingPath?: string; taskId?: string;
  error?: string; updatedAt: number;
}
export interface OfflineManifest { format: string; version: number; chapters: OfflineChapter[]; }
export interface OfflineChapterContent { chapter: OfflineChapter; body?: string; resourcePaths: string[]; }
export interface OfflineStats { chapterCount: number; completeCount: number; pendingCount: number; failedCount: number; bytes: number; }

export const offlineApi = {
  enqueue: (request: OfflineEnqueueRequest) => invokeCmd<{ chapters: OfflineChapter[] }>("offline_enqueue", { request }),
  supply: (request: OfflineSupplyRequest) => invokeCmd<OfflineChapter>("offline_supply_chapter", { request }),
  list: () => invokeCmd<OfflineChapter[]>("offline_list"),
  getChapter: (chapterKey: string) => invokeCmd<OfflineChapterContent>("offline_get_chapter", { chapterKey }),
  control: (request: OfflineControlRequest) => invokeCmd<OfflineChapter>("offline_control", { request }),
  stats: () => invokeCmd<OfflineStats>("offline_stats"),
};

export const offlineEnqueue = offlineApi.enqueue;
export const offlineSupplyChapter = offlineApi.supply;
export const offlineList = offlineApi.list;
export const offlineGetChapter = offlineApi.getChapter;
export const offlineControl = offlineApi.control;
export const offlineStats = offlineApi.stats;
