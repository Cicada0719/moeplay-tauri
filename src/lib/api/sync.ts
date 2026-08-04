// 历史记录 WebDAV 同步 API 封装（task-05 / FR-09）——统一走 invokeCmd 以便测试注入 mock。

import { invokeCmd } from "./core";
import type { SyncResult, SyncStatus, WebDavConfig } from "../types/sync";

export function getSyncConfig(): Promise<WebDavConfig | null> {
  return invokeCmd<WebDavConfig | null>("get_sync_config");
}

export function getSyncStatus(): Promise<SyncStatus> {
  return invokeCmd<SyncStatus>("get_sync_status");
}

export function syncNow(): Promise<SyncResult> {
  return invokeCmd<SyncResult>("sync_now");
}

export function testWebdavConnection(cfg: WebDavConfig, password: string): Promise<void> {
  return invokeCmd<void>("test_webdav_connection", { cfg, password });
}

export function saveWebdavConfig(cfg: WebDavConfig, password: string | null): Promise<void> {
  return invokeCmd<void>("save_webdav_config", { cfg, password });
}

export function clearWebdavConfig(): Promise<void> {
  return invokeCmd<void>("clear_webdav_config");
}

/** 从同步错误载荷提取可展示文案（认证失败文案必须明确）。 */
export function syncErrorMessage(err: unknown): string {
  const payload = err as { kind?: string; message?: string } | null;
  if (payload && typeof payload.message === "string" && payload.message) {
    return payload.message;
  }
  return String(err ?? "");
}
