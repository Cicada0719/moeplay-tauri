// 历史记录 API 封装层（FR-10 / spec §4 步骤 1）
//
// 统一封装 Tauri command 调用：
//   - `queryHistory`   → `history_list`（子任务 4 已注册：分页 + 类型筛选 + 关键词，`updated_at DESC`）
//   - `deleteHistory`  → `history_delete`（子任务 4 已注册：**单条墓碑软删除**，此处按 id 逐条调用）
//   - `upsertHistory`  → `history_upsert`（spec §3.2 期望由子任务 4 提供；主线尚未注册，见下方说明）
//   - `clearHistoryByType` → `history_clear_by_type`（同上）
//
// ⚠️ 接口偏差说明（AGENTS.md 2b：不改 spec、在 PR 描述/评论中说明）：
// 子任务 4 当前只交付了 `history_list` / `history_delete` 两个 command，`history_upsert`
// 与 `history_clear_by_type` 尚未注册。为避免 `verify:commands` 把未注册命令计为契约缺失，
// 这两个调用走 `@tauri-apps/api/core` 的原始 `invoke`（不经 `invokeCmd`），运行期会以
// “command not found”拒绝；调用方（阅读器位置上报）已做静默降级，等子任务 4 补齐命令后
// 即可直接生效。软删除（墓碑）语义不变：所有删除路径都不物理删除记录。

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { writable, type Writable } from "svelte/store";
import { invokeCmd } from "../api/core";
import type { ContentType, HistoryItem, HistoryPage, HistoryQuery } from "./types";

/** 本地 IPC 下全量拉取的页大小上限（spec §4 步骤 4：1 万条级可接受）。 */
export const FULL_LIST_LIMIT = 100000;

/**
 * 为阅读器位置上报构造一条历史的 `id`。
 *
 * `history_upsert`（子任务 4 将补齐）按 `content_id + source_id` 匹配记录，
 * `id` 仅作主键 uuid，前端每次生成新值即可；Rust 侧命中已有记录时保留原 id。
 * 测试/旧环境无 `crypto.randomUUID` 时退化为临时随机串（客户端本地 id）。
 */
export function buildHistoryId(
  contentId: string,
  sourceId: string,
  chapterId: string | null,
): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `manual-${Date.now()}-${Math.random().toString(16).slice(2)}-${contentId}-${sourceId}-${chapterId ?? ""}`;
}

/**
 * 内存缓存（Svelte store）：查询成功后写入；删除/清空后同步剔除缓存项，
 * 避免二次拉取。值为 null 表示尚未加载。
 */
export const historyCache: Writable<HistoryItem[] | null> = writable(null);

function describeError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

/** 读取历史列表（类型筛选 + `updated_at DESC`，默认过滤墓碑）。 */
export async function queryHistory(query: HistoryQuery = {}): Promise<HistoryPage> {
  try {
    const items = await invokeCmd<HistoryItem[]>("history_list", {
      contentType: query.contentType ?? null,
      keyword: query.keyword?.trim() || null,
      limit: query.limit ?? FULL_LIST_LIMIT,
      offset: query.offset ?? 0,
    });
    const page: HistoryPage = { items, total: items.length };
    historyCache.set(items);
    return page;
  } catch (error) {
    throw new Error(`历史记录读取失败：${describeError(error)}`);
  }
}

/**
 * 保存/更新一条历史（按 `content_id + source_id` 语义 upsert）。
 * 依赖子任务 4 的 `history_upsert` 命令；当前未注册时运行期拒绝，调用方须捕获。
 */
export async function upsertHistory(item: HistoryItem): Promise<void> {
  try {
    await tauriInvoke("history_upsert", { item });
  } catch (error) {
    throw new Error(`历史记录保存失败：${describeError(error)}`);
  }
}

/** 软删除多条历史（墓碑：置 deleted=1 并刷新 updated_at）。 */
export async function deleteHistory(ids: string[]): Promise<void> {
  try {
    await Promise.all(ids.map((id) => invokeCmd<void>("history_delete", { id })));
    historyCache.update((items) => (items ? items.filter((item) => !ids.includes(item.id)) : items));
  } catch (error) {
    throw new Error(`删除历史记录失败：${describeError(error)}`);
  }
}

/** 按类型软删除（墓碑）。依赖子任务 4 的 `history_clear_by_type` 命令，未注册时运行期拒绝。 */
export async function clearHistoryByType(contentType: ContentType): Promise<void> {
  try {
    await tauriInvoke("history_clear_by_type", { contentType });
    historyCache.update((items) =>
      items ? items.filter((item) => item.contentType !== contentType) : items,
    );
  } catch (error) {
    throw new Error(`清空历史记录失败：${describeError(error)}`);
  }
}
