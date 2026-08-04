// 历史记录 WebDAV 同步类型（task-05 / FR-09）——与 Rust `sync` 模块 serde 输出对齐。
// 注意：Rust 侧未启用 camelCase rename，故 wire 字段名为 snake_case。

export type SyncMode = "manual" | "auto";

/** WebDAV 配置（不含密码；密码只存 keyring）。 */
export interface WebDavConfig {
  base_url: string;
  username: string;
  mode: SyncMode;
  remote_dir: string;
}

/** 同步结果（spec §3.2）。 */
export interface SyncResult {
  uploaded: number;
  downloaded: number;
  conflicts: number;
  tombstones_purged: number;
  synced_at: number;
}

/** 同步状态快照（spec §3.6）。 */
export interface SyncStatus {
  configured: boolean;
  last_result: SyncResult | null;
  syncing: boolean;
}

/** SyncError 序列化载荷（`{ kind, message }`）。 */
export interface SyncErrorPayload {
  kind: string;
  message: string;
}
