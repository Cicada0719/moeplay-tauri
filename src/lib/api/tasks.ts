// api 域模块：tasks（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";
import type { TaskStatus, AppTask, MigrationInfo, ImageCandidate, PerformanceSnapshot, DiagnosticsReport } from "./types";

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


export async function exportDiagnosticsZip(): Promise<string> {
  return invokeCmd("export_diagnostics_zip");
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

