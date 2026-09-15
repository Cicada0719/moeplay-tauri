// 萌游 MoeGame · API 调用入口（Phase A 拆分类型与核心调用）

import type { CompletionStatus, TagCategory, TagSource, GamePlatform, StoreLink, GameAlias, Tag, GameMetadata, PlaySession, PlaySessionEntry, DailyPlaytime, MonthlyPlaytime, GamePlaytimeRank, PlaytimeSummary, PlayTracker, SaveBackup, SaveData, Game, ImportPreviewCandidate, ScrapeResult, ScrapeSourceStatus, ScrapeResponse, ScrapeDetail, SaveInfo, SaveCandidateDir, SaveSnapshot, SnapshotDiff, SaveConflict, CloudProvider, CloudSyncConfig, Settings, NsfwDisplayMode, NsfwDecision, ChineseMeta, ScrapeMarker, Recommendation, MonthActivity, Collection, DashboardData, ThumbnailInfo, TaskStatus, AppTask, MigrationInfo, ImageCandidate, PerformanceSnapshot, Severity, Issue, SystemInfo, AppInfo, DiagnosticsReport, DownloadStatus, DownloadTask } from "./types";
export type * from "./types";
export { secretDelete, secretSet, secretStatus } from "./secrets";

import { invokeCmd } from "./core";


export * from "./games";
export * from "./scraper";
export * from "./saves";
export * from "./metadata";
export * from "./settings";
export * from "./dashboard";
export * from "./tasks";
export * from "./downloads";
export * from "./format";
export * from "./platformImport";
export * from "./emulators";
export * from "./system";
export * from "./videoExtractor";
